#!/usr/bin/env python3
"""Public versioned connector contract recipe using an isolated server and no provider."""
import argparse,hashlib,json,os,re,signal,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--evidence-dir',type=Path,required=True);p.add_argument('--timeout-seconds',type=int,default=120);a=p.parse_args();assert a.timeout_seconds>0
binary=a.binary.resolve();proof=a.evidence_dir.resolve();proof.mkdir(parents=True,exist_ok=True);checks=[];token='connector-admin-public-fixture'
def cli(root,namespace,op=None,body=None,origin=None,session='',expected=0,auth=token):
 argv=[str(binary),'--root',str(root),'--json']+(['--server',origin]if origin else [])+[namespace]+([op]if op else [])+(['--input','-']if body is not None else [])
 r=subprocess.run(argv,input=json.dumps(body)if body is not None else None,text=True,capture_output=True,timeout=a.timeout_seconds,env=dict(os.environ,SWARM_TRUST_TOKEN=auth,SWARM_WEB_SESSION=session))
 if r.returncode!=expected:
  fd=os.open(proof/'connector-cli-failure.json',os.O_CREAT|os.O_TRUNC|os.O_WRONLY,0o600)
  with os.fdopen(fd,'w')as f:json.dump({'namespace':namespace,'op':op,'exit':r.returncode,'expected':expected,'stdout':r.stdout,'stderr':r.stderr},f)
  raise AssertionError('public connector flow failed; private diagnostics retained')
 return json.loads(r.stdout if expected==0 else r.stderr)
