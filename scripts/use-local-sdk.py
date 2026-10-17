"""Temporarily point the CLI workspace at a sibling Miilo SDK checkout."""

from pathlib import Path
import sys
import re


path = Path(sys.argv[1])
source = path.read_text(encoding="utf-8")
pattern = r'miilo-adapter-sdk = \{ git = "https://github.com/miilo-ai/miilo-sdk", (?:tag|rev) = "[^"\n]+" \}'
match = re.search(pattern, source)
remote = match[0] if match else None
local = 'miilo-adapter-sdk = { path = "../miilo-sdk/rust" }'
if remote is None:
    raise SystemExit("expected pinned Miilo SDK dependency was not found")
path.write_text(source.replace(remote, local), encoding="utf-8")
