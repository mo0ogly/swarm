#!/usr/bin/env python3
"""Isolated CLI -> real local API recipe; no provider account or payment."""
import argparse, hashlib, json, os, re, selectors, signal, subprocess, tempfile, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--secret-broker', action='store_true', help='also verify the fixed helper secret grant')
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--evidence-dir', type=Path, required=True)
p.add_argument('--timeout-seconds', type=int, default=120, help='recipe watchdog only; not a Swarm execution policy')
a = p.parse_args()
a.secret_broker=True
assert a.timeout_seconds > 0
binary = a.binary.resolve()
a.evidence_dir.mkdir(parents=True, exist_ok=True)
checks = []
tokens = {'admin': 'admin-token-remote-recipe', 'alice': 'requester-token-remote-recipe', 'bob': 'approver-token-remote-recipe', 'effect': 'effect-token-remote-recipe'}

def cli(root, action=None, data=None, token='', origin=None, session='', expected=0, raw=False):
    env = dict(os.environ, SWARM_TRUST_TOKEN=token, SWARM_WEB_SESSION=session)
    argv = [str(binary), '--root', str(root), '--json']
    if origin is not None:
        argv += ['--server', origin]
    argv += ['trust', action] if action else ['init']
    if data is not None:
        argv += ['--input', '-']
    r = subprocess.run(argv, input=(data if raw else json.dumps(data)) if data is not None else None, env=env, capture_output=True, text=True, timeout=a.timeout_seconds)
    if r.returncode != expected:
        diagnostic = a.evidence_dir/('remote-failure-'+str(action)+'.json')
        fd = os.open(diagnostic, os.O_WRONLY|os.O_CREAT|os.O_TRUNC, 0o600)
        with os.fdopen(fd, 'w') as stream:
            json.dump({'operation':action,'expected_exit':expected,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr},stream)
        raise AssertionError(f'operation {action}: expected exit {expected}, observed {r.returncode}; private diagnostic saved')
    return json.loads(r.stdout if expected == 0 else r.stderr)

