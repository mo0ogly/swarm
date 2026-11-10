#!/usr/bin/env python3
"""Compare retained agent/launch/provider data and SQL lifecycle projection.

Synthetic files and actual Go/native codecs only. No live root, provider, PID
signal, account call, supervision or complete mission acceptance. Fresh output.
"""
import argparse
import json
import os
from pathlib import Path
import shutil

from recorded_process import capture
from review_increment import digest, publish

ROOT = Path(__file__).resolve().parents[2]


def cases():
    """Explicit wire witnesses and refusals, not provider prices or policy defaults."""
    frames = []

    def add(name, kind, raw, expected='ok', status='', desired=''):
        """Retain literal JSON, including duplicate keys that a dict would erase."""
        frames.append(dict(name=name, kind=kind, raw=raw, expected=expected,
                           status=status, desired=desired))

    for kind in ['agent', 'launch', 'providers']:
        add(kind + '-empty', kind, '{}')
        add(kind + '-null', kind, 'null')
        add(kind + '-scalar-witness', kind, '{}')
        add(kind + '-scalar', kind, '1', 'refused')
        add(kind + '-array', kind, '[]', 'refused')
        add(kind + '-unknown', kind, '{"PRIVATE_unknown":true}', 'ok' if kind == 'agent' else 'refused')
    for name, raw, expected in [
        ('explicit-empty', '{"args":[],"env_allow":[]}', 'ok'),
        ('nil-args', '{"args":null}', 'ok'),
        ('fields', '{"id":"session","status":"queued","capture_output":true,"usage":{"input_tokens":3,"provider_reported_cost_usd":0.01}}', 'ok'),
        ('value-struct-omit', '{"diagnostic":{},"limits":{}}', 'ok'),
        ('duplicate-scalar-null', '{"id":"a","id":null}', 'ok'),
        ('duplicate-slice-null', '{"args":["a"],"args":null}', 'ok'),
        ('unicode', '{"prompt":"café <&> \\u2028"}', 'ok'),
        ('folded-field', '{"ID":"session"}', 'ok'),
        ('wrong-pid-witness', '{"child_pid":0}', 'ok'),
        ('wrong-pid', '{"child_pid":"PRIVATE"}', 'refused'),
        ('fractional-pid', '{"child_pid":1.5}', 'refused'),
        ('pid-overflow', '{"child_pid":9223372036854775808}', 'refused'),
    ]:
        add('agent-' + name, 'agent', raw, expected)
    for name, raw, expected in [
        ('explicit', '{"schema_version":1,"event_id":"launch","expected_revision":1,"timeout_seconds":0,"capture_output":true}', 'ok'),
        ('private-recovery', '{"recoveryCategory":"PRIVATE"}', 'refused'),
        ('fractional-timeout', '{"timeout_seconds":1.5}', 'refused'),
        ('capture-type', '{"capture_output":1}', 'refused'),
        ('schema-null', '{"schema_version":1,"schema_version":null}', 'ok'),
        ('nested-unknown', '{"limits":{"PRIVATE_unknown":true}}', 'refused'),
    ]:
        add('launch-' + name, 'launch', raw, expected)
    for name, raw, expected in [
        ('nil-map', '{"schema_version":1,"providers":null}', 'ok'),
        ('empty-map', '{"schema_version":1,"providers":{}}', 'ok'),
        ('configured', '{"schema_version":1,"providers":{"fixture":{"command":"/synthetic","args":[],"env_allow":null,"preflight_required":true}}}', 'ok'),
        ('nested-unknown', '{"providers":{"fixture":{"PRIVATE_unknown":true}}}', 'refused'),
        ('wrong-command', '{"providers":{"fixture":{"command":1}}}', 'refused'),
    ]:
        add('providers-' + name, 'providers', raw, expected)
    for name, raw, status, desired in [
        ('legacy', '{"id":"s","status":"queued"}', 'starting', ''),
        ('legacy-stop', '{"id":"s","status":"queued"}', 'starting', 'stop'),
        ('registered', '{"status":"queued","supervisor_pid":7}', 'starting', ''),
        ('child', '{"status":"queued","child_pid":9}', 'starting', ''),
        ('heartbeat', '{"status":"queued","heartbeat":"2026-10-19T00:00:00Z"}', 'starting', ''),
        ('queued', '{"status":"queued"}', 'queued', ''),
        ('terminal', '{"status":"running","desired":"stop"}', 'completed', ''),
    ]:
        add('row-' + name, 'row', raw, status=status, desired=desired)
    return frames


