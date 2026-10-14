import type { ObservatoryEvent, StreamEvent } from "./types.js";

const MAX_FRAME_BYTES = 1024 * 1024;

export async function* decodeSse(
  source: AsyncIterable<Uint8Array>,
): AsyncGenerator<StreamEvent> {
  const decoder = new TextDecoder();
  let buffer = "";
  for await (const chunk of source) {
    buffer += decoder.decode(chunk, { stream: true });
    let boundary = frameBoundary(buffer);
    while (boundary !== undefined) {
      const frame = buffer.slice(0, boundary.index);
      buffer = buffer.slice(boundary.index + boundary.length);
      const event = decodeFrame(frame);
      if (event) yield event;
      boundary = frameBoundary(buffer);
    }
    if (Buffer.byteLength(buffer) > MAX_FRAME_BYTES) {
      throw new Error("Miilo event stream frame exceeded 1 MiB");
    }
  }
  buffer += decoder.decode();
  if (buffer.trim()) throw new Error("Miilo event stream ended with an incomplete frame");
}

function frameBoundary(value: string): { index: number; length: number } | undefined {
  const crlf = value.indexOf("\r\n\r\n");
  const lf = value.indexOf("\n\n");
  if (crlf < 0 && lf < 0) return undefined;
  if (crlf >= 0 && (lf < 0 || crlf <= lf)) return { index: crlf, length: 4 };
  return { index: lf, length: 2 };
}

function decodeFrame(frame: string): StreamEvent | undefined {
  let id: string | undefined;
  let kind = "message";
  const data: string[] = [];
  for (const line of frame.split(/\r?\n/u)) {
    if (line.startsWith(":")) continue;
    const separator = line.indexOf(":");
    const field = separator < 0 ? line : line.slice(0, separator);
    const raw = separator < 0 ? "" : line.slice(separator + 1);
    const value = raw.startsWith(" ") ? raw.slice(1) : raw;
    if (field === "id") id = value;
    if (field === "event") kind = value;
    if (field === "data") data.push(value);
  }
  if (data.length === 0) return undefined;
  const payload: unknown = JSON.parse(data.join("\n"));
  if (kind === "miilo.event") {
    if (!id) throw new Error("Miilo event stream frame is missing its resumable ID");
    return { type: "event", streamCursor: id, event: payload as ObservatoryEvent };
  }
  if (kind === "miilo.error") {
    const error = payload as { reason: string; message: string };
    return { type: "error", reason: error.reason, message: error.message };
  }
  return undefined;
}
