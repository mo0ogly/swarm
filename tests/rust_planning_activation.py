#!/usr/bin/env python3
"""Exercise public operator planning claims on an owned inactive Root.

Enablement is the positive control before claim. No model or provider process,
private SQL seeding or automatic conductor is permitted. Failed roots, command
records and binary/source identity remain available for diagnosis and replay.
"""
import argparse
import json
import traceback
from pathlib import Path
import rust_agent_model_route as fixture
from rust_planning_bootstrap import fresh_work, request


def configured(owned, label):
    """Create/enable a separate owned public Work, with no claim or process."""
    owned.work = fresh_work(owned, label + '-create')
    return owned.ok(['planning', 'enable', owned.work], request(event=label + '-enable'))


def claim_request(work, label, **changes):
    """Explicit test lease; zero/default timing is never inferred by the engine."""
    value = dict(schema_version=1, event_id=label, expected_revision=work['revision'],
        scope='root', scope_revision=1, holder='fixture-planner', lease_seconds=60)
    value.update(changes)
    return value


def refused(owned, value, expected):
    """Require the intended safe failure and exact public/agent/reservation state."""
    public = owned.ok(['work', 'show', owned.work])
    before = owned.snapshot()
    code, response = owned.call(['planning', 'claim', owned.work], value)
    assert code != 0 and response['failure']['code'] == expected, response
    assert 'PRIVATE' not in json.dumps(response)
    assert owned.ok(['work', 'show', owned.work]) == public and owned.snapshot() == before
    assert not (owned.root / 'model-received.json').exists()
    return dict(code=expected, mutation=False)


def apply_policy(owned, event, changes, target_schema=None):
    """Adopt a fixture policy through public preview/apply and persistent readback.

    An explicit target_schema is required to propose a new generation; otherwise
    current-version edits preserve their schema. Preview stays pure and old history
    stays intact. No runtime settings journal
    is rewritten, consumed budget reset or preexisting Work modified.
    """
    doc = owned.ok(['engine-settings', 'show'])
    values = dict(doc['values']); values.update(changes)
    proposal = dict(schema_version=doc['schema_version'] if target_schema is None else target_schema, event_id=event,
        expected_revision=doc['revision'], values=values,
        reason='Explicit scope claim fixture policy', preview_token='')
    original = (owned.root / '.swarm/engine-settings.json').read_bytes()
    preview = owned.ok(['engine-settings', 'preview'], proposal)
    assert (owned.root / '.swarm/engine-settings.json').read_bytes() == original
    proposal['preview_token'] = preview['preview_token']
    owned.ok(['engine-settings', 'apply'], proposal)
    saved = owned.ok(['engine-settings', 'show'])
    assert saved['values'] == values and saved['history'][:len(doc['history'])] == doc['history']
    return saved


def negative_cases(owned):
    """Each independent fault follows a healthy claim under matching conditions."""
    source = owned.providers.read_bytes(); cases = []
    specs = [
        ('short-lease', dict(lease_seconds=4), 'planning-lease-outside-policy', None),
        ('long-lease', dict(lease_seconds=301), 'planning-lease-outside-policy', None),
        ('scope-revision', dict(scope_revision=2), 'revision_conflict', None),
        ('holder', dict(holder='../PRIVATE'), 'planning-claim-holder-or-scope-invalid', None),
        ('unapplied-budget', dict(max_tasks=9), 'planning-claim-field-not-applicable', None),
        ('unknown-scope', dict(scope='unknown'), 'planning-scope-not-found', None),
        ('missing-provider', {}, 'agent-provider-catalogue-read', 'missing'),
        ('changed-provider', {}, 'planning-claim-provider-changed', 'changed'),
    ]
    for label, patch, expected, fault in specs:
        witness = configured(owned, label + '-healthy')
        owned.ok(['planning', 'claim', owned.work], claim_request(witness, label + '-healthy-claim'))
        candidate = configured(owned, label + '-fault')
        try:
            if fault == 'missing': owned.providers.unlink()
            if fault == 'changed':
                changed = json.loads(source)
                changed['providers']['fixture']['args'].append('--changed-fixture')
                fixture.publish(owned.providers, changed)
            cases.append(dict(case=label, **refused(owned,
                claim_request(candidate, label + '-claim', **patch), expected)))
        finally:
            if fault: fixture.publish(owned.providers, source)
        # Same refused event can recover because no mutation/event was committed.
        recovered = owned.ok(['planning', 'claim', owned.work], claim_request(candidate, label + '-claim'))
        assert recovered['planning']['activations'] == 1
    return cases


