"""Host requalification after explicitly authorized contract-recovery engine fix."""
import importlib.util,json,subprocess,sys,time,hashlib,re,os,signal
from pathlib import Path
D=Path(__file__).resolve().parent

def main(stage):
 m=json.loads(Path('docs/graph-delivery-contract-revision-manifest.json').read_text())
 assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==m['base_commit']
 for i in m['inputs']:
  assert hashlib.sha256(Path(i['path']).read_bytes()).hexdigest()==i['sha256'],'current candidate input changed: '+i['path']
 spec=importlib.util.spec_from_file_location('historical_harness',D/'verify.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
 spec=importlib.util.spec_from_file_location('historical_summary',D/'verify_d03_final.py');summary_module=importlib.util.module_from_spec(spec);spec.loader.exec_module(summary_module)
 deadline=time.monotonic()+295
 def bounded(cmd,timeout,d):
  left=deadline-time.monotonic();assert left>0,'global control deadline exceeded'
  started=time.monotonic();log=d/(str(len(list(d.glob('*.json'))))+'.log')
  with log.open('w') as f:
   proc=subprocess.Popen(cmd,stdout=f,stderr=subprocess.STDOUT,start_new_session=True)
   try:code=proc.wait(timeout=min(timeout,left))
   except subprocess.TimeoutExpired:
    os.killpg(proc.pid,signal.SIGTERM)
    try:proc.wait(timeout=2)
    except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
    raise AssertionError('command timed out; log='+str(log))
  output=log.read_text();receipt={'command':cmd,'exit_code':code,'seconds':round(time.monotonic()-started,3),'log':str(log),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()};log.with_suffix('.json').write_text(json.dumps(receipt,indent=2)+'\n')
  assert code==0,json.dumps(receipt)+'\n'+output[-4500:]
  if cmd==['python3','tests/supervision_go_suite.py']:
   summary=summary_module.suite_summary(output);summary['log']=receipt;(d/'full-suite-summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({'new_engine_full_suite':summary}))
  return output
 v.run=bounded
 if stage=='D01':
  source=Path('task_definition.go').read_text().split('func validateHierarchicalContractRevision',1)[1]
  guard=Path('store.go').read_text().split('// The ownership guard uses this transaction',1)[1].split('if r.Status == "running"',1)[0]
  fields=Path('model.go').read_text().split('type Request struct {',1)[1].split('MaxAttempts',1)[0]
  print(json.dumps({'authorized_engine_correction_source':{'validator':'func validateHierarchicalContractRevision'+source,'transaction_guard':guard,'request_fields':fields,'refused_recovery_audit_guard':Path('independent_review.go').read_text().split('func (s *Store) recordedContractRevisionMatches',1)[1],'changed_contract_guard':Path('independent_review.go').read_text().split('contractChanged :=',1)[1].split('inputs :=',1)[0]},'phase_order':'functional review before acceptance; four fresh acceptances before public scope closure, then authorized installation; global requirement unchanged'}))
  output=subprocess.run(['go','test','-race','.','-run','^TestHierarchicalContractRevision','-count=1','-json','-timeout=60s'],capture_output=True,text=True,timeout=75)
  assert output.returncode==0,output.stdout[-4000:]+output.stderr
  events=[json.loads(l) for l in output.stdout.splitlines() if l.startswith('{')];assert not any(e.get('Action') in ['skip','fail'] for e in events)
  wanted={'TestHierarchicalContractRevisionPublicCLIReplayAndHistory','TestHierarchicalContractRevisionRejectsUnsafeRequestsAtomically','TestHierarchicalContractRevisionRejectsActiveReviewAndClosedScope','TestHierarchicalContractRevisionArchivesRefusalAndInvalidatesEvidence','TestHierarchicalContractRevisionRefusedResultCanResumeAfterAmendment'};assert wanted<={e.get('Test') for e in events if e.get('Action')=='pass'}
  print(json.dumps({'contract_revision_tests':'PASS','tests':sorted(wanted),'log_sha256':hashlib.sha256((output.stdout+output.stderr).encode()).hexdigest()}))
 if stage=='D04':
  subprocess.run(['python3',str(D/'verify_d04_host.py')],check=True,timeout=max(1,deadline-time.monotonic()))
 else:v.main(stage)
 print(json.dumps({'engine_correction_candidate':m['candidate_id'],'bound_inputs':len(m['inputs']),'scope':'authorized engine correction, isolated checks; accepted original manifests/reports preserved; live closure and installation not implied'}))
if __name__=='__main__':main(sys.argv[1])
