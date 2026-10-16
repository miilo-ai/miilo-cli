import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:http";
import test from "node:test";
import { parseArguments, validateTarget } from "../dist/args.js";
import { MiiloClient } from "../dist/client.js";
import { decodeSse } from "../dist/sse.js";

async function collect(source) {
  const values = [];
  for await (const value of source) values.push(value);
  return values;
}

test("parses the public command surface", () => {
  assert.deepEqual(parseArguments(["--help"]).command, { kind: "help" });
  assert.equal(parseArguments(["--endpoint", "http://localhost:9999", "doctor"]).endpoint, "http://localhost:9999");
  assert.equal(parseArguments(["--url", "http://localhost:9998", "doctor"]).endpoint, "http://localhost:9998");
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

test("drains complete frames before applying the frame size limit", async () => {
  async function* chunks() {
    const comment = `: ${"x".repeat(600_000)}\n\n`;
    yield Buffer.from(comment + comment);
  }
  assert.deepEqual(await collect(decodeSse(chunks())), []);
});

test("rejects unsafe or incomplete event stream frames", async () => {
  async function* oversized() {
    yield Buffer.from(`data: ${"x".repeat(1024 * 1024)}`);
  }
  async function* incomplete() {
    yield Buffer.from("data: {}");
  }
  async function* missingCursor() {
    yield Buffer.from("event: miilo.event\ndata: {}\n\n");
  }
  await assert.rejects(() => collect(decodeSse(oversized())), /exceeded 1 MiB/);
  await assert.rejects(() => collect(decodeSse(incomplete())), /incomplete frame/);
  await assert.rejects(() => collect(decodeSse(missingCursor())), /resumable ID/);
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

test("rejects non-http endpoints and preserves HTTP error evidence", async (context) => {
  assert.throws(() => new MiiloClient("file:///tmp/miilo"), /HTTP or HTTPS/);
  const server = createServer((_request, response) => {
    response.statusCode = 409;
    response.end("snapshot required");
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  context.after(() => server.close());
  const address = server.address();
  const client = new MiiloClient(`http://127.0.0.1:${address.port}`);
  await assert.rejects(() => client.snapshot(), /HTTP 409: snapshot required/);
});

test("remote error bodies redact credentials and redirects are not followed", async (context) => {
  let count = 0;
  const server = createServer((request, response) => {
    count++;
    if (request.url === '/v1/snapshot') {
      response.statusCode = 401;
      response.end('Rejected private-test-token: Bearer private-test-token');
    } else {
      response.statusCode = 302;
      response.setHeader('location', '/should-not-receive-credentials');
      response.end();
    }
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  context.after(() => server.close());
  const client = new MiiloClient(`http://127.0.0.1:${server.address().port}`, 'private-test-token');
  await assert.rejects(() => client.snapshot(), error => {
    assert.equal(error.status, 401);
    assert.ok(!error.message.includes('private-test-token'));
    assert.match(error.message, /redacted/);
    return true;
  });
  await assert.rejects(() => client.health());
  assert.equal(count, 2, 'redirect target must never receive a request');
});
