#!/usr/bin/env python3
"""Exercise explicit destination binding and last-use transport decisions
through the actual qualified CLI/HTTP.

Owned roots only. All business/Trust mutations use public operations. SQL only
observes state. No capsule corruption is exercised here (the admission journey
already proves framing faults; transport never writes a capsule). No paid
provider or watchdog. The sentinel program proves no provider launch on any
refusal or granted decision; counters prove no turn/budget reservation.
"""
import argparse, copy, http.client, json, os, sqlite3, subprocess, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from rust_agent_model_route import Fixture, publish
from rust_native_http_cli import Server
from blp_attribution_history_cli import digest


def main():
    """Qualify destination attribution (grammar, echo TOCTOU, admin-only,
    replay), granted transport decisions (fixed safe fields, key-free,
    non-emissive, replay-stable), freshness refusals (policy change, settings
    change, connection change), the last-use no-write-down check against a
    recorded lower label, identity revocation and the zero-emission invariant.
    Retain every request/result before assert.
    """
    ap = argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:ap.add_argument('--'+name,type=Path,required=True)
    a = ap.parse_args();binary = a.binary.resolve(strict=True);out = a.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile = json.loads(a.profile.read_bytes());assert profile['sha256']==digest(binary)
    assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['no_fixture_feature'] and profile['all_core_features_empty'] and profile['all_core_test_false']
    publish(out/'qualified-profile.json',profile)
    f = Fixture(binary,out,'blp-transport');checks=[];web=None

    def check(name,value):
        """Persist each exact assertion before failure; no empty PASS result."""
        checks.append(dict(name=name,passed=bool(value)));publish(out/'checks.json',checks);assert value,name;print('PASS '+name,flush=True)

    def trust(op,value,token='fixture-admin-token',success=True):
        """Invoke actual CLI, keeping synthetic bearer values in environment."""
        f.environment['SWARM_TRUST_TOKEN']=token
        code,result=f.call(['trust',op],value);assert (code==0)==success,(op,code,result)
        return result

    def trust_revision():
        """Current trust revision through the public read-only show operation."""
        f.environment['SWARM_TRUST_TOKEN']=''
        code,result=f.call(['trust','show'],None);assert code==0,(code,result)
        return result['revision']

    def observed_state():
        """Read the owned database only; transport must not mutate its state."""
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
    # Publish an owned inert api- connection so destination binding has a real
    # target; `https://example.invalid` never resolves, no provider is contacted.
    connections={'local':dict(id='local',label='Owned transport fixture',base_url='https://example.invalid/v1',model='fixture-model',key='PRIVATE_FIXTURE_KEY',disabled=False),
        'other':dict(id='other',label='Owned second fixture',base_url='https://other.invalid/v1',model='other-model',key='PRIVATE_OTHER_KEY',disabled=False)}
    publish(f.root/'.swarm/ai-connections.json',json.dumps(connections).encode())
    os.chmod(f.root/'.swarm/ai-connections.json',0o600)
    f.provider['args']=['--model','fixture-standard'];f.provider['model_policy']['levels']['standard']['effort']='';f.save_provider()
    p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='create',expected_revision=0,title='Transport fixture',text='Fixture need'))
    p=f.ok(['prepare','budget',p['id']],dict(version=1,action='budget',preparation_id=p['id'],event_id='budget',expected_revision=p['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=.5,estimate_source='inert fixture',reference_date='2026-11-01')))
    # The intent targets the api- connection (empty level, fresh capability).
    cap=next(c for c in f.ok(['prepare','capabilities'])['providers'] if c['name']=='api-local')
    intent=dict(version=1,event_id='unsent-intent',preparation_id=p['id'],expected_revision=p['revision'],provider='api-local',capability_hash=cap['digest'],message='PRIVATE_TRANSPORT_INPUT',context_mode='full')
    identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),dict(subject='sender',integrity=1,roles=['requester'],token='fixture-sender-token'),dict(subject='low',integrity=1,roles=['requester'],token='fixture-low-token'),dict(subject='uncleared',integrity=3,roles=['admin'],token='fixture-uncleared-token')]
    config=dict(schema_version=1,event_id='init',expected_revision=0,reason='Owned transport fixture',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=identities)
    trust('config-apply',config,token='')
    label=dict(level='secret',compartments=['a','b'])
    policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public'),dict(id='secret',name_fr='Secret',name_en='Secret'),dict(id='top',name_fr='Très secret',name_en='Top')],compartments=['a','b','c'],clearances=dict(admin=dict(level='top',compartments=['a','b','c']),sender=dict(level='top',compartments=['a','b','c']),low=dict(level='secret',compartments=['a'])),preparation_quarantine=False)
    trust('blp-policy-apply',dict(schema_version=1,event_id='policy',expected_revision=0,expected_trust_revision=1,reason='Explicit transport fixture policy',policy=policy))
    admission=dict(schema_version=1,event_id='admission',expected_revision=1,expected_trust_revision=1,intent=intent)
    baseline=observed_state();state=f.root/'.swarm/trust-actions.json';before=state.read_bytes()
    def dest_req(event,revision=1,trust_revision=1,connection='api-local',classification=None,
        expected_base_url='https://example.invalid/v1',expected_model='fixture-model',**over):
        """Exact observable echo of the currently resolved local connection."""
        return dict(schema_version=1,event_id=event,expected_revision=revision,expected_trust_revision=trust_revision,
            connection=connection,classification=label if classification is None else classification,
            expected_base_url=expected_base_url,expected_model=expected_model,
            reason='Explicit destination decision fixture',**over)
    def go(capsule,revision=1,trust_revision=1,connection='api-local',
        expected_base_url='https://example.invalid/v1',expected_model='fixture-model',**over):
        """Last-use request naming the capsule and the exact intended endpoint."""
        return dict(schema_version=1,admission_id=capsule['admission_id'],expected_revision=revision,expected_trust_revision=trust_revision,
            connection=connection,expected_base_url=expected_base_url,expected_model=expected_model,**over)
    # Destination attribution grammar: a connection outside the `api-` adapter
    # namespace refuses before any resolution or recording.
    refused=trust('blp-preparation-destination-attribute',dest_req('dest-early',connection='fixture'),token='fixture-admin-token',success=False)
    check('attribution-is-connection-grammar',refused['failure']['code']=='blp-destination-invalid' and state.read_bytes()==before)
    # Admin echo mismatch: the producer refuses before recording anything.
    refused=trust('blp-preparation-destination-attribute',dest_req('dest-echo',expected_base_url='https://attacker.invalid/v1'),token='fixture-admin-token',success=False)
    check('echo-mismatch-refuses-before-recording',refused['failure']['code']=='blp-destination-echo-mismatch' and state.read_bytes()==before)
    # Only an administrator can bind a destination label.
    refused=trust('blp-preparation-destination-attribute',dest_req('dest-nonadmin'),token='fixture-sender-token',success=False)
    check('destination-binding-needs-admin',refused['failure']['code']=='authorization_required' and state.read_bytes()==before)
    # A transport request for a missing capsule refuses explicitly.
    refused=trust('blp-preparation-transport',go({'admission_id':'ba-'+'0'*64}),token='fixture-sender-token',success=False)
    check('transport-missing-capsule-explicit',refused['failure']['code']=='blp-admission-not-found')
    # Admission with zero attributed sources refuses; nothing is emitted.
    refused=trust('blp-preparation-admit',admission,token='fixture-sender-token',success=False)
    check('unclassified-admission-refuses',refused['failure']['code']=='blp-classification-required')
    check('unknown-source-no-capsule-or-business-mutation',not list((f.root/'.swarm').glob('blp-admission-*.receipt')) and observed_state()==baseline and state.read_bytes()==before)
    attr=lambda index:dict(schema_version=1,event_id='input-'+str(index),expected_revision=1,expected_trust_revision=1,intent=intent,source_index=index,classification=label,reason='Explicit per-slice classification fixture')
    refused=trust('blp-preparation-input-attribute',attr(0),token='fixture-sender-token',success=False)
    check('classification-needs-admin',refused['failure']['code']=='authorization_required' and state.read_bytes()==before)
    first=trust('blp-preparation-input-attribute',attr(0));count=first['source_count']
    check('actual-pre-admission-source',first['producer_captured'] is True and first['capture_phase']=='pre-admission' and count>1 and 'PRIVATE_TRANSPORT_INPUT' not in json.dumps(first))
    for index in range(1,count):
        code,result=f.call(['trust','blp-preparation-input-attribute'],attr(index))
        check('explicit-source-'+str(index),code==0 or (code!=0 and result['failure']['code']=='blp-attribution-conflict'))
    # Now bind the destination label (admin), with a valid replay proof.
    bound=trust('blp-preparation-destination-attribute',dest_req('dest-bind'))
    check('destination-binding-recorded',bound['capture_phase']=='destination-binding' and bound['producer_captured'] is True and bound['destination']['model']=='fixture-model')
    check('destination-reply-is-key-free','PRIVATE_FIXTURE_KEY' not in json.dumps(bound))
    check('destination-replay-is-exact',trust('blp-preparation-destination-attribute',dest_req('dest-bind'))==bound)
    # Snapshot right before admission: attributions/binding legitimately wrote
    # the journal above; the admission itself must write nothing.
    pre_admit_state=state.read_bytes();pre_admit_observed=observed_state()
    accepted=trust('blp-preparation-admit',admission,token='fixture-sender-token')
    check('authenticated-nonadmin-domain',accepted['principal']=='sender' and accepted['output_classification']==label and accepted['source_count']==count)
    check('admission-is-not-provider-authority',accepted['state']=='admitted-no-transport' and accepted['runtime_authorized'] is False and accepted['runtime_enforced'] is False)
    show=dict(schema_version=1,admission_id=accepted['admission_id'],expected_revision=1,expected_trust_revision=1)
    check('fixed-replay-and-readback',trust('blp-preparation-admit',admission,token='fixture-sender-token')==accepted and trust('blp-preparation-admission-show',show,token='fixture-sender-token')==accepted)
    check('admission-does-not-write-trust-or-sql',state.read_bytes()==pre_admit_state and observed_state()==pre_admit_observed)
    # Granted last-use decision: fixed safe fields, key-free, non-emissive.
    decision=trust('blp-preparation-transport',go(accepted),token='fixture-sender-token')
    check('transport-granted-fixed-fields',decision['authorized'] is True and decision['state']=='transport-authorized-not-sent' and decision['principal']=='sender' and decision['destination']['connection']=='local' and decision['destination_label']==label)
    check('transport-explicitly-non-emissive',decision['runtime_authorized'] is False and decision['runtime_enforced'] is False)
    check('transport-reply-is-key-free','PRIVATE_FIXTURE_KEY' not in json.dumps(decision) and 'PRIVATE_TRANSPORT_INPUT' not in json.dumps(decision))
    check('transport-source-count-matches-capsule',decision['source_count']==accepted['source_count'])
    check('transport-creates-no-turn-or-reservation',observed_state()==baseline and not sentinel.exists())
    # Replay under unchanged conditions returns the identical decision.
    check('transport-replay-is-identical',trust('blp-preparation-transport',go(accepted),token='fixture-sender-token')==decision)
    # Ownership: another principal cannot observe this capsule.
    refused=trust('blp-preparation-transport',go(accepted),token='fixture-admin-token',success=False)
    check('transport-owner-enforced',refused['failure']['code']=='blp-admission-not-found')
    # Wrong echo of the intended endpoint refuses before any capsule read.
    refused=trust('blp-preparation-transport',go(accepted,expected_base_url='https://attacker.invalid/v1'),token='fixture-sender-token',success=False)
    check('transport-echo-mismatch-refuses',refused['failure']['code']=='blp-destination-echo-mismatch')
    # A connection the capsule was not admitted for refuses on the recorded
    # freshness check once both connections are configured.
    refused=trust('blp-preparation-transport',go(accepted,connection='api-other',
        expected_base_url='https://other.invalid/v1',expected_model='other-model'),token='fixture-sender-token',success=False)
    check('transport-other-connection-refuses',refused['failure']['code'] in ('preparation-provider-not-configured','blp-transport-connection-changed'))
    # Last-use no-write-down: bind a public label under a fresh policy revision
    # cannot authorize a secret capsule. Policy renewal resets all attributions,
    # so the flow is: new revision, public binding, secret transport refusal.
    trust('blp-policy-apply',dict(schema_version=1,event_id='policy-public',expected_revision=1,expected_trust_revision=1,reason='Explicit public-binding fixture',policy=policy))
    # The request must name the CURRENT entry revision (2) to pass the outer
    # currency check; the capsule then fails verification against it
    # (policy_revision 1 != 2) with blp-admission-policy-changed.
    refused=trust('blp-preparation-transport',go(accepted,revision=2),token='fixture-sender-token',success=False)
    check('policy-renewal-invalidates-capsule',refused['failure']['code']=='blp-admission-policy-changed')
    # A public rebinding of the already-secret api-local digest is itself a
    # declassification and refuses before anything else; no-write-down needs a
    # FRESH digest whose first binding is public.
    refused=trust('blp-preparation-destination-attribute',dest_req('dest-public',revision=2,classification=dict(level='public',compartments=[])),success=False)
    check('public-rebind-of-secret-digest-refuses',refused['failure']['code']=='blp-declassification-required')
    # Fresh digest: admit a secret capsule for the second connection, then bind
    # a public label there (first binding for that digest) — the last-use
    # no-write-down check must refuse with blp-access-denied.
    cap_other=next(c for c in f.ok(['prepare','capabilities'])['providers'] if c['name']=='api-other')
    intent2=dict(intent,event_id='unsent-intent-2',provider='api-other',capability_hash=cap_other['digest'])
    admission2=dict(schema_version=1,event_id='admission-2',expected_revision=2,expected_trust_revision=1,intent=intent2)
    attr2=lambda index:dict(schema_version=1,event_id='input-2-'+str(index),expected_revision=2,expected_trust_revision=1,intent=intent2,source_index=index,classification=dict(level='secret',compartments=['a','b']),reason='Explicit per-slice classification fixture')
    first2=trust('blp-preparation-input-attribute',attr2(0));count2=first2['source_count']
    for index in range(1,count2):
        code,result=f.call(['trust','blp-preparation-input-attribute'],attr2(index))
        assert code==0 or result['failure']['code']=='blp-attribution-conflict',(index,code,result)
    bound2=trust('blp-preparation-destination-attribute',dest_req('dest-public',revision=2,connection='api-other',
        classification=dict(level='public',compartments=[]),expected_base_url='https://other.invalid/v1',expected_model='other-model'))
    check('public-destination-binding-recorded',bound2['capture_phase']=='destination-binding' and bound2['destination']['model']=='other-model')
    accepted2=trust('blp-preparation-admit',admission2,token='fixture-sender-token')
    # Secret sources joined under the secret policy revision: the capsule output
    # label is secret/a,b; the recorded destination label is public — the
    # last-use no-write-down check must refuse with blp-access-denied.
    refused=trust('blp-preparation-transport',go(accepted2,revision=2,connection='api-other',
        expected_base_url='https://other.invalid/v1',expected_model='other-model'),token='fixture-sender-token',success=False)
    check('transport-no-write-down-refuses',refused['failure']['code']=='blp-access-denied')
    check('no-write-down-emits-nothing',not sentinel.exists() and observed_state()==baseline)
    # Settings freshness: any saved-settings change also invalidates the live
    # capsule. The current capsule (admission-2, revision 2) dies on a change.
    f.settings_change(preparation_call_message_max_bytes='999999')
    refused=trust('blp-preparation-transport',go(accepted2,revision=2,connection='api-other',
        expected_base_url='https://other.invalid/v1',expected_model='other-model'),token='fixture-sender-token',success=False)
    check('settings-change-refuses-transport',refused['failure']['code'] in ('blp-transport-settings-changed','blp-admission-policy-changed'))
    # Identity revocation: a disabled current identity refuses everything.
    disabled=copy.deepcopy(config);disabled.update(event_id='disable-sender',expected_revision=1);disabled['identities'][1]['disabled']=True
    trust('config-apply',disabled)
    refused=trust('blp-preparation-transport',go(accepted2,revision=2,connection='api-other',
        expected_base_url='https://other.invalid/v1',expected_model='other-model'),token='fixture-sender-token',success=False)
    check('revoked-identity-transport-refuses',refused['failure']['code']=='authorization_required')
    check('revoked-identity-emits-nothing',not sentinel.exists() and observed_state()==baseline)
    try:
        web=Server(binary,f.root,out,'transport-http',environment=f.environment)
        web.capture_async(out/'http.raw')
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
        # The refused identity maps to 403 on the protected route.
        status,result,body=request_http('blp-preparation-transport',go(accepted2,revision=2,connection='api-other',
            expected_base_url='https://other.invalid/v1',expected_model='other-model'),token='fixture-sender-token')
        check('http-revoked-identity-maps-403',status==403 and result['failure']['code']=='authorization_required' and b'PRIVATE_FIXTURE_KEY' not in body)
        # Re-enable sender through public config-apply; trust revision rises.
        reenabled=copy.deepcopy(config);reenabled.update(event_id='reenable-sender',expected_revision=2);reenabled['identities'][1]['disabled']=False
        f.environment['SWARM_TRUST_TOKEN']='fixture-admin-token'
        code,r=f.call(['trust','config-apply'],reenabled);assert code==0,(code,r)
        revision3=trust_revision();assert revision3==3,revision3
        # A new trust revision orphans the old policy entry: renew it explicitly.
        trust('blp-policy-apply',dict(schema_version=1,event_id='policy-3',expected_revision=2,expected_trust_revision=revision3,reason='Explicit renewal under new trust revision',policy=policy))
        # A brand-new capsule under the current policy/trust revisions.
        # The settings change also rotated the capability digest; refetch it.
        cap3=next(c for c in f.ok(['prepare','capabilities'])['providers'] if c['name']=='api-local')
        intent3=dict(intent,event_id='unsent-intent-3',capability_hash=cap3['digest'])
        admission3=dict(schema_version=1,event_id='admission-3',expected_revision=3,expected_trust_revision=revision3,intent=intent3)
        refused=trust('blp-preparation-admit',admission3,token='fixture-sender-token',success=False)
        check('new-policy-needs-every-source-again',refused['failure']['code']=='blp-classification-required')
        attr3=lambda index:dict(schema_version=1,event_id='input-3-'+str(index),expected_revision=3,expected_trust_revision=revision3,intent=intent3,source_index=index,classification=label,reason='Explicit per-slice classification fixture')
        first3=trust('blp-preparation-input-attribute',attr3(0));count3=first3['source_count']
        for index in range(1,count3):
            code,result=f.call(['trust','blp-preparation-input-attribute'],attr3(index))
            assert code==0 or result['failure']['code']=='blp-attribution-conflict',(index,code,result)
        bound3=trust('blp-preparation-destination-attribute',dest_req('dest-bind-3',revision=3,trust_revision=revision3))
        check('destination-rebinding-under-new-trust',bound3['capture_phase']=='destination-binding')
        accepted3=trust('blp-preparation-admit',admission3,token='fixture-sender-token')
        check('readmission-under-current-policy',accepted3['principal']=='sender' and accepted3['source_count']==count3)
        go3=lambda **over: go(accepted3,revision=3,trust_revision=revision3,**over)
        status,result,body=request_http('blp-preparation-transport',go3(),token='fixture-sender-token')
        check('http-granted-decision-maps-200',status==200 and result['authorized'] is True and result['state']=='transport-authorized-not-sent' and result['destination_label']==label and b'PRIVATE_FIXTURE_KEY' not in body and b'PRIVATE_TRANSPORT_INPUT' not in body)
        check('http-effect-reopens-through-cli',trust('blp-preparation-transport',go3(),token='fixture-sender-token')==result)
        # Bad grammar on the protected route maps to 400, not a crash.
        status,result,body=request_http('blp-preparation-transport',{**go3(),'schema_version':0})
        check('http-bad-schema-maps-400',status==400 and result['failure']['code']=='blp-transport-invalid')
        # Unknown capsule on the protected route maps to 404.
        status,result,body=request_http('blp-preparation-transport',{**go3(),'admission_id':'ba-'+'0'*64})
        check('http-missing-capsule-maps-404',status==404 and result['failure']['code']=='blp-admission-not-found')
        # Echo mismatch on the protected route maps to 400.
        status,result,body=request_http('blp-preparation-transport',{**go3(),'expected_base_url':'https://attacker.invalid/v1'})
        check('http-echo-mismatch-maps-400',status==400 and result['failure']['code']=='blp-destination-echo-mismatch')
        # Non-owner via HTTP maps to 404 (owner identity is part of the key).
        status,result,body=request_http('blp-preparation-transport',go3(),token='fixture-admin-token')
        check('http-owner-enforced-404',status==404 and result['failure']['code']=='blp-admission-not-found')
    except BaseException as error:primary=error;raise
    finally:
        if web is not None:
            try:web.cleanup()
            except BaseException as secondary:
                if primary is not None:raise BaseExceptionGroup('journey and owned cleanup failed',[primary,secondary])
                raise
    check('no-provider-launch-or-business-mutation',not sentinel.exists() and observed_state()==baseline)
    check('qualified-image-unchanged',digest(binary)==profile['sha256'])
    publish(out/'result.json',dict(status='PASS',assertions=len(checks),binary_sha256=profile['sha256'],artifacts={str(pp.relative_to(out)):digest(pp) for pp in sorted(out.rglob('*')) if pp.is_file()}))
    print('BLP_TRANSPORT_PUBLIC_PASS '+str(len(checks)),flush=True);print('BLP_TRANSPORT_JOURNEY_SHA256 '+digest(out/'result.json'),flush=True)


if __name__=='__main__':main()
