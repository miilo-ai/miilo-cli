#!/bin/sh
set -eu

supports_node() {
  node -e 'const [major, minor] = process.versions.node.split(".").map(Number); process.exit(major > 20 || (major === 20 && minor >= 19) ? 0 : 1)' >/dev/null 2>&1
}

if supports_node; then exec "$@"; fi

if [ -s "$HOME/.nvm/nvm.sh" ]; then
  # nvm intentionally requires a shell source operation.
  . "$HOME/.nvm/nvm.sh"
  nvm use --silent 20.19.0 >/dev/null
  if supports_node; then exec "$@"; fi
fi

echo "Miilo Node CLI development requires Node.js 20.19 or newer." >&2
exit 1
