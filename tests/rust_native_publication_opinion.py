#!/usr/bin/env python3
"""Verify non-passed managed opinion observation and ordinary refusal.

Create a genuine changes-requested native original with an owned inert program,
or resume that same isolated original after a correction without a second call.
No fabricated SQL, accepted task, paid provider or global watchdog.
"""
import argparse,json,os,traceback,uuid
from pathlib import Path
import rust_native_managed_reviewer as native
from rust_native_task_validation import invoke


def run(binary,profile,out,reuse=None,expected_refusal=False):
    """Preserve the actual adverse opinion, marker and reservation across reads.

    Setup is always an actual owned fixture, never a live mission.
    A before-fix refusal is retained explicitly and is not labeled product PASS.
    Successful observation must retain the additional passed-opinion blocker and
    cannot authorize acceptance or retry. Ordinary input refuses without mutation.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False);fixture=native.inspection.native.fixture
    if reuse:
        setup=json.loads((reuse/'setup.json').read_text());root=Path(setup['root']);work=setup['work'];taskid=setup['task']
    else:
        native.inspection.run(binary,profile,out/'inspection',context_bytes=0)
        setup=json.loads((out/'inspection/setup.json').read_text());root=Path(setup['root']);work=setup['work'];taskid='task'
    environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'))
    def call(label,args,value=None,success=True):
        """Invoke the owned public CLI, retain channels and require expected success."""
        code,wire=invoke(binary,root,out,label,args,value,environment=environment)
        assert (code==0)==success,(label,code,wire)
        return wire
    def shown(label):return call(label,['work','show',work])['work']
    if not reuse:
        before=shown('before-controls');request=dict(schema_version=1,event_id='negative-controls-'+uuid.uuid4().hex,expected_revision=before['revision'],task_id=taskid)
        task=next(t for t in before['tasks'] if t['id']==taskid);assert not task.get('independent_review')
        if not task.get('automatic_validation'):
            call('candidate',['task','validation-candidate',work],request)
            call('controls',['task','validate',work],request)
        # This isolated executable is the same configured inert provider path;
        # negative opinion content is derived from the actual sealed cohort.
        program=native.PROGRAM.replace(b"managed-reviewer-invoked.json",b"managed-negative-invoked.json")
        needle=b"verdict='pass',evidence_ref=ref";assert program.count(needle)==1
        program=program.replace(needle,b"verdict='unknown',evidence='The required criterion has no sufficient observed evidence in this fixture'")
        fixture.publish(root/'managed-review-fixture.json',dict(case='pass'))
        fixture.publish(root/'codex',program);(root/'codex').chmod(0o700)
        before=shown('before-negative');request.update(event_id='negative-review-'+uuid.uuid4().hex,expected_revision=before['revision'])
        value=call('native-negative',['task','review',work],request);assert value['review_recorded'] and value['verdict_applicable'] and not value['task_accepted']
        setup=dict(root=str(root),work=work,task=taskid)
    fixture.publish(out/'setup.json',setup)
    before=shown('original');task=next(t for t in before['tasks'] if t['id']==taskid);review=task['independent_review']
    assert task['status']=='blocked' and review['state']=='changes_requested'
    marker=(root/'managed-negative-invoked.json').read_bytes();money=native.finance(root,work)
    request=dict(schema_version=1,event_id='negative-preview-'+uuid.uuid4().hex,expected_revision=before['revision'],task_id=taskid)
    value=call('negative-preview',['task','publication-preview',work],request,not expected_refusal)
    assert shown('after-preview')==before and native.finance(root,work)==money and (root/'managed-negative-invoked.json').read_bytes()==marker
    if expected_refusal:
        assert 'task-validation-submitted-required' in value['failure']['causes'],value
        fixture.publish(out/'result.json',dict(status='EXPECTED_PRE_FIX_REFUSAL',original_preserved=True,root=str(root),work=work))
        print('MANAGED_NEGATIVE_PREVIEW_REPRODUCED',flush=True);return
    assert value['opinion_state']=='changes_requested' and value['native_original_verified'] and value['current_candidate_verified']
    assert not value['publication_ready'] and not value['task_accepted'] and not value['provider_started']
    assert value['blockers']==['managed-publication-write-admission-required','managed-passed-opinion-required']
    denied=call('adverse-publication-refused',['task','publish',work],dict(request,event_id='adverse-publication-'+uuid.uuid4().hex),False)
    assert 'managed-publication-passed-submission-required' in denied.get('failure',{}).get('causes',[]),denied
    assert shown('adverse-after-publication')==before and native.finance(root,work)==money and (root/'managed-negative-invoked.json').read_bytes()==marker
    denied=call('adverse-new-review-refused',['task','review',work],dict(request,event_id='adverse-new-review-'+uuid.uuid4().hex),False)
    assert denied.get('failure',{}).get('causes')
    assert shown('adverse-after-new-review')==before and native.finance(root,work)==money and (root/'managed-negative-invoked.json').read_bytes()==marker
    # A new ordinary Work is created through its public API solely in this owned
    # fixture; preview must not change that Work or infer a managed fallback.
    ordinary=call('ordinary-create',['work','create'],dict(schema_version=1,event_id='ordinary-preview-'+uuid.uuid4().hex,expected_revision=0,title='Ordinary refusal fixture',objective='Observe an explicit managed boundary',scope='Owned fixture',criteria=['No managed fallback']))['work']
    denied=call('ordinary-refused',['task','publication-preview',ordinary['id']],dict(request,expected_revision=ordinary['revision']),False)
    assert 'publication-preview-managed-repository-required' in denied['failure']['causes']
    assert call('ordinary-after',['work','show',ordinary['id']])['work']==ordinary
    assert shown('managed-after-ordinary')==before and native.finance(root,work)==money and (root/'managed-negative-invoked.json').read_bytes()==marker
    fixture.publish(out/'result.json',dict(status='PASS',checks=['actual-changes-requested-opinion-observed-with-extra-blocker','adverse-observation-preserves-original-and-consumption','ordinary-work-refuses-without-fallback-or-mutation','adverse-source-observation-never-grants-a-new-review','actual-adverse-opinion-never-grants-publication'],root=str(root),work=work,accepted=False,provider_started=False,paid_provider=False,global_watchdog=None))
    print('MANAGED_PUBLICATION_OPINION_PASS',flush=True)


def main():
    """Freeze the exercised binary, profile and exact maintained helper closure."""
    p=argparse.ArgumentParser(description=__doc__)
    for n in ['binary','profile','output']:p.add_argument('--'+n,type=Path,required=True)
    p.add_argument('--reuse-negative-from',type=Path);p.add_argument('--expect-status-refusal',action='store_true');a=p.parse_args();out=a.output.resolve();fixture=native.inspection.native.fixture
    inputs=dict(binary_sha256=fixture.digest(a.binary.read_bytes()),profile_sha256=fixture.digest(a.profile.read_bytes()),helpers=native.inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(a.binary.resolve(),a.profile.resolve(),out,a.reuse_negative_from.resolve() if a.reuse_negative_from else None,a.expect_status_refusal)
        assert fixture.digest(a.binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(f).read_bytes())==h for f,h in inputs['helpers'].items())
        fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():fixture.publish(out/'inputs.json',inputs);fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
