#!/usr/bin/env python3
"""Require refusal of known tool declarations on the real native planning CLI.

An owned inert provider announces events but executes no tool. Healthy native
execution precedes the faults. No account, live mission, SQL mutation, global
watchdog, implicit refund, acceptance or confinement claim is involved.
"""
import argparse, json, os, traceback
from pathlib import Path
import rust_planning_native_run as native


def rejected(binary, out, event, label):
    """Observe one provider, original refusal, known reap and unchanged replay.

    Only public operations mutate the owned Root. Errors retain original CLI
    channels and fail the test. A parser refusal may record rejection feedback;
    it must not apply a proposal, lose reserves or create another provider.
    """
    out.mkdir()
    owned = native.fixture.Fixture(binary, out, label)
    program = native.PROGRAM.replace("proposal=json.load(open('planning-proposal.json'))",
        'print(' + repr(json.dumps(event)) + ',flush=True)\n' + "proposal=json.load(open('planning-proposal.json'))")
    native.fixture.publish(owned.root / 'codex', program.encode())
    os.chmod(owned.root / 'codex', 0o700)
    enabled = native.configured(owned, label)
    claim = label + '-claim'
    work = owned.ok(['planning', 'claim', owned.work], native.claim_request(enabled, claim, lease_seconds=120))
    scope = next(s for s in work['planning']['scopes'] if s['id'] == 'root')
    native.fixture.publish(owned.root / 'planning-proposal.json', dict(input_events=scope['delivery']['events'],
        reason='Owned fixture proposal', operations=[dict(kind='task', id='forbidden-worker', title='Future task',
        requirements=['req-1'], deliverable='Future report', criteria=['Future control'])]))
    request = native.preview_request(work, claim)
    assert owned.ok(['planning', 'call-preview', owned.work], request)['prepared_inputs_verified']
    money = native.ledger(owned)
    code, wire = owned.call(['planning', 'run', owned.work], request)
    assert code != 0 and 'planning-output-tool-event-forbidden' in str(wire), wire
    assert 'PRIVATE_TOOL_INPUT' not in str(wire)
    after = owned.ok(['work', 'show', owned.work])
    assert after['work']['planning']['decisions'] == 0 and not after['work'].get('tasks')
    assert any(e['kind'] == 'planning_rejected' for e in after['work']['planning']['inbox'])
    assert native.ledger(owned) == money
    row = native.native_row(owned, claim)
    assert row[3] == 'refused' and row[4], ('Known reaped refusal not terminal', row)
    marker = (owned.root / 'model-received.json').read_bytes()
    snapshot = owned.snapshot()
    owned.denied(['planning', 'run', owned.work], request, 'planning-run-already-recorded')
    owned.denied(['planning', 'reconcile', claim], None, 'planning-receipt-not-successful')
    assert owned.snapshot() == snapshot and owned.ok(['work', 'show', owned.work]) == after
    assert (owned.root / 'model-received.json').read_bytes() == marker and native.ledger(owned) == money
    return dict(root=str(owned.root), claim=claim, known_reaped=True, phase='refused',
                decisions=0, provider_executions=1, reservations_unchanged=True, replay_without_execution=True)


def main():
    """Freeze exact binary bytes in fresh proof; retain all observed failures.

    Only the owned copy executes, so replacing the source cannot alter a later
    fixture. Hashing proves these bytes, not trusted origin or installation.
    Original read/publish/chmod errors propagate before any provider starts.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    source, out = args.binary.resolve(), args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    raw = source.read_bytes()
    digest = native.fixture.digest(raw)
    binary = out / 'swarm-rust'
    native.fixture.publish(binary, raw)
    binary.chmod(0o700)
    assert native.fixture.digest(binary.read_bytes()) == digest
    sources = [Path(__file__), Path('tests/rust_planning_native_run.py'), Path('tests/rust_agent_model_route.py')]
    native.fixture.publish(out / 'inputs.json', dict(binary=str(binary), source_binary=str(source), binary_sha256=digest,
        sources={str(p): native.fixture.digest(p.read_bytes()) for p in sources}))
    try:
        (out / 'healthy').mkdir()
        healthy = native.happy(binary, out / 'healthy')
        events = [
            ('codex-completed', dict(type='item.completed', item=dict(type='command_execution', id='tool', command='PRIVATE_TOOL_INPUT'))),
            ('codex-started', dict(type='item.started', item=dict(type='mcp_tool_call', id='tool', arguments='PRIVATE_TOOL_INPUT'))),
            ('claude-call', dict(type='assistant', message=dict(content=[dict(type='tool_use', id='tool', name='Bash', input=dict(command='PRIVATE_TOOL_INPUT'))]))),
            ('claude-result', dict(type='user', message=dict(content=[dict(type='tool_result', tool_use_id='tool', content='PRIVATE_TOOL_INPUT')]))),
        ]
        cases = {label: rejected(binary, out / label, event, label) for label, event in events}
        assert native.fixture.digest(binary.read_bytes()) == digest
        native.fixture.publish(out / 'result.json', dict(status='PASS', healthy=healthy, cases=cases,
            binary_sha256=digest, scope='Observed declarations only, no actual tool, confinement or project acceptance'))
        print('PLANNING_TOOL_FREE_PUBLIC_PASS')
    except BaseException:
        native.fixture.publish(out / 'failure.json', dict(status='FAIL', cause=traceback.format_exc()))
        raise


if __name__ == '__main__':
    main()
