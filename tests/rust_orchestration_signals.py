#!/usr/bin/env python3
"""Follow a real native refusal, root decision, and child resumption on an owned Root."""
import argparse
import json
import subprocess
from pathlib import Path
from urllib.parse import urlencode

import rust_agent_model_route as fixture
import rust_native_http_cli as native
from rust_orchestration_scopes import run_escalation


def verify(binary, output):
    """Correlate the actual engine history with the protected file and browser."""
    output.mkdir(exist_ok=False)
    escalation = output / 'escalation'
    escalation.mkdir()
    run_escalation(binary, escalation)
    identity = json.loads((escalation / 'orchestration-escalation-fixture.json').read_text())
    root = Path(identity['root'])
    # The scenario creates a new Work after bootstrapping its fixture.
    records = (escalation / 'watch' / 'watch.stdout').read_text().splitlines()
    work = json.loads(records[-1])['work']
    shown = native.cli(binary, root, ['work', 'show', work])['work']
    event, = (event for event in shown['planning']['inbox'] if event['kind'] == 'scope_authority_refused')
    server = native.Server(binary, root, output, 'signals')
    failure = None
    try:
        server.capture_async(output / 'frames.jsonl')
        path = '/api/v1/agent-logs/orchestration-signals?' + urlencode({'work': work})
        value = server.request(path, observe=False)
        assert value['revision'] == shown['revision'] and value['work'] == work
        row, = value['signals']
        assert row['kind'] == event['kind'] and row['id'] == event['id']
        assert row['from'] == 'beta' and row['to'] == 'root' and row['at'] == event['at']
        assert row['received']['claim'] and len(row['received']['context_sha256']) == 64
        assert row['processed']['decision'] == event['decision']
        assert row['processed']['operations'] == []
        assert 'BETA_PRIVATE_REFUSAL_ORIGINAL' not in json.dumps(value)
        print('PASS same escalation transmitted by beta, received in root context and processed', flush=True)
        server.request(path, authorized=False, status=403, observe=False)
        server.request(path + '&work=other', status=400, observe=False)
        assert server.request(path, observe=False) == value
        assert native.cli(binary, root, ['work', 'show', work])['work'] == shown
        print('PASS protected read preserves mission revision and decision', flush=True)
        config = dict(address=server.address, key=server.key, work=work, output=str(output), row=row)
        subprocess.run(['node', 'tests/rust_orchestration_signals_ui.cjs'],
                       input=json.dumps(config), text=True, check=True)
        fixture.publish(output / 'result.json', dict(status='PASS', work=work, receipt=row,
            binary_sha256=fixture.digest(binary.read_bytes()), provider='inert'))
    except BaseException as error:
        failure = error
    finally:
        native.finish(server, failure)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    verify(args.binary.resolve(), args.output.resolve())
