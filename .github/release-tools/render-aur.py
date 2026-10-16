"""Render a binary AUR recipe exclusively from verified GitHub release assets."""
import argparse
import hashlib
from pathlib import Path
import re

parser=argparse.ArgumentParser()
parser.add_argument('--tag',required=True)
parser.add_argument('--repo',required=True)
parser.add_argument('--kind',choices=['cli','ce','owl'],required=True)
parser.add_argument('--assets',type=Path,default=Path('release-assets'))
parser.add_argument('--output',type=Path,default=Path('aur'))
args=parser.parse_args()
if not re.fullmatch(r'v\d+\.\d+\.\d+',args.tag):raise ValueError('AUR publication requires a stable canonical version tag')
if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+',args.repo):raise ValueError('invalid repository')
binary={'cli':'miilo','ce':'miilo-ce','owl':'miilo-owl'}[args.kind]
pkgname=binary+'-bin'
archives={}
for arch,target in [('x86_64','x86_64-unknown-linux-gnu'),('aarch64','aarch64-unknown-linux-gnu')]:
 if args.kind=='owl':
  if arch!='aarch64':continue
  matches=list(args.assets.glob('miilo-owl-widgets-'+args.tag[1:]+'-linux-aarch64.tar.gz'))
 else:matches=list(args.assets.glob('*'+target+'.tar.xz'))
 if len(matches)!=1:raise ValueError('Expected one archive for '+arch)
 path=matches[0]
 manifests=list(args.assets.glob('*.sha256'))+list(args.assets.glob('*SHA256SUMS*'))
 expected=[]
 for manifest in manifests:
  for line in manifest.read_text().splitlines():
   fields=line.split()
   if len(fields)==2 and fields[1].lstrip('*./')==path.name and re.fullmatch('[a-fA-F0-9]{64}',fields[0]):expected.append(fields[0].lower())
 digest=hashlib.sha256(path.read_bytes()).hexdigest()
 if not expected or set(expected)!={digest}:raise ValueError('Published checksum missing or inconsistent for '+path.name)
 archives[arch]=(path.name,digest,target)
lines=[f'pkgname={pkgname}',f'pkgver={args.tag[1:]}','pkgrel=1',"pkgdesc='Miilo private-by-default observation tooling'",'arch=('+ ' '.join("'"+a+"'" for a in archives)+')',f"url='https://github.com/{args.repo}'",f"license=('{'MIT' if args.kind=='owl' else 'Apache-2.0'}')",f"provides=('{binary}')",f"conflicts=('{binary}')"]
for arch,(filename,digest,_) in archives.items():
 lines += [f'source_{arch}=("{filename}::${{url}}/releases/download/v${{pkgver}}/{filename}")',f"sha256sums_{arch}=('{digest}')"]
source='bin/linux-aarch64/miilo-owl' if args.kind=='owl' else '${srcdir}/'+('miilo-cli' if args.kind=='cli' else 'miilo-ce')+'-${CARCH}-unknown-linux-gnu/'+binary
licensefile='LICENSE' if args.kind=='owl' else '${srcdir}/'+('miilo-cli' if args.kind=='cli' else 'miilo-ce')+'-${CARCH}-unknown-linux-gnu/LICENSE'
lines += ['', 'package() {',f'  install -Dm755 "{source}" "$pkgdir/usr/bin/{binary}"',f'  install -Dm644 "{licensefile}" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"','}']
args.output.mkdir(parents=True,exist_ok=True)
(args.output/'PKGBUILD').write_text('\n'.join(lines)+'\n')
print('Rendered',pkgname,args.tag)
