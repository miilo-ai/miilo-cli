# Architecture boundary enforcement

> Retained design record. Unchecked historical milestones are not a current release checklist. See docs/release-status.md for current scope and outstanding validation.


Domain/application owns ports and policy; infrastructure implements those ports.
Executable/runtime composition chooses concrete implementations. Core use cases
must run with fake ports. Compatibility entry points remain explicit at the edge.
The constitution and architecture policy describe the enforced source scope.

The collector depends on ObservationSource, SnapshotSink and Clock ports. Group validation/matching/preview use FolderMetadata; future-enrollment policy receives explicit scope/time. Native discovery, scheduling, files and processes live in adapters. Eighteen Rust tests, twelve Node tests and the compiled CLI-to-widget update/explanation/stale-source fixture journey passed with CE absent.
