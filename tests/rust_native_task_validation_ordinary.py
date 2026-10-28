#!/usr/bin/env python3
"""Ordinary task controls under explicit human/automatic policies, no fake scope.

Public Root task policy -> actual native worker -> original report -> controls.
No Go, seed, planning consent fabrication, paid provider or automatic acceptance.
Every command/result and source/binary identity is retained without a watchdog.
"""
import argparse, json, os, sys, traceback
from pathlib import Path
import rust_native_task_validation as native
import rust_manual_mission as mission


def scenario(binary, out, mode, existing_manual_gate=False):
    """Require real producer/control and keep human manual gates unchanged.

    Root stamps an explicit operator policy before worker launch. Success only
    publishes pending_human/pending_review; attempt and budget history survive,
    exact replay creates neither worker nor control. Preview's absent planning
    coordinates must stay null rather than impersonating a planning authority.
    """
    out.mkdir(); f = native.fixture.Fixture(binary, out, 'ordinary-' + mode)
    native.fixture.publish(f.root / 'codex', mission.PROGRAM.encode()); os.chmod(f.root / 'codex', 0o700)
    w = f.ok(['work', 'show', f.work])['work']; assert not w.get('planning')
    policy = dict(mode=mode, controls=[dict(id='ordinary-check', command=['python3', '-c', native.CONTROL],
        criteria=[1], inputs=['handoff.md'], review_output=True,
        justification='Observe the actual native worker handoff without accepting the task')])
    w = f.ok(['task', 'update', f.work], dict(schema_version=1, event_id='authorize-controls',
        expected_revision=w['revision'], id='task', validation_policy=policy))['work']
    task = w['tasks'][0]; assert task['validation_policy']['authorized_at'] and task['validation_policy']['actor']
    f.request['expected_revision'] = w['revision']; p = f.prepare(True)
    assert not p['process_started']; f.request['model_policy_hash'] = p['agent']['model_route']['policy_hash']
    prepared = f.prepare(); f.run_and_assert(prepared); mission.submit(f)
    if existing_manual_gate:
        assert mode == 'human'
        mission.gate(f, event='prior-human-gate')  # Explicit fixture operator assertion.
    before = f.ok(['work', 'show', f.work])['work']; task = before['tasks'][0]
    if existing_manual_gate: assert task.get('gate')

    assert not before.get('planning') and not task.get('scope_id') and not task.get('plan_role')
    budget = f.reservation(); attempts = task['attempts']; plan_checks = task['plan_checks']
    r = dict(schema_version=1, event_id='ordinary-validation', expected_revision=before['revision'], task_id='task')
    code, preview = f.call(['task', 'validation-preview', f.work], r)
    assert code == 0, ('ordinary native validation unavailable', preview)
    assert preview['policy_mode'] == mode and preview['authority_mode'] == 'operator-task-policy'
    assert preview['scope_generation'] is None and preview['authority_event'] is None and preview['authority_revision'] is None
    assert not preview['execution_authorized'] and not (f.root / 'validation-executed.json').exists()
    result = f.ok(['task', 'validate', f.work], r)
    assert result['controls_passed'] and not result['task_accepted'] and not result['retired']
    after = f.ok(['work', 'show', f.work]); task = after['work']['tasks'][0]
    receipt = task['automatic_validation']; marker = (f.root / 'validation-executed.json').read_bytes()
    assert task['status'] == 'submitted' and task['attempts'] == attempts and f.reservation() == budget
    assert receipt['state'] == ('pending_human' if mode == 'human' else 'pending_review')
    assert receipt['producer_agent_id'] == 'session' and receipt['attempt_id'] == prepared['agent']['attempt_id']
    assert receipt['controls'][0]['executed'] and receipt['controls'][0]['passed']
    assert receipt['artifacts'][receipt['receipt']] == native.fixture.digest((f.root / receipt['receipt']).read_bytes())
    assert task['plan_checks'] == plan_checks
    if mode == 'human':
        assert task.get('gate') == before['tasks'][0].get('gate')
        current_gate = after['current_gates'].get('task')
        assert task['evidence_stale_reason'] == 'human-validation-required'
        assert after['validation']['tasks']['task']['fresh'] is False
        if existing_manual_gate: assert after['validation']['tasks']['task']['blockers']
        if existing_manual_gate:
            assert current_gate is not None and current_gate['valid'] is True  # Calculation is not acceptance freshness.
        else: assert current_gate is None
        code, refused = f.call(['task', 'update', f.work], dict(schema_version=1,event_id='no-implicit-human-accept',
            expected_revision=after['work']['revision'],id='task',status='accepted'))
        assert code != 0 and f.ok(['work','show',f.work]) == after, refused
    else: assert after['current_gates']['task']['valid']
    replay = f.ok(['task', 'validate', f.work], r)
    assert replay['replayed'] and replay['work'] == after['work'] and (f.root / 'validation-executed.json').read_bytes() == marker
    operator_accepted = False
    if existing_manual_gate:
        # The controller preserved the earlier manual gate but kept it stale.
        # Only another explicit operator gate/accept request can close this fixture.
        mission.gate(f, event='renewed-human-gate')
        current = f.ok(['work', 'show', f.work])
        assert current['current_gates']['task']['valid']
        accepted = f.ok(['task', 'update', f.work], dict(schema_version=1,event_id='explicit-operator-fixture-accept',
            expected_revision=current['work']['revision'],id='task',status='accepted'))
        assert accepted['work']['tasks'][0]['status'] == 'accepted'
        replay = f.ok(['task', 'validate', f.work], r)
        assert replay['replayed'] and not replay['task_accepted'] and replay['work']['tasks'][0]['status'] == 'accepted'
        assert (f.root / 'validation-executed.json').read_bytes() == marker
        operator_accepted = True

    return dict(mode=mode, actual_worker=True, actual_control=True, no_planning_identity=True,
        pending=receipt['state'], replay_without_execution=True, no_acceptance=True,
        manual_gate_preserved=mode == 'human', existing_manual_gate=existing_manual_gate,
        explicit_operator_fixture_accepted=operator_accepted, independent_review=False, global_acceptance=False, root=str(f.root))


def main():
    """Retain exact imported recipe sources and binary around the ordinary mode/gate variants."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path)
    p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(exist_ok=False)
    sources={str(Path(m.__file__)):native.fixture.digest(Path(m.__file__).read_bytes())
        for m in list(sys.modules.values()) if getattr(m,'__file__',None)
        and str(Path(m.__file__).resolve()).startswith(str(Path('tests').resolve()))}
    sources[str(Path(__file__))]=native.fixture.digest(Path(__file__).read_bytes())
    inputs=dict(binary=str(binary),binary_sha256=native.fixture.digest(binary.read_bytes()),sources=sources)
    native.fixture.publish(out/'inputs.json',inputs)
    try:
        results=[scenario(binary,out/mode,mode) for mode in ['human','automatic']]
        results.append(scenario(binary,out/'human-existing-gate','human',True))
        assert native.fixture.digest(binary.read_bytes())==inputs['binary_sha256']
        assert all(native.fixture.digest(Path(n).read_bytes())==h for n,h in sources.items())
        result=dict(status='PASS',results=results,real_provider=False,installed=False)
        native.fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:
        native.fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
