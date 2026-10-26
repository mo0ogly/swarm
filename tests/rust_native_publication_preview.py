#!/usr/bin/env python3
"""Check managed prepublication through the real CLI and original native flow.

An owned inert provider creates a genuine cohort opinion; optional reuse names
that actual isolated fixture, never fabricated SQL. Every fault follows a valid
control and restores original bytes. No paid model, acceptance or global watchdog.
"""
import argparse,json,os,subprocess,sys,traceback,uuid
from pathlib import Path
import rust_native_managed_reviewer as native
from rust_native_task_validation import invoke


def run(binary,profile,out,reuse=None):
    """Require complete observations, explicit blockers and refusal/restoration.

    Public calls must preserve Work, call/reservation counts and provider marker.
    Root source reports, private originals and public context/certificate stay
    byte-identical after each owned fault. Reusing a fixture does not reauthenticate
    a provider; profile/source hashes bind the currently exercised executable.
    """
    out.mkdir(mode=0o700,parents=True,exist_ok=False);fixture=native.inspection.native.fixture
    if reuse is None:
        native.run(binary,profile,out/'native','pass');reuse=out/'native'
    previous=json.loads((reuse/'result.json').read_text());root=Path(previous['root']);work=previous['work']
    environment=dict(os.environ,CODEX_HOME=str(root/'codex-home'));checks=[]
    def call(label,args,value=None,success=True):
        """Invoke the owned public CLI, retain channels and require expected success."""
        code,wire=invoke(binary,root,out,label,args,value,environment=environment)
        assert (code==0)==success,(label,code,wire)
        return wire
    before=call('before',['work','show',work])['work'];money=native.finance(root,work)
    marker=(root/'managed-reviewer-invoked.json').read_bytes()
    task=next(t for t in before['tasks'] if t['id']=='task');review=task['independent_review']
    request=dict(schema_version=1,event_id='publication-preview-'+uuid.uuid4().hex,
        expected_revision=before['revision'],task_id=task['id'])
    def unchanged(label):
        """Compare actual Work, reservations and the exclusive provider marker."""
        assert call(label+'-work',['work','show',work])['work']==before
        assert native.finance(root,work)==money and (root/'managed-reviewer-invoked.json').read_bytes()==marker
    def valid(label):
        """Require the complete bound observation, explicit blockers and no effects."""
        value=call(label,['task','publication-preview',work],request)
        assert value['native_original_verified'] and value['current_candidate_verified']
        assert value['candidate_commit']==review['candidate_commit'] and value['context_sha256']==review['context_sha256']
        assert value['opinion_state']=='passed' and value['review_id']==review['id']
        assert value['cohort_tasks']==[t['task'] for t in review['managed_tasks']]
        assert value['validation']['receipt_sha256'] and value['candidate_binding_sha256']
        assert value['previous_candidate']==before['planning']['repository']['candidate_commit']
        assert value['blockers']==['managed-protected-publication-unported','managed-accepted-baseline-authority-unported']
        assert not any(value[k] for k in ['publication_supported','publication_ready','provider_started','task_accepted'])
        serialized=json.dumps(value)
        assert str(root) not in serialized and 'owner_pid' not in serialized and 'stdin' not in serialized
        unchanged(label)
        return value
    valid('valid-current-original');checks.append('actual-native-cohort-current-and-explicit-publication-blockers')
    paths=[('context',root/review['context']),('certificate',root/review['receipt']),
        ('private',root/'.swarm'/('native-review-'+fixture.digest(review['id'].encode())+'.receipt')),
        ('controls',root/task['automatic_validation']['receipt']),('report',root/review['report'])]
    for kind,path in paths:
        valid(kind+'-control');raw=path.read_bytes();fixture.publish(out/(kind+'-original.bytes'),raw)
        try:
            fixture.publish(path,raw+b' ')
            denied=call(kind+'-refused',['task','publication-preview',work],request,False)
            assert denied.get('failure',{}).get('causes'),denied
            unchanged(kind+'-refused')
        finally:fixture.publish(path,raw)
        valid(kind+'-restored');checks.append(kind+'-corruption-refuses-and-restores-without-call-or-state-change')
    setup=json.loads((reuse/'inspection/setup.json').read_text());source=Path(setup['workspace'])/'tracked.txt';raw=source.read_bytes()
    valid('source-control')
    try:
        fixture.publish(source,raw+b'\nChanged source for publication preview refusal\n')
        call('source-refused',['task','publication-preview',work],request,False);unchanged('source-refused')
    finally:fixture.publish(source,raw)
    valid('source-restored');checks.append('live-candidate-source-drift-refuses-and-restores')
    for label,change in [('stale-revision',dict(expected_revision=before['revision']-1)),
        ('missing-task',dict(task_id='absent-task')),('unknown-authority',dict(publication_ready=True))]:
        valid(label+'-control');bad=dict(request,**change)
        call(label+'-refused',['task','publication-preview',work],bad,False);unchanged(label)
        valid(label+'-restored');checks.append(label+'-refusal-preserves-original')
    acceptance=dict(schema_version=1,event_id='still-unported-'+uuid.uuid4().hex,
        expected_revision=before['revision'],id=task['id'],status='accepted')
    refused=call('acceptance-still-refused',['task','update',work],acceptance,False)
    assert 'native-managed-current-review-unported' in refused['failure']['causes'];unchanged('acceptance-still-refused')
    checks.append('verified-preview-never-unlocks-task-acceptance')
    # A new review event after a completed opinion must still refuse; the internal
    # observed-record source purpose is not a fresh invocation allowance.
    valid('fresh-call-control');call('new-review-refused',['task','review',work],dict(request,event_id='new-review-'+uuid.uuid4().hex),False)
    unchanged('new-review-refused');checks.append('recorded-source-observation-never-grants-another-review')
    fixture.publish(out/'result.json',dict(status='PASS',checks=checks,root=str(root),work=work,
        reused_original_from=str(reuse),provider_started=False,accepted=False,paid_provider=False,global_watchdog=None))
    print('MANAGED_PUBLICATION_PREVIEW_PASS '+str(len(checks))+' named assertions',flush=True)


def main():
    """Retain actual binary/profile/helper inputs and original failure output."""
    a=argparse.ArgumentParser(description=__doc__)
    for n in ['binary','profile','output']:a.add_argument('--'+n,type=Path,required=True)
    a.add_argument('--reuse-from',type=Path);args=a.parse_args();out=args.output.resolve();fixture=native.inspection.native.fixture
    inputs=dict(binary_sha256=fixture.digest(args.binary.read_bytes()),profile_sha256=fixture.digest(args.profile.read_bytes()),
        helpers=native.inspection.native.helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(args.binary.resolve(),args.profile.resolve(),out,args.reuse_from.resolve() if args.reuse_from else None)
        assert fixture.digest(args.binary.read_bytes())==inputs['binary_sha256']
        assert all(fixture.digest(Path(f).read_bytes())==h for f,h in inputs['helpers'].items())
        fixture.publish(out/'inputs.json',inputs)
    except BaseException:
        if out.exists():fixture.publish(out/'inputs.json',inputs);fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
