"""Manifest-driven SemVer policy, shared as self-contained copies in release repos."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

SEMVER = re.compile(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?')


def version_key(value):
    match = SEMVER.fullmatch(value)
    if not match:
        raise ValueError('version must be SemVer major.minor.patch with optional prerelease/build')
    pre = match[4]
    parts = []
    for part in pre.split('.') if pre else []:
        if part.isdigit() and len(part) > 1 and part.startswith('0'):
            raise ValueError('numeric prerelease identifiers must not have leading zeroes')
        parts.append((0, int(part)) if part.isdigit() else (1, part))
    return (*map(int, match.group(1, 2, 3)), pre is None, tuple(parts))


def read_version(root):
    def toml(path):
        return tomllib.loads((root / path).read_text())
    if (root / 'helper/Cargo.toml').exists():
        values = [toml('helper/Cargo.toml')['package']['version']]
    elif (root / 'python/pyproject.toml').exists():
        values = [toml('python/pyproject.toml')['project']['version'],
                  toml('rust/Cargo.toml')['package']['version'],
                  json.loads((root / 'typescript/package.json').read_text())['version']]
    else:
        values = [toml('Cargo.toml')['workspace']['package']['version']]
        node = root / 'packages/miilo-cli-node/package.json'
        if node.exists():
            values.append(json.loads(node.read_text())['version'])
            lock = json.loads(node.with_name('package-lock.json').read_text())
            values.extend([lock['version'], lock['packages']['']['version']])
    if len(set(values)) != 1:
        raise ValueError('package manifest versions disagree')
    version_key(values[0])
    return values[0]


def candidate(root, tags, ref=''):
    version = read_version(root)
    tag = 'v' + version
    if ref and ref.removeprefix('refs/tags/') != tag:
        raise ValueError('release tag does not match package versions')
    released = [t for t in tags if t.startswith('v') and SEMVER.fullmatch(t[1:])]
    if not ref and released and version_key(version) < max(version_key(t[1:]) for t in released):
        raise ValueError('package version is older than an existing version tag')
    return {'version': version, 'tag': tag, 'new_version': str(tag not in tags).lower(),
            'prerelease': str(SEMVER.fullmatch(version)[4] is not None).lower()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, default=Path.cwd())
    parser.add_argument('--tag', default='')
    args = parser.parse_args()
    tags = subprocess.check_output(['git', 'tag', '--list'], cwd=args.root, text=True).splitlines()
    result = candidate(args.root, tags, args.tag)
    print(json.dumps(result))
    if output := os.environ.get('GITHUB_OUTPUT'):
        with open(output, 'a') as stream:
            stream.writelines(f'{key}={value}\n' for key, value in result.items())


if __name__ == '__main__':
    main()
