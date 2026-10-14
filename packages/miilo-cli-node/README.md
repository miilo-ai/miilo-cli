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
