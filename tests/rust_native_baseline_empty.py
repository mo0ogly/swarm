#!/usr/bin/env python3
"""Check empty managed baseline versus ordinary Work through the native CLI.

Actual inert public planning creates the managed witness; no worker/control/review,
positive SQL, paid provider, live mission or global watchdog is used. Refusals
follow this valid witness and preserve canonical state and financial rows.
"""
import argparse,json,os,subprocess,traceback
from pathlib import Path
import rust_managed_planning as planning
from rust_native_task_validation import invoke
from rust_mission_report_relay import rows
from types import SimpleNamespace


def run(binary,profile,out,planning_from=None):
    """Require honest empty flags and no baseline fallback on non-managed Work.

    Exact original CLI channels and fixture birth survive failure. An initial
    empty baseline has no verified accepted originals and grants no publication.
    The separate ordinary Work refusal must not change either Work or finance.
    planning_from may reuse only an actual completed no-worker planning fixture,
    retaining its birth and current public readback rather than rerunning planners.
    """
    out.mkdir(parents=True,mode=0o700,exist_ok=False)
    if planning_from is None:
        result,owned=planning.run(binary,profile,out/'planning',return_fixture=True)
        planning_from=out/'planning'
    else:
        result=json.loads((planning_from/'result.json').read_bytes());assert result['status']=='PASS' and not result['worker_started']
        root=Path(json.loads((planning_from/'fixture.json').read_bytes())['root'])
        assert root.is_absolute() and root.name.startswith('swarm-model83-')
        assert any(json.loads(p.read_bytes()).get('root')==str(root) for p in planning_from.rglob('owned-root-*.json'))
        owned=SimpleNamespace(root=root,environment=dict(os.environ,CODEX_HOME=str(root/'codex-home')))
    planning.fixture.publish(out/'planning-reference.json',dict(path=str(planning_from),result_sha256=planning.fixture.digest((planning_from/'result.json').read_bytes())))
    work=result['planned_work'];checks=[]
    def call(label,args,value=None,success=True):
        """Assert the actual public outcome; retain refusal and checked channels."""
        code,wire=invoke(binary,owned.root,out,label,args,value,owned.environment);assert (code==0)==success,(label,wire);return wire
    before=call('managed-before',['work','show',work])['work'];owner=SimpleNamespace(root=owned.root,work=work)
    finance=rows(owner,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(work,))
    request=dict(schema_version=1,event_id='empty-baseline',expected_revision=before['revision'],task_id='task')
    value=call('empty-baseline',['task','baseline-preview',work],request)
    assert not value['baseline_present'] and not value['baseline_verified'] and value['tasks']==[]
    assert value['candidate_commit']==value['base_commit']==before['planning']['repository']['base_commit']
    assert not any(value[key] for key in ['publication_authorized','provider_started','task_accepted'])
    assert call('managed-after',['work','show',work])['work']==before
    assert rows(owner,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(work,))==finance
    checks.append('empty-initial-baseline-is-explicit-and-readonly')
    ordinary=call('ordinary-create',['work','create'],dict(schema_version=1,event_id='ordinary-baseline-create',expected_revision=0,title='Owned ordinary baseline refusal',objective='No managed fallback',scope='Isolated ordinary fixture contract',criteria=['Refusal preserves state']))['work']
    ordinary=call('ordinary-task',['task','add',ordinary['id']],dict(schema_version=1,event_id='ordinary-baseline-task',expected_revision=ordinary['revision'],id='task',title='Owned ordinary task',deliverable='No execution',criteria=['No baseline grant']))['work']
    denied=call('ordinary-refused',['task','baseline-preview',ordinary['id']],dict(request,expected_revision=ordinary['revision']),False)
    assert 'managed-baseline-repository-required' in denied['failure']['causes']
    assert call('ordinary-after',['work','show',ordinary['id']])['work']==ordinary
    assert call('managed-still-unchanged',['work','show',work])['work']==before
    checks.append('ordinary-work-refuses-instead-of-empty-managed-success')
    planning.fixture.publish(out/'result.json',dict(status='PASS',checks=checks,root=str(owned.root),work=work,worker_started=False,paid_provider=False,global_watchdog=None))
    print('NATIVE_BASELINE_EMPTY_VERIFIED 2',flush=True)


def main():
    """Bind the actual no-fixture binary/profile and local helper input closure."""
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--planning-from',type=Path)
    a=p.parse_args();from rust_native_managed_candidate import helper_inputs
    fixture=planning.fixture;inputs=dict(binary_sha256=fixture.digest(a.binary.read_bytes()),profile_sha256=fixture.digest(a.profile.read_bytes()),sources=helper_inputs(Path(__file__)),global_watchdog=None)
    try:
        run(a.binary.resolve(),a.profile.resolve(),a.output.resolve(),a.planning_from.resolve() if a.planning_from else None)
        assert fixture.digest(a.binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(f).read_bytes())==h for f,h in inputs['sources'].items())
        fixture.publish(a.output/'inputs.json',inputs)
    except BaseException:
        if a.output.exists():fixture.publish(a.output/'inputs.json',inputs);fixture.publish(a.output/'failure.json',dict(status='FAIL',cause=traceback.format_exc()))
        raise
if __name__=='__main__':main()
