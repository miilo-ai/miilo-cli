"""Keep the custom GitHub tap repository distinct from its local Homebrew alias."""
import os
from pathlib import Path
import sys

def render(body):
    wrong='brew install miilo-ai/homebrew-cli/miilo'
    supported='brew install miilo-ai/cli/miilo'
    return body.replace(wrong,supported)

if __name__=='__main__':
    Path(sys.argv[1]).write_text(render(os.environ['ANNOUNCEMENT_BODY']))
