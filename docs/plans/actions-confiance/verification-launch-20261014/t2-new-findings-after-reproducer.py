import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import tempfile
import uuid

proof_dir = pathlib.Path('/tmp/swarm-actions-confiance-launch')
binary = proof_dir / 't2-new-findings-fixed-binary'
root = pathlib.Path(tempfile.mkdtemp(prefix='swarm-current-contract-'))
records = []
tokens = {role: role + '-token-new-findings-fixture' for role in ['admin', 'alice', 'bob', 'effect']}

def call(operation, data=None, role=None):
    args = [str(binary), '--root', str(root), '--json', *operation]
    if data is not None:
        args += ['--input', '-']
    env = dict(os.environ)
    env['SWARM_TRUST_TOKEN'] = tokens.get(role, '')
    run = subprocess.run(args, input=json.dumps(data) if data is not None else None, text=True, capture_output=True, env=env, timeout=30)
    records.append({'argv': args, 'input': data, 'identity': role, 'exit_code': run.returncode, 'stdout': run.stdout, 'stderr': run.stderr})
    (proof_dir / 't2-new-findings-after.json').write_text(json.dumps({'root':str(root),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'records': records}, ensure_ascii=False, indent=2)+'\n')
    return run, json.loads(run.stdout or run.stderr)

def cmd(**kw):
    return {'schema_version':1,'event_id':uuid.uuid4().hex,**kw}

r, _ = call(['init'])
assert r.returncode == 0
data = {'schema_version':1,'event_id':uuid.uuid4().hex,'expected_revision':0,'reason':'isolated current contract reproduction','policy':{'mandate_ttl_seconds':3600,'unknown_reconcile_seconds':0,'evidence_retention_days':365,'require_separation':True},'identities':[{'subject':s,'token':tokens[s],'integrity':i,'roles':[role]} for s,i,role in [('admin',3,'admin'),('alice',1,'requester'),('bob',2,'approver'),('effect',2,'effect')]]}
r, _ = call(['trust','config-apply'],data)
assert r.returncode == 0
r, _ = call(['trust','mandate-create'], cmd(mandate_id='fixture-mandate',subject='alice',actions=['settle'],resource='invoice/acme',max_amount_minor=50000,currency='EUR',beneficiary='ACME'), 'admin')
assert r.returncode == 0
request = cmd(actor='admin',integrity=3,mandate_id='fixture-mandate',resource='invoice/acme',amount_minor=4200,currency='EUR',beneficiary='ACME',idempotency_key='fixture-business-key')
r, first = call(['trust','request'],request,'alice')
assert r.returncode == 0
replay = {k:v for k,v in request.items() if k not in ['actor','integrity']}
r, replay_result = call(['trust','request'],replay,'alice')
assert r.returncode == 0 and replay_result == first
changed = {**replay,'event_id':uuid.uuid4().hex,'amount_minor':4201}
r, second = call(['trust','request'],changed,'alice')
assert r.returncode == 3 and second['failure']['code']=='idempotency_conflict'
r, approved = call(['trust','approve'], cmd(action_id=first['id'],mandate_id=first['mandate_id'],amount_minor=first['amount_minor'],currency=first['currency'],beneficiary=first['beneficiary']),'bob')
assert r.returncode == 0 and approved['state']=='denied'
r, approved = call(['trust','approve'], cmd(action_id=first['id'],content_digest=first['content_digest'],mandate_id=first['mandate_id'],amount_minor=first['amount_minor'],currency=first['currency'],beneficiary=first['beneficiary']),'bob')
assert r.returncode == 0 and approved['state']=='approved'
r, state = call(['trust','show'])
assert r.returncode == 0 and len(state['actions'])==1 and len(state['ledger'])==0
print(json.dumps({'ignored_fields_replay':'same original reply','changed_content_same_key_actions':len(state['actions']),'approval_with_exact_digest':approved['state'],'root':str(root)}))