def verify(output):
    """Build separated production/fixture binaries and compare actual observations.

    Fail nonzero on setup, framing, encoding, missing named Go test, wrong expected
    outcome or unequal full serialized bytes. Preserve originals; no retry/watchdog.
    """
    output.mkdir(mode=0o700)
    env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / 'rust/target/r25'),
               CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0')
    commands = []

    def run(label, argv, data=None, extra=None):
        """Record original command/output; failed setup is never a semantic refusal."""
        argv = list(map(str, argv))
        result = capture(argv, cwd=ROOT, env=dict(env, **(extra or {})), data=data,
                         stdout_path=output / (label + '.stdout'), stderr_path=output / (label + '.stderr'))
        commands.append(dict(label=label, argv=argv, exit=result.returncode))
        publish(output / (label + '.command.json'), commands[-1])
        print(label, result.returncode, flush=True)
        if result.returncode:
            raise RuntimeError(label + ': failed, original outputs retained')
        return result.stdout

    cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
    run('fixture-build', ['cargo', 'build', *cargo, '-p', 'swarm-engine', '--features', 'test-fixtures', '--example', 'agent_runtime_probe'])
    probe = output / 'native-agent-runtime-fixture'
    shutil.copyfile(ROOT / 'rust/target/r25/debug/examples/agent_runtime_probe', probe)
    probe.chmod(0o700)
    run('production-build', ['cargo', 'build', *cargo, '-p', 'swarm-cli', '--bin', 'swarm-rust'])
    production = output / 'native-production-cli'
    shutil.copyfile(ROOT / 'rust/target/r25/debug/swarm-rust', production)
    production.chmod(0o700)
    tree = run('production-features', ['cargo', 'tree', *cargo, '-p', 'swarm-cli', '--edges', 'normal', '--format', '{p} features=[{f}]'])
    if b'test-fixtures' in tree:
        raise ValueError('fixture-feature-in-production')
    go = output / 'go-agent-runtime'
    run('go-build', ['go', 'test', '-c', '-o', go, './internal/engine'])
    frames = cases()
    publish(output / 'cases.json', frames)
    encoded = ['{"kind":' + json.dumps(c['kind']) + ',"body":' + c['raw'] + ',"status":' + json.dumps(c['status']) + ',"desired":' + json.dumps(c['desired']) + '}' for c in frames]
    go_input = output / 'go-input.json'
    observations = output / 'go-observations.json'
    go_input.write_text('{"output":' + json.dumps(str(observations)) + ',"frames":[' + ','.join(encoded) + ']}')
    stdout = run('go-observe', [go, '-test.run', '^TestRustMigrationAgentRuntimeProbe$', '-test.v', '-test.timeout=0'], extra={'SWARM_AGENT_RUNTIME_PROBE': str(go_input)})
    if b'--- PASS: TestRustMigrationAgentRuntimeProbe' not in stdout or b'SKIP' in stdout:
        raise ValueError('actual-go-probe-not-executed')
    native = [json.loads(line) for line in run('native-observe', [probe], ('\n'.join(encoded) + '\n').encode()).splitlines()]
    expected = json.loads(observations.read_bytes())
    if len(native) != len(frames) or len(expected) != len(frames):
        raise ValueError('observation-count-mismatch')
    for case, go_value, value in zip(frames, expected, native):
        if value != go_value or value['code'] != case['expected']:
            raise ValueError('agent-runtime-case-differs: ' + case['name'])
    by_name = dict(zip([c['name'] for c in frames], native))
    if (not by_name['row-legacy']['unregistered_claim'] or by_name['row-registered']['unregistered_claim']
            or 'diagnostic' not in json.loads(by_name['agent-empty']['intent'])):
        raise ValueError('agent-runtime-witness-not-exercised')
    publish(output / 'result.json', dict(status='PASS', count=len(frames), binaries={p.name: digest(p) for p in (go, probe, production)}, commands=commands,
            scope='Synthetic DTO/row lifecycle parity only; no launch, process identity, supervision, billing or complete mission'))
    print('PASS', len(frames), 'actual Go/native agent wire comparisons', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    verify(parser.parse_args().output.resolve())
