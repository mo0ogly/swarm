#!/usr/bin/env python3
"""Verify a hierarchy attaches to a populated Work using the public Swarm CLI."""
import argparse
from pathlib import Path

from rust_agent_model_route import Fixture, digest, publish


def run(binary, output):
    """Preserve task history, require complete coverage and leave consent unset."""
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
    before = show()
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

    code, result = change(['planning', 'attach'], 'attach-team',
                          task_scopes=assignments, task_workspaces=workspaces, **config)
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
        profile = current['launch_profile']
        assert profile['workspace'] == workspaces[old['id']]
        assert profile['provider'] == 'fixture' and profile['role'] == 'worker'
        assert {k: v for k, v in current.items() if k not in
                {'scope_id', 'plan_role', 'requirements', 'launch_profile'}} == old
    controls = fixture.ok(['mission', 'show', work_id])
    assert controls['consent_enabled'] is None and controls['autonomy'] is None
    code, result = change(['planning', 'attach'], 'repeat-team',
                          task_scopes=assignments, **config)
    assert code != 0 and 'planning-attach-ordinary-work-required' in str(result), result
    assert show() == after
    print('PASS attachment-retains-graph-and-no-launch-consent', flush=True)
    return {'status': 'PASS', 'work': work_id, 'revision': after['revision'],
            'task_scopes': assignments, 'provider_called': False,
            'binary_sha256': digest(binary.read_bytes())}


def main():
    """Retain the public fixture results, including original refused responses."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(mode=0o700)
    outcome = run(args.binary.resolve(strict=True), args.output)
    publish(args.output / 'result.json', outcome)


if __name__ == '__main__':
    main()
