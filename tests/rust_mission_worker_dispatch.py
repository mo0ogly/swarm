#!/usr/bin/env python3
"""Verify one explicit dispatch admission using qualified normal/SDK binaries.

Public planning/worker failure/retry creates a genuine saved task override before
this SDK runs. Those owned inert setup processes are distinct from dispatch,
which only admits a queued intention. No paid provider, positive SQL seed,
installed service, test watchdog or automatic task acceptance is involved.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time
import traceback

import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as planning
import rust_planning_bootstrap as bootstrap
import rust_workspace_fifo as fifo
import rust_workspace_fifo_completion as completion
import rust_mission_watch_worker as watch


def prepare_target(binary, out, exit_code):
    """Create three root-owned public tasks and a real saved shared profile.

    A healthy exit0 control precedes the fresh exit17 case. Program exit is the
    sole failure variant; public Root/profile/budget and completion proof remain
    real. A later public planner retry authorizes the failed task, preserving
    its attempt/receipt. Explicit reserve100/limit1000 avoids hiding its declared
    cost25 behind an altered cost guard; no consumption is reset.
    """
    owned = fixture.Fixture(binary, out, 'dispatch-public-setup')
    (owned.root / 'dispatch-shared').mkdir(exist_ok=False)
    fixture.publish(owned.root / 'codex', planning.feedback.PROGRAM.encode())
    (owned.root / 'codex').chmod(0o700)
    work = owned.ok(['work', 'create'], dict(schema_version=1,
        event_id='dispatch-work', expected_revision=0, title='Owned dispatch witness',
        objective='Select after blocked contender and preserve dispatch identity',
        scope='Owned inert setup', criteria=['Actual preserved attempt and no new provider']))['work']
    owned.work = work['id']
    checks = {'req-1': [dict(id='actual-report', command=['python3', '-c',
        "from pathlib import Path; assert Path('dispatch-shared/prime-report.md').is_file()"],
        criteria=[1], inputs=['dispatch-shared/prime-report.md'],
        justification='Future report verification; no execution or acceptance here')]}
    work = owned.ok(['planning', 'enable', owned.work], bootstrap.request(
        event='dispatch-enable', max_tasks=3, checks=checks))
    operations = [dict(kind='task', id=name, title='Owned ' + name,
        next='Produce only the explicit owned report', requirements=['req-1'],
        deliverable='dispatch-shared/prime-report.md' if name == 'task' else name + '/handoff.md',
        criteria=['Preserve original process evidence'])
        for name in ('task', 'task-2', 'task-3')]
    planning.proposal(owned, work, 'root', operations)
    value = owned.ok(['planning', 'step', owned.work])
    assert value['run']['decision_applied']
    work = value['run']['work']
    assert [task['id'] for task in work['tasks']] == ['task', 'task-2', 'task-3']
    work = owned.ok(['budget', 'apply', owned.work], dict(schema_version=1,
        event_id='dispatch-budget', expected_revision=work['revision'],
        budget=dict(limit_usd=10, reserve_per_launch_usd=1,
            estimate_source='Owned inert setup, no measured price', reference_date='2026-10-22')))
    program = planning.mission.PROGRAM.replace("'model-received.json'", "'dispatch-prime-received.json'").replace(
        "os.replace(name,'handoff.md')", "os.replace(name,'prime-report.md')") + '\nsys.exit(' + str(exit_code) + ')\n'
    fixture.publish(owned.root / 'codex', program.encode())
    (owned.root / 'codex').chmod(0o700)
    current = owned.ok(['work', 'show', owned.work])['work']
    request = dict(schema_version=1, event_id='dispatch-prime-worker',
        expected_revision=current['revision'], task_id='task', provider='fixture',
        workspace='dispatch-shared', capture_output=True)
    preview = owned.ok(['agent', 'preview', owned.work], request)
    request['model_policy_hash'] = preview['agent']['model_route']['policy_hash']
    prepared = owned.ok(['agent', 'prepare', owned.work], request)
    owned.call(['agent', 'run', 'dispatch-prime-worker'])
    current = owned.ok(['work', 'show', owned.work])['work']
    task = next(task for task in current['tasks'] if task['id'] == 'task')
    expected = 'completed' if exit_code == 0 else 'failed'
    assert task['attempts'][-1]['status'] == expected
    assert Path(task['launch_profile']['workspace']) == owned.root / 'dispatch-shared'
    receipt_path = owned.root / '.swarm' / ('native-completion-' + fixture.digest(b'dispatch-prime-worker') + '.json')
    raw = receipt_path.read_bytes()
    receipt = json.loads(raw)
    assert receipt['exit_code'] == exit_code and receipt['failed'] == (exit_code != 0)
    assert not receipt['incomplete_output'] and receipt['details']['kernel_reaped'] and receipt['details']['output_complete']
    fixture.publish(out / 'original-prime-completion.json', raw)
    assert json.loads((owned.root / 'dispatch-shared/dispatch-prime-received.json').read_bytes())['stdin_sha256'] == fixture.digest(prepared['agent']['prompt'].encode())
    if exit_code == 0:
        return owned, current
    fixture.publish(owned.root / 'codex', planning.feedback.PROGRAM.encode())
    (owned.root / 'codex').chmod(0o700)
    planning.proposal(owned, current, 'root', [dict(kind='retry', id='task',
        next='Retry the original failed shared task from its exact retained completion')])
    value = owned.ok(['planning', 'step', owned.work])
    assert value['run']['decision_applied']
    current = owned.ok(['work', 'show', owned.work])['work']
    retried = next(value for value in current['tasks'] if value['id'] == 'task')
    assert retried['planning_retry'] and retried['attempts'] == task['attempts']
    assert receipt_path.read_bytes() == raw
    owned.ok(['budget', 'apply', owned.work], dict(schema_version=1,
        event_id='dispatch-retry-budget', expected_revision=current['revision'],
        budget=dict(limit_usd=1000, reserve_per_launch_usd=100,
            estimate_source='Explicit inert history envelope; no consumption reset', reference_date='2026-10-22')))
    return owned, owned.ok(['work', 'show', owned.work])['work']


def prepare_pair(binary, out):
    """Create blocker and target through real public operations in the SAME Root.

    Scoped Fixture adapter only retains actual private environment and namespaces
    new public event IDs. The process-local override is always restored; no SQL
    work/profile/queue/claim state is seeded and no concurrent caller uses it.
    """
    prepare_target(binary, out / 'control', 0)
    owned, target = prepare_target(binary, out / 'target', 17)
    original = fixture.Fixture
    fifo.ReusedFixture.original = owned
    fixture.Fixture = fifo.ReusedFixture
    try:
        blocker, reused = planning.flow(binary, out / 'blocker', before_worker=True, return_fixture=True)
    finally:
        fixture.Fixture = original
        fifo.ReusedFixture.original = None
    assert blocker['root'] == str(owned.root) and blocker['work_id'] != target['id']
    return owned, reused, target['id'], blocker['work_id']


def main():
    """Bind profiles, retain channels and exercise the finite SDK assertion set.

    Fresh output only; all prerequisites must pass before SDK execution. Require
    three exact original-namespace/revision refusals before synchronization, with
    full SQL stability and safe typed labels; they are separate from permission
    withdrawal. Reopen
    normal CLI queue after SDK close and check actual queued intention/acquired
    turn. Provider/report inventory remains EXACT from pre-dispatch setup. Failures
    preserve Roots, original receipts, commands and stdout/stderr for diagnosis.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'sdk-binary', 'sdk-profile', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    fixture.publish(out / 'method.py', Path(__file__).read_bytes())
    root = None
    try:
        binary, sdk = args.binary.resolve(), args.sdk_binary.resolve()
        normal, profile = json.loads(args.profile.read_bytes()), json.loads(args.sdk_profile.read_bytes())
        assert normal['no_fixture_feature'] is True and normal['sha256'] == fixture.digest(binary.read_bytes())
        assert profile['example'] == 'mission_worker_dispatch'
        assert profile['engine_features_empty'] and profile['shared_workspace_features_match_normal'] and not profile['test_profile']
        assert profile['binary_sha256'] == fixture.digest(sdk.read_bytes()) and profile['normal_profile_sha256'] == normal['sha256']
        fixture.publish(out / 'profiles.json', dict(normal=normal, sdk=profile))
        sources = {str(Path(module.__file__).resolve()): fixture.digest(Path(module.__file__).read_bytes())
                   for name, module in tuple(sys.modules.items())
                   if name.startswith('rust_') and getattr(module, '__file__', None)}
        sources[str(Path(__file__).resolve())] = fixture.digest(Path(__file__).read_bytes())
        fixture.publish(out / 'inputs.json', dict(sources=sources))
        owned, reused, work, blocker = prepare_pair(binary, out / 'setup')
        root = owned.root
        before_provider = completion.provider_files(root)
        fixture.publish(out / 'provider-before.json', before_provider)
        request = dict(kind='owned-inert-dispatch-fixture', root=str(root), work=work,
                       blocker=blocker, result=str(out / 'sdk-result.json'))
        fixture.publish(root / '.swarm/verification-dispatch.json', request)
        fixture.publish(out / 'request.json', request)
        argv = [str(sdk), str(out / 'request.json')]
        fixture.publish(out / 'command.json', dict(argv=argv, global_watchdog=None))
        started = time.time()
        run = subprocess.run(argv, env=owned.environment, stdin=subprocess.DEVNULL, capture_output=True)
        fixture.publish(out / 'stdout', run.stdout)
        fixture.publish(out / 'stderr', run.stderr)
        fixture.publish(out / 'exit.json', dict(returncode=run.returncode, started=started, finished=time.time()))
        assert run.returncode == 0, 'dispatch SDK failed; retained original channels'
        lines = run.stdout.decode().splitlines()
        for marker in ('PASS first FIFO-blocked contender yields to actual disjoint queued intention',
                       'PASS same dispatch identity replays without new intention or queue mutation',
                       'PASS occupied saved mission capacity stops the pass with an explicit refusal',
                       'PASS permission withdrawal remains visible and keeps acquired ownership',
                       'PASS manual foreign Work and changed original revision refuse before synchronization'):
            assert lines.count(marker) == 1, 'missing/duplicate named SDK assertion'
        result = json.loads((out / 'sdk-result.json').read_bytes())
        assert result['status'] == 'PASS' and result['work'] == work and result['blocker'] == blocker
        assert result['agent_id'] == 'dispatch-selected-agent' and result['task_id'] == 'task-2'
        for name in ('disjoint_after_fifo', 'same_identity_replay', 'capacity_global_pass_stop', 'permission_refusal_visible'):
            assert result[name] is True
        assert result['clean_replay_refusal_count'] == 3
        refusals = result['replay_refusals']
        assert len(refusals) == 3 and {value['case'] for value in refusals} == {
            'manual_namespace', 'other_work', 'original_revision'}
        for value in refusals:
            assert value['phase'] == 'replay' and value['queue'] == 'not_attempted'
            assert value['all_sql_unchanged'] is True
            assert 'agent-launch-event-conflict' in [label for cause in value['causes'] for label in cause['labels']]
        fixture.publish(out / 'replay-refusals.json', refusals)
        assert not result['provider_started_by_dispatch'] and not result['paid_provider'] and not result['task_accepted']
        assert completion.provider_files(root) == before_provider
        agent = owned.ok(['agent', 'show', result['agent_id']])['agent']
        assert agent['status'] == 'queued' and agent['task_id'] == 'task-2'
        watch.verify_current_root_binding(agent['native_mission'], root)
        queue = owned.ok(['mission', 'queue', work])
        assert any(turn == result['turn'] for turn in queue['turns'])
        assert all(turn['state'] == 'released' for turn in queue['turns'] if turn['work'] == work and not turn['agent'])
        assert not (root / 'dispatch-shared/model-received.json').exists() and not (root / 'dispatch-disjoint/model-received.json').exists()
        assert all(fixture.digest(Path(path).read_bytes()) == sha for path, sha in sources.items())
        assert fixture.digest(binary.read_bytes()) == normal['sha256'] and fixture.digest(sdk.read_bytes()) == profile['binary_sha256']
        fixture.publish(out / 'result.json', dict(status='PASS', root=str(root), work=work,
            blocker=blocker, agent_id=agent['id'], task_id=agent['task_id'],
            normal_binary_sha256=normal['sha256'], sdk_binary_sha256=profile['binary_sha256'],
            named_sdk_assertions=5, replay_conflict_cases=3, source_stable=True, provider_started_by_dispatch=False,
            setup_provider='owned inert programs only', installed=False, task_accepted=False, global_watchdog=None))
        print('PASS public dispatch selection replay capacity and permission with no provider launch', flush=True)
        print('PASS three original replay conflicts preserve all SQL before synchronization', flush=True)
        print('REPLAY_EVIDENCE=' + str(out), flush=True)
    except BaseException:
        fixture.publish(out / 'failure.json', dict(status='FAIL', root=str(root) if root else None, cause=traceback.format_exc()))
        raise


if __name__ == '__main__':
    main()
