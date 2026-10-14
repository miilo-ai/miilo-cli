#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
backup=$(mktemp "${TMPDIR:-/tmp}/miilo-cli-cargo.XXXXXX")
lock_backup=$(mktemp "${TMPDIR:-/tmp}/miilo-cli-lock.XXXXXX")
cp "$root/Cargo.toml" "$backup"
lock_existed=false
if [ -f "$root/Cargo.lock" ]; then
  cp "$root/Cargo.lock" "$lock_backup"
  lock_existed=true
fi
restore() {
  cp "$backup" "$root/Cargo.toml"
  if [ "$lock_existed" = true ]; then
    cp "$lock_backup" "$root/Cargo.lock"
  else
    rm -f "$root/Cargo.lock"
  fi
  rm -f "$backup" "$lock_backup"
}
trap restore EXIT INT TERM
python3 "$root/scripts/use-local-sdk.py" "$root/Cargo.toml"
cd "$root"
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
