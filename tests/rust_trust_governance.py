#!/usr/bin/env python3
"""Verify native governance CLI/API/browser on owned roots, without providers.

No global watchdog/retry. Preserve every process result and positive control;
pure signatures, real server restart and UI are distinct evidence. Local tests
do not establish external immutable storage or independent acceptance.
"""
import argparse,base64,copy,hashlib,json,os,subprocess
from http.client import HTTPConnection
from pathlib import Path
from rust_agent_model_route import publish
from rust_native_http_cli import Server

TOKENS={x:'governance-fixture-'+x+'-token' for x in ['requester','approver','executor','admin','second-admin']}
def configuration(revision,event,conflict=False):
    """Explicit operator fixture; all thresholds are test data, not defaults."""
    roles={'requester':(1,['requester']),'approver':(2,['approver']),'executor':(2,['effect']),'admin':(3,['admin']),'second-admin':(3,['admin'])}
    principals={s:'principal-'+s for s in roles}
    if conflict:principals['executor']=principals['requester']
    return dict(schema_version=1,event_id=event,expected_revision=revision,reason='Public isolated governance verification',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=7,require_separation=True),identities=[dict(subject=s,token=TOKENS[s],integrity=i,roles=r) for s,(i,r) in roles.items()],role_separation=dict(enabled=True,incompatible_roles=[['requester','approver'],['requester','effect'],['approver','effect'],['admin','effect']],principals=principals),evidence_signing=dict(enabled=True,key_ttl_seconds=3600))
def command(event,**kw):
    """Build an explicit fixture command; this grants no runtime authority."""
    return dict(schema_version=1,event_id=event,**kw)
