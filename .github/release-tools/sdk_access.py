"""Allow public SDK access or a scoped private-repository read token, never a sibling override."""
import base64
import json
import os
import re
from pathlib import Path
import subprocess
import tomllib
import urllib.error
import urllib.request


def pinned_reference(dependency):
    if dependency.get('git') != 'https://github.com/miilo-ai/miilo-sdk':
        raise ValueError('canonical SDK repository required')
    tag, revision=dependency.get('tag'), dependency.get('rev')
    if bool(tag)==bool(revision):raise ValueError('exactly one SDK tag or revision is required')
    if revision and not re.fullmatch(r'[a-f0-9]{40}',revision):raise ValueError('SDK revision must be a full commit SHA')
    if tag and not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9.-]+)?',tag):raise ValueError('SDK tag must be versioned')
    return revision or tag


def main():
    dependency = tomllib.loads(Path('Cargo.toml').read_text())['workspace']['dependencies']['miilo-adapter-sdk']
    pinned_reference(dependency)
    token = os.environ.get('SDK_READ_TOKEN', '')
    headers = {'Accept': 'application/vnd.github+json', 'User-Agent': 'miilo-release-check'}
    if token:
        headers['Authorization'] = 'Bearer ' + token
    request = urllib.request.Request('https://api.github.com/repos/miilo-ai/miilo-sdk', headers=headers)
    ready = True
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            json.load(response)
    except urllib.error.HTTPError as error:
        if not token and error.code in (401, 404):
            ready = False
        else:
            raise RuntimeError('SDK access check failed; inspect credential scope or service availability') from None
    if ready:
        env = dict(os.environ, GIT_TERMINAL_PROMPT='0')
        if token:
            encoded = base64.b64encode(('x-access-token:' + token).encode()).decode()
            print('::add-mask::' + encoded)
            config = Path(os.environ['RUNNER_TEMP']) / 'miilo-sdk.gitconfig'
            config.touch(mode=0o600, exist_ok=True)
            config.chmod(0o600)
            config.write_text('[http "https://github.com/miilo-ai/miilo-sdk"]\n\textraheader = AUTHORIZATION: basic ' + encoded + '\n')
            env['GIT_CONFIG_GLOBAL'] = str(config)
            with open(os.environ['GITHUB_ENV'], 'a') as stream:
                stream.write(f'GIT_CONFIG_GLOBAL={config}\nCARGO_NET_GIT_FETCH_WITH_CLI=true\n')
        if dependency.get('rev'):
            request=urllib.request.Request('https://api.github.com/repos/miilo-ai/miilo-sdk/commits/'+dependency['rev'], headers=headers)
            try:
                with urllib.request.urlopen(request,timeout=20) as response:
                    commit=json.load(response)
                if commit.get('sha') != dependency['rev']:
                    raise RuntimeError('Pinned SDK commit did not match')
            except urllib.error.HTTPError:
                raise RuntimeError('Pinned SDK commit cannot be fetched; no release may proceed') from None
        else:
            result = subprocess.run(['git', 'ls-remote', '--exit-code', dependency['git'],
                                     'refs/tags/' + dependency['tag']], env=env,
                                    capture_output=True, text=True, timeout=30)
            if result.returncode:
                raise RuntimeError('Pinned SDK tag cannot be fetched; no release may proceed')
    else:
        print('Private SDK access is not configured. No release has been made')
    if output := os.environ.get('GITHUB_OUTPUT'):
        with open(output, 'a') as stream:
            stream.write(f'ready={str(ready).lower()}\n')
    if not ready and (summary := os.environ.get('GITHUB_STEP_SUMMARY')):
        with open(summary, 'a') as stream:
            stream.write('No release has been made\nPrivate SDK requires MIILO_SDK_READ_TOKEN (contents read).\n')


if __name__ == '__main__':
    main()
