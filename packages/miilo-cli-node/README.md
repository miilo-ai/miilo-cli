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
