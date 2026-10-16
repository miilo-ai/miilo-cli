"""Run a digest-pinned workflow validator without installing system tools."""
import hashlib
import io
from pathlib import Path
import platform
import subprocess
import tarfile
import tempfile
import urllib.request

assets={('Linux','x86_64'):('linux_amd64','8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8'),
        ('Darwin','arm64'):('darwin_arm64','aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f')}
host=(platform.system(),platform.machine().lower())
if host not in assets:raise RuntimeError('Workflow lint runs on Linux x86_64 CI or local Apple Silicon')
name,digest=assets[host]
url=f'https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_{name}.tar.gz'
with urllib.request.urlopen(url,timeout=30) as response:data=response.read()
if hashlib.sha256(data).hexdigest()!=digest:raise RuntimeError('actionlint digest mismatch')
with tempfile.TemporaryDirectory(prefix='miilo-workflow-lint-') as folder:
 with tarfile.open(fileobj=io.BytesIO(data),mode='r:gz') as archive:
  binary=Path(folder)/'actionlint';binary.write_bytes(archive.extractfile('actionlint').read());binary.chmod(0o755)
 subprocess.run([str(binary),'-shellcheck=','-pyflakes=',*[str(p) for p in Path('.github/workflows').glob('*.yml')]],check=True)
print('Pinned workflow syntax/context validation passed (ShellCheck/Pyflakes are separate checks).')