def policy_consumption(owned):
    """Saved bounds/zero caps affect real claims; correcting policy permits replay.

    Existing claim deadlines stay unchanged; rejected claims consume no activation.
    Every rejection follows a healthy control, and repair uses the same Work/event.
    """
    witness = configured(owned, 'policy-witness')
    owned.ok(['planning', 'claim', owned.work], claim_request(witness, 'policy-witness-claim', lease_seconds=10))
    work = configured(owned, 'policy')
    proposal = claim_request(work, 'policy-claim', lease_seconds=10)
    # Positive fixture adjusts a configured v27 default together with its bounds;
    # older actual binaries have no such field and acquire no implicit policy.
    native_default = owned.ok(['engine-settings','show'])['values'].get('planning_default_lease_seconds')
    default_change = {'planning_default_lease_seconds':'8'} if native_default is not None else {}
    apply_policy(owned, 'tighten-scope-lease', dict(planning_scope_lease_min_seconds='7', planning_scope_lease_max_seconds='9', **default_change))
    refused(owned, proposal, 'planning-lease-outside-policy')
    apply_policy(owned, 'restore-scope-lease', dict(planning_scope_lease_max_seconds='10'))
    claimed = owned.ok(['planning', 'claim', owned.work], proposal)
    prior_work = owned.work
    prior_scope = claimed['planning']['scopes'][0]
    # Validate event cap separately with a control under the new lease policy.
    witness = configured(owned, 'event-cap-witness')
    owned.ok(['planning', 'claim', owned.work], claim_request(witness, 'event-cap-witness-claim', lease_seconds=7))
    work = configured(owned, 'event-cap'); proposal = claim_request(work, 'event-cap-claim', lease_seconds=7)
    apply_policy(owned, 'zero-scope-events', dict(planning_context_max_events='0'))
    refused(owned, proposal, 'planning-event-batch-disabled')
    apply_policy(owned, 'unlimited-scope-events', dict(planning_context_max_events=None))
    owned.ok(['planning', 'claim', owned.work], proposal)
    assert owned.ok(['planning', 'show', prior_work])['planning']['scopes'][0] == prior_scope
    witness = configured(owned, 'byte-cap-witness')
    owned.ok(['planning', 'claim', owned.work], claim_request(witness, 'byte-cap-witness-claim', lease_seconds=7))
    work = configured(owned, 'byte-cap'); proposal = claim_request(work, 'byte-cap-claim', lease_seconds=7)
    apply_policy(owned, 'zero-scope-bytes', dict(planning_context_max_bytes='0'))
    refused(owned, proposal, 'planning-context-byte-limit-exceeded')
    apply_policy(owned, 'unlimited-scope-bytes', dict(planning_context_max_bytes=None))
    owned.ok(['planning', 'claim', owned.work], proposal)
    return dict(saved_policy_consumed=True, zero_event_and_byte_caps_refuse=True,
        null_caps_recover=True, earlier_deadline=prior_scope['lease_until'])


