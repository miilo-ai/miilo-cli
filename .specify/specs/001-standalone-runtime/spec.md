# Standalone runtime and optional hosting

Public local adapters and standalone engine/CLI; optional HTTP client remains.

## Contract

SDK keeps protocol 0.1 unchanged and adds reusable opt-in Rust runtime support. CLI owns public local readers, explicit import/groups and a miilo-local compatibility entrypoint. Local mode writes an atomic owner-only normalized ObservatorySnapshot cache and can print snapshots/grounded explanations; it never starts CE. Remote mode is explicit and remains authenticated.

Desktop local configuration uses version 2, mode="local", snapshotFile and selected sessions/groups. It contains no endpoint or bearer credential. Existing version-1 remote configurations remain supported. Snapshot files are runtime caches, not durable history; freshness/loss must be visible. The widget connects to this cache through a local consumer adapter and retains existing remote endpoints.

## Acceptance

Clean SDK/CLI checkouts build using their declared public dependencies. Existing selected transcripts update naturally with CE absent and no listener. Two providers with colliding IDs remain isolated. Bootstrap history is idle; disconnect is not healthy sleep. Safe grouping/future opt-in, cancellation, private-file bounds and local/CE semantic equivalence are tested. No unsupported actions are advertised.

## Coordination

Core agent owns SDK, CLI, CE and widget helper/config transport wiring. Omarchy agent owns native desktop rendering/interaction validation on the actual Linux host. See the Omarchy plugin repository handoff for detailed native checks. Use new candidate artifacts and report missing dependencies explicitly.
