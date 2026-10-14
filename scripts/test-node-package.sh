#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/miilo-node-package.XXXXXX")
cleanup() { rm -rf "$scratch"; }
trap cleanup EXIT INT TERM

cd "$root/packages/miilo-cli-node"
"$root/scripts/node-modern.sh" npm pack --pack-destination "$scratch"
"$root/scripts/node-modern.sh" npm install --prefix "$scratch/install" "$scratch/miilo-ai-cli-0.1.0.tgz"
"$scratch/install/node_modules/.bin/miilo" --help | grep -q '^usage: miilo '
