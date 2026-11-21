#!/usr/bin/env python3
"""Verify automatic start refuses missing organization before persisting consent."""
import argparse
from pathlib import Path

from fixture_area import fixture_root
from rust_agent_model_route import digest, publish
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


def main():
    """Bind both named outcomes to the same executable and retained output."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    expected = digest(binary.read_bytes())
    args.output.mkdir(mode=0o700)
    root = scenario(binary, args.output)
    assert digest(binary.read_bytes()) == expected
    publish(args.output / 'result.json', dict(
        status='PASS', root=str(root), binary_sha256=expected, provider_called=False,
        cases=['organization-planning-missing', 'organization-reviewer-missing'],
        consent_unchanged=True,
    ))
    print('PASS organization-planning-missing')
    print('PASS organization-reviewer-missing')


if __name__ == '__main__':
    main()