def main():
    """Run named positive/refusal/restart/browser checks on a new owned Root.

    Preserve failed channels, stop only children created here and propagate
    cleanup errors; no automatic retry or provider call is performed.
    """
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--browser',action='store_true');a=ap.parse_args()
    binary=Path(a.binary).resolve();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);root=out/'root';root.mkdir();checks=[];calls=[]
    def check(name,value):
        assert value,name
        checks.append(name);print('PASS '+name,flush=True)
    def cli(args,body=None,actor='',expected=0,selected_root=None):
        """Invoke the exact binary, retain both channels and assert its exit."""
        env=os.environ.copy();env.pop('SWARM_TRUST_TOKEN',None)
        if actor:env['SWARM_TRUST_TOKEN']=TOKENS[actor]
        argv=[str(binary),'--root',str(selected_root or root),'--json',*args]
        if body is not None:argv+=['--input','-']
        raw=body if isinstance(body,bytes) else json.dumps(body).encode() if body is not None else None
        r=subprocess.run(argv,input=raw,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env)
        n=len(calls);publish(out/f'cli-{n}.stdout',r.stdout);publish(out/f'cli-{n}.stderr',r.stderr);calls.append(dict(argv=argv,exit=r.returncode))
        assert r.returncode==expected,(args,r.returncode,r.stderr.decode())
        return json.loads(r.stdout if r.returncode==0 else r.stderr)
    cli(['init']);cli(['trust','config-apply'],configuration(0,'configure',True))
    server=Server(binary,root,out,'governance-server');server.capture_async(out/'server-stream.jsonl')
    def http(op,body,actor='admin',status=200):
        """Use the owned session/CSRF and selected fixture actor; retain replies."""
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        headers={'Origin':'http://'+server.address,'Content-Type':'application/json','X-Swarm-CSRF':csrf,'Cookie':server.cookie}
        if actor:headers['Authorization']='Bearer '+TOKENS[actor]
        c=HTTPConnection(server.host,server.port,timeout=None)
        try:
            raw=body if isinstance(body,bytes) else json.dumps(body).encode()
            c.request('POST','/api/v1/trust-actions?action='+op,body=raw,headers=headers);r=c.getresponse();payload=r.read();actual=r.status
        finally:c.close()
        n=len(calls);publish(out/f'http-{n}.body',payload);calls.append(dict(operation=op,status=actual,actor=actor))
        assert actual==status,(op,actual,payload.decode())
        return json.loads(payload)
    def transaction(prefix):
        """Establish an admitted mandate/request/exact approval as a witness."""
        params=dict(mandate_id=prefix,resource='invoice/'+prefix,currency='EUR',beneficiary='supplier')
        check(prefix+'-mandate',http('mandate-create',command(prefix+'-m',**params,subject='requester',actions=['settle'],max_amount_minor=1000))['state']=='admitted')
        action=http('request',command(prefix+'-r',**params,amount_minor=450,idempotency_key=prefix+'-effect'),'requester')
        check(prefix+'-request',action['state']=='pending_approval')
        check(prefix+'-approval',http('approve',command(prefix+'-a',**params,action_id=action['id'],content_digest=action['content_digest'],amount_minor=450),'approver')['state']=='approved')
        return action
    try:
        denied=transaction('conflict')
        before=cli(['trust','show']);v=http('commit',command('denied-commit',action_id=denied['id']),'executor')
        after=cli(['trust','show']);check('same-principal-effect-denied',v['state']=='denied' and before['actions']==after['actions'] and after['ledger']==[])
        check('role-policy-reopened',after['role_separation']==configuration(0,'unused',True)['role_separation'])
        cfg=configuration(1,'independent-roles');raw_before=(root/'.swarm/trust-actions.json').read_bytes()
        check('preview-read-only',http('config-preview',cfg)['applied'] is False and (root/'.swarm/trust-actions.json').read_bytes()==raw_before)
        http('config-apply',cfg);action=transaction('independent')
        check('unknown-visible',http('commit',command('lost-response',action_id=action['id'],lose_response=True),'executor')['state']=='unknown')
        audit=http('audit-transaction',dict(action_id=action['id']))
        check('audit-unknown-not-complete',audit['transaction_verified'] and not audit['complete'])
        http('audit-transaction',dict(action_id=action['id']),'requester',403)
        check('audit-current-admin-required',True)
        check('reconcile-single-effect',http('reconcile',command('reconcile',action_id=action['id']),'executor')['state']=='reconciled' and len(cli(['trust','show'])['ledger'])==1)
        audit=http('audit-transaction',dict(action_id=action['id']))
        check('audit-complete-links',audit['chain_verified'] and audit['transaction_verified'] and audit['complete'] and len(audit['events'])==5)
        prepare=command('prepare-key',expected_registry_revision=0,reason='Prepare independent public fixture key')
        prepared=http('evidence-key-prepare',prepare);key_id=prepared['key_id']
        activate=command('activate-key',expected_registry_revision=1,key_id=key_id,reason='Second principal activates exact candidate')
        http('evidence-key-activate',activate,status=403);check('key-self-activation-denied',True)
        activated=http('evidence-key-activate',activate,'second-admin')
        check('second-admin-activation',activated['registry']['keys'][key_id]['status']=='active')
        pins=out/'independent-trust';pins.mkdir();publish(pins/'public-key.json',json.dumps(activated['registry']['keys'][key_id]).encode())
        bundle=http('public-anchor',{})
        decode=lambda raw:json.loads(base64.urlsafe_b64decode(raw+'='*((4-len(raw)%4)%4)))
        proof=dict(export=decode(bundle['export_base64']),anchor=decode(bundle['anchor_base64']),trusted_key=json.loads((pins/'public-key.json').read_bytes()))
        check('no-pin-in-signed-bundle','trusted_key' not in bundle)
        verified=cli(['trust','public-verify'],proof,selected_root=out/'absent-root')
        check('pure-public-verification',verified['verified'] and not verified['checkpoint_verified'] and not (out/'absent-root').exists())
        checkpoint=dict(anchor=proof['anchor'],trusted_key=proof['trusted_key']);publish(pins/'checkpoint.json',json.dumps(checkpoint).encode())
        with_checkpoint=dict(proof,trusted_checkpoint=json.loads((pins/'checkpoint.json').read_bytes()))
        check('separate-checkpoint-verifies',cli(['trust','public-verify'],with_checkpoint)['checkpoint_verified'])
        changed=copy.deepcopy(with_checkpoint);changed['export']['evidence'][0]['actor']='forged'
        check('altered-proof-denied',cli(['trust','public-verify'],changed,expected=2)['failure']['code']=='trust-proof-integrity')
        server.cleanup();server=Server(binary,root,out,'governance-restarted');server.capture_async(out/'restarted-stream.jsonl')
        reopened=server.request('/api/v1/trust-actions',observe=False)
        check('business-replay-after-server-restart',http('reconcile',command('reconcile',action_id=action['id']),'executor')['state']=='reconciled' and len(cli(['trust','show'])['ledger'])==1)
        check('restart-loses-private-capability',reopened['public_evidence_runtime']['private_available'][key_id] is False)
        http('public-anchor',{},status=500);check('no-signing-fallback-after-restart',True)
        prepared2=http('evidence-key-prepare',command('prepare-new',expected_registry_revision=2,reason='Prepare fresh key after real server restart'))
        new_id=prepared2['key_id']
        rotated=http('evidence-key-activate',command('activate-new',expected_registry_revision=3,key_id=new_id,reason='Rotate under a fresh second administrator'),'second-admin')
        check('rotation-retires-old-public-key',rotated['registry']['keys'][key_id]['status']=='retired')
        fresh=http('public-anchor',{})
        extension=dict(export=decode(fresh['export_base64']),anchor=decode(fresh['anchor_base64']),trusted_key=rotated['registry']['keys'][new_id],trusted_checkpoint=checkpoint)
        check('extension-across-rotation',cli(['trust','public-verify'],extension)['checkpoint_verified'])
        rollback=dict(proof,trusted_checkpoint=dict(anchor=extension['anchor'],trusted_key=extension['trusted_key']))
        check('authentic-rollback-denied',cli(['trust','public-verify'],rollback,expected=2)['failure']['code']=='trust-proof-integrity')
        if a.browser:
            cfg=dict(base_url='http://'+server.address,cookie=server.cookie,tokens=TOKENS,output=str(out/'browser'),configuration=configuration(2,'browser-config'),action_id=action['id'])
            publish(out/'browser-input.json',json.dumps(cfg).encode());subprocess.run(['node','tests/rust_trust_governance_ui.cjs',str(out/'browser-input.json')],check=True)
            check('governance-browser-public-flow',True)
        publish(out/'result.json',json.dumps(dict(binary=str(binary),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),assertions=checks,calls=calls,independent_acceptance=False),indent=2).encode())
        print('GOVERNANCE_PUBLIC_PASS '+str(len(checks)),flush=True)
    finally:server.cleanup()

if __name__=='__main__':main()
