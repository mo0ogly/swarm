#!/usr/bin/env python3
"""Verify actual saved task-profile reuse under an owning native mission lease.

Healthy first-worker control precedes exit17 in a fresh identical fixture.
A real inert planner applies a same-task retry; no SQL creates consent/claims/
retry authority. No paid model, live Root, Go or global watchdog. Preserve
original failure roots/channels. Qualified SDK exercises negative data corruption
only after genuine public admission, then real private-gate execution.
"""
import argparse,json,os,subprocess,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_worker_failure_retry as retry
import rust_planning_worker_public_flow as flow


def prepare(binary,out):
    """Create genuine retained manual profile and native retry/model choice.

    Fixture financial policy is explicit public configuration (reserve100 with
    limit1000) so the separately verified reported-cost guard is still active;
    no prior consumption or attempts are reset. No runtime state is forged.
    """
    control=out/'control';control.mkdir()
    healthy,healthy_work=retry.setup(binary,control,0)
    assert healthy_work['tasks'][0]['attempts'][-1]['status']=='completed'
    failed=out/'failed';failed.mkdir()
    owned,work=retry.setup(binary,failed,17)
    task=work['tasks'][0]
    assert task['attempts'][-1]['status']=='failed' and task.get('launch_profile')
    assert not task.get('planning_retry',False)
    fixture.publish(owned.root/'codex',flow.feedback.PROGRAM.encode());(owned.root/'codex').chmod(0o700)
    operation=dict(kind='retry',id='task',next='Retry the same task using current exact input evidence')
    flow.proposal(owned,work,'root',[operation])
    decision=owned.ok(['planning','step',owned.work])
    assert decision['run']['decision_applied']
    current=owned.ok(['work','show',owned.work])['work']
    assert current['tasks'][0]['planning_retry'] and current['tasks'][0]['attempts']==task['attempts']
    fixture.publish(owned.cache,{'models':[
        {'slug':'fixture-simple','visibility':'list'},
        {'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]}]})
    probe=dict(schema_version=1,event_id='task-profile-model-probe',
        expected_revision=current['revision'],task_id='task',provider='fixture',capture_output=True)
    observed=owned.ok(['agent','preview',owned.work],probe)
    policy=observed['agent']['model_route']['policy_hash']
    choice=dict(schema_version=1,event_id='task-profile-model',expected_revision=current['revision'],
        task_id='task',provider='fixture',level='simple',model_policy_hash=policy,inherit=False)
    owned.ok(['task-model','preview',owned.work],choice)
    selected=owned.ok(['task-model','apply',owned.work],choice)
    owned.ok(['budget','apply',owned.work],dict(schema_version=1,event_id='task-profile-envelope',
        expected_revision=selected['revision'],budget=dict(limit_usd=1000,reserve_per_launch_usd=100,
        estimate_source='Owned inert profile fixture envelope, no measured provider prices',reference_date='2026-10-22')))
    fixture.publish(owned.root/'codex',flow.mission.PROGRAM.encode());(owned.root/'codex').chmod(0o700)
    marker=dict(root=str(owned.root),work=owned.work)
    fixture.publish(owned.root/'.swarm/verification-task-profile.json',marker)
    fixture.publish(out/'request.json',marker)
    return owned


def main():
    """Bind normal/SDK qualification, retain originals and require actual effects."""
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','sdk-binary','sdk-profile','output']:
        p.add_argument('--'+name,type=Path,required=True)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    fixture.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();sdk=a.sdk_binary.resolve()
        normal=json.loads(a.profile.read_bytes());qualified=json.loads(a.sdk_profile.read_bytes())
        assert normal['sha256']==fixture.digest(binary.read_bytes()) and normal['no_fixture_feature']
        assert qualified['binary_sha256']==fixture.digest(sdk.read_bytes())
        assert qualified['normal_profile_sha256']==normal['sha256'] and qualified['engine_features_empty']
        owned=prepare(binary,out)
        argv=[str(sdk),str(out/'request.json')]
        run=subprocess.run(argv,env=owned.environment,stdin=subprocess.DEVNULL,capture_output=True)
        fixture.publish(out/'command.json',dict(argv=argv,returncode=run.returncode,global_watchdog=None))
        fixture.publish(out/'stdout',run.stdout);fixture.publish(out/'stderr',run.stderr)
        assert run.returncode==0,run.stderr.decode(errors='replace')
        result=json.loads(run.stdout)
        for field in ['task_profile_precedence','model_choice_precedence','parent_and_task_profile_rechecked','configured_profiles_preserved','actual_automatic_retry']:
            assert result[field] is True,field
        assert result['status']=='PASS' and result['task_accepted'] is False
        witness=json.loads((owned.root/'model-received.json').read_bytes())
        report=(owned.root/'handoff.md').read_bytes()
        assert witness['stdin_sha256'].encode() in report
        fixture.publish(out/'worker-witness.json',witness)
        fixture.publish(out/'handoff.md',report)
        fixture.publish(out/'result.json',dict(status='PASS',root=str(owned.root),normal_sha256=normal['sha256'],
            sdk_sha256=qualified['binary_sha256'],actual_worker=True,actual_retry_decision=True,
            task_accepted=False,real_model=False,installed=False))
        print('PASS actual task profile precedence, model choice, parent and task fencing, automatic retry',flush=True)
    except BaseException as e:
        fixture.publish(out/'failure.json',dict(status='FAIL',message=str(e),traceback=traceback.format_exc()))
        raise


if __name__=='__main__':
    main()