with tempfile.TemporaryDirectory(prefix='swarm-trust-remote-recipe-') as fixture:
    base = Path(fixture)
    root, absent = base/'server', base/'client-must-stay-absent'
    root.mkdir()
    cli(root)
    change = {'schema_version':1,'event_id':'remote-config','expected_revision':0,'reason':'isolated CLI server recipe','policy':{'mandate_ttl_seconds':3600,'unknown_reconcile_seconds':0,'evidence_retention_days':365,'require_separation':True},'sealed_execution':{'enabled':True,'timeout_seconds':15},'identities':[{'subject':s,'token':tokens[k],'integrity':i,'roles':[r]} for k,s,i,r in [('admin','admin',3,'admin'),('alice','alice',1,'requester'),('bob','bob',2,'approver'),('effect','effect-service',2,'effect')]]}
    if a.secret_broker:
        change['secret_broker']={'enabled':True,'material_ttl_seconds':300,'grant_ttl_seconds':60,'max_material_bytes':4096,'max_entries':16}
    change['data_policy']={'enabled':True,'classifications':['internal','restricted'],'destinations':['local-ledger','local-attestation'],'providers':['swarm'],'tools':['simulated-settlement','sealed-attestation'],'resources':[{'resource':'invoice/acme','classification':'internal'}]}
    assert cli(root, 'config-apply', change)['revision'] == 1
    server = subprocess.Popen([str(binary),'--root',str(root),'web','127.0.0.1:0'], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
    stamp=Path('/proc/'+str(server.pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]
    (a.evidence_dir/'data-process-initial.json').write_text(json.dumps({'pid':server.pid,'start':stamp,'root':str(root),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2)+'\n')
    try:
        selector = selectors.DefaultSelector()
        selector.register(server.stdout, selectors.EVENT_READ)
        log = b''
        end = time.monotonic()+a.timeout_seconds
        while b'/session/' not in log:
            remaining = end-time.monotonic()
            assert remaining > 0, 'owned server startup timed out'
            assert selector.select(remaining), 'owned server startup timed out'
            chunk = os.read(server.stdout.fileno(), 8192)
            assert chunk, 'owned server exited before startup'
            log += chunk
        selector.close()
        match = re.search(rb'http://(127\.0\.0\.1:[0-9]+)/session/(session-[a-zA-Z0-9]+)', log)
        assert match, 'owned server session missing'
        origin, session = 'http://'+match[1].decode(), match[2].decode()
        def remote(action, data=None, token='', expected=0, raw=False):
            return cli(absent, action, data, token, origin, session, expected, raw)
        assert remote('show')['revision'] == 1
        assert remote('identity-context', token=tokens['alice'])['principal_id'] == 'alice'
        checks += ['server view without local root', 'authenticated identity context']
        sealed = remote('confinement-check')
        assert sealed['passed'] and sealed['scope'] == 'sealed_helper_only' and len(sealed['checks']) == 14 and all(sealed['checks'].values())
        checks.append('real sealed helper through remote API: 14 assertions')
        mandate = {'schema_version':1,'event_id':'remote-mandate','mandate_id':'remote-mandate','subject':'alice','actions':['settle'],'resource':'invoice/acme','max_amount_minor':50000,'currency':'EUR','beneficiary':'ACME'}
        remote('mandate-create', mandate, tokens['admin'])
        action = remote('request', {'schema_version':1,'event_id':'remote-request','mandate_id':'remote-mandate','resource':'invoice/acme','amount_minor':100,'currency':'EUR','beneficiary':'ACME','idempotency_key':'remote-key','data_context':{'classification':'internal','destination':'local-ledger','provider':'swarm','tool':'simulated-settlement'}}, tokens['alice'])
        approval = {'schema_version':1,'event_id':'remote-approve','action_id':action['id'],'mandate_id':action['mandate_id'],'amount_minor':action['amount_minor'],'currency':action['currency'],'beneficiary':action['beneficiary'],'content_digest':action['content_digest']}
        assert remote('approve', approval, tokens['bob'])['state'] == 'approved'
        if a.secret_broker:
            import hmac
            marker='ephemeral-public-recipe-secret-0123456789abcdef'
            reference=remote('secret-load',marker,tokens['admin'],raw=True)['secret_ref']
            try:
                memory_fd=os.open('/proc/'+str(server.pid)+'/mem',os.O_RDONLY)
            except PermissionError:
                checks.append('same-UID parent cannot open protected server memory')
            else:
                os.close(memory_fd);raise AssertionError('protected server memory remained readable')
            grant=remote('secret-grant',{'action_id':action['id'],'action_digest':action['content_digest'],'secret_ref':reference,'connector_id':'sealed-attestation','destination':'local-attestation'},tokens['effect'])
            before=(root/'.swarm/trust-actions.json').read_bytes()
            remote('secret-consume',{'grant_id':grant['grant_id']},tokens['alice'],expected=4)
            assert before==(root/'.swarm/trust-actions.json').read_bytes()
            consumed=remote('secret-consume',{'grant_id':grant['grant_id']},tokens['effect'])
            message={'ID':grant['grant_id'],'Action':grant['action_id'],'Digest':grant['action_digest'],'Principal':grant['principal_id'],'Connector':grant['connector_id'],'Destination':grant['destination'],'Revision':grant['policy_revision']}
            delivery={'classification':'restricted','destination':'local-attestation','provider':'swarm','tool':'sealed-attestation'}
            assert grant['data_context']==delivery
            message={'domain':'swarm-secret-delivery-v1','grant':message,'data_context':delivery}
            expected=hmac.new(marker.encode(),json.dumps(message,separators=(',',':')).encode(),hashlib.sha256).hexdigest()
            assert consumed['state']=='consumed' and consumed['attestation']==expected
            remote('secret-consume',{'grant_id':grant['grant_id']},tokens['effect'],expected=4)
            assert marker not in json.dumps(remote('export')) and marker.encode() not in (root/'.swarm/trust-actions.json').read_bytes()
            status=remote('secret-status',token=tokens['admin'])
            assert status['loaded_entries']==0 and len(status['grants'])==1 and status['grants'][0]['state']=='consumed'
            checks += ['raw material through authenticated dedicated channel','exact content-bound native HMAC attestation','wrong role denied without mutation','single consumption and replay refusal','raw marker absent from state and proof export','memory consumption separate from persistent policy']
            reference2=remote('secret-load',marker,tokens['admin'],raw=True)['secret_ref']
            grant2=remote('secret-grant',{'action_id':action['id'],'action_digest':action['content_digest'],'secret_ref':reference2,'connector_id':'sealed-attestation','destination':'local-attestation'},tokens['effect'])
            os.killpg(server.pid,signal.SIGTERM);server.wait(timeout=5);server.stdout.close()
            server=subprocess.Popen([str(binary),'--root',str(root),'web',origin.removeprefix('http://')],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,start_new_session=True)
            stamp=Path('/proc/'+str(server.pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]
            (a.evidence_dir/'data-process-restart.json').write_text(json.dumps({'pid':server.pid,'start':stamp,'root':str(root),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2)+'\n')
            readylog=b'';deadline=time.monotonic()+a.timeout_seconds
            readyselector=selectors.DefaultSelector();readyselector.register(server.stdout,selectors.EVENT_READ)
            while b'/session/' not in readylog:
                remaining=deadline-time.monotonic();assert remaining>0 and readyselector.select(remaining),'restart startup timed out'
                chunk=os.read(server.stdout.fileno(),8192);assert chunk,'restart exited';readylog+=chunk
            readyselector.close()
            after_restart=remote('secret-status',token=tokens['admin'])
            assert after_restart['loaded_entries']==0 and after_restart['grant_material_available'][grant2['grant_id']] is False
            before=state_bytes=(root/'.swarm/trust-actions.json').read_bytes()
            remote('secret-consume',{'grant_id':grant2['grant_id']},tokens['effect'],expected=4)
            assert before==(root/'.swarm/trust-actions.json').read_bytes()
            checks.append('server restart loses material and old grant refuses without mutation')
            fresh=remote('secret-load',marker,tokens['admin'],raw=True)['secret_ref']
            fresh_grant=remote('secret-grant',{'action_id':action['id'],'action_digest':action['content_digest'],'secret_ref':fresh,'connector_id':'sealed-attestation','destination':'local-attestation'},tokens['effect'])
            remote('secret-revoke',{'grant_id':fresh_grant['grant_id']},tokens['admin'])
            before=(root/'.swarm/trust-actions.json').read_bytes()
            remote('secret-consume',{'grant_id':fresh_grant['grant_id']},tokens['effect'],expected=4)
            assert before==(root/'.swarm/trust-actions.json').read_bytes()
            checks.append('fresh material works after restart and revoked grant stays closed')

        effect = remote('commit', {'schema_version':1,'event_id':'remote-commit','action_id':action['id']}, tokens['effect'])
        assert effect['state'] == 'committed'
        remote('reconcile', {'schema_version':1,'event_id':'remote-reconcile','action_id':action['id']}, tokens['effect'])
        assert len(remote('show')['ledger']) == 1
        checks.append('request exact approval effect and reconciliation: one effect')
        change.update(event_id='secret-path-withdrawal',expected_revision=1)
        change['data_policy']['destinations']=['local-ledger']
        assert remote('config-apply',change)['revision']==2
        closed_before=(root/'.swarm/trust-actions.json').read_bytes()
        refusal=remote('secret-load','must-not-be-sent',tokens['admin'],expected=4,raw=True)
        assert refusal['failure']['code']=='authorization_required'
        assert (root/'.swarm/trust-actions.json').read_bytes()==closed_before
        checks.append('withdrawn secret destination refuses preparation before raw transfer without mutation')
        state = root/'.swarm/trust-actions.json'
        before = state.read_bytes()
        refusal = cli(absent,'identity-context',None,tokens['alice'],origin,'session-111111111111111111111111',4)
        assert refusal['failure']['code'] == 'authorization_required' and before == state.read_bytes()
        checks.append('wrong operator session refused without mutation')
        assert not absent.exists()
        checks.append('client root never created')
    finally:
        os.killpg(server.pid, signal.SIGTERM)
        try:
            server.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(server.pid, signal.SIGKILL)
            server.wait(timeout=5)
        server.stdout.close()
    result = remote('show', expected=5)
    assert result['failure']['code'] == 'provider_unavailable' and not absent.exists()
    checks.append('stopped server refused without local fallback')
proof = {'schema_version':1,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'checks':checks,'server_stopped':server.poll() is not None,'provider_account_used':False,'real_payment':False}
(a.evidence_dir/'remote.json').write_text(json.dumps(proof,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'checks':len(checks),'server_stopped':True,'binary_sha256':proof['binary_sha256']}))
