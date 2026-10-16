import { closeSync, fstatSync, openSync, readSync } from "node:fs";
import { TextDecoder } from "node:util";

export function resolveCredential(token?: string, path?: string): string | undefined {
  if (token !== undefined && path !== undefined) throw new Error("choose either a token or a token file");
  let value = token;
  if (path !== undefined) {
    let descriptor: number | undefined;
    try {
      descriptor = openSync(path, "r");
      const stat = fstatSync(descriptor);
      if (!stat.isFile() || stat.size > 4096) throw new Error("invalid file");
      if (process.platform !== "win32" && (stat.mode & 0o077) !== 0) throw new Error("unsafe permissions");
      const buffer = Buffer.alloc(4097);
      const length = readSync(descriptor, buffer, 0, buffer.length, 0);
      value = new TextDecoder("utf-8", { fatal: true }).decode(buffer.subarray(0, length)).replace(/[\r\n]+$/, "");
    } catch { throw new Error("credential file must be readable, regular, private, and at most 4096 bytes"); }
    finally { if (descriptor !== undefined) closeSync(descriptor); }
  }
  if (value !== undefined && (!value || Buffer.byteLength(value) > 4096 || /\s/u.test(value))) throw new Error("credential must contain one nonempty token");
  return value;
}
