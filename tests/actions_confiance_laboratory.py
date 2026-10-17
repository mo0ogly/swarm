#!/usr/bin/env python3
"""Public fixed-corpus laboratory recipe using an isolated server and no provider."""
import argparse,hashlib,json,os,re,signal,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--evidence-dir',type=Path,required=True);p.add_argument('--timeout-seconds',type=int,default=120);a=p.parse_args();assert a.timeout_seconds>0
binary=a.binary.resolve();proof=a.evidence_dir.resolve();proof.mkdir(parents=True,exist_ok=True);checks=[];token='laboratory-admin-public-fixture'
def cli(root,namespace,op=None,body=None,origin=None,session='',expected=0,auth=token):
 argv=[str(binary),'--root',str(root),'--json']+(['--server',origin]if origin else [])+[namespace]+([op]if op else [])+(['--input','-']if body is not None else [])
 r=subprocess.run(argv,input=json.dumps(body)if body is not None else None,text=True,capture_output=True,timeout=a.timeout_seconds,env=dict(os.environ,SWARM_TRUST_TOKEN=auth,SWARM_WEB_SESSION=session))
 if r.returncode!=expected:
  fd=os.open(proof/'laboratory-cli-failure.json',os.O_CREAT|os.O_TRUNC|os.O_WRONLY,0o600)
  with os.fdopen(fd,'w')as f:json.dump({'namespace':namespace,'op':op,'exit':r.returncode,'expected':expected,'stdout':r.stdout,'stderr':r.stderr},f)
  raise AssertionError('public laboratory flow failed; private diagnostics retained')
 return json.loads(r.stdout if expected==0 else r.stderr)
def command(event,rev,**kw):return dict(schema_version=1,event_id=event,expected_revision=rev,reason='isolated fixed corpus public recipe',**kw)
with tempfile.TemporaryDirectory(prefix='swarm-laboratory-public-')as tmp:
 base=Path(tmp);root=base/'server';root.mkdir();absent=base/'client-absent';cli(root,'init');ids=[{'subject':s,'token':token if role=='admin'else s+'-laboratory-fixture','integrity':i,'roles':[role]}for s,i,role in [('admin',3,'admin'),('requester',1,'requester'),('approver',2,'approver'),('effect',2,'effect')]]
 cli(root,'trust','config-apply',dict(command('lab-config',0),policy={'mandate_ttl_seconds':3600,'unknown_reconcile_seconds':0,'evidence_retention_days':365,'require_separation':True},identities=ids))
 log=base/'server.log';fd=os.open(log,os.O_CREAT|os.O_WRONLY,0o600);server=subprocess.Popen([str(binary),'--root',str(root),'web','127.0.0.1:0'],stdout=fd,stderr=fd,start_new_session=True);os.close(fd)
 try:
  deadline=time.monotonic()+a.timeout_seconds
  while '/session/'not in log.read_text():
   assert server.poll()is None and time.monotonic()<deadline,'owned fixture startup failed';time.sleep(.05)
  m=re.search(r'http://(127\.0\.0\.1:[0-9]+)/session/(session-[a-zA-Z0-9]+)',log.read_text());assert m;origin='http://'+m[1];session=m[2]
  def remote(op,body=None,**kw):return cli(absent,'laboratory',op,body,origin,session,**kw)
  initial=remote('show');assert initial['policy']=={'enabled':False,'timeout_seconds':30,'max_concurrent_runs':1} and not initial['production_effects'];checks.append('disabled policy and fixed corpus visible through authenticated remote client without local root')
  policy={'enabled':True,'timeout_seconds':20,'max_concurrent_runs':2};body=command('lab-enable',initial['configuration_revision'],policy=policy);state=root/'.swarm/trust-actions.json';before=state.read_bytes();remote('policy-preview',body);assert state.read_bytes()==before;applied=remote('policy-apply',body);assert remote('policy-apply',body)==applied;view=remote('show');assert view['policy']==policy;checks.append('Admin preview does not mutate; non-default timeout/concurrency persist and exact policy replay is idempotent')
  input={'corpus_version':view['corpus']['version'],'corpus_digest':view['corpus_digest'],'seed':'18446744073709551615','scenario_ids':view['corpus']['scenario_ids'],'connector_id':'local-ledger-simulation-v1'}
  for variant in ['production','path','seed-number','corpus']:
   bad=json.loads(json.dumps(input))
   if variant=='production':bad['connector_id']='production'
   elif variant=='path':bad['fixture_root']=str(root)
   elif variant=='seed-number':bad['seed']=42
   else:bad['corpus_digest']='invalid'
   before=state.read_bytes();remote('run',command('bad-'+variant,view['configuration_revision'],input=bad),expected=2);assert state.read_bytes()==before
  checks.append('production connector, caller path, numeric seed and wrong version digest refuse before mutation')
  before=state.read_bytes();remote('run',command('wrong-role',view['configuration_revision'],input=input),auth='requester-laboratory-fixture',expected=4);assert state.read_bytes()==before;checks.append('requester cannot run laboratory; current I3 authority required')
  body=command('lab-run-public',view['configuration_revision'],input=input);run=remote('run',body);assert run['status']=='passed' and len(run['result']['cases'])==5 and all(c['passed']for c in run['result']['cases']);assert len(run['result']['containment']['checks'])==14 and all(run['result']['containment']['checks'].values());checks.append('five native corpus scenarios and fourteen actual OS boundary assertions pass in isolated helper')
  before=state.read_bytes();assert remote('run',body)==run and state.read_bytes()==before;checks.append('same command returns historical result without new simulation or duplicate attempt')
  replay=remote('replay',command('lab-replay-public',view['configuration_revision'],run_id=run['run_id']));assert replay['replay_of']==run['run_id'] and replay['result']['deterministic_digest']==run['result']['deterministic_digest'];checks.append('explicit replay creates attributed separate attempt with identical deterministic result')
  reopened=cli(root,'laboratory','result',{'run_id':run['run_id']});assert reopened==run;assert remote('show')['consumption']=={'active_runs':0,'attempts':2};assert not absent.exists();assert token.encode()not in state.read_bytes() and b'fixture-private-value'not in json.dumps(run).encode();checks.append('reopen retains inputs, corpus, policy and results; attempts preserved without raw credentials or local client fallback')
 finally:
  if server.poll()is None:
   os.killpg(server.pid,signal.SIGTERM)
   try:server.wait(timeout=5)
   except subprocess.TimeoutExpired:os.killpg(server.pid,signal.SIGKILL);server.wait(timeout=5)
(proof/'laboratory-public.json').write_text(json.dumps({'checks':checks,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'at':time.time(),'simulation':True},indent=2)+'\n');print(json.dumps({'checks':len(checks),'simulation':True}))
