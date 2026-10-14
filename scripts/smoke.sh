#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
port=${MIILO_MOCK_PORT:-38765}
export MIILO_MOCK_PORT="$port"
export MIILO_URL="http://127.0.0.1:$port"
export MIILO_TOKEN="smoke-token"

python3 "$root/scripts/mock_api.py" &
server=$!
cleanup() { kill "$server" 2>/dev/null || true; }
trap cleanup EXIT INT TERM

attempt=0
while ! curl -fsS -H "Authorization: Bearer $MIILO_TOKEN" "$MIILO_URL/health" >/dev/null 2>&1; do
  attempt=$((attempt + 1))
  [ "$attempt" -lt 40 ] || { echo "mock API did not start" >&2; exit 1; }
  sleep 0.1
done

rust="$root/target/debug/miilo"
if [ -x "$rust" ]; then
  "$rust" doctor >/dev/null
  "$rust" connections list >/dev/null
  "$rust" work list >/dev/null
  "$rust" work show factory/task-1 >/dev/null
  "$rust" work explain factory/task-1 >/dev/null
fi

"$root/scripts/node-modern.sh" node "$root/packages/miilo-cli-node/dist/main.js" doctor >/dev/null
"$root/scripts/node-modern.sh" node "$root/packages/miilo-cli-node/dist/main.js" connections list >/dev/null
"$root/scripts/node-modern.sh" node "$root/packages/miilo-cli-node/dist/main.js" work list >/dev/null
"$root/scripts/node-modern.sh" node "$root/packages/miilo-cli-node/dist/main.js" work show factory/task-1 >/dev/null
"$root/scripts/node-modern.sh" node "$root/packages/miilo-cli-node/dist/main.js" work explain factory/task-1 >/dev/null

echo "Miilo CLI smoke journey passed."
