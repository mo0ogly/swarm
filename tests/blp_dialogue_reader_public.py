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
with the private prompt still absent. Readonly full-row SQL, journal and
capsule snapshots around each GET must stay identical. After that received
positive control, the incremental reader matrix runs over the same actual
HTTP route: HEAD bare/authorized keep empty bodies (retained raw with
status and headers, never JSON-parsed), an unknown bearer refuses 403
authorization_required, the known low-cleared identity refuses 403
blp-dialogue-access-denied, and the authorized reader recovers the exact
same DTO after each negative, every read bracketed by full snapshots.
Finally the public freshness boundary: a second blp-policy-apply renews the
SAME policy to revision 2, the known-cleared GET and HEAD must then refuse
403 with the single declared reader staleness code
blp-dialogue-policy-stale (the reader adapts the capsule's verified
ordinary staleness refusal to this one reader code only after the full
admission re-verification; the internal admission code
never reaches the HTTP layer) with no private byte and an empty HEAD body,
every read bracketed by
full snapshots, and the supported public recovery — re-attributing every
captured source under revision 2 with new event ids, then re-admitting the
SAME intent as a new admission event — must restore the authorized 200 to
the exact same DTO. After that received recovery, two stored-corruption
negative guards (corrupt_history_guard) corrupt ONLY the fixture's own OLD
stale admission receipt — a wrong-Root stored frame, then an invalid stored
schema JSON, both via the existing blp_admission_public.reframe fault
construction — while the valid current receipt stays untouched; each guard
requires an actual authorized GET to fail 500 with no private byte and the
exact expected fault code (preparation-root-instance-changed for the
wrong-Root frame, blp-admission-record-invalid for the invalid stored schema
— the codes actually observed on the received 325 supervisor run) and
restores the exact original bytes in a finally block. This is not a
poisoned-capsule or malformed-capsule matrix (exactly two recorded faults, no
recipe expansion). No private capsule removal, policy rollback, capability
bypass or fabricated admission participates; if the supported recovery
cannot restore the read (the old matched admission capsule can no longer
re-verify), the exact raw observed refusal stays retained as frontier
evidence. If the public
admission refuses to match the existing turn, the script stops at that exact
refusal with its raw retained evidence instead of fabricating a success.
"""
import argparse, hashlib, http.client, json, os, sqlite3, sys, time
from pathlib import Path
from blp_attribution_history_cli import digest
from blp_admission_public import reframe
from rust_agent_model_route import Fixture, publish
from rust_native_http_cli import Server


def main():
    """Produce, admit and read one real classified dialogue over actual HTTP.

    Requires a normal qualified binary/profile and a fresh output directory.
    Every Trust mutation uses public CLI operations; SQL, journal and capsule
    reads are readonly observation of this owned fixture. After the received
    positive boundary, HEAD probes and negative-reader GETs (unknown bearer,
    low clearance) plus an authorized recovery GET after each refusal run
    over the same actual route, each read bracketed by full readonly
    snapshots. A final public policy-renewal freshness boundary refuses every
    reader with the single declared code blp-dialogue-policy-stale and then
    attempts the supported public recovery (re-attribution under the renewed
    revision, then same-intent re-admission) toward the intended 200,
    retaining the exact raw refusal as frontier evidence if the read cannot
    be restored. After that received recovery, two stored-corruption negative
    guards over the OLD stale admission receipt — a wrong-Root frame and an
    invalid stored schema JSON — each prove a 500 with no private byte, the
    exact expected fault code (see corrupt_history_guard) and an exact-byte
    finally restoration. The owned
    HTTP process is explicitly closed, including on assertion failure. No
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
        profile_sha256=digest(args.profile),source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
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

    def sql_rows(db,table):
        """Snapshot every row and column of one owned table, byte-exact.

        SELECT * pins the comparison to the actual running schema (the full
        initial_schema_37.sql definitions today: preparation_turns
        id,preparation_id,status,body; preparation_reservations
        turn_id,preparation_id,work_id,amount,work_amount,state; preparations
        id,work_id,revision,body) instead of a hand-picked subset, and ORDER
        BY 1 follows the first column, the TEXT PRIMARY KEY of all three
        tables, so the row order is total and deterministic. BLOB cells
        become hex strings: byte-exact for the equality comparison and
        directly JSON-serializable for the published snapshot. Returns
        dict(columns=names,rows=values) so retained evidence is
        self-describing against the schema actually read.
        """
        cursor=db.execute('SELECT * FROM '+table+' ORDER BY 1')
        return dict(columns=[item[0] for item in cursor.description],
            rows=[tuple(cell.hex() if isinstance(cell,bytes) else cell for cell in row)
                for row in cursor.fetchall()])

    def observed_state(label):
        """Publish and return the full readonly mutation marker under a label.

        Full ordered rows and columns of preparation_turns,
        preparation_reservations and preparations (bodies included as hex, so
        an amount, status, revision or body change is detected, not only a
        count change), the trust journal bytes as hex (None while Trust is not
        yet bootstrapped) and every admission capsule digest. Each snapshot
        is published into the owned 0700 output as sql-snapshot-<label>.json
        and nothing is printed: private prompt bytes and synthetic tokens
        stay inside the owned fixture evidence, never on stdout. Admission
        operations intentionally change the journal and capsules between
        baselines, so no single before/after pair spans them; the SQL tables
        themselves must not move at all across any pair.
        """
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            marker=dict(turns=sql_rows(db,'preparation_turns'),
                reservations=sql_rows(db,'preparation_reservations'),
                preparations=sql_rows(db,'preparations'))
        journal=f.root/'.swarm/trust-actions.json'
        marker['trust']=journal.read_bytes().hex() if journal.exists() else None
        marker['capsules']=sorted((path.name,digest(path)) for path in (f.root/'.swarm').glob('blp-admission-*.receipt'))
        publish(out/('sql-snapshot-'+label+'.json'),marker)
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
    baseline=observed_state('baseline')
    primary=None
    try:
        web=Server(binary,f.root,out,'blp-dialogue-reader',environment=f.environment);web.capture_async(out/'http-observer.raw')
        def request_dialogue(name,method,token):
            """One real protected GET/HEAD with the owned cookie plus optional bearer.

            The dialogue route carries the reader credential in the
            Authorization header, which the shared Server helper cannot send,
            so this uses the established http.client pattern with Origin,
            session cookie and Bearer token. Retains the actual status, raw
            headers and raw body bytes under http-<name>.json in the owned
            output before any parsing; no retry and no timeout ceiling.
            GET returns (status, raw body bytes, parsed JSON); HEAD returns
            (status, raw body bytes, None) — an actual HEAD response body is
            empty by RFC 7231 §4.3.2, so it is never JSON-parsed, and the
            empty-body fact is asserted from the retained raw bytes instead.
            """
            headers={'Origin':'http://'+web.address,'Cookie':web.cookie}
            if token:headers['Authorization']='Bearer '+token
            connection=http.client.HTTPConnection(web.host,web.port,timeout=None)
            try:
                connection.request(method,'/api/v1/preparations/dialogue?id='+p['id'],headers=headers)
                response=connection.getresponse();raw=response.read();status=response.status
                retained=dict(status=status,body=raw.decode(),headers=dict(response.getheaders()))
            finally:connection.close()
            publish(out/('http-'+name+'.json'),retained)
            return status,raw,(json.loads(raw) if method=='GET' else None)
        def get_dialogue(name,token):return request_dialogue(name,'GET',token)
        before=observed_state('before-legacy-get')
        status,raw_legacy,legacy_turns=get_dialogue('legacy-bare-200','')
        check('legacy-bare-dialogue-200-before-admission',status==200 and len(legacy_turns)==1
            and not list((f.root/'.swarm').glob('blp-admission-*.receipt'))
            and legacy_turns[0]['id']==turn['id'] and legacy_turns[0]['status']=='answered'
            and legacy_turns[0]['question']=='PRIVATE_BLP_SOURCE' and legacy_turns[0]['provider']=='fixture'
            and legacy_turns[0]['answer']['message']=='Inert BLP answer' and legacy_turns[0].get('prompt','')=='')
        check('legacy-get-no-mutation',observed_state('after-legacy-get')==before)
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
        after=observed_state('after-admission')
        check('actual-public-admission-matches-sent-intent',accepted['principal']=='sender'
            and accepted['state']=='admitted-no-transport' and accepted['runtime_authorized'] is False
            and accepted['output_classification']==label and 'PRIVATE_BLP_SOURCE' not in json.dumps(accepted))
        check('admission-capsule-and-no-sql-mutation',receipt.exists() and after['turns']==baseline['turns']
            and after['reservations']==baseline['reservations'] and after['preparations']==baseline['preparations'])
        before=observed_state('before-bare-refusal')
        status,raw_refused,refused=get_dialogue('bare-refusal','')
        secrets=[b'PRIVATE_BLP_SOURCE',b'Inert BLP answer',b'Inert brief',prompt.encode()]
        check('classified-bare-session-refuses',status==403
            and refused.get('failure',{}).get('code')=='blp-dialogue-authentication-required'
            and all(secret not in raw_refused for secret in secrets))
        check('refusal-no-mutation',observed_state('after-bare-refusal')==before)
        before=observed_state('before-authorized-read')
        status,raw_authorized,authorized=get_dialogue('authorized-200','fixture-sender-token')
        check('authorized-reader-reads-same-answer',status==200 and len(authorized)==1
            and authorized[0]==legacy_turns[0] and authorized[0]['answer']['message']=='Inert BLP answer')
        check('positive-read-no-mutation',observed_state('after-authorized-read')==before)
        check('private-prompt-never-served',all(prompt.encode() not in body for body in [raw_legacy,raw_refused,raw_authorized]))
        def authorize_read(name):
            """One authorized recovery GET with full pre/post readonly snapshots.

            Performs the actual GET as the cleared sender and publishes both
            snapshots and the raw retained response before any assertion, so
            a failed recovery stays exact retained evidence. Returns the
            tuple (before,raw,status,body) for the caller's named check; the
            post snapshot is published by the caller's check expression. No
            retry and no timeout ceiling. The authorized reader MUST receive
            the classified question and answer bytes — that is the exact
            positive control; only the assembled private full prompt must
            never appear on the wire, which the final matrix assertion
            verifies.
            """
            before=observed_state('before-'+name)
            status,raw,body=get_dialogue(name,'fixture-sender-token')
            return before,raw,status,body
        def corrupt_history_guard(name,kind):
            """Stored-corruption negative guard over a valid current equivalent.

            Authority and limits: the ONLY mutated object is this fixture's
            own OLD stale admission receipt file (owned fixture bytes, mode
            0600); the current valid renewal receipt, SQL rows, journal and
            every producer stay untouched — all admission state was produced
            by the public CLI flow, so this is explicit stored-fault
            injection on owned test data, never a production writer and
            never a fabricated acceptance. Sequence:
            (1) a valid authorized GET 200 to the exact same DTO with a full
            readonly snapshot pair proves the store is healthy before the
            fault; (2) the original old-receipt bytes are read and retained,
            then the receipt is replaced by
            blp_admission_public.reframe(raw,'<kind>') through the existing
            same-directory atomic publish — the corruption preserves the SR1
            framing checksum, so the fault reaches the stored-decode layer;
            (3) the actual authorized GET must fail 500 — neither an
            ordinary reader 403 (a stored fault is a verifier/storage
            failure, not a clearance decision) nor a 200 serving any turn —
            with no question, answer, private-prompt or STORED_SECRET byte
            in the retained raw body; the raw response is published before
            any assertion, and the retained failure code must EQUAL the
            exact expected oracle of its fault kind — kind 'root' (wrong
            stored Root identity) → preparation-root-instance-changed, kind
            'schema' (invalid stored schema_version) →
            blp-admission-record-invalid — both codes actually observed on
            the received supervisor 325 run, so a missing, non-string or
            merely-different code fails the guard, additionally pinned by
            its own fault-<name>-exact-code check; the pre/post
            snapshot pair brackets the fault read on a baseline taken AFTER
            the intentional fixture mutation; (4) the exact original bytes
            are restored in a finally block, so restoration happens even on
            assertion failure, with the original and mutated bytes retained
            as a byte-level proof pair, and an authorized GET 200 to the
            exact same DTO plus a full snapshot pair proves the recovery. No
            private capsule removal, no policy rollback, no capability
            bypass: the fixture-only restoration is the fault cleanup, never
            an operational recovery substitute. Raises AssertionError with
            the check name on any unmet condition; no watchdog, retry, poll
            cap or timeout ceiling participates.
            """
            assert kind in ('root','schema'),kind
            fault_code={'root':'preparation-root-instance-changed','schema':'blp-admission-record-invalid'}[kind]
            check('control-before-'+name,authorized[0] is not None and old_receipt.read_bytes()==old_original)
            before=observed_state('before-control-'+name)
            status,raw_control,body_control=get_dialogue('control-before-'+name,'fixture-sender-token')
            check('control-200-before-'+name,status==200 and body_control==[authorized[0]]
                and prompt.encode() not in raw_control
                and observed_state('after-control-'+name)==before)
            try:
                mutated=reframe(old_original,kind)
                publish(old_receipt,mutated)
                publish(out/('old-receipt-mutated-'+name+'.bin'),mutated)
                check('mutation-applied-'+name,old_receipt.read_bytes()==mutated and old_receipt.read_bytes()!=old_original)
                before=observed_state('before-fault-'+name)
                status,raw_fault,fault=get_dialogue('fault-'+name,'fixture-sender-token')
                published_code=fault.get('failure',{}).get('code') if isinstance(fault,dict) else None
                check('fault-'+name+'-refuses-500',status==500
                    and all(secret not in raw_fault for secret in [b'PRIVATE_BLP_SOURCE',prompt.encode(),b'Inert BLP answer',b'Inert brief',b'STORED_SECRET'])
                    and isinstance(published_code,str) and published_code==fault_code)
                check('fault-'+name+'-exact-code',published_code==fault_code)
                check('fault-'+name+'-no-mutation',observed_state('after-fault-'+name)==before)
            finally:
                # Exact-byte restoration on every path, including assertion failure.
                publish(old_receipt,old_original)
            check('restoration-'+name,old_receipt.read_bytes()==old_original)
            publish(out/('old-receipt-original-'+name+'.bin'),old_original)
            before=observed_state('before-restored-'+name)
            status,raw_restored,body_restored=get_dialogue('restored-'+name,'fixture-sender-token')
            check('recovery-after-'+name,status==200 and body_restored==[authorized[0]]
                and prompt.encode() not in raw_restored
                and observed_state('after-restored-'+name)==before)
        # Incremental reader matrix over the same actual route, after the
        # received positive control. Each read below is bracketed by the full
        # readonly snapshot pair; every negative keeps no private byte and is
        # followed by a strict authorized recovery to the exact same DTO.
        before=observed_state('before-head-bare')
        head_status,head_bare_raw,_=request_dialogue('head-bare','HEAD','')
        check('head-bare-empty-403',head_status==403 and head_bare_raw==b''
            and all(secret not in head_bare_raw for secret in secrets))
        check('head-bare-no-mutation',observed_state('after-head-bare')==before)
        before_r,raw_r,status_r,body_r=authorize_read('authorized-recovery-after-head-bare')
        check('authorized-recovery-after-head-bare',status_r==200 and body_r==[authorized[0]]
            and prompt.encode() not in raw_r
            and observed_state('after-authorized-recovery-after-head-bare')==before_r)
        before=observed_state('before-head-authorized')
        head_status,head_auth_raw,_=request_dialogue('head-authorized','HEAD','fixture-sender-token')
        check('head-authorized-empty-200',head_status==200 and head_auth_raw==b''
            and all(secret not in head_auth_raw for secret in secrets))
        check('head-authorized-no-mutation',observed_state('after-head-authorized')==before)
        before_r,raw_r,status_r,body_r=authorize_read('authorized-recovery-after-head-authorized')
        check('authorized-recovery-after-head-authorized',status_r==200 and body_r==[authorized[0]]
            and prompt.encode() not in raw_r
            and observed_state('after-authorized-recovery-after-head-authorized')==before_r)
        before=observed_state('before-unknown-bearer')
        status,raw_unknown,unknown=get_dialogue('unknown-bearer-403','fixture-unknown-token')
        check('unknown-bearer-refuses',status==403
            and unknown.get('failure',{}).get('code')=='authorization_required'
            and all(secret not in raw_unknown for secret in secrets))
        check('unknown-bearer-no-mutation',observed_state('after-unknown-bearer')==before)
        before_r,raw_r,status_r,body_r=authorize_read('authorized-recovery-after-unknown')
        check('authorized-recovery-after-unknown',status_r==200 and body_r==[authorized[0]]
            and prompt.encode() not in raw_r
            and observed_state('after-authorized-recovery-after-unknown')==before_r)
        before=observed_state('before-low-reader')
        status,raw_low,low=get_dialogue('low-reader-403','fixture-low-token')
        check('low-reader-denied',status==403
            and low.get('failure',{}).get('code')=='blp-dialogue-access-denied'
            and all(secret not in raw_low for secret in secrets))
        check('low-reader-no-mutation',observed_state('after-low-reader')==before)
        before_r,raw_r,status_r,body_r=authorize_read('authorized-recovery-after-low')
        check('authorized-recovery-after-low',status_r==200 and body_r==[authorized[0]]
            and prompt.encode() not in raw_r
            and observed_state('after-authorized-recovery-after-low')==before_r)
        check('private-prompt-never-served-matrix',all(prompt.encode() not in body
            for body in [head_bare_raw,head_auth_raw,raw_unknown,raw_low,raw_r]))
        # Freshness boundary after the received matrix: a public renewal of
        # the SAME policy (new revision, same trust revision — the received
        # renewal pattern of tests/blp_admission_public.py) must stale the
        # admitted classified dialogue for every reader, and only the
        # supported public recovery — re-attributing every captured source
        # under the renewed revision, then re-admitting the SAME intent as a
        # new admission event — may restore the read. The single declared
        # staleness refusal is the reader code blp-dialogue-policy-stale:
        # the engine adapts the admission capsule's verified ordinary
        # staleness refusal (trust revision, policy revision,
        # policy digest) to this one reader code only after the full
        # admission re-verification, so the internal
        # blp-admission-policy-changed code never reaches the HTTP layer. No
        # private capsule removal, policy rollback, capability
        # bypass or fabricated admission participates: the old matched
        # receipt stays in place, and if it can no longer re-verify the
        # intended 200 below fails with that exact raw retained refusal as
        # the frontier evidence for the verifier.
        before=observed_state('before-pre-renewal-control')
        status,raw_pre,pre_control=get_dialogue('pre-renewal-valid-get','fixture-sender-token')
        check('pre-renewal-valid-get-200',status==200 and pre_control==[authorized[0]]
            and prompt.encode() not in raw_pre
            and observed_state('after-pre-renewal-control')==before)
        trust('blp-policy-apply',dict(schema_version=1,event_id='policy-renewal',expected_revision=1,expected_trust_revision=1,
            reason='Explicit dialogue reader policy renewal fixture',policy=policy))
        before=observed_state('before-policy-stale-get')
        status,raw_stale,stale=get_dialogue('policy-stale-get','fixture-sender-token')
        check('policy-stale-authorized-get-refuses',status==403
            and stale.get('failure',{}).get('code')=='blp-dialogue-policy-stale'
            and all(secret not in raw_stale for secret in secrets))
        check('policy-stale-get-no-mutation',observed_state('after-policy-stale-get')==before)
        before=observed_state('before-policy-stale-head')
        head_status,head_stale_raw,_=request_dialogue('policy-stale-head','HEAD','fixture-sender-token')
        check('policy-stale-head-empty-403',head_status==403 and head_stale_raw==b''
            and all(secret not in head_stale_raw for secret in secrets))
        check('policy-stale-head-no-mutation',observed_state('after-policy-stale-head')==before)
        renewed0=trust('blp-preparation-input-attribute',{**attr(0),'event_id':'renewal-input-0','expected_revision':2})
        check('renewal-source-0',renewed0['producer_captured'] is True and renewed0['source_count']==count)
        for index in range(1,count):
            code,result=f.call(['trust','blp-preparation-input-attribute'],
                {**attr(index),'event_id':'renewal-input-'+str(index),'expected_revision':2})
            check('renewal-source-'+str(index),code==0 or (code!=0 and result['failure']['code']=='blp-attribution-conflict'))
        renewed=trust('blp-preparation-admit',dict(schema_version=1,event_id='admission-renewal',
            expected_revision=2,expected_trust_revision=1,intent=intent),token='fixture-sender-token')
        after_renewal=observed_state('after-renewal-mutations')
        check('renewal-admission-same-intent',renewed['principal']=='sender'
            and renewed['state']=='admitted-no-transport' and renewed['admission_id']!=accepted['admission_id']
            and renewed['policy_revision']=='2' and renewed['trust_revision']=='1'
            and renewed['source_count']==count and 'PRIVATE_BLP_SOURCE' not in json.dumps(renewed))
        check('renewal-mutations-no-sql-mutation',after_renewal['turns']==baseline['turns']
            and after_renewal['reservations']==baseline['reservations']
            and after_renewal['preparations']==baseline['preparations'])
        before_r,raw_r,status_r,body_r=authorize_read('authorized-recovery-after-policy-renewal')
        check('authorized-recovery-after-policy-renewal',status_r==200 and body_r==[authorized[0]]
            and prompt.encode() not in raw_r
            and observed_state('after-authorized-recovery-after-policy-renewal')==before_r)
        # Stored-corruption negative guards over the OLD stale admission
        # receipt — the fixture's first emitted capsule `receipt` (pre-
        # renewal, so stale under revision 2) — while the valid current
        # renewal receipt stays untouched: the received positive recovery
        # above proves a fully verified CURRENT equivalent exists, so the
        # only refused reads below are the injected stored faults themselves
        # (verifier/storage failures, not clearance decisions). Its exact
        # original bytes are captured before any mutation and restored in
        # the guard's finally block. Two faults only: wrong-Root stored
        # frame and invalid stored schema JSON — neither is one of the four
        # reader 403 clearance codes, so the actual authorized GET must be a
        # 500 that leaks no turn byte. The oracle is exact per fault kind:
        # the received supervisor run 325 observed
        # preparation-root-instance-changed (wrong-Root frame, the Pass 1
        # verify order validating Root before policy currency) and
        # blp-admission-record-invalid (stored schema decode failure at the
        # scan), so each guard asserts that exact failure code — twice,
        # inside fault-<name>-refuses-500 and via its own
        # fault-<name>-exact-code check — where the prior explicit-code-only
        # placeholder would have passed on any nonempty 500.
        old_receipt=receipt
        old_names=sorted(path.name for path in (f.root/'.swarm').glob('blp-admission-*.receipt'))
        check('two-receipts-present',len(old_names)==2 and old_receipt.name in old_names)
        old_original=old_receipt.read_bytes()
        check('old-stale-receipt-captured',old_receipt.exists() and old_original.startswith(b'SR1\n'))
        publish(out/'old-receipt-original-captured.bin',old_original)
        corrupt_history_guard('wrong-root-stored','root')
        corrupt_history_guard('invalid-stored-schema','schema')
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
