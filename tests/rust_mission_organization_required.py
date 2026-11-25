#!/usr/bin/env python3
"""Verify manual and automatic admission require a configured Swarm team."""
import argparse
from pathlib import Path

from fixture_area import fixture_root
from rust_agent_model_route import Fixture, digest, publish
from rust_native_task_validation import invoke


def scenario(binary, evidence):
    """Use the public CLI on an owned root and preserve the actual refusals."""
    root = fixture_root('mission-organization-')

    def call(label, args, event=None, revision=None, **fields):
        request = None if event is None else dict(
            schema_version=1, event_id=event, expected_revision=revision, **fields,
        )
        return invoke(binary, root, evidence, label, args, request)

    def create(label):
        code, data = call(label, ['work', 'create'], label, 0,
                          title='Organization fixture',
                          objective='Observe the automatic-departure guard',
                          scope='Owned fixture', criteria=['Consent unchanged'])
        assert code == 0, data
        return data['work']

    def refused_start(label, work, reason):
        args = ['mission', 'show', work]
        code, before = call(label + '-before', args)
        assert code == 0 and before['consent_enabled'] is None, before
        code, result = call(label, ['mission', 'start', work], label, before['revision'],
                            reason='Observe the automatic-departure guard')
        assert code != 0 and reason in str(result), result
        code, after = call(label + '-after', args)
        assert code == 0 and before == after, after

    code, result = call('init', ['init'])
    assert code == 0, result
    ordinary = create('without-orchestrator')
    refused_start('missing-planning', ordinary['id'], 'organization-planning-missing')

    unreviewed = create('without-reviewer')
    code, result = call('configure-planner', ['planning', 'enable', unreviewed['id']],
                        'configure-planner', unreviewed['revision'],
                        max_activations=1, max_tasks=1, max_decisions=1)
    assert code == 0, result
    refused_start('missing-reviewer', unreviewed['id'], 'organization-reviewer-missing')
    return root



def manual_scenario(binary, evidence):
    """Exercise the same manual request before and after configuring its team.

    Public CLI only. Retain workload and attempt observations on each refusal.
    The positive control uses one owned inert provider, never a remote call.
    """
    fixture = Fixture(binary, evidence, 'manual-organization')
    work = fixture.work
    work_show = ['work', 'show', work]
    initial = fixture.ok(work_show)['work']
    assert initial['revision'] == fixture.request['expected_revision']
    for action in ('preview', 'prepare'):
        code, result = fixture.call(['agent', action, work], fixture.request)
        assert code != 0 and 'organization-planning-missing' in str(result), (action, code)
        assert fixture.ok(work_show)['work'] == initial
        shown, _ = fixture.call(['agent', 'show', fixture.request['event_id']])
        assert shown != 0
        assert fixture.reservation() is None
        assert not (fixture.workspace / 'model-received.json').exists()
    print('PASS manual admission refuses absent organization without a reservation', flush=True)

    flat = fixture.ok(['planning', 'graph', work])
    assert not flat['planning_configured']
    assert flat['scopes'] == [dict(id='root', parent=None, role='root_planner', provider=None)]
    assert flat['reviewer_provider'] is None and flat['conductor_observation'] is None
    status = fixture.ok(['mission', 'status', work])
    assert status['organization']['root_scope'] == 'root' and not status['organization']['ready']
    assert fixture.ok(work_show)['work'] == initial
    print('PASS canonical unconfigured root is observable before assignment', flush=True)

    attached = fixture.configure_team()
    assert attached['id'] == work and attached['tasks'][0]['scope_id'] == 'worker-scope'
    assert fixture.ok(['mission', 'show', work])['consent_enabled'] is None
    prepared = fixture.prepare()
    assert prepared['created'] and not prepared['process_started']
    assert prepared['agent']['task_id'] == 'task'
    completed = fixture.ok(['agent', 'run', prepared['agent']['id']])
    assert completed['agent']['status'] == 'completed' and not completed['task_accepted']
    assert (fixture.workspace / 'model-received.json').is_file()
    print('PASS configured team starts the authorized manual worker without accepting its output', flush=True)
    return fixture.root


def main():
    """Bind manual and automatic outcomes to the same executable and retained output."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    expected = digest(binary.read_bytes())
    args.output.mkdir(mode=0o700)
    manual_root = manual_scenario(binary, args.output)
    root = scenario(binary, args.output)
    assert digest(binary.read_bytes()) == expected
    publish(args.output / 'result.json', dict(
        status='PASS', automatic_root=str(root), manual_root=str(manual_root),
        binary_sha256=expected, remote_provider_called=False,
        cases=['organization-planning-missing', 'organization-reviewer-missing',
               'manual-refusal-without-reservation', 'canonical-root', 'manual-after-attach'],
        consent_unchanged=True,
    ))
    print('PASS organization-planning-missing')
    print('PASS organization-reviewer-missing')


if __name__ == '__main__':
    main()
