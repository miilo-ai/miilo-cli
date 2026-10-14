import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:http";
import test from "node:test";
import { parseArguments, validateTarget } from "../dist/args.js";
import { MiiloClient } from "../dist/client.js";
import { decodeSse } from "../dist/sse.js";

test("parses the public command surface", () => {
  assert.equal(parseArguments(["doctor"]).command.kind, "doctor");
  assert.deepEqual(parseArguments(["work", "list", "--connection", "factory"]).command, { kind: "work-list", connection: "factory" });
  assert.deepEqual(validateTarget("factory/task-1"), ["factory", "task-1"]);
  assert.throws(() => validateTarget("task-1"), /connection/);
});

test("decodes resumable events and ignores heartbeats", async () => {
  async function* chunks() {
    yield Buffer.from(": heartbeat\n\nid: aggregate:2\nevent: miilo.event\ndata: {\"protocolVersion\":\"0.1\",\"eventId\":\"event-2\",\"providerId\":\"factory\",\"occurredAt\":\"2026-10-13T00:00:00Z\",\"kind\":\"updated\",\"summary\":\"changed\"}\n\n");
  }
  const events = [];
  for await (const event of decodeSse(chunks())) events.push(event);
  assert.equal(events.length, 1);
  assert.equal(events[0].streamCursor, "aggregate:2");
});

test("uses configured endpoints and bearer credentials", async (context) => {
  const server = createServer((request, response) => {
    assert.equal(request.headers.authorization, "Bearer scoped-token");
    response.setHeader("content-type", "application/json");
    response.end(JSON.stringify({ ok: true, service: "miilo", protocolVersion: "0.1", providers: 0, identity: true, storage: true }));
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  context.after(() => server.close());
  const address = server.address();
  const client = new MiiloClient(`http://127.0.0.1:${address.port}`, "scoped-token");
  assert.equal((await client.health()).protocolVersion, "0.1");
});
