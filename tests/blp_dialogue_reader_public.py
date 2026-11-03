#!/usr/bin/env python3
"""First real HTTP boundary of the BF-16a classified dialogue reader gate.

One owned Root, one inert local provider double and one genuinely answered
turn through the public producer (prepare create/budget/send). The exact send
intent object is kept and admitted afterwards through real public Trust
operations (config-apply, blp-policy-apply, per-slice
blp-preparation-input-attribute, blp-preparation-admit with the SAME intent),
so the admission capsule matches THIS preparation/provider/request_hash
without any private SQL write or fabricated capsule. Before the injection a
bare-session GET on /api/v1/preparations/dialogue must stay 200 (legacy
zero-matched path, anti-common-reject control); after admission the same bare
GET must refuse 403 blp-dialogue-authentication-required without any private
byte, and the cleared sender must read 200 the exact same nonempty answer
with the private prompt still absent. Readonly SQL/journal/capsule snapshots
around each GET must stay identical. No recipe, HEAD, stale, revoked or
malformed matrix here: this lot is the first boundary only. If the public
admission refuses to match the existing turn, the script stops at that exact
refusal with its raw retained evidence instead of fabricating a success.
"""
import argparse, hashlib, http.client, json, os, sqlite3, sys, time
from pathlib import Path
from blp_attribution_history_cli import digest
from rust_agent_model_route import Fixture, publish
from rust_native_http_cli import Server


