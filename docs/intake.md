# Incoming tasks

This feature is under development on the task-intake branch; it is not part of
the published 0.5.0 binary yet. The SDK, CLI and plugin changes must be delivered
together with compatible dependency pins.

Miilo supports three integration paths:

1. Connect an existing Miilo-compatible intake. That backend owns source discovery,
   authorization and workflow policy; Miilo presents its advertised decisions.
2. Run the optional manual inbox below. No Jira, GitHub board, factory, cloud
   account or model connection is needed to collect and decide tasks.
3. Bridge a source into the manual inbox and configure your own executor adapter.
   Source discovery and execution remain independent. Enabling observation alone
   must never start agent work.

## Manual inbox

Choose a private state directory outside your repository. Use a distinct directory
for each independent inbox. It receives a durable instance identity so two inboxes
cannot accidentally reuse the same executor job identity.

```sh
export MIILO_INTAKE_STATE="$HOME/.local/state/miilo/intake"
miilo inbox --state "$MIILO_INTAKE_STATE" import task.json
miilo inbox --state "$MIILO_INTAKE_STATE" list
```

Example `task.json`:

```json
{
  "id": "release-42",
  "title": "Prepare a release candidate",
  "summary": "Run the registered release checks and prepare the change for review.",
  "revision": "source-r1",
  "workflowId": "release"
}
```

Import does not run anything. Reimporting an identical source revision is a no-op;
reusing a revision for different content is rejected. Material source changes
invalidate an unaccepted decision. Accepted work cannot be silently replaced.
The initial reference store is bounded to 64 items and 10,000 receipts; reaching
capacity fails explicitly rather than discarding old authorization history.

Use the revision returned by `list`, not the source revision from the file:

```sh
miilo inbox --state "$MIILO_INTAKE_STATE" decide manual-intake/release-42   --action accept --revision source-r1:1 --request-id decision-42   --confirmed-at '2026-12-01T12:00:00Z'
```

Replace the example timestamp with your actual current RFC3339 UTC timestamp.
The confirmation must be current for a new decision. For an uncertain retry,
reuse the entire original command, including request ID, revision and timestamp.
The original receipt is returned even after that timestamp expires. Changed
input under the same request or idempotency identity is rejected.

`--action defer --until <future RFC3339 timestamp>` stores a durable reminder.
`--action dismiss` hides a candidate decision without closing its source ticket.
Neither operation cancels accepted execution. Silence does nothing.

Acceptance records intent. Without an executor, it reports
`queued_awaiting_executor`; it does not pretend an agent ran.

## Connect a client

Create a random bearer credential in a private file. For example, with OpenSSL:

```sh
umask 077
mkdir -p "$HOME/.config/miilo"
openssl rand -hex 32 > "$HOME/.config/miilo/intake-token"
miilo inbox --state "$MIILO_INTAKE_STATE" serve   --bind 127.0.0.1:3710 --access-token-file "$HOME/.config/miilo/intake-token"
```

The reference listener is loopback-only. It does not set up public ingress,
remote access or TLS for you. Its token authorizes reading/importing/deciding
this inbox; do not hand it to a read-only observer.

```sh
miilo --endpoint http://127.0.0.1:3710   --token-file "$HOME/.config/miilo/intake-token" inbox list
miilo --endpoint http://127.0.0.1:3710   --token-file "$HOME/.config/miilo/intake-token" watch
```

In Miilo One's menu choose Inbox. Under Settings → Intake, configure the endpoint
and credential-file path. The plugin saves that file reference, never its token
contents. An older CLI will reject the new command; install compatible builds
before enabling the new interface.

The API provides authenticated health/connections, `/v1/snapshot`,
`POST /v1/actions`, `POST /v1/intake/import` and `/v1/stream`. Events invalidate
cached observations; clients reread authoritative state. A cursor that cannot
be resumed yields HTTP 409 so the client can take a fresh snapshot. It does not
claim to retain a historical replay log. Polling the local projection is separate
from how frequently your source adapter checks Jira or another upstream service.

## Bring an executor

Executors are registered separately from the source. A local configuration file
contains only a URL, a credential-file reference and allowed workflow IDs:

```json
{
  "version": 1,
  "endpoint": "https://automation.example.test/",
  "tokenFile": "/absolute/path/to/private/executor-token",
  "workflows": ["release"]
}
```

```sh
miilo inbox --state "$MIILO_INTAKE_STATE" dispatch --config executor.json
```

Dispatch is explicit. `serve` does not silently activate it. A scheduler may run
this command at the desired reconciliation interval. No shell command or repo
path comes from task text. The workflow ID resolves to an executor-owned,
registered repository, worker policy, checks, review gates and budgets.
An unregistered workflow is not dispatched.

The adapter posts to `<endpoint>/v1/intake/jobs`, with a bearer credential and
Idempotency-Key equal to the frozen jobId. Request fields are version, jobId,
namespace, itemId, sourceRevision, workflowId, title and summary. The namespace
is an identity aid, not authorization: bind the credential to allowed scopes
and enforce policy at the executor.

The executor must durably bind each jobId to its exact first request, reject
changed payloads and return its current report for repeated requests. A report
contains jobId, execution, summary and optional HTTPS prUrl. Permitted execution
states are queued/running/checking/ready_for_review/completed/blocked/cancelled.
It must only report ready_for_review after its registered verification and
independent review gates pass. Miilo does not infer those facts from a PR existing.

The inbox journals the exact outbound request before sending. A timeout leaves
its outcome uncertain and retryable under the same identity; it does not invent
a new job. The executor owns admission limits and bounded repairs. Nothing in
this reference adapter authorizes automatic merge or deployment.

## Source adapters

For a custom source, implement a small bridge that maps its stable identity,
content revision and display text into Submission. Use an explicitly enrolled
workflow ID and perform an authoritative source read before applying an owner
decision to mutable upstream work. The manual reference adapter cannot itself
check Jira assignment or transition Jira; those belong to the source adapter.

For bidirectional integrations, read the actual permitted transitions and
map semantic workflow steps to native IDs. Reread before writing; an existing
automation may already have changed the state or attached the PR link. Failed
source synchronization must be visible and may block execution when the
configured workflow requires it. Never put source tokens in task descriptions,
model context, public PR text or SDK metadata.

## Architecture and validation limits

`miilo-intake` separates domain, application, persistence, protocol projection,
HTTP listener and executor HTTP adapter. The application uses an Executor port;
its tests cover uncertain submission and restart with an idempotent fake backend.
CLI integration tests additionally start the real authenticated loopback server
and observe import/decision changes through the actual Miilo HTTP/SSE client.
Neither is a substitute for qualification against your real execution backend,
source permissions, native shell or physical microphone.

## Owner-only local transport

On Unix, a private socket avoids provisioning a second bearer credential for
local clients. Create an owner-only directory, then:

```sh
mkdir -p "$HOME/.local/state/miilo/intake-api"
chmod 700 "$HOME/.local/state/miilo/intake-api"
miilo inbox --state "$MIILO_INTAKE_STATE" serve \
  --listen-socket "$HOME/.local/state/miilo/intake-api/service.sock"
miilo --socket "$HOME/.local/state/miilo/intake-api/service.sock" inbox list
```

The server refuses an existing socket path and insecure parent directories.
The client requires a socket owned by the current user with mode 600. Unix
socket mode deliberately uses OS identity instead of ambient HTTP credentials.
For a remote private runtime, a deliberately configured SSH Unix-socket forward
can retain that boundary without copying the remote service credentials to the
client. Enrollment and SSH host authorization remain explicit operator actions.
