"""Supervisor-owned stage D host checks; missing/skipped feature tests fail closed."""
from pathlib import Path
import hashlib,json,subprocess,sys,tempfile,time,uuid
ROOT=Path(__file__).resolve().parent
EXPECTED={
 'D01':['PublicLifecycle','RequestReplay','Conflict','WorkspaceWait','RestartRetention','ProofFreshness'],
 'D02':['Migration','ArchiveCompatibility','BackupRestoreSafety'],
 'D03':['QualificationManifest'],
 'D04':['InstallVersion','RollbackSafety']}

def protect():
 for name,wanted in json.loads((ROOT/'protected-reports.json').read_text()).items():
  assert hashlib.sha256(Path(name).read_bytes()).hexdigest()==wanted,'accepted report changed: '+name
def passed(inventory,events):
 assert inventory and len(inventory)==len(set(inventory)),'empty/duplicate inventory'
 done={e.get('Test') for e in events if e.get('Action')=='pass'}
 assert not any(e.get('Action') in ['skip','fail'] for e in events),'required feature skipped/failed'
 assert set(inventory)<=done,'feature not all executed'
def run(cmd,timeout,d):
 start=time.monotonic();p=subprocess.run(cmd,capture_output=True,text=True,timeout=timeout)
 log=d/(str(len(list(d.glob('*.json'))))+'.log');log.write_text(p.stdout+p.stderr)
 rec={'command':cmd,'exit_code':p.returncode,'seconds':round(time.monotonic()-start,3),'log':str(log),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()}
 (log.with_suffix('.json')).write_text(json.dumps(rec,indent=2)+'\n')
 assert p.returncode==0,json.dumps(rec)+'\n'+p.stdout[-4500:]+p.stderr[-2000:]
 return p.stdout
def main(stage):
 assert stage in EXPECTED
 protect();assert Path('docs/'+stage+'.md').is_file(),'report missing'
 d=ROOT/'results'/uuid.uuid4().hex;d.mkdir(parents=True)
 pattern='^TestGraphDelivery'+stage
 listing=run(['go','test','./...','-list',pattern],90,d)
 inventory=[n for n in listing.splitlines() if n.startswith('TestGraphDelivery'+stage)]
 wanted={'TestGraphDelivery'+stage+n for n in EXPECTED[stage]}
 assert wanted<=set(inventory),'missing feature tests: '+','.join(sorted(wanted-set(inventory)))
 output=run(['go','test','-race','./...','-run',pattern,'-count=1','-json','-timeout=120s'],150,d)
 events=[json.loads(l) for l in output.splitlines() if l.startswith('{')];passed(inventory,events)
 if stage=='D01':
  recipe=Path('tests/graph_automation_e2e.cjs');assert recipe.is_file(),'native product browser recipe missing'
  with tempfile.TemporaryDirectory(prefix='swarm-delivery-d-') as tmp:
   binary=str(Path(tmp)/'swarm');run(['sh','build.sh',binary],90,d)
   run(['node',str(recipe),binary,str(d/'browser')],150,d)
   p=d/'browser/results.json';assert p.is_file(),'browser receipt missing';b=json.loads(p.read_text());assert b['surface']=='swarm-product'
   assert {v['variant'] for v in b['variants']}=={'fr-sombre','fr-etat','en-sombre','en-etat'}
   req=['prepare_roles','dependency_apply','cycle_rejected','agent_journal','program_replay','content_conflict','workspace_wait','restart_retention','causal_recovery','unknown_cost','focus_restore','graph_preserved']
   for v in b['variants']:
    assert all(v['assertions'].get(k) is True for k in req),v['variant']
    assert not v.get('console_errors') and not v.get('network_errors')
    capture=(p.parent/v['screenshot']).read_bytes();assert capture[:8]==b'\x89PNG\r\n\x1a\n' and len(capture)>1000
   print(json.dumps({'browser':str(p),'variants':[v['variant'] for v in b['variants']],'assertions':req},ensure_ascii=False))
 if stage=='D02':
  run(['python3','tests/graph_delivery_docs.py',str(d/'documentation')],90,d)
 if stage=='D03':
  run(['go','vet','./...'],60,d)
  run(['npm','test'],60,d)
  run(['python3','tools/agent-workflows/check.py'],30,d)
  output=run(['python3','tests/supervision_go_suite.py'],210,d)
  assert 'PASS' in output,'suite completion missing'
 run(['git','diff','--check'],10,d);protect()
 print(json.dumps({'stage':stage,'status':'PASS','tests':inventory,'artifacts':str(d),'scope':'targeted host control; fixtures, no real provider autonomy or installed C claim'},ensure_ascii=False))
if __name__=='__main__':
 if sys.argv[1]=='--self-test':
  passed(['X'],[{'Action':'pass','Test':'X'}])
  for inv,events in [([],[]),(['X'],[]),(['X'],[{'Action':'skip','Test':'X'}]),(['X'],[{'Action':'fail','Test':'X'}]),(['X','X'],[{'Action':'pass','Test':'X'}])]:
   try:passed(inv,events)
   except AssertionError:continue
   raise AssertionError('fail-closed harness regression')
  protect();print('PASS harness rejects missing/skipped/failed/duplicate observations; feature functionality not yet tested')
 else:main(sys.argv[1])
