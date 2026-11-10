"""Real Go control primitive vs native Linux primitive, private roots only."""
import datetime,hashlib,json,os,subprocess,tempfile,time
from pathlib import Path

def qualify(ROOT,output,cli,probe,reference):
 output=Path(output);output.mkdir();scratch=Path(tempfile.mkdtemp(prefix='.swarm-rust-control-',dir=Path.home().resolve()))
 root=scratch/'root';root.mkdir(mode=0o700);commands=[]
 def save(name,value):(output/name).write_text(json.dumps(value,indent=2)+'\n')
 def run(label,args,data=None):
  start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run([str(a)for a in args],cwd=ROOT,input=data,capture_output=True,env=dict(os.environ,TMPDIR=str(scratch)))
  for ext,b in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+ext)).write_bytes(b)
  if data is not None:(output/(label+'.stdin')).write_bytes(data)
  commands.append(dict(label=label,args=[str(a)for a in args],start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode));save('commands.json',commands);print('control',label,'exit',p.returncode,flush=True)
  assert p.returncode==0,label+': original failure retained'
  return json.loads(p.stdout)
 try:
  run('init',[cli,'--root',root,'--json','init']);(root/'.rust-migration-fixture').write_text('Owned control qualification fixture.\n')
  def control(script,**fields):return dict(id='check',command=['python3','-c',script],criteria=[1],justification='Reproducible control fixture',timeout_seconds=60,review_output=True,**fields)
  def native(label,c):return run(label,[probe,root],json.dumps(dict(control=c)).encode()+b'\n')
  def go(label,c):return run(label,[reference],json.dumps(dict(root=str(root),control=c)).encode()+b'\n')
  stable=['id','command','executed','passed','exit_code','output_sha256','summary','environment_failure','review_output','output_bytes','review_output_truncated']
  scripts=[("empty","pass",{}),("streams","import sys;sys.stdout.buffer.write(b'hello');sys.stdout.flush();sys.stderr.buffer.write(b' error');sys.stderr.flush()",{}),("over-cap","import sys;sys.stdout.buffer.write(b'a'*131072);sys.stdout.flush()",{}),("invalid-utf8","import sys;sys.stdout.buffer.write(bytes([97,240,159,146,161,255,122]));sys.stdout.flush()",{}),("failure","import sys;print('failed');sys.exit(9)",{}),("environment","import sys;print('environment');sys.exit(7)",dict(environment_exit_codes=[7]))]
  comparisons=[]
  settings=root/'.swarm/engine-settings.json';before=settings.read_bytes()
  for label,script,fields in scripts:
   c=control(script,**fields);g=go(label+'-go',c);n=native(label+'-native',c);assert n['code']=='ok'
   assert {k:g['result'].get(k)for k in stable}=={k:n['result'].get(k)for k in stable},label+': semantic divergence'
   assert(g['total'],g['retained_hex'])==(n['total'],n['retained_hex']);assert not n['timed_out'];assert'cpu_duration_ms'in g['result']and'cpu_duration_ms'in n['result'];assert n['result']['cpu_duration_ms']>=0
   assert n['result']['wall_duration_ms']>=0 and g['result']['wall_duration_ms']>=0;assert settings.read_bytes()==before
   comparisons.append(label)
  c=control("import signal;print('ready',flush=True);signal.pause()");c['timeout_seconds']=1
  for label,runner in [('deadline-go',go),('deadline-native',native)]:
   value=runner(label,c);assert value['result']['executed']and not value['result']['passed']and value['result']['exit_code']==-1;assert value['retained_hex']==b'ready\n'.hex();assert value['result']['wall_duration_ms']>=1000
  for directory in ['missing','../']:
   assert native('path-valid-'+str(len(commands)),control("print('valid')"))['result']['passed']
   c=control("print('must not execute')",dir=directory);g=go('path-refused-go-'+str(len(commands)),c);n=native('path-refused-native-'+str(len(commands)),c)
   assert not g['result']['executed']and not g['result']['passed'];assert n['code']=='validation-run'and n['operation']=='directory';assert settings.read_bytes()==before
   assert native('path-recovery-'+str(len(commands)),control("print('recovered')"))['result']['passed']
  saved=[]
  for index,cap in enumerate(['5','0',None]):
   doc=run('settings-read-'+str(index),[cli,'--root',root,'--json','engine-settings','show']);values=doc['values'].copy();values['validation_max_output_bytes']=cap
   change=dict(schema_version=doc['schema_version'],event_id='retention-'+str(index),expected_revision=doc['revision'],values=values,reason='Explicit capture retention')
   file=scratch/('settings-'+str(index)+'.json');file.write_text(json.dumps(change));os.chmod(file,0o600);bytes_before=settings.read_bytes()
   preview=run('settings-preview-'+str(index),[cli,'--root',root,'--json','engine-settings','preview','--input',file]);assert settings.read_bytes()==bytes_before
   change['preview_token']=preview['preview_token'];file.write_text(json.dumps(change));run('settings-apply-'+str(index),[cli,'--root',root,'--json','engine-settings','apply','--input',file]);reopened=run('settings-reopen-'+str(index),[cli,'--root',root,'--json','engine-settings','show']);assert reopened['values']['validation_max_output_bytes']==cap
   n=native('configured-capture-'+str(index),control("import sys;sys.stdout.write('abcdefghijk')"));retained=b'abcdefghijk'if cap is None else b'abcdefghijk'[:int(cap)]
   assert n['result']['passed']and n['retained_hex']==retained.hex()and n['total']==11 and n['settings_revision']==reopened['revision'];saved.append(cap)
  result=dict(status='PASS',scope='Native Linux control primitive and persisted-policy effect, not supervisor authorization or full executor acceptance',semantic_parity_cases=comparisons,deadline_both=True,path_refusal_recovery=True,configured_caps=saved,cpu_metric='Per-child CPU measured in both; independent executions have different durations',drain_deadline='Native declared control deadline; Go fixed 1s WaitDelay; deliberate difference',scratch=str(scratch),binaries={str(p):hashlib.sha256(Path(p).read_bytes()).hexdigest()for p in [cli,probe,reference]},global_watchdog=None)
  save('result.json',result);return result
 except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),scratch=str(scratch)));raise
