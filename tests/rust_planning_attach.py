#!/usr/bin/env python3
"""Verify a hierarchy attaches to a populated Work using the public Swarm CLI."""
import argparse
import json
import subprocess
from pathlib import Path

from rust_agent_model_route import Fixture, PROGRAM, digest, publish


def run(binary, output, *, managed=False, graph=False, history=False):
    """Cover existing-work attachment with explicit or managed workspace ownership."""
    fixture = Fixture(binary, output, 'planning-attach')
    work_id = fixture.work

    def show():
        return fixture.ok(['work', 'show', work_id])['work']

    def change(operation, event, **fields):
        return fixture.call(operation + [work_id], dict(
            schema_version=1, event_id=event,
            expected_revision=show()['revision'], **fields,
        ))

    for task, depends in [('grpc', ['task']), ('parallel', [])]:
        code, outcome = change(['task', 'add'], 'add-' + task, id=task, title=task,
                               deliverable='Observed result', criteria=['Evidence'],
                               depends=depends)
        assert code == 0, outcome

    assignments = {'task': 'wasmtime', 'grpc': 'grpc', 'parallel': 'parallel'}
    workspaces = {}
    for task in assignments:
        directory = fixture.root / ('workspace-' + task)
        directory.mkdir(mode=0o700)
        workspaces[task] = str(directory)
    config = dict(provider='fixture', level='standard', max_activations=3,
                  max_review_calls=3, max_tasks=3, max_decisions=3)
    original_receipt = None
    if history:
        # Run a real inert ordinary worker to a nonzero exit before attachment.
        # Its completion is a genuine engine original and is never rewritten.
        provider = fixture.root / 'codex'
        publish(provider, (PROGRAM + '\nsys.exit(17)\n').encode())
        provider.chmod(0o700)
        fixture.request['expected_revision'] = show()['revision']
        fixture.prepare()
        fixture.call(['agent', 'run', 'session'])
        failed = show()['tasks'][0]
        assert failed['status'] == 'blocked' and failed['attempts'][-1]['status'] == 'failed'
        receipt = fixture.root / '.swarm' / ('native-completion-' + digest(b'session') + '.json')
        original_receipt = receipt.read_bytes()
        publish(output / 'ordinary-worker-original.json', original_receipt)

    before = show()
    if graph:
        flat = fixture.ok(['planning', 'graph', work_id])
        assert not flat['planning_configured'] and not flat['scopes']
        assert flat['reviewer_provider'] is None
        assert flat['conductor_observation'] is None and len(flat['tasks']) == 3
        assert show() == before
        print('PASS CLI graph reports absent orchestration without a write', flush=True)
    code, result = change(['planning', 'attach'], 'missing-policy',
                          task_scopes=assignments, **config)
    assert code != 0 and 'planning-attach-validation-unauthorized' in str(result), result
    assert show() == before
    print('PASS attachment-validates-operator-policy', flush=True)

    for task in assignments:
        code, result = change(['task', 'update'], 'authorize-' + task, id=task,
                              validation_policy={'mode': 'human'})
        assert code == 0, result

    before = show()
    code, result = change(['planning', 'attach'], 'missing-task',
                          task_scopes={'task': 'wasmtime', 'grpc': 'grpc'}, **config)
    assert code != 0 and 'planning-attach-task-coverage' in str(result), result
    assert show() == before
    print('PASS attachment-refuses-partial-assignment', flush=True)

    code, result = change(['planning', 'attach'], 'private-workspace',
                          task_scopes=assignments, task_workspaces={'task': '.swarm'}, **config)
    assert code != 0 and 'planning-attach-task-workspace-private' in str(result), result
    assert show() == before
    print('PASS attachment-refuses-private-workspace', flush=True)

    if managed:
        source = fixture.root / 'repository'
        source.mkdir()
        subprocess.run(['git', 'init', '--quiet', str(source)], check=True)
        tracked = source / 'input.txt'
        tracked.write_text('Witness from the pinned commit\n')
        subprocess.run(['git', '-C', source, 'add', 'input.txt'], check=True)
        subprocess.run(['git', '-C', source, '-c', 'user.name=Owned fixture', '-c',
                        'user.email=fixture@example.invalid', 'commit', '--quiet', '-m',
                        'Managed attachment fixture'], check=True)
        base = subprocess.check_output(['git', '-C', source, 'rev-parse', 'HEAD'], text=True).strip()
        tracked.write_text('Dirty input outside the authority\n')
        repo = dict(path='repository', committed_only=True)
        inputs = dict(task_scopes=assignments, repository=repo, **config)
        storage = fixture.root / '.swarm' / 'managed' / work_id
        code, result = change(['planning', 'attach'], 'managed-workspace-conflict',
                              task_workspaces=workspaces, **inputs)
        assert code != 0 and 'planning-attach-managed-workspaces-conflict' in str(result), result
        assert show() == before and not storage.exists()
        code, result = change(['planning', 'attach'], 'managed-dirty-refused',
                              **dict(inputs, repository=dict(repo, include_dirty=['input.txt'])))
        assert code != 0 and 'managed-dirty-snapshot-unported' in str(result), result
        assert show() == before and not storage.exists()
        print('PASS managed attachment refuses ambiguous workspaces and dirty source', flush=True)
    else:
        inputs = dict(task_scopes=assignments, task_workspaces=workspaces, **config)

    code, result = change(['planning', 'attach'], 'attach-team', **inputs)
    assert code == 0, result
    after = show()
    assert after['id'] == work_id and after['revision'] == before['revision'] + 1
    plan = after['planning']
    assert plan['provider'] == 'fixture' and plan['reviewer']['provider'] == 'fixture'
    assert plan['reviewer_required'] and plan['reviewer'].get('calls', 0) == 0
    scopes = {scope['id']: scope for scope in plan['scopes']}
    assert set(scopes) == {'root', 'wasmtime', 'grpc', 'parallel'}
    assert all(scopes[scope]['parent'] == 'root' for scope in assignments.values())
    assert len(after['tasks']) == len(before['tasks'])
    for old, current in zip(before['tasks'], after['tasks'], strict=True):
        assert current['scope_id'] == assignments[old['id']] and current['plan_role'] == 'worker'
        assert current['requirements'] == scopes[current['scope_id']]['requirements']
        if not managed:
            profile = current['launch_profile']
            assert profile['workspace'] == workspaces[old['id']]
            assert profile['provider'] == 'fixture' and profile['role'] == 'worker'
        else:
            assert current.get('launch_profile') == old.get('launch_profile')
        assert {k: v for k, v in current.items() if k not in
                {'scope_id', 'plan_role', 'requirements', 'launch_profile'}} == {
                    k: v for k, v in old.items() if k != 'launch_profile'}
    if managed:
        assert plan['repository']['base_commit'] == base
        storage = Path(plan['repository']['storage'])
        assert storage == fixture.root / '.swarm' / 'managed' / work_id
        bare = storage / 'repository.git'
        assert subprocess.check_output(['git', '--git-dir', bare, 'show', base + ':input.txt']) == b'Witness from the pinned commit\n'
        assert tracked.read_bytes() == b'Dirty input outside the authority\n'
        assert not (storage / 'copies').exists()
        snapshot = (storage / 'repository.json').read_bytes()
        code, result = fixture.call(['planning', 'attach', work_id], dict(
            schema_version=1, event_id='attach-team', expected_revision=before['revision'], **inputs))
        assert code == 0, result
        assert show() == after and (storage / 'repository.json').read_bytes() == snapshot
        print('PASS managed attachment pins committed input without worker copy', flush=True)
    controls = fixture.ok(['mission', 'show', work_id])
    assert controls['consent_enabled'] is None and controls['autonomy'] is None
    if history:
        completion = [event for event in plan['inbox'] if event['kind'] == 'attempt_ended']
        assert len(completion) == 1
        event = completion[0]
        assert (event['scope'], event['task'], event['attempt']) == (
            assignments['task'], 'task', failed['attempts'][-1]['id'])
        assert after['tasks'][0]['attempts'] == failed['attempts']
        assert not after['tasks'][0].get('planning_retry')
        assert receipt.read_bytes() == original_receipt
        print('PASS attachment routes the genuine historical worker completion without granting retry', flush=True)
        status = fixture.ok(['mission', 'status', work_id])
        assert status['organization']['ready'] and status['validated'] == 0
        assert {task['id'] for task in status['tasks']} == set(assignments)
        assert status['active_agents'] == 0 and show() == after
        print('PASS cockpit status preserves historical ordinary completion after managed attachment', flush=True)
        from rust_native_production_retry import negative_frame
        changed = json.loads(original_receipt)
        changed['details']['kernel_reaped'] = False
        with negative_frame(receipt, original_receipt, json.dumps(changed).encode()):
            code, response = fixture.call(['mission', 'status', work_id])
            assert code != 0 and 'agent-completion-wait-not-confirmed' in str(response), response
            assert show() == after
        assert receipt.read_bytes() == original_receipt
        restored = fixture.ok(['mission', 'status', work_id])
        # Runtime clocks and spending windows continue to advance during reads.
        for field in ('organization', 'tasks', 'validated', 'active_agents', 'total'):
            assert restored[field] == status[field], field
        print('PASS cockpit refuses unproven history and recovers after exact restoration', flush=True)
    if graph:
        organized = fixture.ok(['planning', 'graph', work_id])
        assert organized['planning_configured'] and organized['reviewer_provider'] == 'fixture'
        assert organized['conductor_observation'] is None
        assert organized['work'] == work_id and organized['revision'] == after['revision']
        expected_scopes = {
            'root': (None, 'root_planner'),
            **{scope: ('root', 'subplanner') for scope in assignments.values()},
        }
        assert {scope['id']: (scope['parent'], scope['role'])
                for scope in organized['scopes']} == expected_scopes
        assert {task['id']: task['scope'] for task in organized['tasks']} == assignments
        assert {task['id']: task['prerequisites'] for task in organized['tasks']} == {
            'task': [], 'grpc': ['task'], 'parallel': [],
        }
        assert show() == after
        print('PASS CLI graph preserves delegation, validation role and task prerequisites', flush=True)
    code, result = change(['planning', 'attach'], 'repeat-team',
                          task_scopes=assignments, **config)
    assert code != 0 and 'planning-attach-ordinary-work-required' in str(result), result
    assert show() == after
    print('PASS attachment-retains-graph-and-no-launch-consent', flush=True)
    return {'status': 'PASS', 'work': work_id, 'revision': after['revision'],
            'task_scopes': assignments, 'provider_called': False,
            'fixture_worker_ran': history, 'attached_worker_completion': history,
            'binary_sha256': digest(binary.read_bytes())}


def main():
    """Retain the public fixture results, including original refused responses."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--managed', action='store_true')
    parser.add_argument('--graph', action='store_true')
    parser.add_argument('--history', action='store_true')
    args = parser.parse_args()
    args.output.mkdir(mode=0o700)
    outcome = run(args.binary.resolve(strict=True), args.output, managed=args.managed, graph=args.graph, history=args.history)
    publish(args.output / 'result.json', outcome)


if __name__ == '__main__':
    main()
