# Miilo CLI for Node.js

The Node.js implementation of the Miilo observer CLI. It exposes the same
read-only commands as the native Rust binary and works with managed or
self-hosted Miilo APIs.

```bash
npm install --global @miilo-ai/cli
export MIILO_URL=http://127.0.0.1:3700
export MIILO_TOKEN='short-lived-token'
miilo doctor
```

The native Rust binary and Node package are alternative installations of the
same command. Install one globally at a time.

## Credential files

Prefer `MIILO_TOKEN_FILE=/path/to/private/token` or `--token-file /path/to/private/token` over token contents in arguments. Rust and Node accept the same protected-file configuration and reject credential-bearing endpoint URLs and redirects. Legacy `MIILO_TOKEN` remains supported; do not set both methods. Restart a running client after token rotation. See [CE credential operations](https://github.com/miilo-ai/miilo-ce/blob/main/docs/CREDENTIALS.md) for provisioning, rotation, transport and current single-operator limits.

## Local mode (candidate 0.2)

`miilo local discover PROJECT`, `setup PROJECT`, `run`, `snapshot` and
`explain PROVIDER/WORK` invoke the native `miilo-local` engine without a shell.
Install the native CLI engine, or set `MIILO_LOCAL_EXECUTABLE` to that binary.
The npm package does not silently download/execute a native binary. Linux/macOS
local mode needs no URL/token/CE. Missing engine errors identify the required
installation; they never fall back to a cloud backend.

Remote `watch` opens a subscription before reading its initial snapshot. Every
event causes another authoritative read; every reconnect gets a fresh baseline.
Ordinary requests have a 15-second deadline and 2-MiB JSON limit. Error bodies
are capped at 64 KiB; stream headers/idle reads have separate deadlines.
