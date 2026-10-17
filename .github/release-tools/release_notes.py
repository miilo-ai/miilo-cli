"""Keep the custom GitHub tap repository distinct from its local Homebrew alias."""
import os
from pathlib import Path
import sys

def render(body):
    wrong='brew install miilo-ai/miilo-homebrew-tap/miilo'
    supported='brew tap miilo-ai/tap https://github.com/miilo-ai/miilo-homebrew-tap.git\nbrew install miilo-ai/tap/miilo'
    return body.replace(wrong,supported)

if __name__=='__main__':
    Path(sys.argv[1]).write_text(render(os.environ['ANNOUNCEMENT_BODY']))
