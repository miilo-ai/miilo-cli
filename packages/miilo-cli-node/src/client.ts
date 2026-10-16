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
    const headers = this.headers();
    headers.set("accept", "text/event-stream");
    if (cursor) headers.set("last-event-id", cursor);
    const response = await fetch(new URL("v1/stream", this.endpoint), { headers, redirect: "error" });
    await ensureSuccess(response, this.token);
    const contentType = response.headers.get("content-type") ?? "";
    if (!contentType.startsWith("text/event-stream")) {
      throw new Error(`Miilo returned ${contentType || "an unknown content type"} instead of text/event-stream`);
    }
    if (!response.body) throw new Error("Miilo returned an empty event stream");
    yield* decodeSse(readBody(response.body));
  }

  private getJson<T>(path: string): Promise<T> {
    return this.json(path, { method: "GET" });
  }

  private async json<T>(path: string, init: RequestInit): Promise<T> {
    const headers = this.headers();
    headers.set("accept", "application/json");
    if (init.body) headers.set("content-type", "application/json");
    const response = await fetch(new URL(path, this.endpoint), { ...init, headers, redirect: "error" });
    await ensureSuccess(response, this.token);
    return (await response.json()) as T;
  }

  private headers(): Headers {
    const headers = new Headers();
    if (this.token) headers.set("authorization", `Bearer ${this.token}`);
    return headers;
  }
}

async function* readBody(body: ReadableStream<Uint8Array>): AsyncGenerator<Uint8Array> {
  const reader = body.getReader();
  try {
    while (true) {
      const result = await reader.read();
      if (result.done) return;
      yield result.value;
    }
  } finally {
    reader.releaseLock();
  }
}

async function ensureSuccess(response: Response, token?: string): Promise<void> {
  if (response.ok) return;
  const message = (await response.text()) || response.statusText || "unknown error";
  throw new MiiloHttpError(response.status, token ? message.replaceAll(token, "[redacted]") : message);
}
