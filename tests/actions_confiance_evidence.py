#!/usr/bin/env python3
"""Isolated public signing-key lifecycle; no external account or production effect."""
import argparse,hashlib,json,os,re,selectors,signal,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--archive',action='store_true');p.add_argument('--checkpoint',action='store_true');p.add_argument('--binary',type=Path,required=True);p.add_argument('--evidence-dir',type=Path,required=True);p.add_argument('--timeout-seconds',type=int,default=120);a=p.parse_args();assert a.timeout_seconds>0
binary=a.binary.resolve();proof=a.evidence_dir.resolve();proof.mkdir(parents=True,exist_ok=True);checks=[]
tokens={'admin':'admin-evidence-public-fixture','admin-two':'admin-two-evidence-public-fixture','alice':'alice-evidence-public-fixture','bob':'bob-evidence-public-fixture','effect':'effect-evidence-public-fixture'}
def call(root,family,op=None,body=None,who='admin',expected=0,origin=None,session=''):
 argv=[str(binary),'--root',str(root),'--json']+(['--server',origin] if origin else [])+[family]+([op] if op else [])+(['--input','-'] if body is not None else [])
 r=subprocess.run(argv,input=json.dumps(body) if body is not None else None,text=True,capture_output=True,timeout=a.timeout_seconds,env=dict(os.environ,SWARM_TRUST_TOKEN=tokens[who] if who else '',SWARM_WEB_SESSION=session))
 if r.returncode!=expected:
  f=proof/'evidence-cli-failure.json';fd=os.open(f,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
  with os.fdopen(fd,'w')as stream:json.dump({'family':family,'op':op,'expected':expected,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr},stream)
  raise AssertionError('public evidence command failed; private diagnostics retained')
 return json.loads(r.stdout if expected==0 else r.stderr)
def command(event,**kw):return dict(schema_version=1,event_id=event,**kw)
def start(root,address):
 child=subprocess.Popen([str(binary),'--root',str(root),'web',address],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,start_new_session=True);sel=selectors.DefaultSelector();sel.register(child.stdout,selectors.EVENT_READ);log=b'';end=time.monotonic()+a.timeout_seconds
 try:
  while b'/session/'not in log:
   remaining=end-time.monotonic();assert remaining>0 and sel.select(remaining),'owned fixture startup timeout';chunk=os.read(child.stdout.fileno(),8192);assert chunk,'owned fixture exited';log+=chunk
  m=re.search(rb'http://(127\.0\.0\.1:[0-9]+)/session/(session-[a-zA-Z0-9]+)',log);assert m;return child,'http://'+m[1].decode(),m[2].decode()
 except BaseException:
  stop(child);raise
 finally:sel.close()
def stop(child):
 if child.poll()is None:
  os.killpg(child.pid,signal.SIGTERM)
  try:child.wait(timeout=5)
  except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait(timeout=5)
 child.stdout.close()
with tempfile.TemporaryDirectory(prefix='swarm-evidence-public-')as fixture:
 base=Path(fixture);root=base/'server';root.mkdir();absent=base/'client-absent';call(root,'init');policy={'enabled':True,'key_ttl_seconds':3600}
 identities=[{'subject':s,'token':tokens[k],'integrity':i,'roles':[r]}for k,s,i,r in [('admin','admin',3,'admin'),('admin-two','admin-two',3,'admin'),('alice','alice',1,'requester'),('bob','bob',2,'approver'),('effect','effect-service',2,'effect')]]
 call(root,'trust','config-apply',command('evidence-config',expected_revision=0,reason='isolated evidence lifecycle recipe',policy={'mandate_ttl_seconds':3600,'unknown_reconcile_seconds':0,'evidence_retention_days':365,'require_separation':True},identities=identities,evidence_signing=policy,process_policy={'enabled':True},data_policy={'enabled':True,'classifications':['internal'],'destinations':['local-process-state'],'providers':['swarm'],'tools':['process-workflow'],'resources':[{'resource':'invoice/acme','classification':'internal'}]}));server,origin,session=start(root,'127.0.0.1:0')
 try:
  def remote(op,body=None,who='admin',expected=0):return call(absent,'evidence',op,body,who,expected,origin,session)
  def prepare(event):
   v=remote('show');return remote('key-prepare',command(event,expected_registry_revision=v['registry_revision'],reason='prepare isolated public signature key'))
  def activate(v,event):
   k=v['key'];return remote('key-activate',command(event,expected_registry_revision=v['registry_revision'],key_id=k['key_id'],key_digest=k['key_digest'],reason='independent activation of exact key'),'admin-two')
  def anchor():
   v=remote('show');return remote('anchor',{'key_id':v['active_key_id'],'expected_registry_revision':v['registry_revision'],'expected_sequence':v['last_sequence'],'expected_hash':v['last_hash']})
  def process(op,body,who='admin'):return call(absent,'process',op,body,who,0,origin,session)
  label=lambda fr,en:{'fr':fr,'en':en}
  definition={'schema_version':1,'id':'evidence-case','label':label('Revue locale','Local review'),'start':'draft','nodes':[{'id':'draft','label':label('À valider','To approve'),'terminal':False},{'id':'done','label':label('Validé','Approved'),'terminal':True}],'transitions':[{'id':'approve','from':'draft','to':'done','label':label('Valider','Approve'),'kind':'approval','roles':['approver'],'minimum_integrity':2}]}
  d=process('definition-publish',command('proof-process-definition',definition=definition,expected_version=0,reason='publish isolated process for proof'))
  case=process('case-create',command('proof-process-case',definition_id=d['definition_id'],definition_version=d['version'],definition_digest=d['content_digest'],participants=['bob'],resource='invoice/acme',data_context={'classification':'internal','destination':'local-process-state','provider':'swarm','tool':'process-workflow'},title='Evidence consumer fixture',reference='PROOF-CASE-001'),'alice')
  process('case-transition',command('proof-process-approval',case_id=case['id'],expected_revision=case['revision'],definition_digest=case['definition_digest'],input_digest=case['input_digest'],transition_id='approve',reason='independent process content review'),'bob')
  first=prepare('prepare-one');k=first['key'];before=(root/'.swarm/trust-actions.json').read_bytes();bad=command('activate-one',expected_registry_revision=first['registry_revision'],key_id=k['key_id'],key_digest=k['key_digest'],reason='exact independent key activation');remote('key-activate',bad,expected=4);assert before==(root/'.swarm/trust-actions.json').read_bytes();activate(first,'activate-one');checks.append('preparer cannot activate; second I3 admin activates exact key without denied mutation')
  pin=remote('show')['keys'][k['key_id']];bundle=anchor();events=[e for e in bundle['export']['evidence']if e['kind'].startswith('process-')];assert len(events)==3 and all(e['identity_context']['principal_id']==e['actor']for e in events);checks.append('public signature and verifier consume attributed process definition/case/approval evidence');(base/'independent-public-pin.json').write_text(json.dumps(pin))
  verify={'export':bundle['export'],'anchor':bundle['anchor'],'trusted_key':json.load((base/'independent-public-pin.json').open())};assert call(absent,'evidence','verify',verify,who=None)['verified'];assert not absent.exists();checks.append('public verification uses separately acquired pin without project root or private key')
  for name in ['truncation','payload','signature','wrong-key']:
   value=json.loads(json.dumps(verify))
   if name=='truncation':value['export']['evidence'].pop()
   elif name=='payload':value['export']['evidence'][0]['reason']='altered'
   elif name=='signature':value['anchor']['signature']='invalid'
   else:value['trusted_key']['key_id']='untrusted-key'
   call(absent,'evidence','verify',value,who=None,expected=2)
  checks.append('truncation, payload alteration, wrong signature and wrong pinned key are refused')
  if a.archive:
   snapshot=remote('show');body=command('archive-public-one',expected_revision=snapshot['configuration_revision'],expected_sequence=bundle['anchor']['sequence'],expected_hash=bundle['anchor']['hash'],anchor=bundle['anchor'],reason='archive exact signed prefix without deleting originals')
   before=(root/'.swarm/trust-actions.json').read_bytes();preview=remote('archive-preview',body);assert before==(root/'.swarm/trust-actions.json').read_bytes();assert not (root/'.swarm/trust-archives').exists();receipt=remote('archive-create',body);assert receipt['archive_id']==preview['archive_id'];assert remote('archive-create',body)==receipt
   stored=remote('archive-show',{'archive_id':receipt['archive_id']});assert stored['bundle']['export']==bundle['export'] and stored['protection']['protected'];checks.append('archive preview creates nothing; exact signed prefix copied once and returned identically after replay')
   before=(root/'.swarm/trust-actions.json').read_bytes();remote('archive-release',command('archive-release-early',expected_revision=snapshot['configuration_revision'],archive_id=receipt['archive_id'],reason='early release must preserve protected originals'),expected=4);assert before==(root/'.swarm/trust-actions.json').read_bytes();checks.append('minimum retention rejects early release without mutation or deletion')
  if a.checkpoint:
   witness=base/'independent-witness';witness.mkdir();checkpoint={'anchor':bundle['anchor'],'trusted_key':pin};(witness/'checkpoint.json').write_text(json.dumps(checkpoint));checkpoint=json.loads((witness/'checkpoint.json').read_text())
   checked=call(absent,'evidence','verify',dict(verify,trusted_checkpoint=checkpoint),who=None);assert checked['checkpoint_verified'] and checked['checkpoint_sequence']==checkpoint['anchor']['sequence'];checks.append('separate witness checkpoint verifies exact history without local project root')
  second=prepare('prepare-two');activate(second,'activate-two');retired=remote('show')['keys'][k['key_id']];assert retired['state']=='retired';verify['trusted_key']=retired;assert call(absent,'evidence','verify',verify,who=None)['verified'];checks.append('rotation preserves historical verification with explicitly retired key')
  if a.checkpoint:
   latest=anchor();latest_view=remote('show');latest_pin=latest_view['keys'][latest_view['active_key_id']];old_checkpoint={'anchor':bundle['anchor'],'trusted_key':retired};new_checkpoint={'anchor':latest['anchor'],'trusted_key':latest_pin}
   checked=call(absent,'evidence','verify',dict(latest,trusted_key=latest_pin,trusted_checkpoint=old_checkpoint),who=None);assert checked['checkpoint_verified'];checks.append('rotated key accepts extension containing independently pinned earlier history')
   before=(root/'.swarm/trust-actions.json').read_bytes();call(absent,'evidence','verify',dict(verify,trusted_checkpoint=new_checkpoint),who=None,expected=2)
   remote('verify',dict(verify,trusted_checkpoint=new_checkpoint),who=None,expected=2);assert before==(root/'.swarm/trust-actions.json').read_bytes();checks.append('valid older signed bundle is refused below independent checkpoint via CLI and API without mutation')
  second_bundle=anchor();view=remote('show');key2=view['keys'][view['active_key_id']];remote('key-revoke',command('revoke-two',expected_registry_revision=view['registry_revision'],key_id=key2['key_id'],key_digest=key2['key_digest'],reason='immediate attributed compromise withdrawal'));revoked=remote('show')['keys'][key2['key_id']];assert revoked['state']=='revoked';call(absent,'evidence','verify',dict(second_bundle,trusted_key=revoked),who=None,expected=4);checks.append('revoked key blocks trust acceptance, distinct from historical cryptographic validity')
  if a.archive:
   before=(root/'.swarm/trust-actions.json').read_bytes();remote('archive-show',{'archive_id':receipt['archive_id']},who='alice',expected=4);assert before==(root/'.swarm/trust-actions.json').read_bytes();checks.append('archive reads require current I3 authority and denied reads do not mutate state')
  third=prepare('prepare-three');activate(third,'activate-three');before_restart=remote('show');third_bundle=anchor();key3=before_restart['active_key_id'];stop(server);server,origin,session=start(root,origin.removeprefix('http://'));v=remote('show');assert v['active_key_id']==key3 and not v['material_available'][key3];before=(root/'.swarm/trust-actions.json').read_bytes();remote('anchor',{'key_id':key3,'expected_registry_revision':v['registry_revision'],'expected_sequence':v['last_sequence'],'expected_hash':v['last_hash']},expected=4);assert before==(root/'.swarm/trust-actions.json').read_bytes();checks.append('server restart loses only private material; old registry preserved and new signature fails closed')
  if a.archive:
   reopened_archive=remote('archive-show',{'archive_id':receipt['archive_id']});assert reopened_archive['bundle']['export']==bundle['export'] and reopened_archive['protection']['protected'];checks.append('restart preserves archive and minimum protection without private signing material')
  fourth=prepare('prepare-four');activate(fourth,'activate-four');assert anchor()['anchor']['key_id']==fourth['key']['key_id'];checks.append('new dual controlled rotation restores signing after restart')
  current=remote('show');remote('policy-apply',command('disable-signing',expected_revision=current['configuration_revision'],policy={'enabled':False,'key_ttl_seconds':3600},reason='disable signing before emergency withdrawal'));current=remote('show');key=current['keys'][fourth['key']['key_id']];remote('key-revoke',command('disabled-policy-revoke',expected_registry_revision=current['registry_revision'],key_id=key['key_id'],key_digest=key['key_digest'],reason='emergency withdrawal while signing disabled'));assert remote('show')['keys'][key['key_id']]['state']=='revoked';checks.append('emergency key withdrawal remains available while public signing policy is disabled')
  try:fd=os.open('/proc/'+str(server.pid)+'/mem',os.O_RDONLY)
  except PermissionError:checks.append('protected signing server memory cannot be opened by same UID parent')
  else:os.close(fd);raise AssertionError('signing server remained dumpable')
  assert not absent.exists();assert remote('show')['private_keys_persisted']is False;checks.append('CLI uses persistent server exclusively; no local root fallback or private key export')
 finally:stop(server)
(proof/'evidence-public.json').write_text(json.dumps({'checks':checks,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'at':time.time(),'simulation':True},indent=2)+'\n');print(json.dumps({'checks':len(checks),'simulation':True}))
