# Local agent compatibility

Miilo imports only sessions you explicitly select. Discovery and collection do not
start or resume agents, call models, sign users in, or require a Miilo server.

| Source | Observation adapter | Boundary |
| --- | --- | --- |
| Codex | Existing session discovery and selected JSONL transcript | Linux and macOS |
| Cursor | Selected agent transcript | Linux and macOS |
| Claude Code | Selected main-session JSONL, assistant text and tool names | Transcript shape qualified against 2.1.300 on macOS |
| OpenCode | Local SQLite sessions, assistant text and tool names | SQLite schema qualified against 1.19.3 |
| Devin CLI | Local SQLite sessions and assistant text on the current branch | Schema qualified against 3000.11.92; local CLI only |
| Grok CLI | Selected `chat_history.jsonl`, visible assistant strings | Format 1, qualified against 1.0.60 |
| Hermes | Local CLI sessions and active assistant message rows in SQLite | Live replies and follow-up observation qualified against 0.22.2 on macOS |

Claude Desktop chat, Cowork and Devin Cloud sessions are not covered by these adapters. A local
Devin sign-in does not grant Miilo access to Devin Cloud sessions. The remote
Miilo endpoint workflow remains separate.

## Onboarding

Run `miilo local discover /path/to/project`, then `miilo local setup /path/to/project`
and choose the existing sessions to observe. The native launcher session picker
consumes the same discovery/import contract. Folder groups support Codex, Cursor,
Claude Code, OpenCode, Devin, Grok, Hermes, or all supported sources. Existing enrollments are preserved.

Default database locations follow `XDG_DATA_HOME`, or `~/.local/share`:

- OpenCode: `opencode/opencode.db`.
- Devin CLI: `devin/cli/sessions.db`.

Claude Code uses `~/.claude/projects/<project>/<session-id>.jsonl`, or the
`projects` directory under `CLAUDE_CONFIG_DIR`. Discovery reads bounded metadata
and labels sessions by project rather than exposing the first prompt. Nested
subagent transcripts are excluded. Start Claude Code with `claude` in a terminal;
the regular Claude Desktop chat experience does not create these CLI transcripts.

Claude's [transcript format is internal and may change](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored).
This adapter is version-qualified and observes appended public activity, not an
authoritative view of conversation branches after rewinds or compaction. Unknown
record/block types are ignored. No hooks, configuration edits or resumptions are
required for existing sessions.

Grok CLI uses `~/.grok/sessions/<workspace>/<session>/chat_history.jsonl`, or
`$GROK_HOME/sessions`. Discovery verifies format-1 metadata in `summary.json`.
Only assistant string records are observed; reasoning, system/user messages,
tool payloads and unfamiliar shapes are ignored. Its history has no per-message
timestamp, so bootstrap uses file modification time and later appends use receipt
time. Rewinds/truncations require explicit reconnection. This covers the local
[Grok CLI session store](https://docs.x.ai/build/features/sessions), not Grok web chat.

Hermes uses `$HERMES_HOME/state.db` or `~/.hermes/state.db`; `MIILO_HERMES_DB` may
select a nonstandard absolute database path. Set `HERMES_HOME` explicitly for a
named profile; Miilo does not read Hermes' active-profile/configuration files or
aggregate other profiles silently. The reader requires the published `sessions`
and `messages` columns, selects unarchived `cli` sessions and `active=1` assistant
content, and excludes reasoning, tool-call payloads, messaging channels and archived
generations. Missing tables fail closed without migrations. See the
[upstream storage contract](https://github.com/NousResearch/hermes-agent/blob/main/website/docs/developer-guide/session-storage.md).

Use `hermes` for an interactive CLI session. Hermes 0.22.2 saves top-level `-z`
and answer-and-exit invocations as `oneshot` sessions; these are outside this
adapter's `cli`-only enrollment scope. Its agent runtime also requires a model
endpoint serving at least 64,000 context tokens. Configure the actual server
capacity rather than increasing only the client's context setting. See the
[Hermes quickstart](https://hermes-agent.nousresearch.com/docs/getting-started/quickstart/).

For nonstandard storage, set `MIILO_OPENCODE_DB` or `MIILO_DEVIN_DB` to the absolute
path of the corresponding database before discovery/import. The collector stores
the selected canonical path and session ID, not credentials. Missing tools and
empty histories do not cause sessions to be invented or created.

## Privacy and behavior

Database access is read-only, uses parameterized session IDs, and never migrates
an agent database. Queries extract bounded public assistant text and tool names;
user/system messages, reasoning, tool arguments/results, authentication tables,
and raw model errors are excluded. Devin follows the current parent chain and
excludes abandoned branches. OpenCode API errors become a generic error summary.
Session titles and project paths remain visible discovery metadata. An agent may
automatically derive a title from a prompt; rename sensitive titles in that agent
before sharing Miilo screenshots or snapshots.

Historical messages bootstrap as idle. Repeated polling is not activity. SQLite
WAL changes wake fresh reads, with the existing collector heartbeat as a fallback.
These are inferred observations, not authoritative execution status or approval
notifications. Miilo offers no control actions for these local sessions.

Missing/hidden sessions, incompatible schemas, symlinks and replacement database
files fail closed. Reconnect explicitly after replacement or an incompatible
upgrade. Older OpenCode JSON-directory storage is not supported by this adapter.

## Validation scope

Adapter fixtures cover WAL writes, updated text, privacy exclusions, Devin branch
isolation, provider isolation, polling stability and invalid storage.

Native Linux acceptance on October 18, 2026 exercised discovery, explicit import
and snapshot publication using isolated profiles and one authorized test session
per tool. Devin CLI 3000.11.92 returned the expected assistant reply. OpenCode
1.19.3 created a real session, but its configured provider rejected inference with
HTTP 403; Miilo observed the normalized error. Source database and WAL bytes were
unchanged during observation. Neither adapter exposed control actions.

OpenCode on macOS subsequently completed a user-started inference request. Discovery,
explicit import and snapshot publication matched its visible assistant reply. A
user-sent follow-up changed the summary through a running observer without restarting
or reimporting the session. Neither snapshot exposed control actions. Model
availability is an agent configuration concern, separate from observation
compatibility. This validation does not qualify Devin Cloud support.

Claude Code 2.1.300 on macOS was validated against an existing user-started session:
discovery, explicit import and snapshot publication passed. A user-sent follow-up
appeared in a refreshed snapshot with a newer activity timestamp and changed
assistant summary, without control actions. Continuous appended/partial records,
unchanged polls, private-block exclusion and replacement protection passed fixture
tests. No Claude prompts were sent by Miilo or the validation tools.

Grok CLI 1.0.60 completed one authorized harmless test prompt with the expected
reply. Format-1 discovery/import and snapshot observation are validated separately
from inference. Partial appends, privacy exclusions and replacement handling have
fixture coverage.

Hermes 0.19.0 on Linux stopped before inference because no provider/model was
configured. Hermes 0.22.2 on macOS subsequently completed authorized harmless
prompts through a configured OpenAI-compatible endpoint serving 65,536 context
tokens. A real interactive `cli` session passed discovery, explicit import and
snapshot publication with an exact visible-reply match. A follow-up appeared
through the running observer without restarting or reimporting. Both turns
finished with zero tool calls; snapshots exposed no control actions. This
qualifies session observation, not tool execution or messaging-gateway sessions.
