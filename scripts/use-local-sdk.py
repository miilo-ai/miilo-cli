"""Temporarily point the CLI workspace at a sibling Miilo SDK checkout."""

from pathlib import Path
import sys


path = Path(sys.argv[1])
source = path.read_text(encoding="utf-8")
remote = 'miilo-adapter-sdk = { git = "https://github.com/miilo-ai/miilo-sdk", tag = "v0.1.0" }'
local = 'miilo-adapter-sdk = { path = "../miilo-sdk/rust" }'
if remote not in source:
    raise SystemExit("expected tagged Miilo SDK dependency was not found")
path.write_text(source.replace(remote, local), encoding="utf-8")
