#!/usr/bin/env python3
"""Small deterministic Miilo API used only by local and CI smoke tests."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os


SNAPSHOT = {
    "protocolVersion": "0.1",
    "snapshotId": "smoke-1",
    "observedAt": "2026-10-13T00:00:00Z",
    "cursor": "smoke:1",
    "providers": [{
        "id": "factory", "name": "Fixture factory",
        "kind": "automation_factory", "health": "healthy", "nativeUrl": None,
        "capabilities": {"observe": ["tasks"], "control": [], "features": {}},
    }],
    "workItems": [{
        "id": "task-1", "providerId": "factory", "kind": "task",
        "title": "Validate the observer", "summary": "The fixture is ready.",
        "lifecycle": "waiting", "phase": "reviewing", "attention": "approval",
        "health": "healthy", "providerState": "ready", "revision": "fixture:1",
        "dependencyIds": [], "actions": [], "updatedAt": "2026-10-13T00:00:00Z",
    }],
    "actors": [], "attempts": [], "artifacts": [], "evidence": [], "attention": [],
}


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.headers.get("authorization") != "Bearer smoke-token":
            return self.reply(401, {"message": "missing token"})
        if self.path == "/health":
            return self.reply(200, {"ok": True, "service": "miilo", "protocolVersion": "0.1", "providers": 1, "identity": True, "storage": True})
        if self.path == "/v1/connections":
            return self.reply(200, {"connections": [{"id": "factory", "name": "Fixture factory", "adapter": "fixture", "enabled": True, "credentialConfigured": True}]})
        if self.path == "/v1/snapshot":
            return self.reply(200, SNAPSHOT)
        self.reply(404, {"message": "not found"})

    def do_POST(self):
        if self.headers.get("authorization") != "Bearer smoke-token":
            return self.reply(401, {"message": "missing token"})
        if self.path != "/v1/explanations":
            return self.reply(404, {"message": "not found"})
        length = int(self.headers.get("content-length", "0"))
        request = json.loads(self.rfile.read(length))
        if request.get("expectedRevision") != "fixture:1":
            return self.reply(409, {"message": "revision changed"})
        self.reply(200, {
            "protocolVersion": "0.1", "explanationId": "explanation-1",
            "providerId": "factory", "workItemId": "task-1",
            "title": "Validate the observer", "revision": "fixture:1",
            "generatedAt": "2026-10-13T00:00:00Z", "observedAt": "2026-10-13T00:00:00Z",
            "condition": "waiting", "cause": "The fixture is ready.",
            "impact": "A person can review the output.", "recommendation": "Review it.",
            "spokenSummary": "The fixture is ready for review.", "confidence": "supported",
            "evidence": [], "changes": [], "actions": [], "uncertainty": [], "sourceIds": ["task-1"],
        })

    def reply(self, status, value):
        payload = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_args):
        return


if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", int(os.environ.get("MIILO_MOCK_PORT", "38765"))), Handler).serve_forever()
