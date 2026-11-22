#!/usr/bin/env python3
"""Verify isolated root and child decisions with a sandbox-compatible inert CLI."""
import argparse
import json
from pathlib import Path
import traceback

import rust_agent_model_route as fixture
import rust_mission_report_relay as relay
from rust_planning_bootstrap import request


PROGRAM = '''#!/usr/bin/python3
import json
from pathlib import Path
import sys

context = json.loads(sys.stdin.read().strip().splitlines()[-1])
scope = context['scope']['id']
requirements = sorted(context['requirements'])
if scope == 'root':
    assert requirements == ['req-1', 'req-2']
    operations = [
        dict(kind='delegate', id=child, title=child, requirements=[requirement],
             max_tasks=1, max_activations=1)
        for child, requirement in zip(('alpha', 'beta'), requirements, strict=True)
    ]
else:
    expected = {'alpha': 'req-1', 'beta': 'req-2'}
    assert requirements == [expected[scope]]
    assert all(event['scope'] == scope for event in context['events'])
    operations = [dict(kind='task', id='produce-' + scope, title='Owned task',
                       requirements=requirements, deliverable=scope + '/handoff.md',
                       criteria=['Fixture proof'])]
Path('owned-context.json').write_text(json.dumps(context))
decision = dict(input_events=[event['id'] for event in context['events']],
                reason='Inert fixture decision', operations=operations)
print(json.dumps(dict(type='result', structured_output=decision,
                      usage=dict(input_tokens=7, output_tokens=8))), flush=True)
'''

IDLE_WORK_PROGRAM = '''#!/usr/bin/python3
import json
import sys

context = json.loads(sys.stdin.read().strip().splitlines()[-1])
decision = dict(input_events=[event['id'] for event in context['events']],
                reason='No worker needed in this inert fixture', operations=[])
print(json.dumps(dict(type='result', structured_output=decision,
                      usage=dict(input_tokens=7, output_tokens=8))), flush=True)
'''


def run_automatic_planning(binary, output):
    """Let the public conductor resolve one open decision with no ready worker."""
    owned = fixture.Fixture(binary, output, 'orchestration-idle')
    provider = owned.root / 'codex'
    fixture.publish(provider, IDLE_WORK_PROGRAM.encode())
    provider.chmod(0o700)
    work = owned.ok(['work', 'create'], dict(schema_version=1, event_id='idle-create',
        expected_revision=0, title='Owned idle fixture', objective='Resolve one decision',
        scope='Owned fixture decision only', criteria=['Inert decision']))['work']
    owned.work = work['id']
    checks = {'req-1': [dict(id='witness', command=['python3', '-c', 'print(1)'],
                             inputs=['input.txt'], criteria=[1],
                             justification='Explicit fixture control')]}
    work = owned.ok(['planning', 'enable', owned.work],
                    request(event='idle-enable', max_activations=2, checks=checks))
    started = owned.ok(['mission', 'start', owned.work], dict(
        schema_version=1, event_id='idle-mission-start', expected_revision=work['revision'],
        reason='Owned idle conductor fixture', slots=1,
        profile=dict(provider='fixture', role='worker', workspace=owned.workspace.name,
                     level='standard', capture_output=True)))
    assert started['revision'] > work['revision']
    events = relay.cycle(owned, output / 'watch', completed_event='mission-planning-step')
    applied = [event for event in events if event.get('decision_applied') is True]
    assert len(applied) == 1 and applied[0]['selected_scope'] == 'root', events
    assert not any(event['event'] == 'mission-worker-started' for event in events)
    final = owned.ok(['work', 'show', owned.work])['work']
    assert final['planning']['activations'] == 1
    assert owned.ok(['planning', 'step', owned.work])['status'] == 'idle'
    assert owned.ok(['work', 'show', owned.work])['work'] == final
    fixture.publish(output / 'result.json', dict(status='PASS', planner_activations=1,
                                                worker_starts=0, paid_provider=False))


def run(binary, output):
    """Run three public decisions, then prove an unchanged inbox spends no call."""
    owned = fixture.Fixture(binary, output, 'orchestration-scope')
    provider = owned.root / 'codex'
    fixture.publish(provider, PROGRAM.encode())
    provider.chmod(0o700)
    work = owned.ok(['work', 'create'], dict(schema_version=1, event_id='scopes-create',
        expected_revision=0, title='Owned scope fixture', objective='Two child decisions',
        scope='Fixture only', criteria=['Alpha requirement', 'Beta requirement']))['work']
    owned.work = work['id']
    checks = {requirement: [dict(id='witness', command=['python3', '-c', 'print(1)'],
                               inputs=['input.txt'], criteria=[1],
                               justification='Explicit inherited fixture control')]
              for requirement in ('req-1', 'req-2')}
    owned.ok(['planning', 'enable', owned.work],
             request(event='scopes-enable', max_activations=3, checks=checks))
    for scope in ('root', 'alpha', 'beta'):
        result = owned.ok(['planning', 'step', owned.work])
        assert result['selected_scope'] == scope and result['run']['decision_applied'], result
    final = owned.ok(['work', 'show', owned.work])['work']
    assert {task['id']: task['scope_id'] for task in final['tasks']} == {
        'produce-alpha': 'alpha', 'produce-beta': 'beta'}
    assert {scope['id']: scope['parent'] for scope in final['planning']['scopes']
            if scope['id'] != 'root'} == {'alpha': 'root', 'beta': 'root'}
    witnesses = sorted((owned.root / '.swarm' / 'biba-i0' / 'planning').glob('*/owned-context.json'))
    assert len(witnesses) == 3
    before = {str(path): fixture.digest(path.read_bytes()) for path in witnesses}
    idle = owned.ok(['planning', 'step', owned.work])
    assert idle['status'] == 'idle' and idle['claim_id'] is None and idle['run'] is None
    assert owned.ok(['work', 'show', owned.work])['work'] == final
    assert {str(path): fixture.digest(path.read_bytes()) for path in
            witnesses[0].parent.parent.glob('*/owned-context.json')} == before
    return dict(status='PASS', scopes=['root', 'alpha', 'beta'], unchanged_idle=True,
                paid_provider=False, tasks_accepted=False)


def main():
    """Retain the qualified CLI identity, visible decisions and any original error."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    profile = json.loads(args.profile.read_bytes())
    assert profile['no_fixture_feature'] and profile['sha256'] == fixture.digest(args.binary.read_bytes())
    try:
        binary = args.binary.resolve()
        result = run(binary, output)
        run_automatic_planning(binary, output / 'automatic')
        fixture.publish(output / 'result.json', result)
    except BaseException:
        fixture.publish(output / 'failure.json', dict(status='FAIL', cause=traceback.format_exc()))
        raise
    print('PASS two isolated sub-orchestrator decisions and idle without a new provider call', flush=True)
    print('PASS public conductor plans when no worker can advance', flush=True)


if __name__ == '__main__':
    main()
