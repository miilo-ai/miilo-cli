"""Exercise the exact native archive before its publishing job can start."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request
import zipfile
from version import read_version

root = Path.cwd()
for target in os.environ['TARGETS'].split(','):
    archives = [p for p in (root/'target/distrib').glob('*'+target+'*') if p.name.endswith(('.tar.xz','.tar.gz','.zip'))]
    if len(archives) != 1:raise RuntimeError('Expected exactly one native archive per target')
    with tempfile.TemporaryDirectory(prefix='miilo-installed-') as folder:
        stage=Path(folder)
        if archives[0].suffix=='.zip':
            with zipfile.ZipFile(archives[0]) as archive:
                for member in archive.namelist():
                    if not (stage/member).resolve().is_relative_to(stage):raise RuntimeError('Unsafe archive member')
                archive.extractall(stage)
        else:
            with tarfile.open(archives[0]) as archive:archive.extractall(stage,filter='data')
        name='miilo-ce' if (root/'crates/server').exists() else 'miilo'
        binaries=[p for p in stage.rglob(name+('.exe' if os.name=='nt' else '')) if p.is_file()]
        if len(binaries)!=1:raise RuntimeError('Native archive has no unique executable')
        binary=binaries[0]
        reported = subprocess.check_output([str(binary),'--version'],text=True,timeout=15).strip()
        if reported.split()[-1] != read_version(root):raise RuntimeError('Installed binary version differs from release manifests')
        if not list(stage.rglob('LICENSE')):raise RuntimeError('Native archive is missing LICENSE')
        print(reported)
        env=dict(os.environ)
        for key in ['MIILO_TOKEN','MIILO_TOKEN_FILE','MIILO_CE_TOKEN','MIILO_CE_TOKEN_FILE']:env.pop(key,None)
        if name=='miilo-ce':
            env.update(MIILO_CE_BINARY=str(binary),MIILO_CREDENTIAL_CLI_CHECK='0')
            subprocess.run([sys.executable,'scripts/test-credential-lifecycle.py'],env=env,check=True,timeout=30)
        else:
            local_binaries=[p for p in stage.rglob('miilo-local'+('.exe' if os.name=='nt' else '')) if p.is_file()]
            if len(local_binaries)!=1:raise RuntimeError('Native CLI archive must include miilo-local')
            local_version=subprocess.check_output([str(local_binaries[0]),'--version'],text=True,timeout=15).strip()
            if local_version.split()[-1]!=read_version(root):raise RuntimeError('Local engine archive version mismatch')
            env['MIILO_MOCK_PORT']='38765'
            server=subprocess.Popen([sys.executable,'scripts/mock_api.py'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            try:
                endpoint='http://127.0.0.1:38765'
                for _ in range(50):
                    try:
                        request=urllib.request.Request(endpoint+'/health',headers={'Authorization':'Bearer smoke-token'})
                        urllib.request.urlopen(request,timeout=1).close();break
                    except OSError:time.sleep(.1)
                else:raise RuntimeError('Fixture API failed to start')
                token=stage/'token';token.write_text('smoke-token\n');token.chmod(0o600)
                result=subprocess.run([str(binary),'--endpoint',endpoint,'--token-file',str(token),'--json','doctor'],env=env,capture_output=True,text=True,timeout=15,check=True)
                assert json.loads(result.stdout)['health']['ok'] is True
                assert 'smoke-token' not in result.stdout+result.stderr
                if os.name!='nt' and os.environ.get('MIILO_OWL_BINARY'):
                    env['MIILO_LOCAL_BINARY']=str(local_binaries[0])
                    subprocess.run([sys.executable,'scripts/test-local-widget.py'],env=env,check=True,timeout=30)
            finally:
                server.terminate();server.wait(timeout=5)
        print('Installed archive accepted:',archives[0].name)
