# Engineering constitution

- Hexagonal boundaries; small handwritten modules and explicit imports. Preserve upstream vendored licenses.
- Standalone SDK/CLI has no required CE process, Miilo URL/account, AWS or network listener.
- Provider credentials, consent and advertised action authority remain mandatory where applicable.
- Shared runtime owns observation/revisions/events; UI and hosting are adapters.
- Preserve other agents' changes; coordinate file ownership before concurrent edits.
- Test actual consumers, failure recovery and installed artifacts. Label fixture versus live evidence.
- No invented health/completion/approvals, secret logging, hidden transcript enrollment or silent backend fallback.
- Existing CE/managed clients remain available. Publication requires owner approval.