def main():
    """Produce, admit and read one real classified dialogue over actual HTTP.

    Requires a normal qualified binary/profile and a fresh output directory.
    Every Trust mutation uses public CLI operations; SQL, journal and capsule
    reads are readonly observation of this owned fixture. The owned HTTP
    process is explicitly closed, including on assertion failure. No
    watchdog, retry, poll cap, paid provider or external network participates.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();binary=args.binary.resolve(strict=True)
    out=args.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile=json.loads(args.profile.read_bytes());sha=digest(binary)
    assert profile['sha256']==sha and profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE'
    assert profile['no_fixture_feature'] and profile['all_core_features_empty'] and profile['all_core_test_false']
    publish(out/'qualified-profile.json',profile)
    publish(out/'run-inputs.json',dict(argv=[str(item) for item in sys.argv],binary=str(binary),binary_sha256=sha,
        profile_sha256=digest(args.profile.read_bytes()),source_sha256=digest(Path(__file__).read_bytes()),
        scope='Environment not serialized: inherits the fixture environment holding synthetic tokens only'))
    f=Fixture(binary,out,'blp-dialogue-reader');checks=[];web=None

    def check(name,value):
        """Retain each named assertion before raising, preserving first failure."""
        checks.append(dict(name=name,passed=bool(value)));publish(out/'checks.json',checks)
        assert value,name
        print('PASS '+name,flush=True)

    def trust(op,value,token='fixture-admin-token',success=True):
        """Invoke the actual CLI; bearer values are synthetic fixture environment data.

        Each raw exit code and result is published before any assertion, so an
        unexpected refusal (notably an admission refusing the existing turn)
        remains exact retained evidence; no fabricated success is possible.
        """
        f.environment['SWARM_TRUST_TOKEN']=token
        code,result=f.call(['trust',op],value)
        publish(out/('trust-'+op+'-'+str(len(checks))+'.json'),dict(exit_code=code,result=result))
        assert (code==0)==success,(op,code,result)
        return result

    def observed_state():
        """Readonly marker of owned SQL rows, trust journal and admission capsules.

        Compares preparation/reservation rows, journal bytes (None while Trust
        is not yet bootstrapped) and every capsule digest around one GET.
        Admission operations intentionally change this marker between reads;
        no single before/after pair spans them.
        """
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            marker=dict(turns=db.execute('SELECT count(*) FROM preparation_turns').fetchone()[0],
                reservations=db.execute('SELECT count(*) FROM preparation_reservations').fetchone()[0],
                preparations=db.execute('SELECT id,revision,hex(body) FROM preparations ORDER BY id').fetchall())
        journal=f.root/'.swarm/trust-actions.json'
        marker['trust']=journal.read_bytes() if journal.exists() else None
        marker['capsules']=sorted((path.name,digest(path)) for path in (f.root/'.swarm').glob('blp-admission-*.receipt'))
        return marker

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
    p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='create',expected_revision=0,title='BLP dialogue reader fixture',text='Fixture need'))
    p=f.ok(['prepare','budget',p['id']],dict(version=1,action='budget',preparation_id=p['id'],event_id='budget',expected_revision=p['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=.5,estimate_source='Inert local fixture',reference_date='2026-11-01')))
    capability=next(c for c in f.ok(['prepare','capabilities'])['providers'] if c['name']=='fixture')
    # The exact send intent object is kept: the later public admission must
    # capture the identical preparation/provider/request_hash.
    intent=dict(version=1,preparation_id=p['id'],event_id='send',expected_revision=p['revision'],provider='fixture',capability_hash=capability['digest'],message='PRIVATE_BLP_SOURCE',context_mode='full')
    turn=f.ok(['prepare','send',p['id']],intent)
    previous=None
    while True:
        turn=next(t for t in f.ok(['prepare','dialogue',p['id']]) if t['id']==turn['id'])
        if turn['status'] not in ['pending','running','stopping']:break
        if previous!=turn['status']:print('OBSERVE '+turn['status'],flush=True);previous=turn['status']
        time.sleep(.05)
    check('actual-public-send-with-inert-provider',turn['status']=='answered' and turn['answer']['brief']=='Inert brief')
    with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
        row=json.loads(db.execute('SELECT body FROM preparation_turns WHERE id=?',(turn['id'],)).fetchone()[0])
    manifest=row['_native']['prompt_provenance'];prompt=row['prompt']
    check('actual-producer-provenance',manifest['prompt_sha256']==hashlib.sha256(prompt.encode()).hexdigest())
    publish(out/'prompt-provenance.json',manifest)
    baseline=observed_state()
    primary=None
    try:
        web=Server(binary,f.root,out,'blp-dialogue-reader',environment=f.environment);web.capture_async(out/'http-observer.raw')
        def get_dialogue(name,token):
            """One real protected GET with the owned cookie plus optional bearer.

            The dialogue route carries the reader credential in the
            Authorization header, which the shared Server helper cannot send,
            so this uses the established http.client pattern with Origin,
            session cookie and Bearer token. Retains status and the raw body
            in the owned output before parsing; no retry and no timeout
            ceiling. Returns (status, raw bytes, parsed JSON).
            """
            headers={'Origin':'http://'+web.address,'Cookie':web.cookie}
            if token:headers['Authorization']='Bearer '+token
            connection=http.client.HTTPConnection(web.host,web.port,timeout=None)
            try:
                connection.request('GET','/api/v1/preparations/dialogue?id='+p['id'],headers=headers)
                response=connection.getresponse();raw=response.read();status=response.status
            finally:connection.close()
            publish(out/('http-'+name+'.json'),dict(status=status,body=raw.decode()))
            return status,raw,json.loads(raw)
        before=observed_state()
        status,raw_legacy,legacy_turns=get_dialogue('legacy-bare-200','')
        check('legacy-bare-dialogue-200-before-admission',status==200 and len(legacy_turns)==1
            and not list((f.root/'.swarm').glob('blp-admission-*.receipt'))
            and legacy_turns[0]['id']==turn['id'] and legacy_turns[0]['status']=='answered'
            and legacy_turns[0]['question']=='PRIVATE_BLP_SOURCE' and legacy_turns[0]['provider']=='fixture'
            and legacy_turns[0]['answer']['message']=='Inert BLP answer' and legacy_turns[0].get('prompt','')=='')
        check('legacy-get-no-mutation',observed_state()==before)
        label=dict(level='secret',compartments=['a','b'])
        identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),dict(subject='sender',integrity=1,roles=['requester'],token='fixture-sender-token'),dict(subject='low',integrity=1,roles=['requester'],token='fixture-low-token'),dict(subject='uncleared',integrity=3,roles=['admin'],token='fixture-uncleared-token')]
        trust('config-apply',dict(schema_version=1,event_id='init',expected_revision=0,reason='Owned dialogue reader fixture',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=identities),token='')
        policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public'),dict(id='secret',name_fr='Secret',name_en='Secret'),dict(id='top',name_fr='Très secret',name_en='Top')],compartments=['a','b','c'],clearances=dict(admin=dict(level='top',compartments=['a','b','c']),sender=dict(level='top',compartments=['a','b','c']),low=dict(level='secret',compartments=['a'])),preparation_quarantine=True)
        trust('blp-policy-apply',dict(schema_version=1,event_id='policy',expected_revision=0,expected_trust_revision=1,reason='Explicit dialogue reader fixture policy',policy=policy))
        attr=lambda index:dict(schema_version=1,event_id='input-'+str(index),expected_revision=1,expected_trust_revision=1,
            intent=intent,source_index=index,classification=label,reason='Explicit per-slice classification fixture')
        first=trust('blp-preparation-input-attribute',attr(0));count=first['source_count']
        check('actual-source-attribution',first['producer_captured'] is True and first['capture_phase']=='pre-admission'
            and count>1 and 'PRIVATE_BLP_SOURCE' not in json.dumps(first))
        for index in range(1,count):
            code,result=f.call(['trust','blp-preparation-input-attribute'],attr(index))
            check('explicit-source-'+str(index),code==0 or (code!=0 and result['failure']['code']=='blp-attribution-conflict'))
        admission=dict(schema_version=1,event_id='admission',expected_revision=1,expected_trust_revision=1,intent=intent)
        accepted=trust('blp-preparation-admit',admission,token='fixture-sender-token')
        receipt=f.root/'.swarm'/('blp-admission-'+accepted['admission_id'][3:]+'.receipt')
        after=observed_state()
        check('actual-public-admission-matches-sent-intent',accepted['principal']=='sender'
            and accepted['state']=='admitted-no-transport' and accepted['runtime_authorized'] is False
            and accepted['output_classification']==label and 'PRIVATE_BLP_SOURCE' not in json.dumps(accepted))
        check('admission-capsule-and-no-sql-mutation',receipt.exists() and after['turns']==baseline['turns']
            and after['reservations']==baseline['reservations'] and after['preparations']==baseline['preparations'])
        before=observed_state()
        status,raw_refused,refused=get_dialogue('bare-refusal','')
        secrets=[b'PRIVATE_BLP_SOURCE',b'Inert BLP answer',b'Inert brief',prompt.encode()]
        check('classified-bare-session-refuses',status==403
            and refused.get('failure',{}).get('code')=='blp-dialogue-authentication-required'
            and all(secret not in raw_refused for secret in secrets))
        check('refusal-no-mutation',observed_state()==before)
        before=observed_state()
        status,raw_authorized,authorized=get_dialogue('authorized-200','fixture-sender-token')
        check('authorized-reader-reads-same-answer',status==200 and len(authorized)==1
            and authorized[0]==legacy_turns[0] and authorized[0]['answer']['message']=='Inert BLP answer')
        check('positive-read-no-mutation',observed_state()==before)
        check('private-prompt-never-served',all(prompt.encode() not in body for body in [raw_legacy,raw_refused,raw_authorized]))
    except BaseException as error:
        primary=error
        raise
    finally:
        if web is not None:
            try:web.cleanup()
            except BaseException as secondary:
                if primary is not None:raise BaseExceptionGroup('journey and owned cleanup failed',[primary,secondary])
                raise
    check('qualified-image-unchanged',digest(binary)==profile['sha256'])
    publish(out/'result.json',dict(status='PASS',assertions=len(checks),binary_sha256=profile['sha256'],
        artifacts={str(path.relative_to(out)):digest(path) for path in sorted(out.rglob('*')) if path.is_file()}))
    print('BLP_DIALOGUE_READER_PUBLIC_PASS '+str(len(checks)),flush=True)
    print('BLP_DIALOGUE_READER_JOURNEY_SHA256 '+digest(out/'result.json'),flush=True)


if __name__=='__main__':main()
