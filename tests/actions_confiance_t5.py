#!/usr/bin/env python3
"""Public T5 recipe: isolated CLI state and independently built web application."""
import argparse,hashlib,json,os,signal,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--criterion',type=int,choices=range(5),required=True);p.add_argument('--root',type=Path,required=True);p.add_argument('--timeout-seconds',type=int,required=True);p.add_argument('--binary',type=Path);p.add_argument('--evidence-dir',type=Path);a=p.parse_args();assert a.timeout_seconds>0
repo=a.root.resolve();tmp=tempfile.TemporaryDirectory(prefix='swarm-process-public-');base=Path(tmp.name);binary=a.binary.resolve() if a.binary else base/'swarm';proof=a.evidence_dir.resolve() if a.evidence_dir else base/'proof';proof.mkdir(parents=True,exist_ok=True);cases=[]
def execute(argv,**kwargs):
 child=subprocess.Popen(argv,cwd=repo,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True,**kwargs)
 try:out,err=child.communicate(timeout=a.timeout_seconds)
 except subprocess.TimeoutExpired:
  os.killpg(child.pid,signal.SIGTERM)
  try:child.communicate(timeout=5)
  except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.communicate()
  raise AssertionError('owned recipe process group timed out and was stopped')
 return child.returncode,out,err
if not a.binary:
 code,out,err=execute(['sh','build.sh',str(binary)]);assert code==0,'canonical build failed'
root=base/'workspace';root.mkdir();tokens={'admin':'admin-process-public-fixture','alice':'alice-process-public-fixture','bob':'bob-process-public-fixture','effect-service':'effect-process-public-fixture'}
def cli(family,op=None,body=None,who='admin',expected=0):
 argv=[str(binary),'--root',str(root),'--json',family]+([op] if op else [])+(['--input','-'] if body is not None else [])
 child=subprocess.run(argv,cwd=repo,input=json.dumps(body) if body is not None else None,text=True,capture_output=True,timeout=a.timeout_seconds,env=dict(os.environ,SWARM_TRUST_TOKEN=tokens[who]))
 if child.returncode!=expected:
  f=proof/'process-cli-failure.json';f.write_text(json.dumps({'family':family,'operation':op,'exit':child.returncode,'stdout':child.stdout,'stderr':child.stderr}));f.chmod(0o600);raise AssertionError('public CLI failed; private diagnostics retained')
 return json.loads(child.stdout if expected==0 else child.stderr)
