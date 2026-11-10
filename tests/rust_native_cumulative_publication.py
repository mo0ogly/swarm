#!/usr/bin/env python3
"""Verify cumulative native publication on actual retained two-worker originals.

The optional baseline source reuses genuine completed controls/review; no provider
is relaunched, old task rewritten, SQL success seeded or budget refunded. Negative
faults affect only this owned isolated Root and are restored exactly. No watchdog.
"""
import argparse,contextlib,json,os,sqlite3,subprocess,traceback,uuid
from pathlib import Path
import rust_native_baseline as baseline
from rust_native_task_validation import invoke

fixture=baseline.fixture


@contextlib.contextmanager
def restoration(cleanup):
    """Restore fixture state, preserving both primary and cleanup failures.

    The caller supplies only its owned restoration operation. A late failure
    never implies restoration succeeded; both exact exceptions are retained.
    """
    try:yield
    except BaseException as primary:
        try:cleanup()
        except BaseException as secondary:
            raise BaseExceptionGroup('cumulative fixture and restoration failed',[primary,secondary]) from None
        raise
    else:cleanup()


def run(binary,profile,out,baseline_from=None,resume_from=None,rollback_sdk=None):
    """Publish C2 from a C0 worker onto C1 and verify custody/replay/refusal.

    Every fault has a valid witness and mandatory exact restoration. Optional
    resume requires this recipe's actual saved publication and original before
    snapshot, never synthetic accepted state. Public refusal must leave canonical
    Work/jobs/finance unchanged. Root and originals are retained on every failure.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False)
    qualified=json.loads(profile.read_bytes());assert qualified['no_fixture_feature'] and qualified['sha256']==fixture.digest(binary.read_bytes())
    if resume_from:
        prior=json.loads((resume_from/'published.json').read_bytes());before=json.loads((resume_from/'before.json').read_bytes());request=prior['request'];root=Path(prior['root']);work=prior['work']
    else:
        if baseline_from is None:
            baseline.run(binary,profile,out/'baseline',pending_dependent=True);baseline_from=out/'baseline'
        source=json.loads((baseline_from/'result.json').read_bytes());assert source['status']=='PASS' and not source['cumulative_publication'] and not source['paid_provider'];root=Path(source['root']);work=source['work']
        fixture.publish(out/'baseline-reference.json',dict(path=str(baseline_from),sha256=fixture.digest((baseline_from/'result.json').read_bytes())))
    assert root.is_absolute() and root.name.startswith('swarm-model83-') and (root/'managed-reviewer-invoked.json').is_file() and (root/'baseline-reviewer-invoked.json').is_file()
    env=dict(os.environ,CODEX_HOME=str(root/'codex-home'));checks=[]
    def call(label,args,value=None,success=True):
        """Retain raw CLI channels and require the exact intended exit family."""
        code,reply=invoke(binary,root,out,label,args,value,env);assert (code==0)==success,(label,reply);return reply
    @contextlib.contextmanager
    def db(write=False):
        """Open this identified fixture only; SQL writes below are negative faults."""
        c=sqlite3.connect('file:'+str(root/'.swarm/state.db')+('?mode=rw' if write else '?mode=ro'),uri=True)
        try:yield c
        finally:c.close()
    def state():
        """Hash actual canonical tables even when a corrupt public read refuses."""
        with db() as c:tables={n:c.execute('SELECT * FROM '+n+' ORDER BY 1').fetchall() for n in ['works','events','managed_attempts','native_validation_jobs','native_validation_steps','planning_calls','reservations']}
        return fixture.digest(repr(tables).encode())
    def markers():
        """Identify actual inert provider invocations without exposing their text."""
        return {p.name:fixture.digest(p.read_bytes()) for pattern in ['*reviewer-invoked.json','model-received*.json'] for p in root.glob(pattern)}
    if not resume_from:
        before=call('before',['work','show',work])['work'];request=dict(schema_version=1,event_id='cumulative-'+uuid.uuid4().hex,expected_revision=before['revision'],task_id='task-2')
    fixture.publish(out/'before.json',before)
    first=next(t for t in before['tasks'] if t['id']=='task');second=next(t for t in before['tasks'] if t['id']=='task-2');assert first['status']=='accepted'
    original_markers=markers();finance=baseline.workers.publication.native.finance(root,work)
    with db() as c:
        firstrow=c.execute('SELECT base_commit,result_commit,detail FROM managed_attempts WHERE agent_id=?',(first['independent_review']['producer'],)).fetchone()
        firstevent=c.execute('SELECT request,payload FROM events WHERE id=?',(firstrow[2],)).fetchone()
        secondrow=c.execute('SELECT base_commit,state FROM managed_attempts WHERE agent_id=?',(second['independent_review']['producer'],)).fetchone()
    assert firstrow and firstevent and secondrow and secondrow[0]==before['planning']['repository']['base_commit']
    assert second['independent_review']['previous_candidate']==before['planning']['repository']['candidate_commit']!=secondrow[0]
    if not resume_from:
        assert second['status']=='submitted' and secondrow[1]=='ready'
        preview=call('valid-preview',['task','publication-preview',work],request)
        assert preview['publication_supported'] and not preview['publication_ready'] and preview['opinion_state']=='passed' and set(preview['cohort_tasks'])=={'task','task-2'}
        untouched=state();checks.append('actual-full-cohort-before-cumulative-publication')
        for label,patch in [('stale',dict(expected_revision=before['revision']-1)),('unknown-authority',dict(accept=True))]:
            call(label,['task','publish',work],dict(request,**patch),False);assert state()==untouched and markers()==original_markers
        checks.append('invalid-write-envelope-refuses-without-mutation')
        if rollback_sdk:
            # TEMP triggers live only on the already-open SDK connection. The
            # public Root opener correctly rejects persistent schema injections.
            marker=root/'.rust-migration-fixture';created=not marker.exists()
            if created:fixture.publish(marker,b'Owned inert cumulative rollback fixture; no authority.\n')
            command=[str(rollback_sdk),'--ignored','--exact','root_store::managed_publication::tests::first_publication_sql_rollback_preserves_all_state','--nocapture']
            input_file=out/'rollback-sdk.input.json';fixture.publish(input_file,request)
            environment=dict(env,SWARM_FIRST_PUBLICATION_FIXTURE=str(root),SWARM_FIRST_PUBLICATION_REQUEST=str(input_file),SWARM_FIRST_PUBLICATION_WORK=work)
            fixture.publish(out/'rollback-sdk.command.json',dict(argv=command,binary_sha256=fixture.digest(rollback_sdk.read_bytes()),global_watchdog=None))
            def restore_marker():
                """Remove only our diagnostic marker, never retained original data."""
                if created:marker.unlink()
            with restoration(restore_marker):
                result=subprocess.run(command,env=environment,capture_output=True)
                fixture.publish(out/'rollback-sdk.stdout',result.stdout);fixture.publish(out/'rollback-sdk.stderr',result.stderr)
                assert result.returncode==0 and b'1 passed' in result.stdout
                for name in ['row-abort','event-abort','silent-row','transaction-rollback']:assert ('MANAGED_PUBLICATION_ROLLBACK_VERIFIED '+name).encode() in result.stdout
                assert state()==untouched and markers()==original_markers
            checks.append('four-transactional-faults-preserve-work-event-and-worker-with-recovery')
        published=call('publish',['task','publish',work],request)
        assert published['task_accepted'] and not published['replayed'] and not published['provider_started']
        fixture.publish(out/'published.json',dict(root=str(root),work=work,request=request,outcome=published))
    else:
        published=call('resume-publication-replay',['task','publish',work],request)
        assert published['replayed'] and published['candidate_commit']==prior['outcome']['candidate_commit']
        fixture.publish(out/'published.json',dict(root=str(root),work=work,request=request,outcome=published,resumed_from=str(resume_from)))
    after=call('after',['work','show',work])['work'];assert after['revision']==before['revision']+1 and after['planning']['repository']['candidate_commit']==second['independent_review']['candidate_commit']
    assert next(t for t in after['tasks'] if t['id']=='task')==first
    accepted=next(t for t in after['tasks'] if t['id']=='task-2');assert accepted['status']=='accepted' and accepted['independent_review']==second['independent_review'] and accepted['automatic_validation']==second['automatic_validation']
    with db() as c:
        event=c.execute('SELECT revision,request,payload FROM events WHERE id=?',(request['event_id'],)).fetchone()
        row=c.execute('SELECT base_commit,state,result_commit,detail FROM managed_attempts WHERE agent_id=?',(second['independent_review']['producer'],)).fetchone()
        assert c.execute('SELECT request,payload FROM events WHERE id=?',(firstrow[2],)).fetchone()==firstevent
    record=json.loads(event[1]);assert event[0]==after['revision'] and event[1]==event[2] and record['schema_version']==2 and record['accepted_work']==after
    assert record['worker_base']==secondrow[0] and record['previous_candidate']!=record['worker_base']
    assert record['previous_publication']==dict(event_id=firstrow[2],sha256=fixture.digest(firstevent[0]))
    assert row==(secondrow[0],'integrated',record['result_commit'],request['event_id'])
    checks.append('atomic-v2-publication-preserves-v1-task-and-distinct-worker-base')
    settled=state()
    def valid(label):
        """Verify current candidate coverage with two unchanged publication originals."""
        result=call(label,['task','baseline-preview',work],dict(request,expected_revision=after['revision']))
        assert result['baseline_verified'] and result['baseline_present'] and result['candidate_commit']==record['candidate_commit'] and len(result['tasks'])==2
        assert {t['task_id']:t['review_id'] for t in result['tasks']}=={t['id']:t['independent_review']['id'] for t in after['tasks'] if t['status']=='accepted'}
        assert not any(result[k] for k in ['publication_authorized','provider_started','task_accepted'])
        assert state()==settled and markers()==original_markers
        return result
    valid('current-two-task-baseline');checks.append('current-candidate-proof-keeps-both-original-reviews')
    for task in [first,second]:
        agent=task['independent_review']['producer'];recovered=call('reconcile-'+task['id'],['agent','reconcile',agent]);assert recovered['agent']['id']==agent and recovered['agent']['status']=='completed' and state()==settled
    checks.append('both-integrated-workers-readable-after-second-publication')
    oldrecord=json.loads(firstevent[0]);oldrequest=dict(schema_version=1,event_id=oldrecord['event_id'],expected_revision=oldrecord['expected_revision'],task_id=oldrecord['task_id'])
    for label,value in [('old-publication-replay',oldrequest),('new-publication-replay',request)]:
        replay=call(label,['task','publish',work],value);assert replay['replayed'] and replay['task_accepted'] and replay['work']==after and state()==settled
    call('event-revision-conflict',['task','publish',work],dict(request,expected_revision=after['revision']),False);assert state()==settled
    checks.append('historical-and-current-replay-without-new-call-debit-or-event')
    for task in [first,second]:
        valid(task['id']+'-original-witness');path=root/'.swarm'/('native-review-'+fixture.digest(task['independent_review']['id'].encode())+'.receipt');raw=path.read_bytes()
        with restoration(lambda:fixture.publish(path,raw)):
            fixture.publish(path,raw+b' ');call(task['id']+'-original-corrupt',['task','baseline-preview',work],dict(request,expected_revision=after['revision']),False);assert state()==settled
        valid(task['id']+'-original-restored')
    checks.append('historical-and-latest-private-original-refusal-and-restoration')
    needle=('"previous_publication":'+json.dumps(record['previous_publication'],separators=(',',':'))).encode();assert event[1].count(needle)==1
    replacement=('"previous_publication":'+json.dumps(dict(record['previous_publication'],sha256='0'*64),separators=(',',':'))).encode()
    for label,changed,row_fault in [('predecessor-hash',event[1].replace(needle,replacement,1),False),('worker-base',event[1].replace(('"worker_base":"'+record['worker_base']+'"').encode(),('"worker_base":"'+record['previous_candidate']+'"').encode(),1),True)]:
        valid(label+'-witness')
        def restore_row():
            """Restore both exact canonical channels and the original worker base."""
            with db(True) as c:
                c.execute('UPDATE events SET request=?,payload=? WHERE id=?',(event[1],event[2],request['event_id']))
                if row_fault:c.execute('UPDATE managed_attempts SET base_commit=? WHERE agent_id=?',(record['worker_base'],record['producer']))
                c.commit()
        with restoration(restore_row):
            with db(True) as c:
                c.execute('UPDATE events SET request=?,payload=? WHERE id=?',(changed,changed,request['event_id']))
                if row_fault:c.execute('UPDATE managed_attempts SET base_commit=? WHERE agent_id=?',(record['previous_candidate'],record['producer']))
                c.commit()
            invalid=state();denied=call(label+'-refused',['task','baseline-preview',work],dict(request,expected_revision=after['revision']),False);assert denied['failure']['causes'] and state()==invalid
        valid(label+'-restored')
    checks.append('canonical-predecessor-and-worker-base-tamper-refuse-and-restore')
    assert baseline.workers.publication.native.finance(root,work)==finance and markers()==original_markers and state()==settled and call('final-work',['work','show',work])['work']==after
    fixture.publish(out/'result.json',dict(status='PASS',checks=checks,root=str(root),work=work,candidate_commit=record['candidate_commit'],publication_event=request['event_id'],task_accepted=True,cumulative_publication=True,provider_started=False,paid_provider=False,mission_accepted=False,global_watchdog=None))
    print('NATIVE_CUMULATIVE_PUBLICATION_VERIFIED '+str(len(checks)),flush=True)


def main():
    """Freeze actual exercised inputs, retain all failures and scoped resume facts."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:parser.add_argument('--'+name,type=Path,required=True)
    for name in ['baseline-from','resume-from','rollback-sdk']:parser.add_argument('--'+name,type=Path)
    args=parser.parse_args();out=args.output.resolve();inputs=dict(binary_sha256=fixture.digest(args.binary.read_bytes()),profile_sha256=fixture.digest(args.profile.read_bytes()),sources=baseline.workers.publication.native.inspection.native.helper_inputs(Path(__file__)),rollback_sdk_sha256=fixture.digest(args.rollback_sdk.read_bytes()) if args.rollback_sdk else None,global_watchdog=None)
    try:
        run(args.binary.resolve(),args.profile.resolve(),out,args.baseline_from.resolve() if args.baseline_from else None,args.resume_from.resolve() if args.resume_from else None,args.rollback_sdk.resolve() if args.rollback_sdk else None)
        assert fixture.digest(args.binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(n).read_bytes())==h for n,h in inputs['sources'].items())
        fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():fixture.publish(out/'inputs.json',inputs);fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
