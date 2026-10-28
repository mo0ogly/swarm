#!/usr/bin/env python3
"""Prove a real dependent worker receives the published managed candidate.

Creates an isolated native planning/worker/control/review/publication chain using
inert providers and public CLI operations. No SQL writes, live mission, model
identity, automatic retry or global watchdog is implied. An explicitly reused
cumulative publication is checked as recorded state, never performed here.
"""
import argparse,json,os,subprocess,traceback
from pathlib import Path
from types import SimpleNamespace
from rust_mission_report_relay import rows
import rust_native_first_publication as publication
from rust_native_task_validation import invoke
import rust_managed_repository_bootstrap as repository


def run(binary,profile,out,published_from=None,native_from=None,ensure_review_capacity=False):
    """Require actual Git/copy/content, refusal/restoration and one worker debit.

    Fixture budgets and dependency are saved at birth. Publication proves the
    prerequisite before a fresh manual admission; an absent copy must refuse
    without granting a provider, then restoration permits the same queued agent.
    native_from may reuse a genuine completed native opinion before publication;
    its actual originals are revalidated by the public publisher without a new
    provider call. published_from may reuse a genuine completed publication fixture, with
    its birth retained; current public prerequisite proof is checked again and
    a prior failed admission must have left the dependent task unattempted.
    Public terminal readback and originals survive; task completion is not task
    acceptance. Failure retains channels and owned fixture state for diagnosis.
    ensure_review_capacity explicitly permits a fixture-only public quota increase
    to consumed+1 when exhausted, preserving usage/history and never invoking a
    reviewer. It is not an automatic product budget change or default fallback.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False)
    fixture=publication.native.inspection.native.fixture
    qualified=json.loads(profile.read_bytes());assert qualified['no_fixture_feature'] and qualified['sha256']==fixture.digest(binary.read_bytes())
    if published_from is None:
        publication.run(binary,profile,out/'publication',reuse=native_from,pending_dependent=True)
        published_from=out/'publication'
    base_result=json.loads((published_from/'result.json').read_bytes());assert base_result['status']=='PASS' and base_result['task_accepted'] and not base_result['paid_provider']
    root=Path(base_result['root']);work=base_result['work']
    births=list(published_from.rglob('owned-root-*.json'))
    if native_from is not None:births.extend(native_from.rglob('owned-root-*.json'))
    assert any(json.loads(f.read_bytes()).get('root')==str(root) for f in births), 'retained actual birth required'
    fixture.publish(out/'reused-publication.json',dict(path=str(published_from),result_sha256=fixture.digest((published_from/'result.json').read_bytes())))
    env=dict(os.environ,CODEX_HOME=str(root/'codex-home'));checks=[]
    def call(label,args,value=None,success=True):
        """Retain exact actual CLI response and require the intended outcome."""
        code,wire=invoke(binary,root,out,label,args,value,env)
        assert (code==0)==success,(label,wire)
        return wire
    def shown(label):
        """Read the authoritative Work; never replace a refusal with empty data."""
        return call(label,['work','show',work])['work']
    capacity=None
    if ensure_review_capacity:
        current=shown('capacity-work');quota=call('capacity-before',['quotas','show',work])
        used=quota['consumed']['review_calls'];limit=quota['limits']['review_calls']
        assert isinstance(used,int) and isinstance(limit,int) and used<=limit
        if used==limit:
            values=dict(quota['limits'],review_calls=used+1)
            intent=dict(schema_version=1,event_id='dependent-review-capacity-'+out.name,expected_revision=quota['revision'],limits=values,reason='Explicit owned fixture capacity for the next worker, preserving completed review consumption')
            preview=call('capacity-preview',['quotas','preview',work],intent)
            assert preview['after']['consumed']==quota['consumed'] and shown('capacity-preview-readonly')==current
            call('capacity-apply',['quotas','apply',work],intent)
            saved=call('capacity-reopen',['quotas','show',work]);changed=shown('capacity-after')
            assert saved['limits']['review_calls']==used+1 and saved['consumed']==quota['consumed']
            assert changed['tasks']==current['tasks'] and changed['planning']['repository']==current['planning']['repository']
            capacity=dict(before=limit,after=used+1,consumed=used,publicly_applied=True)
    before=shown('before');repo=before['planning']['repository'];base=repo['candidate_commit'];storage=Path(repo['storage']);subdir=repo.get('project_subdir','')
    assert base!=repo['base_commit']
    task=next(t for t in before['tasks'] if t['id']=='task');assert task['status']=='accepted'
    accepted_before={t['id']:t for t in before['tasks'] if t['status']=='accepted'}
    original=fixture.digest(json.dumps(task,sort_keys=True).encode());review_marker=(root/'managed-reviewer-invoked.json').read_bytes()
    money=publication.native.finance(root,work)
    observer=SimpleNamespace(root=root,work=work)
    reservation_query='SELECT agent_id,work_id,state,amount FROM reservations WHERE work_id=? ORDER BY agent_id'
    reservations=rows(observer,reservation_query,(work,))
    dependent=next(t for t in before['tasks'] if t['id']=='dependent')
    assert dependent['status']=='todo' and not dependent.get('attempts')
    checks.append('predecessor-really-published-and-dependent-refused-before-publication')
    fixture.publish(root/'codex',fixture.PROGRAM.replace('model-received.json','dependent-received.json').encode());(root/'codex').chmod(0o700)
    request=dict(schema_version=1,event_id='post-publication-dependent',expected_revision=before['revision'],task_id='dependent',provider='fixture',capture_output=True)
    prepared=call('prepare',['agent','prepare',work],request);agent=prepared['agent'];workspace=Path(agent['workspace']);copy=storage/'copies/dependent-1'
    assert workspace==copy/subdir and workspace.is_dir()
    assert repository.git(copy,'rev-parse','HEAD').decode().strip()==base
    proof=json.loads((storage/('native-worker-'+agent['id']+'.json')).read_bytes())
    assert proof['base']==base
    for task_id in accepted_before:
        report_path=(subdir+'/' if subdir else '')+'docs/'+task_id+'.md'
        report=subprocess.check_output(['git','--git-dir',str(storage/'repository.git'),'show',base+':'+report_path])
        assert report and (workspace/('docs/'+task_id+'.md')).read_bytes()==report
    assert (workspace/'tracked.txt').read_bytes()==b'Committed planning input\n'
    checks.append('real-copy-head-original-base-and-published-report-match')
    replay=call('prepare-replay',['agent','prepare',work],request)
    assert replay['agent']==agent
    queued=shown('queued');parked=copy.with_name(copy.name+'.owned-held');assert not parked.exists()
    try:
        copy.rename(parked)
        call('copy-absent',['agent','run',agent['id']],success=False)
        assert shown('copy-absent-work')==queued and not (parked/subdir/'dependent-received.json').exists()
    finally:
        if parked.exists():parked.rename(copy)
    checks.append('same-admission-replay-and-absent-copy-refusal-with-restoration')
    result=call('run',['agent','run',agent['id']]);assert result['agent']['status']=='completed'
    marker=json.loads((workspace/'dependent-received.json').read_bytes());assert marker['stdin_sha256']==fixture.digest(agent['prompt'].encode())
    after=shown('after');accepted=next(t for t in after['tasks'] if t['id']=='task');dependent=next(t for t in after['tasks'] if t['id']=='dependent')
    assert fixture.digest(json.dumps(accepted,sort_keys=True).encode())==original
    assert {t['id']:t for t in after['tasks'] if t['status']=='accepted'}==accepted_before
    assert dependent['status']=='blocked' and len(dependent['attempts'])==1
    assert after['planning']['repository']['candidate_commit']==base and (root/'managed-reviewer-invoked.json').read_bytes()==review_marker
    assert publication.native.finance(root,work)==money
    observed=rows(observer,reservation_query,(work,));assert len(observed)==len(reservations)+1
    assert all(row in observed for row in reservations)
    assert [row for row in observed if row[0]==agent['id']]==[(agent['id'],work,'estimated',1.0)]
    checks.append('actual-worker-completion-one-reservation-and-predecessor-proof-preserved')
    assert fixture.digest(binary.read_bytes())==qualified['sha256']
    fixture.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,checks=checks,candidate_commit=base,agent_id=agent['id'],paid_provider=False,cumulative_publication=base_result.get('cumulative_publication',False),accepted_task_count=len(accepted_before),review_capacity_change=capacity,mission_accepted=False,global_watchdog=None,binary_sha256=qualified['sha256']))
    print('POST_PUBLICATION_WORKER_VERIFIED '+str(len(checks)),flush=True)


def main():
    """Retain exact source/profile/binary inputs and original failures in fresh output."""
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--published-from',type=Path)
    p.add_argument('--native-from',type=Path)
    p.add_argument('--ensure-review-capacity',action='store_true')
    a=p.parse_args();fixture=publication.native.inspection.native.fixture
    inputs=dict(binary_sha256=fixture.digest(a.binary.read_bytes()),profile_sha256=fixture.digest(a.profile.read_bytes()),sources=publication.native.inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        publication.native.inspection.native.managed.worker.subreaper_scope(lambda:run(a.binary.resolve(),a.profile.resolve(),a.output.resolve(),a.published_from.resolve() if a.published_from else None,a.native_from.resolve() if a.native_from else None,a.ensure_review_capacity),lambda error:None,publication.native.inspection.native.managed.parent_death.subreaper)
        assert all(fixture.digest(Path(f).read_bytes())==h for f,h in inputs['sources'].items())
        fixture.publish(a.output/'inputs.json',inputs)
    except BaseException:
        if a.output.exists():
            fixture.publish(a.output/'inputs.json',inputs);fixture.publish(a.output/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
