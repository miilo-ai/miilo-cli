#!/usr/bin/env bash
set -euo pipefail
tap_directory=$(cd "$1" && pwd)
formula=miilo-ai/cli/miilo
brew tap miilo-ai/cli "$tap_directory"
brew trust --formula "$formula"
brew audit --strict "$formula"
if [ -n "${2:-}" ]; then
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
