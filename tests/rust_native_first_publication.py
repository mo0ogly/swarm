#!/usr/bin/env python3
"""Verify first atomic managed publication using real retained native originals.

Owned inert fixture birth and actual successful native lifecycle are mandatory.
No positive SQL seed, paid provider, live mission or global verification timeout.
Only named rollback/silent-write fault triggers mutate isolated fixture SQL.
"""
import argparse, contextlib, hashlib, json, os, sqlite3, subprocess, sys, traceback
from pathlib import Path
import rust_native_managed_reviewer as native
from rust_native_task_validation import invoke


def run(binary, profile, out, reuse=None, probe=None, rollback_sdk=None):
    """Retain each public attempt and require atomic state, originals and no debit.

    A genuine passed witness precedes injected faults; faults are restored before
    publication. Replay/collisions and current accepted proof are separate checks.
    Original attempts/failures remain retained and no model is rerun on reuse.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    fixture=native.inspection.native.fixture
    if reuse is None:
        native.run(binary,profile,out/'native','pass');reuse=out/'native'
    source=json.loads((reuse/'result.json').read_text())
    assert source['status']=='PASS' and source['case']=='pass' and not source['accepted'] and not source['paid_provider']
    root=Path(source['root']);work=source['work'];environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'))
    births=list((reuse/'inspection').rglob('owned-root-*.json'))
    assert any(json.loads(f.read_text()).get('root')==str(root) for f in births),'actual fixture birth required'
    checks=[]
    def call(label,args,value=None,success=True):
        """Invoke one actual CLI, retain all channels and require the named result."""
        code,wire=invoke(binary,root,out,label,args,value,environment)
        assert (code==0)==success,(label,wire)
        return wire
    def shown(label):
        """Read canonical Work without replacing a refusal with an empty object."""
        return call(label,['work','show',work])['work']
    @contextlib.contextmanager
    def db(write=False):
        """Close the owned fixture connection, retaining operation/close failures."""
        connection=sqlite3.connect('file:'+str(root/'.swarm/state.db')+('?mode=rw' if write else '?mode=ro'),uri=True)
        try:yield connection
        finally:connection.close()
    def state():
        """Observe normalized settlement, event coordinates and financial rows."""
        with db() as connection:
            rows=connection.execute('SELECT * FROM managed_attempts ORDER BY agent_id').fetchall()
            events=connection.execute('SELECT id,revision,kind,hex(request),hex(payload) FROM events WHERE work_id=? ORDER BY revision',(work,)).fetchall()
        return dict(attempts=rows,events=events,finance=native.finance(root,work))
    before=shown('before');task=next(t for t in before['tasks'] if t.get('independent_review',{}).get('state')=='passed')
    review=task['independent_review'];request=dict(schema_version=1,event_id='first-publication-'+hashlib.sha256(str(out).encode()).hexdigest(),expected_revision=before['revision'],task_id=task['id'])
    baseline=state();marker=(root/'managed-reviewer-invoked.json').read_bytes()
    if rollback_sdk:
        envelope=out/'rollback-envelope.json';fixture.publish(envelope,request)
        mark=root/'.rust-migration-fixture';created=not mark.exists()
        if created:fixture.publish(mark,b'owned retained inert SQL rollback diagnostic\n')
        sdk_env=dict(environment,SWARM_FIRST_PUBLICATION_FIXTURE=str(root),SWARM_FIRST_PUBLICATION_WORK=work,SWARM_FIRST_PUBLICATION_REQUEST=str(envelope))
        command=[str(rollback_sdk),'--ignored','--exact','root_store::managed_publication::tests::first_publication_sql_rollback_preserves_all_state','--nocapture']
        try:
            r=subprocess.run(command,capture_output=True,env=sdk_env)
            fixture.publish(out/'rollback.command.json',dict(argv=command,global_watchdog=None));fixture.publish(out/'rollback.stdout',r.stdout);fixture.publish(out/'rollback.stderr',r.stderr)
            assert r.returncode==0 and b'1 passed; 0 failed' in r.stdout,(r.stdout,r.stderr)
            for label in ['row-abort','event-abort','silent-row','transaction-rollback']:
                assert ('MANAGED_PUBLICATION_ROLLBACK_VERIFIED '+label).encode() in r.stdout
            checks.append('four-temporary-sql-faults-preserve-work-worker-and-event')
        finally:
            if created:mark.unlink()
    preview=call('valid-preflight',['task','publication-preview',work],request)
    assert preview['native_original_verified'] and preview['current_candidate_verified'] and preview['opinion_state']=='passed'
    def unchanged(label):
        """A refusal must preserve Work, all event/worker/financial rows and call marker."""
        assert shown(label)==before and state()==baseline and (root/'managed-reviewer-invoked.json').read_bytes()==marker
    for label,modified in [('bad-revision',dict(request,expected_revision=before['revision']+1)),('unknown-field',dict(request,allow=True))]:
        call(label,['task','publish',work],modified,False);unchanged(label+'-unchanged');checks.append(label+'-refuses-without-mutation')
    private=root/'.swarm'/('native-review-'+fixture.digest(review['id'].encode())+'.receipt')
    for label,path in [('private-original',private),('public-certificate',root/review['receipt']),('root-report',root/review['report'])]:
        original=path.read_bytes()
        try:
            fixture.publish(path,original+b'\ncorrupt\n');call(label,['task','publish',work],request,False);unchanged(label+'-unchanged')
        finally:fixture.publish(path,original)
        restored=call(label+'-restored',['task','publication-preview',work],request)
        assert restored['current_candidate_verified'];checks.append(label+'-refuses-and-restores')
    # SQL rollback faults live in the ignored Rust SDK on this same retained
    # fixture: TEMP connection-local triggers preserve authoritative Root schema.
    result=call('publish',['task','publish',work],request)
    assert result['task_accepted'] and not result['replayed'] and not result['provider_started']
    after=shown('after');accepted=next(t for t in after['tasks'] if t['id']==task['id'])
    assert accepted['status']=='accepted' and after['revision']==before['revision']+1
    assert after['planning']['repository']['candidate_commit']==review['candidate_commit']
    assert accepted['independent_review']==review and accepted['automatic_validation']==task['automatic_validation']
    assert native.finance(root,work)==baseline['finance'] and (root/'managed-reviewer-invoked.json').read_bytes()==marker
    with db() as connection:
        row=connection.execute('SELECT state,result_commit,detail FROM managed_attempts WHERE agent_id=?',(review['producer'],)).fetchone()
        event=connection.execute('SELECT kind,revision,request,payload FROM events WHERE id=?',(request['event_id'],)).fetchone()
    record=json.loads(event[2]);assert row==('integrated',record['result_commit'],request['event_id'])
    assert event[0]=='managed.publication' and event[1]==after['revision'] and event[2]==event[3] and record['accepted_work']==after
    checks.append('publication-acceptance-pointer-worker-and-event-atomic')
    recovered=call('integrated-worker-reconcile',['agent','reconcile',review['producer']])
    assert recovered['agent']['id']==review['producer'] and shown('integrated-worker-after')==after
    checks.append('historical-integrated-worker-recovery-without-new-execution')
    settled=state();replay=call('replay',['task','publish',work],request)
    assert replay['replayed'] and replay['task_accepted'] and replay['work']==after and state()==settled
    for label,modified in [('event-revision-collision',dict(request,expected_revision=after['revision'])),('event-task-collision',dict(request,task_id='task-2'))]:
        call(label,['task','publish',work],modified,False);assert shown(label+'-after')==after and state()==settled
    checks.append('exact-replay-and-collisions-without-call-debit-or-write')
    if probe:
        # An owned marker is only the existing diagnostic example's guard; it
        # grants no engine permission and is removed after this one observation.
        mark=root/'.rust-migration-fixture';created=not mark.exists()
        if created:fixture.publish(mark,b'owned retained native publication diagnostic\n')
        command=[str(probe),str(root),task['id'],'current'];r=subprocess.run(command,input=json.dumps(after).encode(),capture_output=True,env=environment)
        fixture.publish(out/'current-proof.command.json',dict(argv=command,global_watchdog=None));fixture.publish(out/'current-proof.stdout',r.stdout);fixture.publish(out/'current-proof.stderr',r.stderr)
        assert r.returncode==0,(r.stdout,r.stderr);assert json.loads(r.stdout)==dict(consistent=True,configured=True)
        checks.append('actual-current-accepted-reader-verifies-canonical-publication-and-git')
        def check_current(label,expected):
            """Exercise the actual current authority reader, retaining both channels."""
            r=subprocess.run(command,input=json.dumps(after).encode(),capture_output=True,env=environment)
            fixture.publish(out/(label+'.stdout'),r.stdout);fixture.publish(out/(label+'.stderr'),r.stderr)
            assert (r.returncode==0)==expected,(label,r.stdout,r.stderr)
        # Negative mutation of actual fixture receipt; restore exact original
        # bytes, never synthesize a successful publication or modify a live Root.
        with db() as connection:original_event=connection.execute('SELECT request,payload FROM events WHERE id=?',(request['event_id'],)).fetchone()
        try:
            with db(True) as connection:connection.execute('UPDATE events SET request=?,payload=? WHERE id=?',(original_event[0]+b' ',original_event[1]+b' ',request['event_id']));connection.commit()
            check_current('noncanonical-event-refused',False)
        finally:
            with db(True) as connection:connection.execute('UPDATE events SET request=?,payload=? WHERE id=?',(*original_event,request['event_id']));connection.commit()
        check_current('canonical-event-restored',True)
        obj=root/after['planning']['repository']['storage']/'repository.git/objects'/review['candidate_commit'][:2]/review['candidate_commit'][2:]
        held=obj.with_name(obj.name+'.publication-fixture-held');assert obj.is_file() and not held.exists()
        try:
            obj.rename(held);check_current('committed-candidate-missing-refused',False)
        finally:held.rename(obj)
        check_current('committed-candidate-restored',True)
        checks.append('canonical-receipt-and-committed-git-refusal-and-restoration')
        if created:mark.unlink()
    assert (root/'managed-reviewer-invoked.json').read_bytes()==marker and state()==settled
    fixture.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,checks=checks,task_accepted=True,mission_accepted=False,provider_started=False,paid_provider=False,global_watchdog=None))
    print('MANAGED_FIRST_PUBLICATION_PASS',len(checks),flush=True)


def main():
    """Freeze exercised binary/profile/helper hashes and retain original failures."""
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--reuse-from',type=Path);p.add_argument('--probe',type=Path);p.add_argument('--rollback-sdk',type=Path);a=p.parse_args()
    fixture=native.inspection.native.fixture
    inputs=dict(binary_sha256=fixture.digest(a.binary.read_bytes()),profile_sha256=fixture.digest(a.profile.read_bytes()),helpers=native.inspection.native.helper_inputs(Path(__file__)),probe_sha256=fixture.digest(a.probe.read_bytes()) if a.probe else None,rollback_sdk_sha256=fixture.digest(a.rollback_sdk.read_bytes()) if a.rollback_sdk else None,global_watchdog=None)
    out=a.output.resolve()
    try:
        run(a.binary.resolve(),a.profile.resolve(),out,a.reuse_from.resolve() if a.reuse_from else None,a.probe.resolve() if a.probe else None,a.rollback_sdk.resolve() if a.rollback_sdk else None)
        assert fixture.digest(a.binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(f).read_bytes())==h for f,h in inputs['helpers'].items())
        fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():fixture.publish(out/'inputs.json',inputs);fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
