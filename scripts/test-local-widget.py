"""Installed-binary standalone journey. Uses fixtures; never starts agent work or CE."""
import json,os,subprocess,tempfile,time
from pathlib import Path

cli=Path(os.environ['MIILO_LOCAL_BINARY']).resolve()
widget=Path(os.environ['MIILO_OWL_BINARY']).resolve()
identifier='22222222-2222-2222-2222-222222222222'
def run(binary,*args,env,input=None):
 result=subprocess.run([str(binary),*args],input=input,env=env,text=True,capture_output=True,timeout=20)
 if result.returncode:raise RuntimeError('Installed fixture command failed: '+result.stderr[:500])
 return json.loads(result.stdout)
with tempfile.TemporaryDirectory(prefix='miilo-installed-local-') as folder:
 home=Path(folder).resolve();project=home/'work';project.mkdir()
 transcript=home/'cursor'/'workspace'/'agent-transcripts'/identifier/(identifier+'.jsonl');transcript.parent.mkdir(parents=True);transcript.write_text('{}\n')
 env=dict(os.environ,HOME=str(home),MIILO_LOCAL_CONFIG=str(home/'local.json'),XDG_STATE_HOME=str(home/'state'),MIILO_OWL_CONFIG_DIR=str(home/'widget'),MIILO_CURSOR_PROJECTS=str(home/'cursor'),MIILO_CODEX_EXECUTABLE=str(home/'no-codex'),MIILO_CE_EXECUTABLE=str(home/'no-ce'),MIILO_OWL_BINARY=str(widget))
 receipt=run(cli,'import',env=env,input=json.dumps({'project':str(project),'sessions':[{'kind':'cursor','id':identifier}],'startService':False})+'\n')
 assert receipt['mode']=='local'
 assert run(cli,'doctor',env=env)['mode']=='local'
 config=json.loads((home/'local.json').read_text());assert 'endpoint' not in config and 'tokenFile' not in config
 installed=json.loads((home/'widget/config.json').read_text());assert installed['endpoints'][0]['url']=='local://desktop'
 baseline=run(widget,'refresh',env=env);assert len(baseline['snapshot']['providers'])==1
 cache=Path(config["snapshotFile"]);initial=json.loads(cache.read_text())["snapshotId"]
 observer=subprocess.Popen([str(cli),'run'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 try:
  ready=time.monotonic()+8
  while json.loads(cache.read_text())["snapshotId"]==initial:
   if time.monotonic()>ready:raise RuntimeError('Observer did not publish its bootstrap')
   time.sleep(.05)
  with transcript.open('a') as file:file.write(json.dumps({'role':'assistant','message':{'content':[{'type':'text','text':'Fixture deployment checked'}]}})+'\n')
  deadline=time.monotonic()+10
  while True:
   current=run(widget,'refresh',env=env);work=current['snapshot']['workItems'][0]
   if work['summary']=='Fixture deployment checked':break
   if time.monotonic()>deadline:raise RuntimeError('Native file update did not reach installed widget consumer')
   time.sleep(.1)
  assert not work['actions']
  explained=run(widget,'explain',work['_key'],work['_revisionKey'],env=env);assert explained['status']=='completed';assert 'No verification evidence' in explained['message']
  # A widget's own cache cannot extend the lifetime of dead local data.
  run(widget,'snapshot',env=env)
  observer.terminate();observer.wait(timeout=5)
  cache=Path(config['snapshotFile']);stale=json.loads(cache.read_text());stale['observedAt']='2020-01-01T00:00:00Z';cache.write_text(json.dumps(stale));cache.chmod(0o600)
  assert run(widget,'snapshot',env=env)['status']=='partial'
  print('Installed standalone CLI → widget update → grounded explanation → stale-source journey passed (fixtures; CE absent).')
 finally:
  if observer.poll() is None:observer.terminate();observer.wait(timeout=5)
