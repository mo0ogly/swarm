#!/usr/bin/env python3
"""Verify the current native accepted baseline before another candidate/job.

Owned public fixture only: actual first publication and two automatic workers, immutable
handoff, canonical/current baseline, refusal/restoration, second full control/review
cohort. No positive SQL seed, paid provider, global watchdog or cumulative acceptance.
Successful setup/originals may be explicitly reused; attempts are never overwritten.
"""
import argparse,contextlib,json,os,sqlite3,subprocess,traceback,uuid
from pathlib import Path
import rust_post_publication_worker as workers
from rust_native_task_validation import invoke

fixture=workers.publication.native.inspection.native.fixture


def setup(binary,profile,out,pending_dependent=False):
    """Create first publication from actual automatic workers with disjoint outputs.

    All output names, optional dependency-blocked worker and successful controls
    are declared before fixture birth.
    Existing original snapshots/policies are never rewritten to remove conflicts.
    Manual-worker retention is a distinct public contract outside this recipe.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False)
    workers.publication.native.inspection.native.managed.worker.subreaper_scope(lambda:workers.publication.run(binary,profile,out/'publication',cumulative_fixture=True,pending_dependent=pending_dependent),lambda error:None,workers.publication.native.inspection.native.managed.parent_death.subreaper)
    source=json.loads((out/'publication/result.json').read_bytes());assert source['status']=='PASS' and source['task_accepted'] and not source['paid_provider']
    root=Path(source['root']);work=source['work'];assert root.is_absolute() and root.name.startswith('swarm-model83-')
    assert any(json.loads(p.read_bytes()).get('root')==str(root) for p in out.rglob('owned-root-*.json'))
    env=dict(os.environ,CODEX_HOME=str(root/'codex-home'))
    fixture.publish(out/'setup.json',dict(status='READY_FOR_BASELINE',root=str(root),work=work,target_task_id='task-2',paid_provider=False))
    return root,work,env


def run(binary,profile,out,setup_from=None,before_binary=None,*,pending_dependent=False):
    """Require genuine baseline before faults, unchanged state, current whole cohort.

    Only negative mutations touch owned fixture proof files/canonical event bytes;
    exact restoration is mandatory. Original Work/jobs/finance and reviewer marker
    remain unchanged after refusal. A second review does not authorize publication.
    pending_dependent declares a future worker at birth for a later copy journey;
    it is not part of the two-task control/review cohort and is never launched here.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False)
    qualified=json.loads(profile.read_bytes());assert qualified['no_fixture_feature'] and qualified['sha256']==fixture.digest(binary.read_bytes())
    if setup_from is None:root,work,env=setup(binary,profile,out/'setup',pending_dependent);setup_from=out/'setup'
    else:
        source=json.loads((setup_from/'setup.json').read_bytes());assert source['status']=='READY_FOR_BASELINE' and not source['paid_provider'];root=Path(source['root']);work=source['work'];env=dict(os.environ,CODEX_HOME=str(root/'codex-home'))
    fixture.publish(out/'setup-reference.json',dict(path=str(setup_from),sha256=fixture.digest((setup_from/'setup.json').read_bytes())))
    def call(label,args,value=None,success=True,exe=None):
        """Retain original channels and assert the named public outcome."""
        code,wire=invoke(exe or binary,root,out,label,args,value,env);assert (code==0)==success,(label,wire);return wire
    before=call('before',['work','show',work])['work'];accepted=next(t for t in before['tasks'] if t['id']=='task');review=accepted['independent_review'];old_marker=(root/'managed-reviewer-invoked.json').read_bytes();checks=[]
    @contextlib.contextmanager
    def db(write=False):
        """Open only the identified inert fixture; SQL writes are negative faults."""
        c=sqlite3.connect('file:'+str(root/'.swarm/state.db')+('?mode=rw' if write else '?mode=ro'),uri=True)
        try:yield c
        finally:c.close()
    def state():
        """Observe raw canonical state even when its public projection must refuse."""
        with db() as c:
            result={table:c.execute('SELECT * FROM '+table+' ORDER BY 1').fetchall() for table in ['works','events','managed_attempts','native_validation_jobs','native_validation_steps','planning_calls','reservations']}
        assert (root/'managed-reviewer-invoked.json').read_bytes()==old_marker
        return result
    request=dict(schema_version=1,event_id='baseline-'+uuid.uuid4().hex,expected_revision=before['revision'],task_id='task-2')
    original_state=state();storage=Path(before['planning']['repository']['storage'])
    def candidate_inventory():
        """Bind actual immutable candidate manifests, not merely their count."""
        return {str(p.relative_to(storage)):fixture.digest(p.read_bytes()) for p in (storage/'candidates').glob('*/candidate.json')}
    def valid(label):
        """Read a fully bound current historical baseline, never an execution permit."""
        r=call(label,['task','baseline-preview',work],request)
        assert r['baseline_present'] and r['baseline_verified'] and r['candidate_commit']==before['planning']['repository']['candidate_commit']
        assert r['target_task_id']=='task-2' and r['work_revision']==before['revision'] and len(r['tasks'])==1
        t=r['tasks'][0];assert t['task_id']=='task' and t['review_id']==review['id'] and t['producer_agent_id']==review['producer']
        for name in ['publication_sha256','original_sha256','validation_receipt_sha256','candidate_binding_sha256','accepted_task_sha256','policy_sha256']:assert len(t[name])==64
        assert not any(r[n] for n in ['publication_authorized','provider_started','task_accepted'])
        assert str(root) not in json.dumps(r) and state()==original_state
        return r
    if before_binary:
        # Valid candidate first, then corrupt an accepted original. This isolates
        # missing historical validation, not a missing command in the old CLI.
        old=call('old-valid-candidate',['task','validation-candidate',work],request,exe=before_binary);assert old['cohort_task_count']==2
        path=root/'.swarm'/('native-review-'+fixture.digest(review['id'].encode())+'.receipt');raw=path.read_bytes()
        try:
            fixture.publish(path,raw+b' ');old=call('old-corrupt-accepted-original',['task','validation-candidate',work],request,exe=before_binary);assert old['cohort_task_count']==2 and not old['job_created']
        finally:fixture.publish(path,raw)
        assert state()==original_state;checks.append('old-candidate-accepted-corrupt-baseline-reproduced')
    baseline=valid('valid');checks.append('canonical-current-originals-and-policies-verified-without-authority')
    for label,patch in [('stale',dict(expected_revision=before['revision']-1)),('foreign-target',dict(task_id='missing')),('unknown-authority',dict(publication_authorized=True))]:
        call(label,['task','baseline-preview',work],dict(request,**patch),False);assert state()==original_state
    checks.append('invalid-envelope-refuses-without-state-or-finance-change')
    files=[('private-original',root/'.swarm'/('native-review-'+fixture.digest(review['id'].encode())+'.receipt')),('certificate',root/review['receipt']),('context',root/review['context']),('controls',root/accepted['automatic_validation']['receipt']),('report',root/review['report'])]
    for label,path in files:
        valid(label+'-witness');raw=path.read_bytes();candidates=candidate_inventory()
        try:
            fixture.publish(path,raw+b' ')
            for action in ['baseline-preview','validation-candidate','validate']:
                denied=call(label+'-'+action,['task',action,work],request,False);assert denied['failure']['causes']
                assert state()==original_state and candidate_inventory()==candidates
        finally:fixture.publish(path,raw)
        assert valid(label+'-restored')==baseline;checks.append(label+'-refuses-candidate-and-job-then-restores')
    valid('canonical-witness')
    event=baseline['tasks'][0]['publication_event']
    with db() as c:original=c.execute('SELECT request,payload FROM events WHERE id=?',(event,)).fetchone()
    value=json.loads(original[0]);needle=('\"binding_sha256\":\"'+value['binding_sha256']+'\"').encode();assert original[0].count(needle)==1
    changed=original[0].replace(needle,b'\"binding_sha256\":\"'+b'0'*64+b'\"',1)
    try:
        with db(True) as c:c.execute('UPDATE events SET request=?,payload=? WHERE id=?',(changed,changed,event));c.commit()
        invalid_state=state();candidates=candidate_inventory()
        for action in ['baseline-preview','validation-candidate','validate']:
            denied=call('canonical-'+action,['task',action,work],request,False)
            assert 'managed-publication-original-candidate-binding' in denied['failure']['causes'],denied
            assert state()==invalid_state and candidate_inventory()==candidates
    finally:
        with db(True) as c:c.execute('UPDATE events SET request=?,payload=? WHERE id=?',(*original,event));c.commit()
    assert valid('canonical-restored')==baseline;checks.append('canonical-candidate-binding-refuses-and-restores')
    candidate=call('next-candidate',['task','validation-candidate',work],request)
    assert candidate['cohort_task_count']==2 and candidate['control_count']==6 and not candidate['job_created']
    assert candidate['candidate_commit']!=baseline['candidate_commit'];assert state()==original_state
    controls=call('new-cohort-controls',['task','validate',work],dict(request,event_id='cohort-controls-'+out.name))
    assert controls['controls_passed'] and not controls['task_accepted']
    current=call('after-controls',['work','show',work])['work'];assert next(t for t in current['tasks'] if t['id']=='task')==accepted
    inputs=call('new-cohort-review-inputs',['task','review-inputs',work],dict(request,event_id='cohort-inputs-'+out.name,expected_revision=current['revision']))
    context=inputs['managed_protocol']['context'];assert {t['task'] for t in context['tasks']}=={'task','task-2'}
    assert context['candidate_commit']==candidate['candidate_commit'] and context['previous_candidate']==baseline['candidate_commit']
    marker='baseline-reviewer-invoked.json';fixture.publish(root/'codex',workers.publication.native.PROGRAM.replace(b'managed-reviewer-invoked.json',marker.encode()));(root/'codex').chmod(0o700)
    fixture.publish(root/'managed-review-fixture.json',dict(case='pass',source='unused-for-pass'))
    opinion=call('new-cohort-review',['task','review',work],dict(request,event_id='cohort-review-'+out.name,expected_revision=current['revision']))
    assert opinion['verdict_applicable'] and not opinion['task_accepted']
    current=call('after-review',['work','show',work])['work'];target=next(t for t in current['tasks'] if t['id']=='task-2');assert target['independent_review']['state']=='passed'
    assert {t['task'] for t in target['independent_review']['managed_tasks']}=={'task','task-2'}
    assert next(t for t in current['tasks'] if t['id']=='task')==accepted
    final_request=dict(request,event_id='cumulative-publication-'+out.name,expected_revision=current['revision'])
    preview=call('cumulative-preview',['task','publication-preview',work],final_request)
    assert preview['native_original_verified'] and preview['current_candidate_verified'] and preview['publication_supported'] and not preview['publication_ready']
    assert call('after-publication-preview',['work','show',work])['work']==current
    assert (root/'managed-reviewer-invoked.json').read_bytes()==old_marker
    checks.append('second-candidate-controls-and-whole-cohort-opinion-preserve-old-task-without-publication')
    fixture.publish(out/'result.json',dict(status='PASS',checks=checks,root=str(root),work=work,baseline_commit=baseline['candidate_commit'],next_candidate_commit=candidate['candidate_commit'],cumulative_publication=False,paid_provider=False,global_watchdog=None,setup_from=str(setup_from)))
    print('NATIVE_BASELINE_VERIFIED '+str(len(checks)),flush=True)


def main():
    """Freeze actual profile/binary/helpers and retain every original failure."""
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,type=Path,required=True)
    for name in ['setup-from','before-binary']:p.add_argument('--'+name,type=Path)
    a=p.parse_args();inputs=dict(binary_sha256=fixture.digest(a.binary.read_bytes()),profile_sha256=fixture.digest(a.profile.read_bytes()),sources=workers.publication.native.inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(a.binary.resolve(),a.profile.resolve(),a.output.resolve(),*[getattr(a,k).resolve() if getattr(a,k) else None for k in ['setup_from','before_binary']])
        assert fixture.digest(a.binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(f).read_bytes())==h for f,h in inputs['sources'].items())
        fixture.publish(a.output/'inputs.json',inputs)
    except BaseException:
        if a.output.exists():fixture.publish(a.output/'inputs.json',inputs);fixture.publish(a.output/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
