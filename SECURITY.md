# Security policy

Report vulnerabilities through GitHub private security advisories for
`miilo-ai/miilo-cli`, not public issues.

The CLI accepts an API endpoint and optional bearer token. It must never print a
token, persist one without explicit user action, follow redirects to a different
authority with credentials attached, or execute provider-supplied content.
Reports involving credential disclosure, URL confusion, unbounded stream frames,
cursor replay, terminal escape injection, or action-policy bypass are especially
important.

Version `0.1` is pre-stable. Advisories will identify affected native and Node
implementations separately.