def command(event,rev,**kw):return dict(schema_version=1,event_id=event,expected_revision=rev,reason='isolated fixed corpus public recipe',**kw)
with tempfile.TemporaryDirectory(prefix='swarm-connector-public-')as tmp:
 base=Path(tmp);root=base/'server';root.mkdir();absent=base/'client-absent';cli(root,'init');ids=[{'subject':s,'token':token if role=='admin'else s+'-connector-fixture','integrity':i,'roles':[role]}for s,i,role in [('admin',3,'admin'),('requester',1,'requester'),('approver',2,'approver'),('effect',2,'effect')]]
 cli(root,'trust','config-apply',dict(command('lab-config',0),policy={'mandate_ttl_seconds':3600,'unknown_reconcile_seconds':0,'evidence_retention_days':365,'require_separation':True},identities=ids))
 log=base/'server.log';fd=os.open(log,os.O_CREAT|os.O_WRONLY,0o600);server=subprocess.Popen([str(binary),'--root',str(root),'web','127.0.0.1:0'],stdout=fd,stderr=fd,start_new_session=True);os.close(fd)
 try:
  deadline=time.monotonic()+a.timeout_seconds
  while '/session/'not in log.read_text():
   assert server.poll()is None and time.monotonic()<deadline,'owned fixture startup failed';time.sleep(.05)
  m=re.search(r'http://(127\.0\.0\.1:[0-9]+)/session/(session-[a-zA-Z0-9]+)',log.read_text());assert m;origin='http://'+m[1];session=m[2]
  def remote(op,body=None,**kw):return cli(absent,'trust',op,body,origin,session,**kw)
  state=root/'.swarm/trust-actions.json'
  view=remote('show');catalog=view['connector_catalog'];assert catalog['schema_version']==1 and not catalog['arbitrary_plugins'];assert len(catalog['connectors'])==2 and all(c['simulation']and c['readback']and c['idempotency']and not c['external_effects']for c in catalog['connectors']);checks.append('closed versioned catalog exposes actual local capabilities without provider claims')
  data_policy={'enabled':True,'classifications':['internal'],'destinations':['local-ledger','local-access-state'],'providers':['swarm'],'tools':['simulated-settlement','simulated-access-grant'],'resources':[{'resource':'invoice/connector','classification':'internal'},{'resource':'access/connector','classification':'internal'}]}
  remote('config-apply',command('connector-data',view['revision'],policy=view['policy'],identities=ids,data_policy=data_policy));assert remote('show')['data_policy']==data_policy;checks.append('shared Admin data policy persists both exact simulated paths');identities_before=remote('show')['identities']
  for access in [False,True]:
   suffix='access'if access else'ledger';resource=suffix+'/connector'if access else'invoice/connector';mid='mandate-'+suffix;selection={'id':'local-access-simulation','version':1,'access':{'subject':'employee','role':'reader'}}if access else None
   fields={'resource':resource,**({'connector':selection}if access else{'max_amount_minor':1000,'currency':'EUR','beneficiary':'ACME'})}
   remote('mandate-create',command('mandate-'+suffix,0,mandate_id=mid,subject='requester',actions=['grant-access'if access else'settle'],**fields))
   request=command('request-'+suffix,0,mandate_id=mid,idempotency_key='key-'+suffix,**{k:v for k,v in fields.items()if k!='max_amount_minor'},**({}if access else{'amount_minor':100}),data_context={'classification':'internal','destination':'local-access-state'if access else'local-ledger','provider':'swarm','tool':'simulated-access-grant'if access else'simulated-settlement'})
   if access:
    bad=json.loads(json.dumps(request));bad['event_id']='wrong-path';bad['data_context']['destination']='local-ledger';before=state.read_bytes();remote('request',bad,auth='requester-connector-fixture',expected=4);assert state.read_bytes()==before;checks.append('access refuses settlement destination before storing original or action')
   action=remote('request',request,auth='requester-connector-fixture');assert action['state']=='pending_approval'
   bypass=remote('commit',command('bypass-'+suffix,0,action_id=action['id']),auth='effect-connector-fixture');assert bypass['state']=='denied';checks.append(suffix+': direct effect without approval refused')
   approval=command('approve-'+suffix,0,action_id=action['id'],content_digest=action['content_digest'],mandate_id=mid,amount_minor=action['amount_minor'],currency=action['currency'],beneficiary=action['beneficiary'],**({'connector':selection}if access else{}))
   approved=remote('approve',approval,auth='approver-connector-fixture');assert approved['state']=='approved'
   committed=remote('commit',command('commit-'+suffix,0,action_id=action['id'],lose_response=True),auth='effect-connector-fixture');assert committed['state']=='unknown'
   assert remote('commit',command('retry-'+suffix,0,action_id=action['id']),auth='effect-connector-fixture')['state']=='unknown';view=remote('show');assert len(view['ledger'])==1;assert len(view['access_ledger'])==(1 if access else 0)
   receipt=view['effect_receipts']['key-'+suffix];assert receipt['intent']['action_digest']==action['content_digest'] and receipt['intent']['approval_digests'] and receipt['result']['reconciliation_required'];checks.append(suffix+': admitted intent, exact approval and unknown state preserve one native effect')
   assert cli(root,'trust','show')['effect_receipts']==view['effect_receipts'];result=remote('reconcile',command('reconcile-'+suffix,0,action_id=action['id']),auth='effect-connector-fixture');assert result['state']=='reconciled';receipt=remote('show')['effect_receipts']['key-'+suffix];assert receipt['result']['state']=='reconciled'and not receipt['result']['reconciliation_required'];checks.append(suffix+': readback after independent Store reopen confirms result without duplication')
  for i,selection in enumerate([None,{'id':'production','version':1},{'id':'local-access-simulation','version':1,'capabilities':{'external_effects':True}},{'id':'local-access-simulation','version':2,'access':{'subject':'a','role':'reader'}}]):
   before=state.read_bytes();remote('mandate-create',command('invalid-'+str(i),0,connector=selection),expected=2);assert state.read_bytes()==before
  checks.append('null, unknown implementation, unsupported version and capability overstatement refused without mutation')
  view=remote('show');assert len(view['ledger'])==1 and len(view['access_ledger'])==1;assert view['identities']==identities_before;assert not absent.exists();assert b'employee' in state.read_bytes() and token.encode()not in state.read_bytes();checks.append('distinct simulated registries persist without changing real identities or creating client root')
 finally:
  if server.poll()is None:
   os.killpg(server.pid,signal.SIGTERM)
   try:server.wait(timeout=5)
   except subprocess.TimeoutExpired:os.killpg(server.pid,signal.SIGKILL);server.wait(timeout=5)
(proof/'connector-public.json').write_text(json.dumps({'checks':checks,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'at':time.time(),'simulation':True},indent=2)+'\n');print(json.dumps({'checks':len(checks),'simulation':True}))
