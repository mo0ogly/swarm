#!/usr/bin/env python3
"""Historical CLI budget reads/preview with persisted warning policy and recovery.

Public init/work/settings/budget apply create all business state on an isolated
Root. Explicit historical planning ledger fixtures exercise sums, absent/zero
cost and corrupt-row refusals. No live mission/provider, retry or watchdog.
"""
import argparse
import hashlib
import json
from pathlib import Path
from rust_agent_model_route import publish
from rust_preparation_reads import Driver


def verify(binary, output):
    """Assert observable historical DTOs, financial preview and saved policies.

    All original argv/channels survive failure. SQL fixture edits are confined
    to one named planning row, never Work or identity rows; fault restoration
    runs before recovery assertions. Reads/previews must preserve the full dump.
    """
    output.mkdir(parents=True, exist_ok=False)
    d = Driver(binary, output)
    checks = []

    def check(name, condition):
        """Expose each unique meaningful assertion without manufacturing PASS."""
        assert condition, name
        assert name not in checks, name
        checks.append(name)
        print('PASS budget-history-' + name, flush=True)

    def proposal(limit, reserve, revision, event='budget-preview'):
        """Use explicit test policy amounts; actor/time must never grant authority."""
        return dict(schema_version=1, event_id=event, expected_revision=revision,
            budget=dict(limit_usd=limit, reserve_per_launch_usd=reserve,
                estimate_source='Explicit fixture estimate', reference_date='2026-10-25',
                actor='forged-not-authority', updated='forged-not-a-clock'))

    def rejected(args, value=None):
        """Require a typed failure with empty successful output and unchanged SQL."""
        before = d.database()
        code, raw, error = d.call(args, value)
        assert code != 0 and not raw, (code, raw)
        assert json.loads(error)['failure']['code'], error
        assert d.database() == before

    def warning(percent):
        """Adopt only the existing Admin setting through preview/apply, no fallback."""
        document = d.ok(['engine-settings', 'show'])
        values = document['values']
        # SettingCount uses decimal strings to preserve exact JS representation.
        encoded = None if percent is None else str(percent)
        values['work_budget_warning_percent'] = encoded
        change = dict(schema_version=document['schema_version'], event_id='warning-' + str(len(checks)),
            expected_revision=document['revision'], reason='Budget warning fixture policy',
            values=values, preview_token='')
        before = d.database()
        preview = d.ok(['engine-settings', 'preview'], change)
        check('settings-preview-' + str(percent), before == d.database())
        change['preview_token'] = preview['preview_token']
        d.ok(['engine-settings', 'apply'], change)
        check('settings-persist-' + str(percent), d.ok(['engine-settings', 'show'])['values']['work_budget_warning_percent'] == encoded)

    d.ok(['init'])
    work = d.ok(['work', 'create'], dict(schema_version=1, event_id='budget-work', expected_revision=0,
        title='Historical budget fixture', objective='Observe exact budget contracts',
        scope='Owned inactive Root', criteria=['Preview does not reserve money']))['work']['id']
    before = d.database()
    initial = d.ok(['budget', 'show', work])
    check('show-envelope', set(initial) == {'revision', 'budget', 'coverage'})
    view = initial['budget']
    check('absent-unlimited-unknown', view['budget']['limit_usd'] == 0 and view['actual_cost_usd'] is None
        and view['reserved_usd'] == view['committed_estimate_usd'] == view['remaining_usd'] == 0)
    check('show-no-mutation', d.database() == before)
    check('show-language-stable-JSON', d.ok(['budget', 'show', work, '--lang', 'en']) == initial)
    rejected(['budget', 'show', work], {})
    rejected(['budget', 'preview', work])
    check('invalid-transport-refused', True)
    d.ok(['budget', 'apply', work], proposal(10, 2, initial['revision'], 'budget-apply'))
    configured = d.ok(['budget', 'show', work])
    revision = configured['revision']
    check('apply-readback', configured['budget']['budget']['limit_usd'] == 10
        and configured['budget']['budget']['actor'] not in ['', 'forged-not-authority'])
    before = d.database()
    preview = d.ok(['budget', 'preview', work], proposal(5, 1, revision))
    check('preview-envelope', set(preview) == {'revision', 'current', 'proposed', 'proposed_remaining_usd',
        'proposed_unlimited', 'new_launch_budget_allowed', 'coverage'})
    check('preview-advisory', preview['revision'] == revision and preview['current'] == configured['budget']
        and preview['proposed_remaining_usd'] == 5 and preview['new_launch_budget_allowed'] is True
        and preview['proposed_unlimited'] is False)
    check('preview-strips-metadata', preview['proposed']['actor'] == preview['proposed']['updated'] == '')
    check('preview-no-mutation', d.database() == before)
    check('preview-language-stable-JSON', d.ok(['budget', 'preview', work, '--lang', 'en'], proposal(5, 1, revision)) == preview)
    for name, change in [('stale', proposal(5, 1, revision - 1)), ('negative', proposal(-1, 1, revision)),
                         ('unknown', {**proposal(5, 1, revision), 'unknown_contract_field': True})]:
        rejected(['budget', 'preview', work], change)
        check(name + '-refused', True)
    rejected(['budget', 'show', 'missing-budget-work'])
    check('missing-refused', True)
    usage = json.dumps(dict(input_tokens=1, output_tokens=1, provider_reported_cost_usd=0.75)).encode()
    d.write_fixture('INSERT INTO planning_calls(id,work_id,scope_id,estimate,usage,state,created_at) VALUES(?,?,?,?,?,?,?)',
        ('budget-planning', work, 'fixture-scope', 4.0, usage, 'recorded', '2026-10-25T00:00:00Z'))
    before = d.database()
    view = d.ok(['budget', 'show', work])['budget']
    check('planning-estimate-and-cost', view['reserved_usd'] == 0 and view['committed_estimate_usd'] == 4
        and view['actual_cost_usd'] == 0.75 and view['remaining_usd'] == 6)
    limited = d.ok(['budget', 'preview', work], proposal(4, 1, revision))
    unlimited = d.ok(['budget', 'preview', work], proposal(0, 0, revision))
    check('preview-blocked', limited['new_launch_budget_allowed'] is False and limited['proposed_remaining_usd'] == 0)
    check('preview-unlimited', unlimited['proposed_unlimited'] is True and unlimited['new_launch_budget_allowed'] is True)
    check('planning-reads-no-mutation', d.database() == before)
    for percent, expected in [(40, True), (90, False), (None, False)]:
        warning(percent)
        check('warning-consumed-' + str(percent), d.ok(['budget', 'show', work])['budget']['warning'] is expected)
    for label, column, bad, good in [('negative-estimate', 'estimate', -1.0, 4.0),
                                    ('invalid-usage', 'usage', b'{invalid', usage)]:
        # Valid witness before each fault; static column choices are fixture-only.
        d.ok(['budget', 'show', work])
        d.write_fixture('UPDATE planning_calls SET ' + column + '=? WHERE id=?', (bad, 'budget-planning'))
        try:
            rejected(['budget', 'show', work])
            rejected(['budget', 'preview', work], proposal(5, 1, revision))
            check(label + '-refused', True)
        finally:
            d.write_fixture('UPDATE planning_calls SET ' + column + '=? WHERE id=?', (good, 'budget-planning'))
        check(label + '-recovery', d.ok(['budget', 'show', work])['budget']['actual_cost_usd'] == 0.75)
    zero = json.dumps(dict(input_tokens=1, output_tokens=1, provider_reported_cost_usd=0)).encode()
    d.write_fixture('UPDATE planning_calls SET usage=?,state=? WHERE id=?', (zero, 'released', 'budget-planning'))
    view = d.ok(['budget', 'show', work])['budget']
    check('released-not-committed-zero-reported', view['committed_estimate_usd'] == 0 and view['actual_cost_usd'] == 0)
    publish(output / 'result.json', dict(status='PASS', assertions=checks, root=str(d.root), binary=str(binary),
        sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), limits=['No real provider', 'No Go binary differential',
        'Planning SQL fixtures only; worker/assist/preparation launch paths not exercised']))


def main():
    """Require a binary and fresh evidence output; preserve all original failures."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    verify(args.binary.resolve(), args.output.resolve())


if __name__ == '__main__':
    main()
