#!/usr/bin/env python3
"""Isolated CLI -> real local API recipe; no provider account or payment."""
import argparse, hashlib, json, os, re, selectors, signal, subprocess, tempfile, time, shutil, shlex
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--secret-broker', action='store_true', help='also verify the fixed helper secret grant')
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--evidence-dir', type=Path, required=True)
p.add_argument('--timeout-seconds', type=int, default=120, help='recipe watchdog only; not a Swarm execution policy')
a = p.parse_args()
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
    assert cli(root, 'config-apply', change)['revision'] == 1
    server_image=root/'server-image';shutil.copyfile(binary,server_image);server_image.chmod(0o700)
    server = subprocess.Popen([str(server_image),'--root',str(root),'web','127.0.0.1:0'], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
    stamp=Path('/proc/'+str(server.pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]
    (a.evidence_dir/'image-process.json').write_text(json.dumps({'pid':server.pid,'start':stamp,'root':str(root),'server_image':str(server_image),'source_binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2)+'\n')
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
        action = remote('request', {'schema_version':1,'event_id':'remote-request','mandate_id':'remote-mandate','resource':'invoice/acme','amount_minor':100,'currency':'EUR','beneficiary':'ACME','idempotency_key':'remote-key'}, tokens['alice'])
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
            capture=root/'substitute-capture'
            names=['work_read','work_write','outside_denied','traversal_denied','symlink_denied','proc_denied','binary_denied','tcp_denied','udp_denied','unix_denied','foreign_fd_closed','minimal_environment','no_new_privs','not_dumpable']
            ready=json.dumps({'ready':True,'result':{'landlock_abi':7,'checks':dict.fromkeys(names,True)}})
            replacement=root/'replacement'
            script="#!/bin/sh\numask 077\nIFS= read -r initial\nprintf '%s\\n' "+shlex.quote(ready)+"\n/bin/cat > "+shlex.quote(str(capture))+"\nprintf '%s\\n' "+shlex.quote(json.dumps({'attestation':'0'*64}))+"\n"
            replacement.write_text(script);replacement.chmod(0o700);os.replace(replacement,server_image)
            consumed=remote('secret-consume',{'grant_id':grant['grant_id']},tokens['effect'])
            expected=hmac.new(marker.encode(),json.dumps({'ID':grant['grant_id'],'Action':action['id'],'Digest':action['content_digest'],'Principal':'effect-service','Connector':'sealed-attestation','Destination':'local-attestation','Revision':1},separators=(',',':')).encode(),'sha256').hexdigest()
            verdict={'schema_version':1,'source_binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'substituted_image_executed':capture.exists(),'fixture_material_captured':capture.exists() and capture.read_text()==marker,'real_attestation_matches':consumed.get('attestation')==expected,'state':consumed.get('state'),'provider_account_used':False,'real_secret_used':False}
            (a.evidence_dir/'image-replacement.json').write_text(json.dumps(verdict,indent=2)+'\n')
            assert not capture.exists() and verdict['real_attestation_matches'],'helper image substitution guard failed'
            sealed_after=remote('confinement-check')
            assert sealed_after['passed'] and all(sealed_after['checks'].values()) and len(sealed_after['checks'])==14
            verdict['sealed_helper_after_path_replacement']=True
            (a.evidence_dir/'image-replacement.json').write_text(json.dumps(verdict,indent=2)+'\n')
    finally:
        os.killpg(server.pid,signal.SIGTERM)
        try:server.wait(timeout=5)
        except subprocess.TimeoutExpired:os.killpg(server.pid,signal.SIGKILL);server.wait(timeout=5)
        server.stdout.close()
print(json.dumps({'helper_image_pinned':True,'fixture_only':True}))
