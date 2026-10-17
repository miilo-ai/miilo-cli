import { decodeSse } from "./sse.js";
import type {
  ConnectionsResponse,
  ExplanationBundle,
  ExplanationRequest,
  HealthResponse,
  ObservatorySnapshot,
  StreamEvent,
} from "./types.js";

export class MiiloHttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(`Miilo returned HTTP ${status}: ${message}`);
  }
}

export class MiiloClient {
  readonly endpoint: URL;

  constructor(endpoint: string, private readonly token?: string) {
    this.endpoint = new URL(endpoint.endsWith("/") ? endpoint : `${endpoint}/`);
    if (this.endpoint.username || this.endpoint.password || this.endpoint.search || this.endpoint.hash) throw new Error("Miilo endpoint must not contain credentials, a query, or a fragment");
    if (!["http:", "https:"].includes(this.endpoint.protocol)) {
      throw new Error(`Miilo endpoint must use HTTP or HTTPS, not ${this.endpoint.protocol}`);
    }
  }

  health(): Promise<HealthResponse> {
    return this.getJson("health");
  }

  connections(): Promise<ConnectionsResponse> {
    return this.getJson("v1/connections");
  }

  snapshot(): Promise<ObservatorySnapshot> {
    return this.getJson("v1/snapshot");
  }

  explain(request: ExplanationRequest): Promise<ExplanationBundle> {
    return this.json("v1/explanations", { method: "POST", body: JSON.stringify(request) });
  }

  async *stream(cursor?: string): AsyncGenerator<StreamEvent> {
    const opened=await this.openStream(cursor);
    try {yield* opened.events;} finally {opened.cancel();}
  }

  async *observe(): AsyncGenerator<{snapshot:ObservatorySnapshot;reason:"bootstrap"|"event"}> {
    const opened=await this.openStream();
    try {
      yield {snapshot:await this.snapshot(),reason:"bootstrap"};
      for await(const event of opened.events) {
        if(event.type==="error") {
          if(["unauthorized","authentication_expired"].includes(event.reason))throw new MiiloHttpError(401,"Stream access expired");
          throw new Error("Stream requires a fresh snapshot and subscription");
        }
        yield {snapshot:await this.snapshot(),reason:"event"};
      }
    } finally {opened.cancel();await opened.events.return(undefined);}
  }

  private async openStream(cursor?:string):Promise<{events:AsyncGenerator<StreamEvent>;cancel:()=>void}> {
    const headers = this.headers();
    headers.set("accept", "text/event-stream");
    if (cursor) headers.set("last-event-id", cursor);
    const controller=new AbortController();
    const deadline=setTimeout(()=>controller.abort(),15_000);
    let response: Response;
    try { response=await fetch(new URL("v1/stream", this.endpoint),{headers,redirect:"error",signal:controller.signal}); }
    catch(error){clearTimeout(deadline);controller.abort();throw error;}
    try { await ensureSuccess(response, this.token); } catch(error){controller.abort();throw error;} finally{clearTimeout(deadline);}
    const contentType = response.headers.get("content-type") ?? "";
    if (!contentType.startsWith("text/event-stream")) {
      controller.abort();
      throw new Error(`Miilo returned ${contentType || "an unknown content type"} instead of text/event-stream`);
    }
    if (!response.body) {controller.abort();throw new Error("Miilo returned an empty event stream");}
    return {events:decodeSse(readBody(response.body,controller)),cancel:()=>controller.abort()};
  }

  private getJson<T>(path: string): Promise<T> {
    return this.json(path, { method: "GET" });
  }

  private async json<T>(path: string, init: RequestInit): Promise<T> {
    const headers = this.headers();
    headers.set("accept", "application/json");
    if (init.body) headers.set("content-type", "application/json");
    const response = await fetch(new URL(path, this.endpoint), { ...init, headers, redirect: "error", signal: AbortSignal.timeout(15_000) });
    await ensureSuccess(response, this.token);
    return JSON.parse(await boundedText(response, 2 * 1024 * 1024)) as T;
  }

  private headers(): Headers {
    const headers = new Headers();
    if (this.token) headers.set("authorization", `Bearer ${this.token}`);
    return headers;
  }
}

async function* readBody(body: ReadableStream<Uint8Array>, controller: AbortController): AsyncGenerator<Uint8Array> {
  const reader = body.getReader();
  try {
    while (true) {
      const idle=setTimeout(()=>controller.abort(),45_000);
      let result: ReadableStreamReadResult<Uint8Array>;
      try {result=await reader.read();} finally {clearTimeout(idle);}
      if (result.done) return;
      yield result.value;
    }
  } finally {
    await reader.cancel().catch(()=>undefined);
    reader.releaseLock();
    controller.abort();
  }
}

async function ensureSuccess(response: Response, token?: string): Promise<void> {
  if (response.ok) return;
  const message = (await boundedText(response,64*1024).catch(()=>"Remote error response unavailable or oversized")) || response.statusText || "unknown error";
  throw new MiiloHttpError(response.status, token ? message.replaceAll(token, "[redacted]") : message);
}

async function boundedText(response:Response,limit:number):Promise<string> {
 if(!response.body)return "";
 const reader=response.body.getReader();const chunks:Uint8Array[]=[];let length=0;
 try {while(true){const result=await reader.read();if(result.done)break;length+=result.value.length;if(length>limit)throw new Error("Miilo response exceeded the client size limit");chunks.push(result.value);}}
 finally {await reader.cancel().catch(()=>undefined);reader.releaseLock();}
 const bytes=new Uint8Array(length);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}return new TextDecoder().decode(bytes);
}
