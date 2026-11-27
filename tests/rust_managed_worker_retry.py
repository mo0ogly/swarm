#!/usr/bin/env python3
"""Verify a managed retry through public commands with two inert local workers."""

import argparse
import json
from pathlib import Path

import rust_agent_model_route as fixture
from rust_managed_repository_bootstrap import git
from rust_planning_bootstrap import request
from rust_mission_report_relay import rows


def run(binary, output):
    """Retain original processes, copies and public refusals on one owned Root."""
    assert not output.exists()
    f = fixture.Fixture(binary, output, 'managed-retry')
    work = f.work
    source = f.root / 'repository'
    source.mkdir(mode=0o700)
    git(source, 'init', '--quiet')
    fixture.publish(source / 'input.txt', b'Committed retry input\n')
    git(source, 'add', 'input.txt')
    git(source, '-c', 'user.name=Owned fixture', '-c',
        'user.email=fixture@example.invalid', '-c', 'commit.gpgsign=false',
        'commit', '--quiet', '-m', 'Managed worker retry fixture')

    def show():
        return f.ok(['work', 'show', work])['work']

    def retained_state():
        return (
            show(),
            rows(f, 'SELECT id, status, body FROM agents WHERE work_id=? ORDER BY id', (work,)),
            rows(f, 'SELECT agent_id, state, amount FROM reservations WHERE work_id=? ORDER BY agent_id',
                 (work,)),
        )

    def refuse(args, value, reason):
        before = retained_state()
        code, answer = f.call([*args, work], value)
        assert code != 0 and reason in json.dumps(answer), answer
        assert retained_state() == before

    authorize = dict(schema_version=1, event_id='retry-validation-policy',
                     expected_revision=show()['revision'], id='task',
                     validation_policy=dict(mode='human'))
    authorized = f.ok(['task', 'update', work], authorize)['work']
    attach = request(event='retry-attach', expected_revision=authorized['revision'],
                     task_scopes={'task': 'envelopes'}, repository=dict(
                         path='repository', committed_only=True),
                     checks={'req-1': [dict(
                         id='owned-input', command=['git', 'diff', '--check'],
                         criteria=[1], justification='Keep fixture repository input stable')]})
    attached = f.ok(['planning', 'attach', work], attach)
    program = f.root / 'codex'
    fixture.publish(program, (fixture.PROGRAM + '\nsys.exit(17)\n').encode())
    program.chmod(0o700)
    first = dict(schema_version=1, event_id='first-managed-worker',
                 expected_revision=attached['revision'], task_id='task',
                 provider='fixture', capture_output=True)
    first_agent = f.ok(['agent', 'prepare', work], first)['agent']
    f.call(['agent', 'run', first_agent['id']])
    failed = show()
    task = failed['tasks'][0]
    assert task['status'] == 'blocked' and not task.get('plan_max_attempts')
    assert len(task['attempts']) == 1 and task['attempts'][0]['status'] == 'failed'
    event = next(e for e in failed['planning']['inbox']
                 if e['kind'] == 'attempt_ended' and e['attempt'] == task['attempts'][0]['id'])
    prior_copy = Path(first_agent['workspace'])
    receipt = f.root / '.swarm' / ('native-completion-' + fixture.digest(
        first_agent['id'].encode()) + '.json')
    original = receipt.read_bytes()
    first_result = json.loads(original)
    assert first_result['exit_code'] == 17 and first_result['details']['kernel_reaped']
    marker = (prior_copy / 'model-received.json').read_bytes()
    fixture.publish(output / 'first-completion.json', original)

    cap = dict(schema_version=1, event_id='retry-cap', expected_revision=failed['revision'],
               id='task', max_attempts=2, confirm_contract_revision=True,
               contract_revision_reason='Initialize precisely one additional fixture attempt')
    refuse(['task', 'update'], {**cap, 'expected_revision': failed['revision'] - 1},
           'revision_conflict')
    refuse(['task', 'update'], {**cap, 'max_attempts': 3},
           'managed-worker-retry-capacity-invalid')
    saved = f.ok(['task', 'update', work], cap)['work']
    assert f.ok(['task', 'update', work], cap)['work'] == saved
    assert saved['tasks'][0] == {**task, 'plan_max_attempts': 2}
    assert saved['planning'] == failed['planning']
    refuse(['task', 'update'], {**cap, 'event_id': 'extend-cap',
                                'expected_revision': saved['revision']},
           'managed-worker-retry-capacity-invalid')

    scope = next(s for s in saved['planning']['scopes'] if s['id'] == 'envelopes')
    settings = f.ok(['engine-settings', 'show'])['values']
    claim = dict(schema_version=1, event_id='retry-claim', expected_revision=saved['revision'],
                 scope=scope['id'], scope_revision=scope['revision'], holder='retry-fixture',
                 lease_seconds=int(settings['planning_default_lease_seconds']))
    claimed = f.ok(['planning', 'claim', work], claim)
    leased = next(s for s in claimed['planning']['scopes'] if s['id'] == claim['scope'])
    decision = dict(schema_version=1, event_id='retry-decision',
                    expected_revision=claimed['revision'], scope=leased['id'],
                    scope_revision=leased['revision'], holder=leased['holder'],
                    generation=leased['generation'], input_events=[event['id']],
                    operations=[dict(kind='retry', id='task',
                                     next='Produce evidence from the fresh managed copy')],
                    reason='Bind the retry to the original failed worker event')
    refuse(['planning', 'decide'], {**decision, 'generation': leased['generation'] - 1},
           'stale_lease')
    decided = f.ok(['planning', 'decide', work], decision)
    assert f.ok(['planning', 'decide', work], decision) == decided
    assert decided['tasks'][0]['planning_retry']
    assert decided['tasks'][0]['attempts'] == task['attempts']
    assert next(e for e in decided['planning']['inbox'] if e['id'] == event['id']
                )['decision'] == decision['event_id']

    fixture.publish(program, fixture.PROGRAM.encode())
    program.chmod(0o700)
    def budget_limit(work_revision, label, amount):
        intent = dict(schema_version=1, event_id=label, expected_revision=work_revision,
                      budget=dict(limit_usd=amount, reserve_per_launch_usd=1,
                                  estimate_source='Owned fixture estimate',
                                  reference_date='2026-10-24'))
        return f.ok(['budget', 'apply', work], intent)

    limited = budget_limit(decided['revision'], 'deny-second-reservation', 1)
    second = {**first, 'event_id': 'second-managed-worker',
              'expected_revision': limited['revision']}
    refuse(['agent', 'prepare'], second, 'budget_exhausted')
    assert not prior_copy.with_name('task-2').exists()
    assert not (prior_copy.parent.parent / 'native-worker-second-managed-worker.json').exists()
    restored = budget_limit(limited['revision'], 'restore-fixture-reservation', 10)
    second['expected_revision'] = restored['revision']
    second_agent = f.ok(['agent', 'prepare', work], second)['agent']
    fresh_copy = Path(second_agent['workspace'])
    assert fresh_copy == prior_copy.with_name('task-2')
    assert (fresh_copy / 'input.txt').read_bytes() == b'Committed retry input\n'
    assert f.ok(['agent', 'prepare', work], second)['agent'] == second_agent
    ran = f.ok(['agent', 'run', second_agent['id']])
    assert ran['agent']['status'] == 'completed' and not ran['task_accepted']
    observed = json.loads((fresh_copy / 'model-received.json').read_bytes())
    assert observed['stdin_sha256'] == fixture.digest(second_agent['prompt'].encode())
    assert (prior_copy / 'model-received.json').read_bytes() == marker
    assert receipt.read_bytes() == original
    final = show()
    assert final['tasks'][0]['attempts'][0] == task['attempts'][0]
    assert len(final['tasks'][0]['attempts']) == 2
    assert final['tasks'][0]['status'] == 'blocked'
    fixture.publish(output / 'result.json', dict(status='PASS', root=str(f.root),
                    work=work, revision=final['revision'], attempts=2,
                    first=first_agent['id'], second=second_agent['id'],
                    binary_sha256=fixture.digest(binary.read_bytes())))
    print('PASS managed retry from original native failure through a distinct copy')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.output.resolve())
