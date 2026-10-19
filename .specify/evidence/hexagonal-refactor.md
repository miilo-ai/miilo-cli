# Architecture boundary checkpoint

> Historical validation record. Results apply only to the revision and environment recorded below; they do not describe current release readiness.


Domain/application code depends on explicit ports; executable/runtime adapters
compose concrete infrastructure. `.specify/architecture.json` and its CI workflow
check inward dependencies and module sizes. Negative tests prove reversed imports
and silent size growth are rejected. Empty layer matches fail instead of passing.

Behavioral tests and native consumer qualification remain separate from these
lexical source checks. This checkpoint does not certify release, native UI, hosted
identity or every uninspected file. No release has been made.

## Implemented boundary

The collector depends on ObservationSource, SnapshotSink and Clock ports. Group validation/matching/preview use FolderMetadata; future-enrollment policy receives explicit scope/time. Native discovery, scheduling, files and processes live in adapters. Eighteen Rust tests, twelve Node tests and the compiled CLI-to-widget update/explanation/stale-source fixture journey passed with CE absent.
