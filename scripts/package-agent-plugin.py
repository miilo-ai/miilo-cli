"""Build a skills-only, local-executor package; directory submission stays manual."""
import json
from pathlib import Path
import tomllib
import zipfile

root=Path(__file__).resolve().parents[1]
version=tomllib.loads((root/'Cargo.toml').read_text())['workspace']['package']['version']
skill=root/'skills/miilo-connect/SKILL.md'
text=skill.read_text()
assert text.startswith('---\nname: miilo-connect\n') and 'MIILO_TOKEN_FILE' in text
manifest={'$schema':'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json','name':'miilo-connect','version':version,
 'description':'Observe explicitly selected local sessions or a chosen remote Miilo endpoint.',
 'author':{'name':'Viratec Interactive LLC','email':'contact@miilo.tech'},'license':'Apache-2.0',
 'repository':'https://github.com/miilo-ai/miilo-cli','homepage':'https://miilo.tech',
 'extensions':{'com.openai':{'requires_local_executor':True,'onboardingSkill':'./skills/miilo-connect/SKILL.md'}}}
output=root/'target/distrib';output.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(output/f'miilo-agent-plugin-{version}.zip','w',compression=zipfile.ZIP_DEFLATED) as archive:
 archive.writestr('plugin.json',json.dumps(manifest,indent=2)+'\n')
 archive.write(skill,'skills/miilo-connect/SKILL.md');archive.write(root/'LICENSE','LICENSE')
 for host in ['claude','cursor']:
  legacy={key:value for key,value in manifest.items() if key not in ['$schema','extensions']}
  legacy['skills']='./skills/'
  archive.writestr(f'.{host}-plugin/plugin.json',json.dumps(legacy,indent=2)+'\n')
 archive.writestr('README.md','Miilo connection skill. Local execution required. Local mode requires the native engine and explicit session selection, without an API URL. Configure MIILO_URL and MIILO_TOKEN_FILE only for optional remote mode. This package observes; it does not authorize provider actions. Marketplace review and approval are separate from this build.\n')
print(output/f'miilo-agent-plugin-{version}.zip')
