#!/usr/bin/env python3
"""Real detached processes and terminal; fake provider, no model/network calls."""
import argparse
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time
import uuid
import fcntl

parser = argparse.ArgumentParser()
parser.add_argument('--binary', required=True)
parser.add_argument('--report', required=True)
args = parser.parse_args()
binary = str(Path(args.binary).resolve())
checks = []

with tempfile.TemporaryDirectory(prefix='swarm-cockpit-smoke-') as directory:
    root = Path(directory)
    def cli(*command, payload=None):
        argv = [binary, '--root', str(root), '--json', *command]
        if payload is not None:
            argv += ['--input', '-']
        result = subprocess.run(argv, input=json.dumps(payload) if payload is not None else None,
                                capture_output=True, text=True, timeout=15)
        if result.returncode:
            raise AssertionError((argv, result.returncode, result.stderr))
        return json.loads(result.stdout) if result.stdout.strip().startswith(('{', '[')) else result.stdout
    def mutation(*command, revision, **data):
        return cli(*command, payload=dict(schema_version=1, event_id=uuid.uuid4().hex,
                                          expected_revision=revision, **data))['work']
    def poll(agent_id, predicate):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            result = cli('agent', 'show', agent_id)
            if predicate(result):
                return result
            time.sleep(.1)
        raise AssertionError(('timeout', result))
    cli('init')
    work = mutation('work', 'create', revision=0, title='Démonstration cockpit', objective='Tester les vrais processus',
                    scope='Fixture locale isolée', criteria=['arrêt confirmé'], next='Lancer la fixture')
    wid = work['id']
    work = mutation('task', 'add', wid, revision=work['revision'], id='demo', title='Processus de démonstration',
                    deliverable='Handoff de fixture', criteria=['log observé'], owner='fixture')
    provider = root / 'provider.py'
    provider.write_text('''#!/usr/bin/python3
import json, sys, time
text=sys.stdin.read()
print(json.dumps({"type":"item.started","item":{"type":"command_execution"}}), flush=True)
print("FIXTURE ONLY — no model called", flush=True)
if "FINISH_QUICKLY" not in text:
    time.sleep(60)
print("Handoff: fixture completed", flush=True)
''')
    provider.chmod(0o700)
    (root / '.swarm/providers.json').write_text(json.dumps({'schema_version': 1, 'providers': {
        'fixture': {'command': str(provider), 'args': [], 'env_allow': []}}}))
    request = dict(schema_version=1, event_id='smoke-first', expected_revision=work['revision'],
                   task_id='demo', provider='fixture', capture_output=True, timeout_seconds=30)
    launched = cli('agent', 'start', wid, payload=request)
    aid = launched['agent']['id']
    running = poll(aid, lambda data: data['agent']['status'] == 'running')
    assert running['agent']['supervisor_pid'] != os.getpid()
    repeat = cli('agent', 'start', wid, payload=request)
    assert not repeat['created']
    checks.append('detached start and idempotent duplicate')
    # PTY console uses the real renderer/keyboard handling, without a browser.
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 140, 0, 0))
    before = termios.tcgetattr(slave)
    console = subprocess.Popen([binary, '--root', str(root), 'console', wid], stdin=slave, stdout=slave, stderr=slave, env={**os.environ, "TERM": "xterm-256color"})
    captured = bytearray()
    def read_until(needle, timeout=6):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if needle in captured:
                return
            ready, _, _ = select.select([master], [], [], .2)
            if ready:
                captured.extend(os.read(master, 65536))
        raise AssertionError(('terminal text missing', needle, bytes(captured[-5000:])))
    read_until(b'swarm>')
    os.write(master, b'\r')
    read_until('Actions — demo'.encode())
    os.write(master, b'\x1b')
    time.sleep(.15)
    os.write(master, b'select smoke-first\r')
    read_until(b'smoke-first')
    os.write(master, b'priority demo 9\r')
    time.sleep(.3)
    os.write(master, b'q\r')
    deadline = time.monotonic() + 8
    while console.poll() is None and time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], .1)
        if ready:
            captured.extend(os.read(master, 65536))
    if console.poll() is None:
        console.terminate()
        console.wait(timeout=5)
        raise AssertionError('console did not exit after q')
    assert termios.tcgetattr(slave) == before, 'terminal not restored'
    os.close(master); os.close(slave)
    assert poll(aid, lambda data: data['agent']['status'] == 'running')
    snapshot = cli('console', wid)
    assert snapshot['priority']['demo'] == 9
    checks.append('interactive PTY, keyboard controls, terminal restoration, agent survives console exit')
    cli('agent', 'stop', aid)
    stopped = poll(aid, lambda data: data['agent']['status'] == 'interrupted')
    assert stopped['agent']['exit_code'] is not None
    logs = cli('agent', 'logs', aid)
    assert any('FIXTURE ONLY' in item['message'] for item in logs)
    assert any('Arrêt demandé' in item['message'] for item in logs)
    checks.append('stop confirmed, exit code and durable provider/command logs')
    # Retry uses an explicit new attempt and checkpoint, never an implicit last conversation.
    # Wait for the task reconciliation written after the process's final state.
    time.sleep(.3)
    retry = cli('control', wid, payload={'command': 'retry smoke-first FINISH_QUICKLY', 'capture_output': True})
    second = poll(retry['selected_agent'], lambda data: data['agent']['status'] == 'completed')
    assert second['agent']['attempt_id'] != stopped['agent']['attempt_id']
    assert second['agent']['previous'] == aid
    checks.append('explicit retry with new attempt and prior session reference')
    time.sleep(.3)
    archive = root / 'history.zip'
    cli('export', wid, '--output', str(archive))
    with tempfile.TemporaryDirectory(prefix='swarm-cockpit-import-') as other:
        subprocess.run([binary, '--root', other, 'init'], check=True, capture_output=True)
        subprocess.run([binary, '--root', other, 'import', '--input', str(archive)], check=True, capture_output=True)
        result = subprocess.run([binary, '--root', other, '--json', 'agent', 'list', wid], check=True, capture_output=True, text=True)
        assert json.loads(result.stdout)['agents'] == []
        result = subprocess.run([binary, '--root', other, '--json', 'agent', 'history', wid], check=True, capture_output=True, text=True)
        assert len(json.loads(result.stdout)['history']['agents']) == 2
    checks.append('export/import keeps history inert; no agent restarted')
    Path(args.report).write_text(json.dumps({'status':'PASS', 'provider':'local fixture, no model call',
        'checks':checks, 'terminal':'40 rows / 140 columns, PTY', 'terminal_excerpt':captured.decode(errors='replace')[-18000:]}, ensure_ascii=False, indent=2))
print(json.dumps({'status': 'PASS', 'checks': checks}, ensure_ascii=False))
