#!/usr/bin/env bash
set -euo pipefail
tap_directory=$(cd "$1" && pwd)
formula=miilo-ai/cli/miilo
brew tap miilo-ai/cli "$tap_directory"
brew trust --formula "$formula"
brew audit --strict "$formula"
if [ -n "${2:-}" ]; then
  checksum=$(brew info --json=v2 "$formula" | python3 -c 'import json,sys; print(json.load(sys.stdin)["formulae"][0]["urls"]["stable"]["checksum"])')
  printf '%s  %s\n' "$checksum" "$2" | shasum -a 256 --check
  cache=$(brew --cache --build-from-source "$formula")
  mkdir -p "$(dirname "$cache")"
  cp "$2" "$cache"
fi
brew install "$formula"
brew test "$formula"
prefix=$(brew --prefix "$formula")
bash "$tap_directory/tools/test-archive-install.sh" "$prefix/bin/miilo"
"$prefix/bin/miilo-local" --version
brew uninstall "$formula"
