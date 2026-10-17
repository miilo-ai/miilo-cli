# Candidate implementation and handoff

Candidate: 0.2.0. Responsibility: Public local adapters/engine, native compatibility executable, Node delegation and remote clients.

## Shared boundaries

SDK runtime is an opt-in feature in the existing Rust package (not a new native
binding forced into Python/TypeScript). CLI/CE pin SDK source commit
`1fccba94eb2c125408906d26c0ec688be9a68a4a`. Protocol stays 0.1. Local config 2
contains mode/local, private snapshotFile, sessions/groups/project/knownSessions;
no CE URL or credential. Legacy config 1 stays explicit CE publication mode.

One owner-controlled collector per configuration. Selected transcript paths are
canonical and fixed; replacement/truncation fails closed. Native file changes
wake authoritative rereads, with coalescing and a 5-second freshness heartbeat.
Local consumers reject caches older than 15 seconds. Remote consumers subscribe
before bootstrap, read fresh state after invalidations, and acquire a new baseline
after reconnect/history loss. Do not apply arbitrary event deltas over a snapshot.

## Validation

`cargo test --workspace --locked -j2`; clippy; npm tests/package; cargo-dist archive validation.

Fixture evidence is distinct from live behavior. Current-chat Codex discovery and
import were exercised, but no natural update arrived in the 10-second window.
Omarchy native service/import/repeat-import/future-group and rendering acceptance
belongs to the separate agent on the designated Omarchy test host. Record commands, source revision,
OS/architecture and outcome in `.specify/evidence`. Windows local transcript
observation is unsupported in this candidate; remote CLI remains supported.

## Publication

No new release/tag/registry submission is authorized by this file. Candidate
branches are `codex/standalone-runtime`. Missing optional publication credentials
must still produce “No release has been made”. AUR registration is externally
closed; do not script retries. Do not count source checks or skipped CI as a
qualified installed platform artifact. Native acceptance precedes owner release.