def legacy_adoption(binary, legacy, output):
    """Real v23 Root refuses claim until explicit v24 policy adoption.

    Configuration remains available before adoption. Preserve old settings/history
    and retry the refused event on the same Work, with no private journal rewrite.
    """
    owned = fixture.Fixture(legacy, output, 'legacy-scope'); owned.binary = binary
    doc = owned.ok(['engine-settings', 'show']); assert doc['schema_version'] == 23
    work = configured(owned, 'legacy-scope'); proposal = claim_request(work, 'legacy-scope-claim')
    refused(owned, proposal, 'settings-planning-actions-not-configured')
    defaults = owned.ok(['engine-settings', 'describe'])['defaults']
    fields = ['planning_scope_lease_min_seconds', 'planning_scope_lease_max_seconds',
        'planning_context_max_events', 'planning_context_max_bytes']
    values = dict(doc['values']); values.update({key: defaults[key] for key in fields})
    change = dict(schema_version=24, event_id='adopt-scope-policy', expected_revision=doc['revision'],
        values=values, reason='Explicit activation policy adoption', preview_token='')
    original = (owned.root / '.swarm/engine-settings.json').read_bytes()
    preview = owned.ok(['engine-settings', 'preview'], change)
    assert (owned.root / '.swarm/engine-settings.json').read_bytes() == original
    change['preview_token'] = preview['preview_token']; owned.ok(['engine-settings', 'apply'], change)
    adopted = owned.ok(['engine-settings', 'show'])
    assert adopted['schema_version'] == 24 and adopted['history'][:len(doc['history'])] == doc['history']
    claimed = owned.ok(['planning', 'claim', owned.work], proposal)
    assert claimed['planning']['activations'] == 1
    assert not (owned.root / 'model-received.json').exists()
    fixture.publish(output / 'legacy-scope-adoption.json', dict(status='PASS',
        root=str(owned.root), old_version=23, new_version=24,
        history_preserved=True, activation_consumed_once=True, provider_started=False))


def main():
    """Require a persisted current lease/delivery through the actual public CLI.

    Input paths select an existing normal binary and a fresh proof directory.
    Assertions or child refusals preserve original output and fixture ownership;
    success means local claim only, not authenticated identity or autonomous work.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--legacy-binary', type=Path)
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    fixture.publish(output / 'inputs.json', {
        'binary': str(binary), 'binary_sha256': fixture.digest(binary.read_bytes()),
        'sources': {str(p): fixture.digest(p.read_bytes()) for p in [
            Path(__file__), Path('tests/rust_planning_bootstrap.py'),
            Path('tests/rust_agent_model_route.py')]},
        'scope': 'Owned operator CLI fixture; no provider account or process',
    })
    owned = None
    try:
        owned = fixture.Fixture(binary, output, 'activation')
        enabled = configured(owned, 'activation')
        assert enabled['planning']['activations'] == 0
        claimed = owned.ok(['planning', 'claim', owned.work], dict(
            schema_version=1, event_id='activation-claim', expected_revision=enabled['revision'],
            scope='root', scope_revision=1, holder='fixture-planner', lease_seconds=60))
        planning = claimed['planning']
        scope = next(s for s in planning['scopes'] if s['id'] == 'root')
        assert planning['activations'] == 1 and scope['activations'] == 1
        assert scope['holder'] == 'fixture-planner' and scope['generation'] == 1
        assert scope['lease_until'] and scope['workflow']['sha256']
        assert scope['delivery']['context_sha256']
        assert scope['delivery']['events'] == ['activation-enable']
        original = claim_request(enabled, 'activation-claim')
        # Active lease and event conflict are separate failures, under a valid
        # already-recorded claim. Replaying the identical event must not reclaim.
        refused(owned, claim_request(claimed, 'busy-claim'), 'lease_busy')
        refused(owned, {**original, 'holder': 'other-holder'}, 'event_conflict')
        source = owned.providers.read_bytes()
        try:
            owned.providers.unlink()
            assert owned.ok(['planning', 'claim', owned.work], original) == claimed
        finally:
            fixture.publish(owned.providers, source)
        cases = negative_cases(owned)
        policy = policy_consumption(owned)
        if args.legacy_binary:
            legacy_adoption(binary, args.legacy_binary.resolve(), output)
        assert not (owned.root / 'model-received.json').exists()
        fixture.publish(output / 'result.json', dict(status='PASS', root=str(owned.root),
            negative_cases=cases, policy_consumption=policy,
            scope='Public operator claim only; no agent identity or automatic dispatch proved'))
    except BaseException:
        fixture.publish(output / 'failure.json', dict(status='FAIL',
            root=str(owned.root) if owned else None,
            cause=traceback.format_exc(), provider_process_requested=False))
        raise


if __name__ == '__main__':
    main()
