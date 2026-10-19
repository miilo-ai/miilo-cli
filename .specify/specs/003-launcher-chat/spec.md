# Chat-first launcher experience

> Retained design record. Unchecked historical milestones are not a current release checklist. See docs/release-status.md for current scope and outstanding validation.


Status: in progress, 2026-10-17. No release or native acceptance is implied.

## Goal and boundaries

One conversational experience for Omarchy and Omalauncher, grounded in the
user's explicitly enrolled systems. Typed input is the release target. Voice is
a follow-up and must not block chat, observation, evidence or existing actions.

SDK owns versioned request/event/reference contracts and shared fixtures in Rust,
Python and TypeScript. It owns neither credentials nor model HTTP transport.
CLI owns discovery/enrollment/observation; a future chat entry forwards to the
same helper rather than implementing another intelligence stack.
Widgets owns the shared OWL presentation and native helper: private chat config,
fresh context assembly through ports, OpenAI-compatible HTTP/SSE adapter and
typed NDJSON output. Plugin owns native shell presentation/lifecycle and consumes
the shared helper. No inference engine or credential belongs in QML/OWL.
Optional CE/hosted adapters must not become prerequisites for local chat.

## Interaction and privacy

Opening chat on a workflow binds it to its opaque work key and revision key.
Changing target starts a new conversation; ambiguous references ask for clarity.
Each turn refreshes observations and rejects stale selected revisions before
inference. Global turns disclose partial/unavailable sources and bounded context.
Only normalized titles/summaries/status and linked evidence summaries are sent;
exclude raw transcripts, change contents, credentials, dedicated path/URI fields
and extension blobs. These summaries can still be sensitive: explicit endpoint/context consent
is required. No implicit discovery or additional enrollment. No durable chat log.

Endpoint configuration: OpenAI-compatible base URL ending in /v1, model,
optional private credential file, explicit permission to send normalized context.
Require HTTPS except loopback. Redirects disabled; bound input/context/output,
connection/turn time and SSE frame size. Never echo remote bodies or secrets.
Configuration preview and connection validation are distinct from saving consent.

Chat may explain and suggest; it cannot execute tools or issue approvals. Existing
actions retain provider authority, exact revisions and explicit confirmation.
Treat observation text as untrusted evidence, never as system instructions.
No Jev, RFDT or policy-library dependency is required for this milestone.

## Milestones and evidence

- [x] M1: SDK request/context/reference/stream contracts and negative fixtures.
- [x] M2: shared helper config + fresh-context application + bounded streaming
      adapter; fake-port and loopback HTTP tests; terminal NDJSON smoke.
- [ ] M3: shared OWL chat panel, setup access, session-only conversation, streamed
      text, supporting references, cancel/retry and selected workflow retention.
- [ ] M4: native plugin integration and Omalauncher acceptance on actual hosts:
      scrolling, keyboard/touch, themes, focus/Escape, many robots and close/reopen.
- [ ] M5: actual selected Gemma endpoint proof: exact model/revision, latency,
      grounded answers, unavailable/empty/truncated errors; record sanitized demo.

M1/M2 portable evidence is not M3/M4 UI acceptance or M5 live model evidence.
Conversation routes initially explain current work/status. Historical semantic
search is a separate capability; never claim absent history was searched.

## Definition of done

On both launcher hosts: open a robot/workflow, ask a question, see streaming text
and evidence, retain target, recover from outage, cancel without late output and
observe subsequent state updates. Stale/ambiguous targets cannot silently change.
All changed contract/core/adapter regressions and architecture gates pass; native
versions/screenshots and live inference evidence recorded. Release gates remain.

## First checkpoint evidence

SDK contracts passed Rust, Python, TypeScript and JSON Schema checks. Helper core,
HTTP/SSE and compiled-command tests use fake or loopback models, including a
private local observation file with CE absent. This is not actual Gemma inference.
Graphical chat (M3), native acceptance (M4) and live model proof (M5) remain pending.
