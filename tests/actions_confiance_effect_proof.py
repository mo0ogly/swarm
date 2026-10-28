#!/usr/bin/env python3
"""Public regression of protected evidence chains and exact effect receipts."""
import argparse,base64,hashlib,json,os,subprocess,tempfile,time,uuid
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('--binary',type=Path,required=True)
p.add_argument('--evidence-dir',type=Path,required=True)
p.add_argument('--timeout-seconds',type=int,required=True,help='recipe watchdog only')
p.add_argument('--legacy-receipt',type=Path)
a=p.parse_args();assert a.timeout_seconds>0
binary=a.binary.resolve();out=a.evidence_dir.resolve();out.mkdir(parents=True,exist_ok=True)
checks=[];roots=[];runs=[]
tokens={k:k+'-proof-regression-fixture-token' for k in ['admin','requester','approver','effect']}
def body(**kw):return dict(schema_version=1,event_id=uuid.uuid4().hex,**kw)
def cli(root,operation,data=None,actor=None,expected=0):
 argv=[str(binary),'--root',str(root),'--json',*operation.split()]
 if data is not None:argv+=['--input','-']
 started=time.time()
 r=subprocess.run(argv,input=json.dumps(data) if data is not None else None,env=dict(os.environ,SWARM_TRUST_TOKEN=tokens.get(actor,'')),capture_output=True,text=True,timeout=a.timeout_seconds)
 runs.append(dict(argv=argv,started_unix=started,ended_unix=time.time(),exit_code=r.returncode,expected=expected))
 if r.returncode!=expected:
  fd=os.open(out/'private-failure.json',os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
  with os.fdopen(fd,'w') as f:json.dump(dict(argv=argv,stdout=r.stdout,stderr=r.stderr,expected=expected,observed=r.returncode),f)
  raise AssertionError('public flow failed; private original retained')
 return json.loads(r.stdout) if r.stdout.strip() else None
def fixture(access=False):
 root=Path(tempfile.mkdtemp(prefix='swarm-effect-proof-'));roots.append(str(root));cli(root,'init')
 config=body(expected_revision=0,reason='isolated evidence proof regression',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=[dict(subject=k,token=v,integrity=3 if k=='admin' else 1 if k=='requester' else 2,roles=[k if k!='admin' else 'admin']) for k,v in tokens.items()])
 cli(root,'trust config-apply',config)
 connector=dict(id='local-access-simulation',version=1,access=dict(subject='employee',role='reader')) if access else None
 m=body(mandate_id='proof-mandate',subject='requester',actions=['grant-access' if access else 'settle'],resource='record/proof',max_amount_minor=0 if access else 1000,currency='' if access else 'EUR',beneficiary='' if access else 'ACME')
 if connector:m['connector']=connector
 cli(root,'trust mandate-create',m,'admin')
 req=body(mandate_id='proof-mandate',resource=m['resource'],amount_minor=0 if access else 100,currency=m['currency'],beneficiary=m['beneficiary'],idempotency_key='proof-key')
 if connector:req['connector']=connector
 action=cli(root,'trust request',req,'requester')
 approve=body(action_id=action['id'],mandate_id=action['mandate_id'],amount_minor=action['amount_minor'],currency=action['currency'],beneficiary=action['beneficiary'],content_digest=action['content_digest'])
 if connector:approve['connector']=connector
 cli(root,'trust approve',approve,'approver')
 return root,action
for access in [False,True]:
 root,action=fixture(access)
 cli(root,'trust commit',body(action_id=action['id'],lose_response=True),'effect')
 exp=cli(root,'trust export')
 event=next(e for e in exp['evidence'] if e['kind']=='commit' and e['result']=='unknown')
 snapshot=json.dumps(event,sort_keys=True)
 effect=event['effect_receipt'];policy=event['policy_context']
 assert policy['schema_version']==1 and policy['revision']==1 and policy['policy']['require_separation'] is True
 assert effect['schema_version']==1 and effect['intent']['action_digest']==action['content_digest']
 assert effect['result']['result_digest']==hashlib.sha256(json.dumps(effect['native_result'],separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
 checks.append(('access' if access else 'ledger')+': exact policy/intent/result/native digest exported')
 cli(root,'trust reconcile',body(action_id=action['id']),'effect')
 export=cli(root,'trust export')
 assert json.dumps(next(e for e in export['evidence'] if e['kind']=='commit'),sort_keys=True)==snapshot
 assert next(e for e in export['evidence'] if e['kind']=='reconcile')['effect_receipt']['result']['state']=='reconciled'
 checks.append(('access' if access else 'ledger')+': reconciliation preserves committed snapshot')
 anchor=cli(root,'trust anchor');key=(root/'.swarm/trust-actions-proof.key').read_text().strip()
 cli(root,'trust verify',body(export=export,anchor=anchor,verification_key=key))
 checks.append(('access' if access else 'ledger')+': HMAC verifies the reopened new export')
 for variant in ['result','native','policy']:
  changed=json.loads(json.dumps(export));ev=next(e for e in changed['evidence'] if e['kind']=='commit')
  if variant=='result':ev['effect_receipt']['result']['result_digest']='0'*64
  elif variant=='native':ev['effect_receipt']['native_result']['resource']='record/changed'
  else:ev.pop('policy_context')
  cli(root,'trust verify',body(export=changed,anchor=anchor,verification_key=key),expected=2)
  checks.append(('access' if access else 'ledger')+': rejects changed '+variant)
 root,action=fixture(access);state=root/'.swarm/trust-actions.json'
 tampered=json.loads(state.read_text());tampered['evidence'][0]['reason']='owned fixture corruption'
 state.write_text(json.dumps(tampered)+'\n');before=state.read_bytes()
 cli(root,'trust commit',body(action_id=action['id']),'effect',expected=2)
 assert state.read_bytes()==before
 current=cli(root,'trust show');assert len(current.get('ledger') or [])+len(current.get('access_ledger') or [])==0
 checks.append(('access' if access else 'ledger')+': corrupt chain refused before mutation')
if a.legacy_receipt:
 legacy=json.loads(a.legacy_receipt.read_text())
 original=next(r['input'] for r in legacy['runs'] if 'verify' in r['argv'] and r['exit_code']==0)
 result=cli(Path(roots[0]),'trust verify',original)
 assert result['verified'] is True
 checks.append('historical unchanged export and HMAC anchor verified by corrected binary')
proof=dict(schema_version=1,checks=checks,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),roots=roots,runs=runs,legacy_receipt_sha256=hashlib.sha256(a.legacy_receipt.read_bytes()).hexdigest() if a.legacy_receipt else None,live_state_modified=False,global_acceptance=False)
(out/'effect-proof-public.json').write_text(json.dumps(proof,indent=2)+'\n')
print(json.dumps(dict(checks=len(checks),binary_sha256=proof['binary_sha256'],legacy_verified=bool(a.legacy_receipt))))