def command(event,**fields):return dict(schema_version=1,event_id=event,**fields)
label=lambda fr,en:{'fr':fr,'en':en}
doc={'schema_version':1,'id':'review','label':label('Revue locale','Local review'),'start':'new','nodes':[{'id':'new','label':label('Nouveau','New'),'terminal':False},{'id':'review','label':label('À valider','To approve'),'terminal':False},{'id':'done','label':label('Validé','Approved'),'terminal':True}],'transitions':[{'id':'submit','from':'new','to':'review','label':label('Soumettre','Submit'),'kind':'advance','roles':['requester'],'minimum_integrity':1},{'id':'approve','from':'review','to':'done','label':label('Valider','Approve'),'kind':'approval','roles':['approver'],'minimum_integrity':2},{'id':'reopen','from':'done','to':'review','label':label('Rouvrir','Reopen'),'kind':'exception','roles':['admin'],'minimum_integrity':3}]}
data={'enabled':True,'classifications':['internal'],'destinations':['local-process-state'],'providers':['swarm'],'tools':['process-workflow'],'resources':[{'resource':'invoice/acme','classification':'internal'}]}
if a.criterion in (0,1,2,3):
 cli('init');identities=[{'subject':s,'token':tokens[s],'integrity':i,'roles':[r]} for s,i,r in [('admin',3,'admin'),('alice',1,'requester'),('bob',2,'approver'),('effect-service',2,'effect')]]
 cli('trust','config-apply',command('setup',expected_revision=0,reason='isolated public process recipe',policy={'mandate_ttl_seconds':3600,'unknown_reconcile_seconds':0,'evidence_retention_days':365,'require_separation':True},identities=identities,process_policy={'enabled':True},data_policy=data))
 published=cli('process','definition-publish',command('publish-one',definition=doc,expected_version=0,reason='publish isolated first version'))
 create=command('create-case',definition_id='review',definition_version=1,definition_digest=published['content_digest'],participants=['bob'],resource='invoice/acme',data_context={'classification':'internal','destination':'local-process-state','provider':'swarm','tool':'process-workflow'},title='Local invoice',reference='FIXTURE-001')
 c=cli('process','case-create',create,'alice');cli('process','definition-publish',command('publish-two',definition=doc,expected_version=1,reason='publish isolated second version'));read=lambda:cli('process','case-show',{'case_id':c['id']},'alice')
 assert read()['definition_version']==1 and len(cli('process','show')['definitions']['review'])==2
 if a.criterion in (0,1):cases.append({'id':'published-version-and-pinned-case','status':'pass','assertions':3,'observed':{'published_versions':2,'case_version':1,'input_digest':c['input_digest']}})
 if a.criterion in (0,2,3):
  transition=lambda event,rev,edge,reason='':command(event,case_id=c['id'],expected_revision=rev,definition_digest=c['definition_digest'],input_digest=c['input_digest'],transition_id=edge,reason=reason)
  cli('process','case-transition',transition('submit',1,'submit'),'alice');before=(root/'.swarm/trust-actions.json').read_bytes();approve=transition('approve',2,'approve','independent exact review')
  cli('process','case-transition',approve,'alice',4);assert before==(root/'.swarm/trust-actions.json').read_bytes();cli('process','case-transition',approve,'bob');reopen=transition('reopen',3,'reopen','authorized documented exception');cli('process','case-transition',reopen,'bob',4);cli('process','case-transition',reopen);after=read();assert after['node_id']=='review' and after['revision']==4 and [h['actor'] for h in after['history']]==['alice','bob','admin'];assert len(cli('trust','show')['ledger'])==0
  if a.criterion in (0,2):cases.append({'id':'attributed-approval-and-exception','status':'pass','assertions':5,'observed':{'revision':4,'actors':['alice','bob','admin'],'payment_effects':0}})
  if a.criterion in (0,3):
   disabled=dict(data,enabled=False);cli('process','policy-apply',command('withdraw-admission',expected_revision=1,policy={'enabled':True},data_policy=disabled,reason='withdraw isolated data admission'));before=(root/'.swarm/trust-actions.json').read_bytes();cli('process','case-create',create,'alice',4);cli('process','case-show',{'case_id':c['id']},'alice',4);assert before==(root/'.swarm/trust-actions.json').read_bytes();cli('process','policy-apply',command('restore-admission',expected_revision=2,policy={'enabled':True},data_policy=data,reason='restore isolated data admission'));assert read()['history']==after['history'];cases.append({'id':'current-admission-replay-and-reopen','status':'pass','assertions':4,'observed':{'withdrawn_reads_and_replays':'refused_without_mutation','history_restored':True}})
if a.criterion in (0,4):
 browser_dir=proof/'browser';code,out,err=execute(['node','tests/actions_confiance_process_ui.cjs',str(binary),str(browser_dir)])
 (proof/'browser.stdout').write_bytes(out);(proof/'browser.stderr').write_bytes(err)
 assert code==0,'browser recipe failed; retained diagnostics identify the fixture'
 v=json.load((browser_dir/'process-ui.json').open());assert not v['unexpected_diagnostics'] and v['checks'];cases.append({'id':'separate-process-frontend','status':'pass','assertions':len(v['checks']),'observed':{'independent_build':True,'checks':v['checks'],'unexpected_diagnostics':0,'expected_denials':v['expected_denials']}})
assert cases
print(json.dumps({'version':1,'task':'T5','criterion':a.criterion,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'cases':cases},ensure_ascii=False,separators=(',',':')))
