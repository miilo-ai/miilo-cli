# Candidate dependency map

```text
SDK contracts + optional Rust runtime
  ├─ public CLI local adapters → one owned collector → private cache → robots
  └─ CE memory/HTTP/auth adapters → optional remote clients
```

Runtime source pins are recorded in each consuming Cargo manifest and lockfile.
Candidate versions: SDK/CLI/CE 0.2.0, widget helper 1.4.0; protocol 0.1.
All coordinated work is on codex/standalone-runtime. Exact tested source revisions
and artifact digests are recorded in repository evidence; never infer them from
a version string alone before release.

Local adapters/public CLI own file discovery, explicit consent, scoped groups,
creation-time-qualified future enrollment, native notifications and collector
lifecycle. SDK owns domain validation, aggregation, deduplication, bounded replay
and grounded explanation. Widgets own rendering and thin file/HTTP transports.
CE owns optional HTTP/auth/event broadcasting. Homebrew/AUR consume qualified
artifacts, not application logic.

Local transcript sources expose no approvals. CE remains ephemeral. The initial
native local adapters are Codex/Cursor; broader provider coverage is separate work. Provider authors can use SDK ports/core independently; broader
CLI provider plugins remain a separately scoped extension.
