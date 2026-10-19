# Release scope and validation

Current published package: 0.3.0 (reviewed October 18, 2026).
For available assets and subsequent versions, use GitHub Releases and the package manifests.
Documentation-only changes do not require a package version bump.

## Supported scope

Local observation supports Codex, Claude Code, Cursor, OpenCode, Devin CLI,
Grok CLI and Hermes through the CLI. Consult the CLI compatibility matrix for
storage formats, tested tool versions and the distinction between import and live updates.
Local observation is read-only; provider actions require a separately connected provider.

## Outstanding work

- Voice capture/playback is deferred. The helper voice command deliberately returns
  `not_implemented`; endpoint configuration is not microphone or streaming-audio support.
- Full conversational chat-panel acceptance is distinct from the working model-backed
  Explain path. Do not mark all chat milestones complete based on explanation demos.
- Keep platform-specific acceptance explicit: physical Android, Windows local observation,
  fractional scaling, large fleets and hidden/reduced-motion performance need their own evidence.
- Marketplace approval and AUR availability require external confirmation; a public
  repository or GitHub release does not establish a store listing.
- The protected Homebrew tap needs a PR-compatible publishing flow. Formula CI and
  an existing published formula do not establish automatic future tap updates.

## Documentation authority

README and docs describe supported installation and operation. `.specify/specs`
retains design requirements; `.specify/evidence` records dated test results.
Old candidate plans and unchecked prelaunch checklists are historical, not current
release blockers or proof of missing implementation. Do not erase recorded test
limitations or treat screenshots as exhaustive platform certification.
