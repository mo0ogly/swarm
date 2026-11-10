#!/usr/bin/env python3
"""Exercise native Rust Biba through CLI/API on owned roots, no providers.

No global watchdog or retry. Positive witness precedes refusals. Retains named
assertions, immutable binary/source hashes and original stdout/stderr. This proves
local core behavior, never OS confinement, federated identity or global acceptance.
"""
import argparse,copy,hashlib,json,os,shutil,subprocess,tempfile
from http.client import HTTPConnection
from pathlib import Path
from rust_agent_model_route import publish
from rust_native_http_cli import Server
TOKENS={'requester':'biba-fixture-requester-token','approver':'biba-fixture-approver-token','admin':'biba-fixture-admin-token'}
def configuration(revision=0,event='configure'):
    """Explicit fixture policy/identities. Tokens are synthetic, not user secrets."""
    return dict(schema_version=1,event_id=event,expected_revision=revision,reason='Isolated Biba fixture configuration',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=[dict(subject=s,token=t,integrity={'requester':1,'approver':2,'admin':3}[s],roles={'requester':['requester'],'approver':['approver'],'admin':['admin','effect']}[s]) for s,t in TOKENS.items()])
def command(event,**fields):return dict(schema_version=1,event_id=event,**fields)
def main():
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);p.add_argument('--go-reference');p.add_argument('--browser',action='store_true');a=p.parse_args()
    binary=Path(a.binary).resolve();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);os.chmod(out,0o700)
    root=Path(tempfile.mkdtemp(prefix='biba-public-',dir=out));records=[];assertions=[]
    def check(name,condition):
        assert condition,name
        assertions.append(name);print('PASS '+name,flush=True)
    def cli(args,document=None,actor='',expected=0):
        env=os.environ.copy();env.pop('SWARM_TRUST_TOKEN',None)
        if actor:env['SWARM_TRUST_TOKEN']=TOKENS[actor]
        argv=[str(binary),'--root',str(root),'--json',*args]
        if document is not None:argv+=['--input','-']
        raw=document if isinstance(document,bytes) else json.dumps(document,ensure_ascii=False,separators=(',',':')).encode() if document is not None else None
        r=subprocess.run(argv,input=raw,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env)
        n=len(records);publish(out/f'{n:03d}.stdout',r.stdout);publish(out/f'{n:03d}.stderr',r.stderr)
        records.append(dict(argv=argv,exit=r.returncode,actor=actor,stdout=f'{n:03d}.stdout',stderr=f'{n:03d}.stderr'))
        assert r.returncode==expected,(args,r.returncode,r.stderr.decode())
        return json.loads(r.stdout if r.returncode==0 else r.stderr),r.stdout
    def trust(op,v=None,actor='',expected=0):return cli(['trust',op],v,actor,expected)[0]
    cli(['init']);initial=trust('show');check('inactive-installation',initial['revision']==0 and not initial['actions'])
    proposal=configuration();before=set((root/'.swarm').iterdir());preview=trust('config-preview',proposal);check('preview-pure',preview['applied'] is False and not (root/'.swarm/trust-actions.json').exists())
    configured=trust('config-apply',proposal);check('configuration-explicit-reopened',configured['revision']==1 and trust('show')['policy']==proposal['policy'])
    m=command('mandate',mandate_id='mandate',subject='requester',actions=['settle'],resource='invoice/one',max_amount_minor=1000,currency='EUR',beneficiary='supplier')
    ack=trust('mandate-create',m,'admin');check('I3-named-endorsement',ack['kind']=='trust-status-ack-v1' and ack['state']=='admitted' and trust('show')['mandates']['mandate']['integrity']==2)
    req=command('request',mandate_id='mandate',resource='invoice/one',amount_minor=450,currency='EUR',beneficiary='supplier',idempotency_key='unique-key')
    action=trust('request',req,'requester');check('I1-request-new-official-object',action['state']=='pending_approval')
    before_actions=copy.deepcopy(trust('show')['actions'])
    forged=command('forged',actor='admin',integrity=3,resource='invoice/one')
    check('claimed-I3-cannot-write',trust('direct-write',forged,'requester')['state']=='denied')
    check('I1-cannot-invoke-I2',trust('commit',command('early',action_id=action['id']),'requester')['state']=='denied')
    check('denial-no-business-mutation',trust('show')['actions']==before_actions and trust('show')['ledger']==[])
    approved=command('approve',action_id=action['id'],mandate_id='mandate',content_digest=action['content_digest'],resource='invoice/one',amount_minor=451,currency='EUR',beneficiary='supplier')
    check('inexact-agreement-refused',trust('approve',approved,'approver')['state']=='denied')
    approved['event_id']='approve-exact';approved['amount_minor']=450
    check('I2-exact-agreement',trust('approve',approved,'approver')['state']=='approved')
    lost=command('commit',action_id=action['id'],lose_response=True)
    check('response-loss-visible',trust('commit',lost,'admin')['state']=='unknown')
    check('same-event-replay',trust('commit',lost,'admin')['state']=='unknown')
    other=dict(lost,event_id='commit-again');check('new-event-no-duplicate',trust('commit',other,'admin')['state']=='unknown')
    check('reconcile-single-native-effect',trust('reconcile',command('reconcile',action_id=action['id']),'admin')['state']=='reconciled' and len(trust('show')['ledger'])==1)
    before=(root/'.swarm/trust-actions.json').read_bytes()
    check('no-read-down-original',trust('read-original',command('read-original',action_id=action['id']),'approver',4)['failure']['code']=='authorization_required')
    check('no-read-down-official-object',trust('read-object',command('read-high',action_id=action['id']),'admin',4)['failure']['code']=='authorization_required')
    check('read-up-official-object',trust('read-object',command('read-low',action_id=action['id']),'requester')['id']==action['id'])
    check('read-own-original-exact',trust('read-original',command('read-own',action_id=action['id']),'requester')==req)
    check('object-reads-no-mutation',(root/'.swarm/trust-actions.json').read_bytes()==before)
    export,export_raw=cli(['trust','export']);anchor,anchor_raw=cli(['trust','anchor']);key=(root/'.swarm/trust-actions-proof.key').read_text()
    proof=b'{"export":'+export_raw+b',"anchor":'+anchor_raw+b',"verification_key":'+json.dumps(key).encode()+b'}'
    check('independently-keyed-anchor-verifies',trust('verify',proof)['verified'] is True)
    altered=proof.replace(b'"sequence":1',b'"sequence":999',1)
    check('altered-export-refused',trust('verify',altered,expected=2)['failure']['code']=='trust-proof-integrity')
    for name,field,value in [('null','mandate_ttl_seconds',None),('negative','unknown_reconcile_seconds',-1),('zero-retention','evidence_retention_days',0),('fraction','mandate_ttl_seconds',1.5)]:
        invalid=configuration(1,'invalid-'+name);invalid['policy'][field]=value
        check('policy-'+name+'-refused',trust('config-apply',invalid,expected=2)['failure']['code']=='format')
        check('policy-'+name+'-no-mutation',(root/'.swarm/trust-actions.json').read_bytes()==before)
    invalid=configuration(1,'unsupported');invalid['data_policy']={'enabled':True}
    check('active-unported-policy-closed',trust('config-apply',invalid,expected=2)['failure']['code']=='command_not_ported')
    check('unsupported-no-mutation',(root/'.swarm/trust-actions.json').read_bytes()==before)
    if a.go_reference:
        reference=Path(a.go_reference).resolve();go_root=out/'go-root';go_root.mkdir();initialized=subprocess.run([str(reference),'--root',str(go_root),'--json','init'],stdout=subprocess.PIPE,stderr=subprocess.PIPE);publish(out/'go-init.stdout',initialized.stdout);publish(out/'go-init.stderr',initialized.stderr);check('Go-fixture-initialized',initialized.returncode==0)
        for name in ['trust-actions.json','trust-actions-proof.key']:shutil.copy2(root/'.swarm'/name,go_root/'.swarm'/name)
        go_argv=[str(reference),'--root',str(go_root),'--json','trust','verify','--input','-']
        g=subprocess.run(go_argv,input=proof,stdout=subprocess.PIPE,stderr=subprocess.PIPE);publish(out/'go-verify.stdout',g.stdout);publish(out/'go-verify.stderr',g.stderr)
        check('Go-verifies-Rust-exact-proof',g.returncode==0 and json.loads(g.stdout)['verified'] is True)
        go_export=subprocess.run([str(reference),'--root',str(go_root),'--json','trust','export'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        check('Go-reads-Rust-state',go_export.returncode==0 and json.loads(go_export.stdout)['evidence']==export['evidence'])
        publish(out/'go-export.stdout',go_export.stdout);publish(out/'go-export.stderr',go_export.stderr)
        generated=dict(req,event_id='go-produced-request',idempotency_key='go-produced-key');env=os.environ.copy();env['SWARM_TRUST_TOKEN']=TOKENS['requester']
        go_request=subprocess.run([str(reference),'--root',str(go_root),'--json','trust','request','--input','-'],input=json.dumps(generated).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env);publish(out/'go-request.stdout',go_request.stdout);publish(out/'go-request.stderr',go_request.stderr);check('Go-producer-positive',go_request.returncode==0 and json.loads(go_request.stdout)['state']=='pending_approval')
        go_export=subprocess.run([str(reference),'--root',str(go_root),'--json','trust','export'],stdout=subprocess.PIPE,stderr=subprocess.PIPE);go_anchor=subprocess.run([str(reference),'--root',str(go_root),'--json','trust','anchor'],stdout=subprocess.PIPE,stderr=subprocess.PIPE);check('Go-export-anchor-positive',go_export.returncode==0 and go_anchor.returncode==0)
        generated_proof=b'{"export":'+go_export.stdout+b',"anchor":'+go_anchor.stdout+b',"verification_key":'+json.dumps(key).encode()+b'}';check('Rust-verifies-Go-exact-proof',trust('verify',generated_proof)['verified'] is True)

    server=Server(binary,root,out,'biba-server')
    # Browser HTTP conversations must not block on an undrained CLI output pipe.
    server.capture_async(out/'biba-server-stream.jsonl')
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        def http(op,body,actor='',status=200,session=True):
            headers={'Origin':'http://'+server.address,'Content-Type':'application/json','X-Swarm-CSRF':csrf}
            if session:headers['Cookie']=server.cookie
            if actor:headers['Authorization']='Bearer '+TOKENS[actor]
            c=HTTPConnection(server.host,server.port,timeout=None)
            try:
                c.request('POST','/api/v1/trust-actions?action='+op,body=json.dumps(body).encode(),headers=headers);r=c.getresponse();raw=r.read();actual=r.status
            finally:c.close()
            check('HTTP-'+op+'-'+str(status),actual==status)
            return json.loads(raw) if actual!=403 or raw.startswith(b'{') else raw
        check('HTTP-audit-projection',server.request('/api/v1/trust-actions',observe=False)['actions']==trust('show')['actions'])
        for route,source in [('/trust-actions.html','web/trust-native.html'),('/trust-native.js','web/trust-native.js'),('/trust-native.css','web/trust-native.css'),('/i18n-en.js','web/i18n-en.js')]:
            c=HTTPConnection(server.host,server.port,timeout=None)
            try:c.request('GET',route,headers={'Cookie':server.cookie});r=c.getresponse();actual=r.read();check('served-bytes-'+route,r.status==200 and actual==Path(source).read_bytes())
            finally:c.close()

        http('commit',command('http-denied',action_id=action['id']),status=403)
        http('commit',command('http-session',action_id=action['id']),'admin',403,False)
        check('HTTP-authenticated-readback',http('commit',command('http-committed',action_id=action['id']),'admin')['state']=='reconciled')
        check('HTTP-unknown-command-closed',http('arbitrary',{},'admin',400)['failure']['code']=='trust-action-unknown')
        snapshot=(root/'.swarm/trust-actions.json').read_bytes()
        escalation=configuration(1,'http-escalation');escalation['identities'][0]['integrity']=3;escalation['identities'][0]['roles']=['admin','requester']
        for op in ['config-preview','config-apply']:
            http(op,escalation,'requester',403)
        check('HTTP-I1-admin-refusals-no-mutation',(root/'.swarm/trust-actions.json').read_bytes()==snapshot)
        check('HTTP-current-I3-admin-preview',http('config-preview',configuration(1,'http-admin-preview'),'admin')['applied'] is False)
        readback=http('commit',command('http-read-guard',action_id=action['id']),'admin')
        check('HTTP-I3-status-only',readback['kind']=='trust-status-ack-v1' and readback['content_redacted'] is True and 'amount_minor' not in readback)
        if a.browser:
            config=dict(address=server.address,cookie=server.cookie,root=str(root),output=str(out),tokens=TOKENS)
            b=subprocess.run(['node','tests/rust_trust_biba_ui.cjs'],input=json.dumps(config).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            publish(out/'browser.stdout',b.stdout);publish(out/'browser.stderr',b.stderr);check('browser-public-journey',b.returncode==0)
    finally:server.cleanup()
    revoke=trust('mandate-revoke',command('revoke',mandate_id='mandate'),'admin');check('revocation-attributed',revoke['state']=='revoked' and bool(trust('show')['mandates']['mandate']['revoked_at']))
    check('revoked-mandate-refused',trust('request',dict(req,event_id='revoked-request'),'requester')['state']=='denied')
    check('committed-effect-preserved',len(trust('show')['ledger'])==(2 if a.browser else 1))
    expected_ledger=len(trust('show')['ledger'])
    current=trust('show');disabled=configuration(current['revision'],'suspend');disabled['identities'][0]['disabled']=True;trust('config-apply',disabled)
    before=(root/'.swarm/trust-actions.json').read_bytes()
    check('suspended-identity-cannot-replay',trust('request',req,'requester',4)['failure']['code']=='authorization_required')
    check('suspension-refusal-no-mutation',(root/'.swarm/trust-actions.json').read_bytes()==before)
    revision=trust('show')['revision'];write=configuration(revision,'broken-output');argv=[str(binary),'--root',str(root),'--json','trust','config-apply','--input','-']
    process=subprocess.Popen(argv,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    publish(out/'brokenpipe-process.json',dict(argv=argv,pid=process.pid,owned=True,global_watchdog=None))
    process.stdout.close();process.stdin.write(json.dumps(write).encode());process.stdin.close();diagnostic=process.stderr.read();process.stderr.close();code=process.wait();publish(out/'brokenpipe.stderr',diagnostic)
    failure=json.loads(diagnostic);check('late-output-failure-retains-publication',code==2 and failure['trust_publication']=='recorded' and failure['trust_state_may_be_recorded'] is True)
    check('late-output-state-reopens',trust('show')['revision']==revision+1 and len(trust('show')['ledger'])==expected_ledger)
    publish(out/'result.json',dict(schema_version=1,scope='local-trust-core',global_acceptance=False,assertions=assertions,commands=records,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),watchdog=None))
    print('BIBA_PUBLIC_PASS '+str(len(assertions)),flush=True)
if __name__=='__main__':main()
