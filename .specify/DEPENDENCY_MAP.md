# Candidate dependency map

```text
SDK contracts + optional Rust runtime
  ├─ public CLI local adapters → one owned collector → private cache → robots
  ├─ CE memory/HTTP/auth adapters → optional remote clients
  └─ managed aggregation → existing web/mobile, voice and trusted GenUI
```

Runtime source pin: SDK 1fccba94eb2c125408906d26c0ec688be9a68a4a.
Candidate versions: SDK/CLI/CE 0.2.0, widget helper 1.4.0; protocol 0.1.
All coordinated work is on codex/standalone-runtime. Exact tested source revisions
and artifact digests are recorded in repository evidence; never infer them from
a version string alone before release.

Local adapters/public CLI own file discovery, explicit consent, scoped groups,
creation-time-qualified future enrollment, native notifications and collector
lifecycle. SDK owns domain validation, aggregation, deduplication, bounded replay
and grounded explanation. Widgets own rendering and thin file/HTTP transports.
CE owns optional HTTP/auth/event broadcasting; managed product owns accounts,
tenants, durable storage and its existing voice/GenUI composition. Homebrew/AUR
consume qualified artifacts, not application logic.

Local transcript sources expose no approvals. CE remains ephemeral and does not
gain managed account/voice/control API parity through this extraction. Existing
managed features are preserved; paid/mobile/glasses work is not an open-source
release blocker. The first native local adapters are Codex/Cursor, not every
custom factory. Provider authors can use SDK ports/core independently; broader
CLI provider plugins remain a separately scoped extension.
