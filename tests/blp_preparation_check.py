#!/usr/bin/env python3
"""Check actual prepared-source attribution through qualified CLI and HTTP.

Creates one owned Root and runs one inert local Codex double through public send.
Only readonly SQL observes its private provenance; all Trust mutations use public
commands. No paid provider, live database, watchdog or implicit retry. Retains
raw channels and binds them to a printed final manifest hash.
"""
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import sqlite3
import time

from blp_attribution_history_cli import digest
from rust_agent_model_route import Fixture, publish
from rust_native_http_cli import Server


def main():
    """Verify current labels, authenticated join, refusal, replay and recovery.

    Requires a normal qualified binary/profile and fresh output directory.
    Fixture roots follow persisted Admin configuration. Errors preserve outputs;
    the owned HTTP process is explicitly closed, including on assertion failure.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--producer-attribution',action='store_true',help='Attribute through the captured-source public producer instead of caller hashes')
    args=parser.parse_args();binary=args.binary.resolve(strict=True)
    out=args.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile=json.loads(args.profile.read_bytes());sha=digest(binary)
    assert profile['sha256']==sha and profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE'
    assert profile['no_fixture_feature'] and profile['all_core_features_empty'] and profile['all_core_test_false']
    publish(out/'qualified-profile.json',profile)
    f=Fixture(binary,out,'blp-preparation');checks=[];web=None

    def check(name,condition):
        """Retain each named assertion before raising, preserving first failure."""
        checks.append(dict(name=name,passed=bool(condition)));publish(out/'checks.json',checks)
        assert condition,name
        print('PASS '+name,flush=True)

    def trust(op,value,token='fixture-admin-token',success=True):
        """Use the actual CLI; credentials are synthetic environment-only data."""
        f.environment['SWARM_TRUST_TOKEN']=token
        code,result=f.call(['trust',op],value)
        assert (code==0)==success,(op,code,result)
        return result

    f.settings_change(preparation_write_enabled='1',preparation_call_enabled='1',
        agent_allow_unverified_provider_context='1',agent_environment_allow=['PATH','HOME','CODEX_HOME'])
    program='''#!/usr/bin/python3
import json,sys
sys.stdin.buffer.read()
print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':json.dumps({'message':'Inert BLP answer','brief':'Inert brief'})}}),flush=True)
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':1,'output_tokens':1}}),flush=True)
'''
    publish(f.root/'codex',program.encode());os.chmod(f.root/'codex',0o700)
    f.provider['args']=['--model','fixture-standard'];f.provider['model_policy']['levels']['standard']['effort']='';f.save_provider()
    p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='create',expected_revision=0,title='BLP source fixture',text='Fixture need'))
    p=f.ok(['prepare','budget',p['id']],dict(version=1,action='budget',preparation_id=p['id'],event_id='budget',expected_revision=p['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=.5,estimate_source='Inert local fixture',reference_date='2026-11-01')))
    capability=next(c for c in f.ok(['prepare','capabilities'])['providers'] if c['name']=='fixture')
    turn=f.ok(['prepare','send',p['id']],dict(version=1,preparation_id=p['id'],event_id='send',expected_revision=p['revision'],provider='fixture',capability_hash=capability['digest'],message='PRIVATE_BLP_SOURCE',context_mode='full'))
    previous=None
    while True:
        turn=next(t for t in f.ok(['prepare','dialogue',p['id']]) if t['id']==turn['id'])
        if turn['status'] not in ['pending','running','stopping']:break
        if previous!=turn['status']:print('OBSERVE '+turn['status'],flush=True);previous=turn['status']
        time.sleep(.05)
    check('actual-public-send-with-inert-provider',turn['status']=='answered' and turn['answer']['brief']=='Inert brief')
    with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
        row=json.loads(db.execute('SELECT body FROM preparation_turns WHERE id=?',(turn['id'],)).fetchone()[0])
    manifest=row['_native']['prompt_provenance'];raw=row['prompt'].encode()
    check('actual-producer-provenance',manifest['prompt_sha256']==hashlib.sha256(raw).hexdigest())
    publish(out/'prompt-provenance.json',manifest)
    parts=manifest['parts'];cursor=0
    for part in parts:
        assert part['start']==cursor and part['end']>cursor
        assert hashlib.sha256(raw[cursor:part['end']]).hexdigest()==part['sha256'];cursor=part['end']
    check('complete-byte-coverage',cursor==len(raw))
    trust('config-apply',dict(schema_version=1,event_id='trust-init',expected_revision=0,reason='Owned source fixture',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),dict(subject='worker',integrity=1,roles=['requester'],token='fixture-worker-token'),dict(subject='approver',integrity=2,roles=['approver'],token='fixture-approver-token'),dict(subject='uncleared',integrity=3,roles=['admin'],token='fixture-uncleared-token')]),token='')
    policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public'),dict(id='secret',name_fr='Secret',name_en='Secret')],compartments=['a','b'],clearances=dict(admin=dict(level='secret',compartments=['a','b']),worker=dict(level='secret',compartments=['a'])))
    trust('blp-policy-apply',dict(schema_version=1,event_id='policy',expected_revision=0,expected_trust_revision=1,reason='Owned source fixture',policy=policy))
    request=dict(schema_version=1,expected_revision=1,preparation_id=p['id'],turn_id=turn['id'])
    state=f.root/'.swarm/trust-actions.json';before=state.read_bytes()
    refused=trust('blp-preparation-check',request,success=False)
    check('unattributed-source-refuses',refused.get('failure',{}).get('code')=='blp-classification-required')
    check('missing-attribution-does-not-mutate',state.read_bytes()==before)
    if args.producer_attribution:
        base_capture=dict(schema_version=1,event_id='invalid-capture',expected_revision=1,expected_trust_revision=1,
            preparation_id=p['id'],turn_id=turn['id'],expected_turn_revision=turn['source_revision'],source_index=0,
            classification=dict(level='secret',compartments=['a','b']),reason='Explicit captured fixture source')
        producer_cases=[
            ('session-before-row','',{**base_capture,'turn_id':'missing'},'authorization_required',403),
            ('weak-role-before-row','fixture-worker-token',{**base_capture,'turn_id':'missing'},'authorization_required',403),
            ('uncleared-before-row','fixture-uncleared-token',{**base_capture,'turn_id':'missing'},'blp-access-denied',403),
            ('invalid-label','fixture-admin-token',{**base_capture,'classification':dict(level='missing',compartments=[])},'blp-classification-invalid',400),
            ('bad-schema','fixture-admin-token',{**base_capture,'schema_version':0},'blp-preparation-attribute-invalid',400),
            ('bad-index','fixture-admin-token',{**base_capture,'source_index':len(parts)},'blp-preparation-source-not-found',404),
            ('stale-turn','fixture-admin-token',{**base_capture,'expected_turn_revision':turn['source_revision']+1},'preparation-revision-conflict',409),
            ('wrong-parent','fixture-admin-token',{**base_capture,'preparation_id':'other'},'blp-preparation-parent-mismatch',409),
            ('stale-policy','fixture-admin-token',{**base_capture,'expected_revision':0},'blp-policy-stale',409),
            ('stale-trust','fixture-admin-token',{**base_capture,'expected_trust_revision':0},'blp-policy-stale',409),
            ('forged-digest','fixture-admin-token',{**base_capture,'canonical_digest':'a'*64},'format',400),
            ('forged-path','fixture-admin-token',{**base_capture,'project_path':str(f.root)},'format',400),
            ('null-label','fixture-admin-token',{**base_capture,'classification':None},'format',400),
            ('empty-reason','fixture-admin-token',{**base_capture,'reason':''},'blp-preparation-attribute-invalid',400),
            ('missing-turn','fixture-admin-token',{**base_capture,'turn_id':'missing'},'preparation-turn-not-found',404)]
        for name,token,value,reason,_ in producer_cases:
            refused=trust('blp-preparation-attribute',value,token=token,success=False)
            check('producer-cli-'+name,refused.get('failure',{}).get('code')==reason and state.read_bytes()==before
                and 'PRIVATE_BLP_SOURCE' not in json.dumps(refused))
    seen=set()
    for part in parts:
        sha_part=part['sha256']
        if sha_part in seen:continue
        seen.add(sha_part);secret=part['origin'].get('field')=='user_message'
        classification=dict(level='secret' if secret else 'public',compartments=['a','b'] if secret else ['a'])
        if args.producer_attribution:
            capture=dict(schema_version=1,event_id='part-'+str(len(seen)),expected_revision=1,expected_trust_revision=1,
                preparation_id=p['id'],turn_id=turn['id'],expected_turn_revision=turn['source_revision'],source_index=parts.index(part),
                classification=classification,reason='Explicit captured fixture slice')
            attributed=trust('blp-preparation-attribute',capture)
            check('producer-captured-source-'+str(len(seen)),attributed['canonical_digest']==sha_part and attributed['producer_captured'] is True
                and attributed['source_index']==parts.index(part) and attributed['source_count']==len(parts))
            snapshot=state.read_bytes()
            check('producer-replay-'+str(len(seen)),trust('blp-preparation-attribute',capture)==attributed and state.read_bytes()==snapshot)
        else:
            trust('blp-object-attribute',dict(schema_version=1,event_id='part-'+str(len(seen)),expected_revision=1,expected_trust_revision=1,canonical_digest=sha_part,project_path=str(f.root),classification=classification,reason='Exact recorded fixture slice'))
    if args.producer_attribution:
        refused=trust('blp-preparation-attribute',{**capture,'reason':'Changed decision under reused event'},success=False)
        check('producer-event-conflict',refused.get('failure',{}).get('code')=='event_conflict' and state.read_bytes()==snapshot)
        refused=trust('blp-preparation-attribute',{**capture,'source_index':0},success=False)
        check('producer-selector-conflict',refused.get('failure',{}).get('code')=='event_conflict' and state.read_bytes()==snapshot)
        refused=trust('blp-preparation-attribute',{**capture,'event_id':'duplicate-source'},success=False)
        check('producer-duplicate-object-refuses',refused.get('failure',{}).get('code')=='blp-attribution-conflict' and state.read_bytes()==snapshot)
    witness=trust('blp-preparation-check',request)
    check('source-derived-join',witness['output_classification']==dict(level='secret',compartments=['a','b']))
    check('diagnostic-is-not-dispatch-authority',witness['runtime_authorized'] is False and witness['diagnostic_only'] is True)
    check('source-count-and-prompt-binding',witness['source_count']==len(parts) and witness['prompt_sha256']==manifest['prompt_sha256'])
    before=state.read_bytes()
    for name,token,value,reason in [
        ('session-alone','',request,'authorization_required'),
        ('clearance-compartment','fixture-worker-token',request,'blp-access-denied'),
        ('no-grant-before-row-lookup','fixture-approver-token',{**request,'turn_id':'missing'},'blp-access-denied'),
        ('stale-policy','fixture-admin-token',{**request,'expected_revision':0},'blp-policy-stale'),
        ('forged-label','fixture-admin-token',{**request,'classification':dict(level='public',compartments=[])},'format'),
        ('wrong-parent','fixture-admin-token',{**request,'preparation_id':'other'},'blp-preparation-parent-mismatch')]:
        result=trust('blp-preparation-check',value,token=token,success=False)
        check(name,result.get('failure',{}).get('code')==reason and 'PRIVATE_BLP_SOURCE' not in json.dumps(result))
    check('cli-refusals-do-not-mutate',state.read_bytes()==before)
    check('exact-read-replay',trust('blp-preparation-check',request)==witness)
    primary=None
    try:
        web=Server(binary,f.root,out,'blp-preparation-http',environment=f.environment)
        web.capture_async(out/'http-observer.raw')
        csrf=web.request('/api/v1/native-session',observe=False)['csrf_token']
        cases=[('join','fixture-admin-token',request,200,None),
            ('refusal-session','',request,403,'authorization_required'),
            ('refusal-compartment','fixture-worker-token',request,403,'blp-access-denied'),
            ('invalid-schema','fixture-admin-token',{**request,'schema_version':0},400,'blp-preparation-check-invalid'),
            ('stale-policy','fixture-admin-token',{**request,'expected_revision':0},409,'blp-policy-stale'),
            ('missing-turn','fixture-admin-token',{**request,'turn_id':'missing'},404,'preparation-turn-not-found'),
            ('wrong-parent','fixture-admin-token',{**request,'preparation_id':'other'},409,'blp-preparation-parent-mismatch'),
            ('forged-label','fixture-admin-token',{**request,'classification':dict(level='public',compartments=[])},400,'format')]
        for name,token,value,expected_status,expected_code in cases:
            headers={'Origin':'http://'+web.address,'Cookie':web.cookie,'X-Swarm-CSRF':csrf,'Content-Type':'application/json'}
            if token:headers['Authorization']='Bearer '+token
            connection=http.client.HTTPConnection(web.host,web.port,timeout=None)
            try:
                connection.request('POST','/api/v1/trust-actions?action=blp-preparation-check',body=json.dumps(value).encode(),headers=headers)
                response=connection.getresponse();body=response.read();status=response.status
            finally:connection.close()
            observed=json.loads(body)
            publish(out/('http-'+name+'.json'),dict(status=status,body=observed))
            check('http-'+name,status==expected_status and
                (observed==witness if expected_status==200 else observed.get('failure',{}).get('code')==expected_code)
                and b'PRIVATE_BLP_SOURCE' not in body)
        check('http-read-does-not-mutate',state.read_bytes()==before)
        if args.producer_attribution:
            # CLI cases carry (reason, status); the HTTP runner needs (status, reason).
            producer_http=[('replay','fixture-admin-token',capture,200,None)]+[
                (name,token,value,status,reason) for name,token,value,reason,status in producer_cases]
            for name,token,value,expected_status,expected_code in producer_http:
                headers={'Origin':'http://'+web.address,'Cookie':web.cookie,'X-Swarm-CSRF':csrf,'Content-Type':'application/json'}
                if token:headers['Authorization']='Bearer '+token
                connection=http.client.HTTPConnection(web.host,web.port,timeout=None)
                try:
                    connection.request('POST','/api/v1/trust-actions?action=blp-preparation-attribute',body=json.dumps(value).encode(),headers=headers)
                    response=connection.getresponse();body=response.read();status=response.status
                finally:connection.close()
                observed=json.loads(body);publish(out/('producer-http-'+name+'.json'),dict(status=status,body=observed))
                check('producer-http-'+name,status==expected_status and
                    (observed==attributed if expected_status==200 else observed.get('failure',{}).get('code')==expected_code)
                    and b'PRIVATE_BLP_SOURCE' not in body and state.read_bytes()==before)

            # A genuine HTTP publication, not just an HTTP replay of CLI data.
            # Renew the policy explicitly: previous object labels remain floors.
            trust('blp-policy-apply',dict(schema_version=1,event_id='policy-quarantine-renewal',expected_revision=1,
                expected_trust_revision=1,reason='Explicit fixture quarantine and renewal',policy={**policy,'preparation_quarantine':True}))
            renewal={**capture,'event_id':'http-source-renewal','expected_revision':2}
            before_write=state.read_bytes()
            headers={'Origin':'http://'+web.address,'Cookie':web.cookie,'X-Swarm-CSRF':csrf,'Content-Type':'application/json','Authorization':'Bearer fixture-admin-token'}
            connection=http.client.HTTPConnection(web.host,web.port,timeout=None)
            try:
                connection.request('POST','/api/v1/trust-actions?action=blp-preparation-attribute',body=json.dumps(renewal).encode(),headers=headers)
                response=connection.getresponse();body=response.read();status=response.status
            finally:connection.close()
            written=json.loads(body);publish(out/'producer-http-publication.json',dict(status=status,body=written))
            check('producer-http-real-publication-under-quarantine',status==200 and written['producer_captured'] is True
                and written['policy_revision']=='2' and state.read_bytes()!=before_write and b'PRIVATE_BLP_SOURCE' not in body)
            after_write=state.read_bytes()
            check('producer-http-effect-reopens-through-cli',trust('blp-preparation-attribute',renewal)==written and state.read_bytes()==after_write)
            show=trust('blp-show',{})
            check('producer-does-not-lift-quarantine',show['capability']['preparation_quarantined'] is True and show['capability']['runtime_enforced'] is False)
    except BaseException as error:
        primary=error
        raise
    finally:
        if web is not None:
            try:web.cleanup()
            except BaseException as secondary:
                if primary is not None:raise BaseExceptionGroup('Journey and owned cleanup failed',[primary,secondary])
                raise
    check('qualified-image-unchanged',digest(binary)==profile['sha256'])
    files={str(p.relative_to(out)):digest(p) for p in sorted(out.rglob('*')) if p.is_file()}
    publish(out/'result.json',dict(status='PASS',assertions=len(checks),artifacts=files,binary_sha256=profile['sha256']))
    print('BLP_PREPARATION_PUBLIC_PASS '+str(len(checks)),flush=True)
    print('BLP_PREPARATION_JOURNEY_SHA256 '+digest(out/'result.json'),flush=True)


if __name__=='__main__':main()
