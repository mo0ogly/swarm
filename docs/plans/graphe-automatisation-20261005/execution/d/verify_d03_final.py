"""D03 host qualification, one global suite with bounded total runtime."""
import importlib.util,json,hashlib,re,sys,time,subprocess,os,signal
from pathlib import Path
D=Path(__file__).resolve().parent

def suite_summary(output):
 inventory=re.search(r'GO SUITE package=(\S+) tests=(\d+) shards=(\d+) coverage=disjoint-complete compile=once',output)
 completion=re.search(r'PASS full discovered Go suite: (\d+) tests; all shards exited 0\.',output)
 assert inventory and completion,'full suite inventory/completion missing'
 total=int(inventory[2]);shards=int(inventory[3]);assert total>0 and int(completion[1])==total
 records=re.findall(r'SHARD (\d+) exit_code=(\d+) elapsed=([0-9.]+)s covered=(\d+)/(\d+) passed=(\d+)',output)
 assert len(records)==shards and len({r[0] for r in records})==shards,'missing/duplicate shard receipts'
 assert all(r[1]=='0' and r[3]==r[4] for r in records),'failed/incomplete shard'
 assert sum(int(r[4]) for r in records)==total,'coverage count mismatch'
 skips=[json.loads(line) for line in output.splitlines() if line.startswith('{') and 'optional_skip' in line]
 required=('TestGraphDeliveryD','TestAutomation','TestGraphDraftB','TestGraphPerformance','TestReviewTokenCache')
 assert not any(s['optional_skip'].startswith(required) for s in skips),'required delivery/automation test skipped'
 passed=sum(int(r[5]) for r in records)
 top_skips=[s for s in skips if '/' not in s['optional_skip']]
 assert passed+len(top_skips)==total,'passed/skipped accounting mismatch'
 return {'discovered':total,'passed':passed,'shards':shards,'optional_skips':skips,'compile':'once','coverage':'disjoint-complete'}

def main():
 spec=importlib.util.spec_from_file_location('d_verify',D/'verify.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
 m=json.loads((D/'d02-fresh-browser-manifest.json').read_text())
 assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==m['base_git']
 for n,h in {**m['candidate_inputs'],**m['artifacts'],**json.loads((D/'protected-d01.json').read_text())}.items():
  assert hashlib.sha256(Path(n).read_bytes()).hexdigest()==h,'browser/protected input changed: '+n
 receipt=json.loads(Path(m['browser_receipt']).read_text());assert receipt['surface']=='swarm-product'
 assert {x['variant'] for x in receipt['variants']}=={'fr-sombre','fr-etat','en-sombre','en-etat'}
 assert all(len(x['assertions'])==12 and all(x['assertions'].values()) and not x['console_errors'] and not x['network_errors'] for x in receipt['variants'])
 deadline=time.monotonic()+295;summary=None
 def bounded(cmd,timeout,d):
  nonlocal summary
  remaining=deadline-time.monotonic();assert remaining>0,'global qualification deadline exhausted'
  started=time.monotonic();log=d/(str(len(list(d.glob('*.json'))))+'.log')
  with log.open('w') as f:
   proc=subprocess.Popen(cmd,stdout=f,stderr=subprocess.STDOUT,start_new_session=True)
   try:code=proc.wait(timeout=min(timeout,remaining))
   except subprocess.TimeoutExpired:
    os.killpg(proc.pid,signal.SIGTERM)
    try:proc.wait(timeout=2)
    except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
    raise AssertionError('qualification command deadline exceeded: '+str(cmd)+'; log='+str(log))
  output=log.read_text();record={'command':cmd,'exit_code':code,'seconds':round(time.monotonic()-started,3),'log':str(log),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()}
  log.with_suffix('.json').write_text(json.dumps(record,indent=2)+'\n')
  assert code==0,json.dumps(record)+'\n'+output[-4000:]
  if cmd==['python3','tests/supervision_go_suite.py']:
   summary=suite_summary(output);summary['log']=record; (d/'full-suite-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
  return output
 v.run=bounded;v.main('D03');assert summary is not None
 print(json.dumps({'global_qualification':'PASS','suite':summary,'browser_candidate_bound':m['browser_receipt'],'limits':'isolated fixtures; provider autonomy and installed D not demonstrated'}))

if __name__=='__main__':
 if '--self-test' in sys.argv:
  sample='GO SUITE package=x tests=2 shards=1 coverage=disjoint-complete compile=once\nSHARD 0 exit_code=0 elapsed=1.0s covered=2/2 passed=1\n'+json.dumps({'optional_skip':'TestOptionalProbe','observed_output':['opt-in disabled']})+'\nPASS full discovered Go suite: 2 tests; all shards exited 0.'
  assert suite_summary(sample)['passed']==1
  for wrong in [sample.replace('PASS full discovered','MISSING full discovered'),sample.replace('TestOptionalProbe','TestGraphDeliveryD03QualificationManifest'),sample.replace('covered=2/2','covered=1/2'),sample.replace('passed=1','passed=2')]:
   try:suite_summary(wrong)
   except AssertionError:continue
   raise AssertionError('invalid suite receipt accepted')
  print('PASS wrapper self-test only; no functional/global suite executed')
 else:main()
