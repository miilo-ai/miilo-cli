#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/miilo-node-package.XXXXXX")
cleanup() { rm -rf "$scratch"; }
trap cleanup EXIT INT TERM

cd "$root/packages/miilo-cli-node"
"$root/scripts/node-modern.sh" npm pack --pack-destination "$scratch"
set -- "$scratch"/*.tgz
[ "$#" -eq 1 ] && [ -f "$1" ] || { echo "Expected exactly one Node package" >&2; exit 1; }
archive=$1
"$root/scripts/node-modern.sh" npm install --prefix "$scratch/install" "$archive"
"$scratch/install/node_modules/.bin/miilo" --help | grep -q '^usage: miilo '

# Package removal must not remove an independently installed native engine.
mkdir -p "$scratch/install/bin"
printf '%s\n' 'independent native engine' > "$scratch/install/bin/miilo-local"
"$root/scripts/node-modern.sh" npm uninstall --prefix "$scratch/install" @miilo-ai/cli
test ! -e "$scratch/install/node_modules/.bin/miilo"
test ! -e "$scratch/install/node_modules/@miilo-ai/cli"
grep -q '^independent native engine$' "$scratch/install/bin/miilo-local"
"$root/scripts/node-modern.sh" npm install --prefix "$scratch/install" "$archive"
"$scratch/install/node_modules/.bin/miilo" --help | grep -q '^usage: miilo '
grep -q '^independent native engine$' "$scratch/install/bin/miilo-local"
printf '%s\n' 'Node package install/uninstall/reinstall passed; independent native engine retained.'
