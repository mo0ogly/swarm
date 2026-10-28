#!/usr/bin/env python3
import argparse,base64,hashlib,hmac,json,os,signal,subprocess,tempfile,uuid
from pathlib import Path

p=argparse.ArgumentParser();p.add_argument('--criterion',type=int,choices=range(5),required=True);p.add_argument('--root',type=Path,required=True);p.add_argument('--timeout-seconds',type=int,required=True);p.add_argument('--evidence-dir',type=Path);p.add_argument('--binary',type=Path);a=p.parse_args();repo=a.root.resolve()
if not (repo/'internal/engine/trust_actions.go').is_file():raise SystemExit('candidate T2 source missing')
tmp=tempfile.TemporaryDirectory(prefix='swarm-t2-recipe-');base=Path(tmp.name);binary=a.binary.resolve() if a.binary else base/'swarm'
if not a.binary:subprocess.run(['sh','build.sh',str(binary)],cwd=repo,check=True,stdout=subprocess.DEVNULL,timeout=a.timeout_seconds)
assert binary.is_file() and os.access(binary,os.X_OK),'executable binary required'
def bounded_browser(argv,timeout):
 child=subprocess.Popen(argv,cwd=repo,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
 try:stdout,stderr=child.communicate(timeout=timeout)
 except subprocess.TimeoutExpired:
  os.killpg(child.pid,signal.SIGTERM)
  try:stdout,stderr=child.communicate(timeout=5)
  except subprocess.TimeoutExpired:
   os.killpg(child.pid,signal.SIGKILL);stdout,stderr=child.communicate()
  raise AssertionError('browser recipe timed out; owned process group terminated')
 return subprocess.CompletedProcess(argv,child.returncode,stdout,stderr)
TOKENS={'admin':'admin-token-public-recipe','alice':'requester-token-recipe','bob':'approver-token-recipe','effect':'effect-token-public-recipe'}
CLI_RUNS=[]
def event():return uuid.uuid4().hex
def call(root,action,data=None,token=None,ok=True):
 env=dict(os.environ);env['SWARM_TRUST_TOKEN']=token or ''
 argv=[str(binary),'--root',str(root),'--json','trust',action]
 if data is not None:argv+=['--input','-']
 child=subprocess.Popen(argv,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=env)
 try:stdout,stderr=child.communicate(json.dumps(data) if data is not None else None,timeout=a.timeout_seconds)
 except subprocess.TimeoutExpired:
  child.kill();child.communicate();raise
 r=subprocess.CompletedProcess(argv,child.returncode,stdout,stderr);CLI_RUNS.append({'operation':action,'pid':child.pid,'exited':child.poll() is not None})
 if ok and r.returncode:raise AssertionError(action+': '+r.stderr)
 if not ok:return r
 return json.loads(r.stdout)
def config(root,preview=False,revision=0,event_id=None):
 root.mkdir(parents=True,exist_ok=True)
 if not (root/'.swarm/state.db').is_file():
  initialized=subprocess.run([str(binary),'--root',str(root),'--json','init'],text=True,capture_output=True,timeout=a.timeout_seconds)
  assert initialized.returncode==0,initialized.stderr
  assert (root/'.swarm/state.db').is_file(),'public init did not persist fixture storage'
 data={'schema_version':1,'event_id':event_id or event(),'expected_revision':revision,'policy':{'mandate_ttl_seconds':3600,'unknown_reconcile_seconds':0,'evidence_retention_days':365,'require_separation':True},'reason':'configuration recette publique T2','identities':[{'subject':'admin','token':TOKENS['admin'],'integrity':3,'roles':['admin']},{'subject':'alice','token':TOKENS['alice'],'integrity':1,'roles':['requester']},{'subject':'bob','token':TOKENS['bob'],'integrity':2,'roles':['approver']},{'subject':'effect-service','token':TOKENS['effect'],'integrity':2,'roles':['effect']}]}
 return call(root,'config-preview' if preview else 'config-apply',data),data
def cmd(**kw):return {'schema_version':1,'event_id':event(),**kw}
def mandate(root,mid='mandate-recipe'):
 return call(root,'mandate-create',cmd(mandate_id=mid,subject='alice',actions=['settle'],resource='invoice/acme',max_amount_minor=50000,currency='EUR',beneficiary='ACME'),TOKENS['admin'])
def request(root,mid='mandate-recipe',key='invoice-42',amount=4200):return call(root,'request',cmd(actor='admin',integrity=3,mandate_id=mid,resource='invoice/acme',amount_minor=amount,currency='EUR',beneficiary='ACME',idempotency_key=key),TOKENS['alice'])
def approve(root,action,amount=None):return call(root,'approve',cmd(action_id=action['id'],mandate_id=action['mandate_id'],amount_minor=action['amount_minor'] if amount is None else amount,currency=action['currency'],beneficiary=action['beneficiary'],content_digest=action['content_digest']),TOKENS['bob'])
def case(cid,assertions,**observed):return {'id':cid,'status':'pass','assertions':assertions,'observed':observed}
def c1():
 root=base/'c1';config(root);mandate(root);before=call(root,'show');act=request(root);wrong=approve(root,act,act['amount_minor']+1);missing_digest=call(root,'approve',cmd(action_id=act['id'],mandate_id=act['mandate_id'],amount_minor=act['amount_minor'],currency=act['currency'],beneficiary=act['beneficiary']),TOKENS['bob']);assert wrong['state']=='denied' and missing_digest['state']=='denied' and len(call(root,'show')['ledger'])==0 and act['requester']=='alice';approve(root,act)
 denied=[]
 for op in ['direct-write','elevate-integrity']:denied.append(call(root,op,cmd(resource='official-policy'),TOKENS['alice'])['state'])
 missing=call(root,'request',cmd(mandate_id='absent',resource='invoice/acme',amount_minor=10,currency='EUR',beneficiary='ACME',idempotency_key='absent'),TOKENS['alice']);assert missing['state']=='denied' and denied==['denied','denied'] and len(call(root,'show')['ledger'])==0
 unauth=call(root,'request',cmd(mandate_id='mandate-recipe',resource='invoice/acme',amount_minor=10,currency='EUR',beneficiary='ACME',idempotency_key='unauth'),None,False);assert unauth.returncode==4
 replay_input=cmd(mandate_id='mandate-recipe',resource='invoice/acme',amount_minor=100,currency='EUR',beneficiary='ACME',idempotency_key='principal-replay')
 first=call(root,'request',replay_input,TOKENS['alice']);state_path=root/'.swarm/trust-actions.json';retained=state_path.read_bytes();normalized=call(root,'request',dict(replay_input,actor='untrusted-claim',integrity=3),TOKENS['alice']);assert normalized==first and state_path.read_bytes()==retained;foreign=call(root,'request',replay_input,TOKENS['bob'],False);assert foreign.returncode==4 and retained==state_path.read_bytes();same=call(root,'request',replay_input,TOKENS['alice']);assert same==first
 pending=request(root,key='revoked-before-approval');call(root,'mandate-revoke',cmd(mandate_id='mandate-recipe',reason='public recipe revokes before approval'),TOKENS['admin']);retained_action=call(root,'show')['actions'][pending['id']];revoked_approval=approve(root,pending);assert revoked_approval['state']=='denied' and call(root,'show')['actions'][pending['id']]==retained_action and len(call(root,'show')['ledger'])==0
 invalid=call(root,'mandate-create',cmd(mandate_id='bad-path',subject='alice',actions=['settle'],resource='../outside',max_amount_minor=50,currency='EUR',beneficiary='ACME'),TOKENS['admin'],False);assert invalid.returncode!=0
 epochroot=base/'c1-configuration-epoch';config(epochroot);mandate(epochroot);epoch_pending=request(epochroot,key='epoch-pending');epoch_ready=approve(epochroot,request(epochroot,key='epoch-approved'));old_actions=call(epochroot,'show')['actions'];changed,_=config(epochroot,revision=1);stale_approval=approve(epochroot,epoch_pending);stale_commit=call(epochroot,'commit',cmd(action_id=epoch_ready['id']),TOKENS['effect']);after_epoch=call(epochroot,'show');assert changed['revision']==2 and stale_approval['state']=='denied' and stale_commit['state']=='denied' and after_epoch['actions']==old_actions and len(after_epoch['ledger'])==0
 mandate(epochroot,mid='renewed-configuration');renewed=approve(epochroot,request(epochroot,mid='renewed-configuration',key='renewed-configuration-key'));renewed_commit=call(epochroot,'commit',cmd(action_id=renewed['id']),TOKENS['effect']);assert renewed_commit['state']=='committed' and len(call(epochroot,'show')['ledger'])==1
 return [case('identity-exact-approval-biba',18,configuration_revision=changed['revision'],stale_approval=stale_approval['state'],stale_commit=stale_commit['state'],renewed_mandate_committed=renewed_commit['state'],revoked_approval=revoked_approval['state'],requester=act['requester'],wrong_approval=wrong['state'],direct_denials=denied,ledger_before=len(before['ledger'])),case('invalid-and-unauthenticated-inputs',7,foreign_replay_exit=foreign.returncode,replay_state_unchanged=True,same_authority_replay=True,unauthenticated_exit=unauth.returncode,traversal_exit=invalid.returncode,missing_mandate=missing['reason'])]
def c2():
 root=base/'c2';config(root);mandate(root);act=approve(root,request(root));unknown=call(root,'commit',cmd(action_id=act['id'],lose_response=True),TOKENS['effect']);assert unknown['state']=='unknown';commit_process=CLI_RUNS[-1]
 for operation in ['commit','reconcile']:
  denied=call(root,operation,cmd(action_id=act['id'],resource='invoice/other'),TOKENS['effect']);assert denied['state']=='denied' and len(call(root,'show')['ledger'])==1
 reconciled=call(root,'reconcile',cmd(action_id=act['id']),TOKENS['effect']);reconcile_process=CLI_RUNS[-1];assert commit_process['exited'] and reconcile_process['exited'] and commit_process['pid']!=reconcile_process['pid'];same=call(root,'commit',cmd(action_id=act['id']),TOKENS['effect']);conflict=call(root,'request',cmd(mandate_id='mandate-recipe',resource='invoice/acme',amount_minor=4300,currency='EUR',beneficiary='ACME',idempotency_key='invoice-42'),TOKENS['alice'],False);view=call(root,'show');assert reconciled['state']=='reconciled' and same['id']==act['id'] and conflict.returncode==3 and len(view['ledger'])==1
 return [case('lost-response-restart-reconcile',10,contradictory_resource_denied=['commit','reconcile'],unknown=unknown['state'],reconciled=reconciled['state'],ledger_entries=len(view['ledger']),commit_process=commit_process,reconcile_process=reconcile_process),case('content-bound-idempotency',3,same_action=same['id'],conflict_exit=conflict.returncode)]
def c3():
 root=base/'c3';preview,change=config(root,True);assert not (root/'.swarm/trust-actions.json').exists();applied,_=config(root,event_id=change['event_id']);replayed=call(root,'config-apply',change);assert applied['revision']==replayed['revision']==1
 stale=dict(change,event_id=event(),reason='révision ancienne refusée');bad=call(root,'config-apply',stale,None,False);assert bad.returncode==3
 recovery_root=base/'c3-interrupted-publication';_,initial_change=config(recovery_root);state_path=recovery_root/'.swarm/trust-actions.json';key_path=recovery_root/'.swarm/trust-actions-proof.key';state_bytes=state_path.read_bytes();key_bytes=key_path.read_bytes();pending_path=Path(str(state_path)+'.initializing');pending_bytes=json.dumps({'schema_version':1,'state':base64.b64encode(state_bytes).decode(),'state_sha256':hashlib.sha256(state_bytes).hexdigest(),'proof_key':base64.b64encode(bytes.fromhex(key_bytes.decode().strip())).decode()}).encode();pending_path.write_bytes(pending_bytes);pending_path.chmod(0o600);state_path.unlink();key_path.unlink()
 refused_pending=call(recovery_root,'request',cmd(mandate_id='pending'),None,False);assert refused_pending.returncode!=0 and 'initial_configuration_pending' in refused_pending.stderr and not state_path.exists() and not key_path.exists() and pending_path.read_bytes()==pending_bytes
 refused_other=call(recovery_root,'config-apply',dict(initial_change,event_id=event()),None,False);assert refused_other.returncode!=0 and not state_path.exists() and not key_path.exists() and pending_path.read_bytes()==pending_bytes
 recovered=call(recovery_root,'config-apply',initial_change);assert recovered['revision']==1 and state_path.read_bytes()==state_bytes and key_path.read_bytes()==key_bytes and not pending_path.exists()
 original_mandate=cmd(mandate_id='mandate-recipe',subject='alice',actions=['settle'],resource='invoice/acme',max_amount_minor=50000,currency='EUR',beneficiary='ACME');call(root,'mandate-create',original_mandate,TOKENS['admin']);act=approve(root,request(root));call(root,'commit',cmd(action_id=act['id']),TOKENS['effect']);missing_target='absent-public-action'
 for operation,token in [('approve',TOKENS['bob']),('commit',TOKENS['effect']),('reconcile',TOKENS['effect'])]:
  denial=call(root,operation,cmd(action_id=missing_target),token);assert denial['state']=='denied' and denial['business_mutated'] is False
 export=call(root,'export');assert [entry['object_id'] for entry in export['evidence'][-3:]]==[missing_target]*3 and [entry['kind'] for entry in export['evidence'][-3:]]==['approve','commit','reconcile'];original_bytes=json.dumps(original_mandate).encode();original_hash=hashlib.sha256(original_bytes).hexdigest();assert export['schema_version']==2 and base64.b64decode(export['originals'][original_hash])==original_bytes and all(e['original_digest'] in export['originals'] for e in export['evidence']);anchor=call(root,'anchor');key=(root/'.swarm/trust-actions-proof.key').read_text().strip();verified=call(root,'verify',cmd(export=export,anchor=anchor,verification_key=key));empty_anchor={'schema_version':1,'sequence':0,'hash':'','signature':hmac.new(b'',b'0:',hashlib.sha256).hexdigest()};empty_verify=call(root,'verify',cmd(export={'schema_version':2,'simulation':True,'evidence':[],'originals':{}},anchor=empty_anchor,verification_key=' '),None,False);assert empty_verify.returncode!=0;raw=json.dumps(export);assert verified['verified'] and all(secret not in raw for secret in [*TOKENS.values(),key])
 key_path=root/'.swarm/trust-actions-proof.key';key_bytes=key_path.read_bytes();key_path.unlink();target=base/'known-fixture-key';known=('a'*64+'\n').encode();target.write_bytes(known);target.chmod(0o600);key_path.symlink_to(target);key_before=(root/'.swarm/trust-actions.json').read_bytes()
 try:
  bad_key=call(root,'anchor',ok=False);assert bad_key.returncode!=0 and key_before==(root/'.swarm/trust-actions.json').read_bytes() and target.read_bytes()==known
 finally:
  key_path.unlink();key_path.write_bytes(key_bytes);key_path.chmod(0o600)
 altered=json.loads(raw);altered['evidence'][0]['reason']='tampered';tamper=call(root,'verify',cmd(export=altered,anchor=anchor,verification_key=key),None,False);assert tamper.returncode!=0
 return [case('preview-version-persistence',13,pending_business_exit=refused_pending.returncode,pending_business_unchanged=True,different_recovery_exit=refused_other.returncode,exact_recovery_revision=recovered['revision'],recovery_originals_preserved=True,preview_applied=preview['applied'],revision=applied['revision'],stale_exit=bad.returncode,replay_revision=replayed['revision']),case('attributed-proof-and-separate-key',14,missing_action_targets=3,empty_proof_exit=empty_verify.returncode,symlink_anchor_exit=bad_key.returncode,key_target_unchanged=True,sequence=anchor['sequence'],verified=verified['verified'],tamper_exit=tamper.returncode,secrets_exported=False,original_bytes_preserved=True,originals=len(export['originals']))]
def c4():
 required=['web/trust-actions.html','web/trust-actions.js','web/trust-actions.css','docs/actions-confiance-T2.md','docs/en/actions-confiance-T2.md','tests/actions_confiance_t2_ui.cjs'];missing=[x for x in required if not (repo/x).is_file()];assert not missing
 uiout=a.evidence_dir.resolve()/'browser' if a.evidence_dir else base/'ui';uiout.mkdir(parents=True,exist_ok=True,mode=0o700);r=bounded_browser(['node','tests/actions_confiance_t2_ui.cjs',str(binary),str(uiout)],a.timeout_seconds);assert r.returncode==0,r.stderr
 proof=json.loads((uiout/'ui.json').read_text());assert not proof['errors'] and len(proof['checks'])>=8
 receipts=[]
 inputs=['scripts/i18n/build.cjs','tests/i18n_test.cjs','locales/en.json','web/i18n-en.js','locales/trust-actions-en.json','web/trust-actions-en.js','web/i18n.js','web/pilot-graph.js']
 before={name:hashlib.sha256((repo/name).read_bytes()).hexdigest() for name in inputs}
 for argv,marker in [(['npm','run','i18n:build','--','--check'],'"status":"pass"'),(['npm','run','test:i18n'],'PASS i18n build check accepts current output and refuses stale output without mutation')]:
  checked=subprocess.run(argv,cwd=repo,text=True,capture_output=True,timeout=a.timeout_seconds)
  assert checked.returncode==0 and marker in checked.stdout,checked.stdout+checked.stderr
  receipts.append({'argv':argv,'exit_code':checked.returncode,'stdout':checked.stdout,'stderr':checked.stderr,'input_manifest_sha256':hashlib.sha256(json.dumps(before,sort_keys=True,separators=(',',':')).encode()).hexdigest()})
 assert before=={name:hashlib.sha256((repo/name).read_bytes()).hexdigest() for name in inputs},'qualification changed tested sources'
 proof['checks'].append('read-only catalogue generation parity and named npm i18n regressions with source hashes')
 labels=sorted({control['id'] for view in proof['viewportMeasurements'] for control in view['controls']})
 rows=[[view['lang'],view['theme'],view['width'],view['overflow'],view['inputFont'],view['columns'],[[labels.index(c['id']),c['height'],c['width']] for c in view['controls']]] for view in proof['viewportMeasurements']]
 restored=[{'lang':row[0],'theme':row[1],'width':row[2],'overflow':row[3],'inputFont':row[4],'columns':row[5],'controls':[{'id':labels[c[0]],'height':c[1],'width':c[2]} for c in row[6]]} for row in rows]
 assert restored==proof['viewportMeasurements'],'viewport receipt changed an observed measurement'
 geometry={'viewport_columns':['lang','theme','width','overflow','inputFont','columns','controls'],'control_columns':['id_index','height','width'],'control_ids':labels,'rows':rows}
 return [case('public-fr-en-responsive-browser-recipe',len(proof['checks']),checks=proof['checks'],i18n_inputs_sha256=before,i18n_receipts=receipts,contrast=proof['contrastMeasurements'],viewports=geometry,checkbox_targets=proof['toggleMeasurements'],expected_network_errors=proof['expectedNetworkErrors'],diagnostics=proof['errors'],simulation=True)]
all_cases=[]
for n,fn in [(1,c1),(2,c2),(3,c3),(4,c4)]:
 if a.criterion in (0,n):all_cases.extend(fn())
print(json.dumps({'version':1,'task':'T2','criterion':a.criterion,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'cases':all_cases},ensure_ascii=False,separators=(',',':')))
