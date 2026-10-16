"""Install cargo-dist from a reviewed version and pinned asset digest, without a shell bootstrap."""
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import tarfile
import urllib.request
import zipfile

machine=platform.machine().lower()
arch='aarch64' if machine in ('arm64','aarch64') else 'x86_64' if machine in ('amd64','x86_64') else None
suffix={'Darwin':'apple-darwin','Linux':'unknown-linux-gnu','Windows':'pc-windows-msvc'}.get(platform.system())
if not arch or not suffix:raise RuntimeError('Unsupported cargo-dist host')
extension='.zip' if os.name=='nt' else '.tar.xz'
asset=f'cargo-dist-{arch}-{suffix}{extension}'
digests=json.loads(Path(__file__).with_name('dist-assets.json').read_text())
with urllib.request.urlopen('https://github.com/axodotdev/cargo-dist/releases/download/v0.33.0/'+asset,timeout=60) as response:data=response.read()
if hashlib.sha256(data).hexdigest()!=digests[asset]:raise RuntimeError('cargo-dist asset digest mismatch')
name='dist.exe' if os.name=='nt' else 'dist'
if extension=='.zip':
 with zipfile.ZipFile(io.BytesIO(data)) as archive:
  members=[p for p in archive.namelist() if Path(p).name==name]
  if len(members)!=1:raise RuntimeError('Unexpected cargo-dist archive contents')
  executable=archive.read(members[0])
else:
 with tarfile.open(fileobj=io.BytesIO(data),mode='r:xz') as archive:
  members=[p for p in archive.getmembers() if Path(p.name).name==name and p.isfile()]
  if len(members)!=1:raise RuntimeError('Unexpected cargo-dist archive contents')
  executable=archive.extractfile(members[0]).read()
directory=Path(os.environ['RUNNER_TEMP'])/'miilo-tools';directory.mkdir(exist_ok=True,parents=True)
binary=directory/('dist.exe' if os.name=='nt' else 'dist');binary.write_bytes(executable);binary.chmod(0o755)
with open(os.environ['GITHUB_PATH'],'a') as stream:stream.write(str(directory)+'\n')
print('Verified cargo-dist v0.33.0 installed for',platform.system(),arch)
