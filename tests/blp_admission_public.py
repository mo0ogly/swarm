#!/usr/bin/env python3
"""Exercise fixed pre-transport domains through the actual qualified CLI/HTTP.

Owned roots only. All business/Trust mutations use public operations. SQL only
observes state; file corruption is limited to this fixture's emitted capsule,
with a valid control and exact atomic restore. No paid provider or watchdog.
"""
import argparse, copy, hashlib, http.client, json, os, sqlite3, subprocess
from pathlib import Path
from rust_agent_model_route import Fixture, publish
from rust_native_http_cli import Server
from blp_attribution_history_cli import digest


def main():
    """Qualify source attribution, authenticated immutable admission, refusal,
    replay, framing/Root/label corruption recovery and policy revocation. The
    sentinel program proves no provider launch; counters prove no turn/budget
    reservation, beyond process exit. Retain every request/result before assert.
    """
    ap=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:ap.add_argument('--'+name,type=Path,required=True)
    a=ap.parse_args();binary=a.binary.resolve(strict=True);out=a.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile=json.loads(a.profile.read_bytes());assert profile['sha256']==digest(binary)
    assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['no_fixture_feature'] and profile['all_core_features_empty'] and profile['all_core_test_false']
    publish(out/'qualified-profile.json',profile)
    f=Fixture(binary,out,'blp-admission');checks=[];web=None

    def check(name,value):
        """Persist each exact assertion before failure; no empty PASS result."""
        checks.append(dict(name=name,passed=bool(value)));publish(out/'checks.json',checks);assert value,name;print('PASS '+name,flush=True)

    def trust(op,value,token='fixture-admin-token',success=True):
        """Invoke actual CLI, keeping synthetic bearer values in environment."""
        f.environment['SWARM_TRUST_TOKEN']=token
        code,result=f.call(['trust',op],value);assert (code==0)==success,(op,code,result)
        return result

    def observed_state():
        """Read the owned database only; admissions must not mutate its state."""
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            return dict(turns=db.execute('SELECT count(*) FROM preparation_turns').fetchone()[0],
                reservations=db.execute('SELECT count(*) FROM preparation_reservations').fetchone()[0],
                preparations=db.execute('SELECT id,revision,hex(body) FROM preparations ORDER BY id').fetchall())

    f.settings_change(preparation_write_enabled='1',preparation_call_enabled='1',agent_allow_unverified_provider_context='1')
    sentinel=f.root/'provider-invoked'
    program='#!/usr/bin/python3\nfrom pathlib import Path\nPath('+repr(str(sentinel))+').write_text("provider invoked")\n'
    publish(f.root/'codex',program.encode());os.chmod(f.root/'codex',0o700)
    # Valid control for the exact configured executable, before the no-launch assertion.
    control=subprocess.run([str(f.root/'codex')],capture_output=True,check=True)
    check('sentinel-executable-control',sentinel.read_text()=='provider invoked')
    publish(out/'provider-control.txt',sentinel.read_bytes());sentinel.unlink()
    f.provider['args']=['--model','fixture-standard'];f.provider['model_policy']['levels']['standard']['effort']='';f.save_provider()
    p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='create',expected_revision=0,title='Fixed domain fixture',text='Fixture need'))
    p=f.ok(['prepare','budget',p['id']],dict(version=1,action='budget',preparation_id=p['id'],event_id='budget',expected_revision=p['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=.5,estimate_source='inert fixture',reference_date='2026-11-01')))
    cap=next(c for c in f.ok(['prepare','capabilities'])['providers'] if c['name']=='fixture')
    intent=dict(version=1,event_id='unsent-intent',preparation_id=p['id'],expected_revision=p['revision'],provider='fixture',capability_hash=cap['digest'],message='PRIVATE_ADMISSION_INPUT',context_mode='full')
    identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),dict(subject='sender',integrity=1,roles=['requester'],token='fixture-sender-token'),dict(subject='low',integrity=1,roles=['requester'],token='fixture-low-token'),dict(subject='uncleared',integrity=3,roles=['admin'],token='fixture-uncleared-token')]
    config=dict(schema_version=1,event_id='init',expected_revision=0,reason='Owned admission fixture',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=identities)
    trust('config-apply',config,token='')
    label=dict(level='secret',compartments=['a','b'])
    policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public'),dict(id='secret',name_fr='Secret',name_en='Secret'),dict(id='top',name_fr='Très secret',name_en='Top')],compartments=['a','b','c'],clearances=dict(admin=dict(level='top',compartments=['a','b','c']),sender=dict(level='top',compartments=['a','b','c']),low=dict(level='secret',compartments=['a'])),preparation_quarantine=True)
    trust('blp-policy-apply',dict(schema_version=1,event_id='policy',expected_revision=0,expected_trust_revision=1,reason='Explicit quarantine and classification fixture',policy=policy))
    admission=dict(schema_version=1,event_id='admission',expected_revision=1,expected_trust_revision=1,intent=intent)
    baseline=observed_state();state=f.root/'.swarm/trust-actions.json';before=state.read_bytes()
    no_source=trust('blp-preparation-admit',admission,token='fixture-sender-token',success=False)
    check('unclassified-admission-refuses',no_source['failure']['code']=='blp-classification-required')
    check('unknown-source-no-capsule-or-business-mutation',not list((f.root/'.swarm').glob('blp-admission-*.receipt')) and observed_state()==baseline and state.read_bytes()==before)
    attr=lambda index:dict(schema_version=1,event_id='input-'+str(index),expected_revision=1,expected_trust_revision=1,intent=intent,source_index=index,classification=label,reason='Explicit per-slice classification fixture')
    refused=trust('blp-preparation-input-attribute',attr(0),token='fixture-sender-token',success=False)
    check('classification-needs-admin',refused['failure']['code']=='authorization_required' and state.read_bytes()==before)
    first=trust('blp-preparation-input-attribute',attr(0));count=first['source_count']
    check('actual-pre-admission-source',first['producer_captured'] is True and first['capture_phase']=='pre-admission' and count>1 and 'PRIVATE_ADMISSION_INPUT' not in json.dumps(first))
    for index in range(1,count):
        code,result=f.call(['trust','blp-preparation-input-attribute'],attr(index))
        check('explicit-source-'+str(index),code==0 or (code!=0 and result['failure']['code']=='blp-attribution-conflict'))
    check('source-attribution-does-not-create-turns-or-reservations',observed_state()==baseline and not sentinel.exists())
    before=state.read_bytes()
    cases=[('session','',admission,'authorization_required',403),
        ('no-grant-before-context','fixture-uncleared-token',{**admission,'intent':{**intent,'preparation_id':'missing'}},'blp-access-denied',403),
        ('insufficient-clearance','fixture-low-token',admission,'blp-access-denied',403),
        ('forged-actor','fixture-sender-token',{**admission,'actor':'admin'},'format',400),
        ('forged-label','fixture-sender-token',{**admission,'classification':dict(level='public',compartments=[])},'format',400),
        ('stale-policy','fixture-sender-token',{**admission,'expected_revision':0},'blp-policy-stale',409),
        ('stale-trust','fixture-sender-token',{**admission,'expected_trust_revision':0},'blp-policy-stale',409),
        ('bad-schema','fixture-sender-token',{**admission,'schema_version':0},'blp-admission-invalid',400)]
    for name,token,value,reason,status in cases:
        refused=trust('blp-preparation-admit',value,token=token,success=False)
        check('cli-'+name,refused['failure']['code']==reason and state.read_bytes()==before and observed_state()==baseline)
    accepted=trust('blp-preparation-admit',admission,token='fixture-sender-token')
    check('authenticated-nonadmin-domain',accepted['principal']=='sender' and accepted['output_classification']==label and accepted['source_count']==count)
    check('domain-is-source-join-not-maximum-clearance',accepted['output_classification']!=policy['clearances']['sender'])
    check('admission-is-not-provider-authority',accepted['state']=='admitted-no-transport' and accepted['runtime_authorized'] is False and accepted['runtime_enforced'] is False)
    show=dict(schema_version=1,admission_id=accepted['admission_id'],expected_revision=1,expected_trust_revision=1)
    receipt=f.root/'.swarm'/('blp-admission-'+accepted['admission_id'][3:]+'.receipt');original=receipt.read_bytes()
    check('capsule-private-storage',receipt.stat().st_mode&0o777==0o600)
    check('fixed-replay-and-readback',trust('blp-preparation-admit',admission,token='fixture-sender-token')==accepted and trust('blp-preparation-admission-show',show,token='fixture-sender-token')==accepted and receipt.read_bytes()==original)
    check('admission-does-not-write-trust-or-sql',state.read_bytes()==before and observed_state()==baseline)
    wrong=trust('blp-preparation-admission-show',show,token='fixture-admin-token',success=False)
    check('admin-does-not-impersonate-owner',wrong['failure']['code']=='blp-admission-not-found')
    changed={**admission,'intent':{**intent,'message':'NEW_UNCLASSIFIED_SOURCE'}}
    refused=trust('blp-preparation-admit',changed,token='fixture-sender-token',success=False)
    check('event-cannot-change-fixed-context',refused['failure']['code']=='blp-admission-event-conflict' and receipt.read_bytes()==original)
    refused=trust('blp-preparation-admit',{**changed,'event_id':'new-domain'},token='fixture-sender-token',success=False)
    check('new-context-needs-new-source-attribution',refused['failure']['code']=='blp-classification-required' and len(list((f.root/'.swarm').glob('blp-admission-*.receipt')))==1)
    primary=None
    try:
        web=Server(binary,f.root,out,'admission-http',environment=f.environment);web.capture_async(out/'http.raw')
        csrf=web.request('/api/v1/native-session',observe=False)['csrf_token']
        def request_http(op,value,token='fixture-sender-token'):
            """Call the actual protected route; preserve status/body for each assertion."""
            headers={'Origin':'http://'+web.address,'Cookie':web.cookie,'X-Swarm-CSRF':csrf,'Content-Type':'application/json'}
            if token:headers['Authorization']='Bearer '+token
            conn=http.client.HTTPConnection(web.host,web.port,timeout=None)
            try:
                conn.request('POST','/api/v1/trust-actions?action='+op,body=json.dumps(value).encode(),headers=headers)
                response=conn.getresponse();body=response.read();status=response.status
            finally:conn.close()
            result=json.loads(body);publish(out/('http-'+str(len(checks))+'.json'),dict(status=status,body=result));return status,result,body
        for name,token,value,reason,status in cases:
            actual,result,body=request_http('blp-preparation-admit',value,token)
            check('http-'+name,actual==status and result['failure']['code']==reason and b'PRIVATE_ADMISSION_INPUT' not in body and receipt.read_bytes()==original)
        status,result,body=request_http('blp-preparation-admission-show',{**show,'admission_id':'../trust-actions.json'})
        check('http-no-caller-selected-path',status==400 and result['failure']['code']=='blp-admission-invalid')
        status,result,body=request_http('blp-preparation-admission-show',{**show,'admission_id':'ba-'+'0'*64})
        check('http-missing-capsule-is-explicit',status==404 and result['failure']['code']=='blp-admission-not-found')
        status,result,body=request_http('blp-preparation-admit',{**admission,'event_id':'http-admission'})
        check('http-real-domain-publication',status==200 and result['principal']=='sender' and result['output_classification']==label and b'PRIVATE_ADMISSION_INPUT' not in body)
        check('http-effect-reopens-through-cli',trust('blp-preparation-admit',{**admission,'event_id':'http-admission'},token='fixture-sender-token')==result)
        # Valid control precedes every fault; restores use same-dir atomic publish.
        for name,mutate in [('truncated-frame',lambda raw:b'invalid owned frame'),('root-mismatch',lambda raw:reframe(raw,'root')),('label-downgrade',lambda raw:reframe(raw,'classification')),('invalid-stored-json',lambda raw:reframe(raw,'schema'))]:
            check('control-before-'+name,trust('blp-preparation-admission-show',show,token='fixture-sender-token')==accepted)
            publish(receipt,mutate(original));status,result,body=request_http('blp-preparation-admission-show',show)
            check('http-refuses-'+name,status==500 and b'PRIVATE_ADMISSION_INPUT' not in body and b'STORED_SECRET' not in body)
            publish(receipt,original)
            check('recovery-after-'+name,trust('blp-preparation-admission-show',show,token='fixture-sender-token')==accepted and receipt.read_bytes()==original)
        trust('blp-policy-apply',dict(schema_version=1,event_id='renewal',expected_revision=1,expected_trust_revision=1,reason='Explicit fixture policy revision',policy=policy))
        status,result,body=request_http('blp-preparation-admission-show',{**show,'expected_revision':2})
        check('policy-change-cannot-refresh-old-domain',status==409 and result['failure']['code']=='blp-admission-policy-changed' and receipt.read_bytes()==original)
        lower={**attr(0),'event_id':'http-source-lower','expected_revision':2,'classification':dict(level='public',compartments=[])}
        before_control=state.read_bytes()
        status,result,body=request_http('blp-preparation-input-attribute',lower,'fixture-admin-token')
        check('http-input-attribution-cannot-declassify',status==409 and result['failure']['code']=='blp-declassification-required' and state.read_bytes()==before_control)
        status,result,body=request_http('blp-preparation-input-attribute',{**attr(0),'event_id':'http-source-renewal','expected_revision':2},'fixture-admin-token')
        check('http-source-attribution-real-publication',status==200 and result['attributed'] is True and result['capture_phase']=='pre-admission' and state.read_bytes()!=before_control and b'PRIVATE_ADMISSION_INPUT' not in body)
        status,result,body=request_http('blp-preparation-admit',{**admission,'event_id':'new-policy-domain','expected_revision':2})
        check('policy-renewal-needs-every-source-again',status==409 and result['failure']['code']=='blp-classification-required')
        disabled=copy.deepcopy(config);disabled.update(event_id='disable-sender',expected_revision=1);disabled['identities'][1]['disabled']=True
        trust('config-apply',disabled)
        status,result,body=request_http('blp-preparation-admission-show',show)
        check('current-identity-revocation-refuses',status==403 and result['failure']['code']=='authorization_required' and receipt.read_bytes()==original)
    except BaseException as error:primary=error;raise
    finally:
        if web is not None:
            try:web.cleanup()
            except BaseException as secondary:
                if primary is not None:raise BaseExceptionGroup('journey and owned cleanup failed',[primary,secondary])
                raise
    check('no-provider-launch-or-business-mutation',not sentinel.exists() and observed_state()==baseline)
    check('qualified-image-unchanged',digest(binary)==profile['sha256'])
    publish(out/'result.json',dict(status='PASS',assertions=len(checks),binary_sha256=profile['sha256'],artifacts={str(p.relative_to(out)):digest(p) for p in sorted(out.rglob('*')) if p.is_file()}))
    print('BLP_ADMISSION_PUBLIC_PASS '+str(len(checks)),flush=True);print('BLP_ADMISSION_JOURNEY_SHA256 '+digest(out/'result.json'),flush=True)


def reframe(raw,kind):
    """Corrupt one owned valid capsule while preserving its framing checksum.

    This is explicit fault construction from the observed protocol, never a
    production writer, authority token or acceptance bypass.
    """
    magic,length,sha,body=raw.split(b'\n',3);assert magic==b'SR1' and int(length)==len(body) and hashlib.sha256(body).hexdigest().encode()==sha
    value=json.loads(body)
    if kind=='root':value['root']['directory'][1]+=1
    elif kind=='classification':value['classification']=dict(level='public',compartments=[])
    elif kind=='schema':value['schema_version']='STORED_SECRET'
    else:raise AssertionError(kind)
    body=json.dumps(value).encode();return magic+b'\n'+str(len(body)).zfill(len(length)).encode()+b'\n'+hashlib.sha256(body).hexdigest().encode()+b'\n'+body


if __name__=='__main__':main()
