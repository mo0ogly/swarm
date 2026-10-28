#!/usr/bin/env python3
"""Five actual normal-binary operator journeys, no personal daemon/OS proof.

Use the installation driver's explicit four env inputs and fresh retained output.
Its separately frozen test-only helper prepares control-granted synthetic data;
all actual publication/enrollment/configuration effects use the normal CLI.
No live database/provider/network/watchdog or raw secret stdin log.
"""
import hashlib
import types
import json
from pathlib import Path
import struct
import sys
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
HELPER_SOURCE = ROOT / 'tests/paea_service_installation_cli_test.py'
M = types.ModuleType('installation_driver')
M.__file__ = str(HELPER_SOURCE)
# Execute actual declared source bytes, avoiding a stale imported helper cache.
exec(compile(HELPER_SOURCE.read_bytes(), str(HELPER_SOURCE), 'exec'), M.__dict__)
SCHEMA = json.loads((ROOT / 'schemas/paea/personal-service-response-v1.schema.json').read_text())
Draft202012Validator.check_schema(SCHEMA)
VALIDATE = Draft202012Validator(SCHEMA)
HEADER = b'swarm-personal-service-request-v1\n'
DOMAIN = b'swarm-personal-service-public-request-v1\0'
PASSWORD = b'synthetic\0native-call-password\n'
CHALLENGE = bytes(range(32))  # Explicit fixture correlation, no production entropy claim.
CASES = []


def change(event='native-call-create', head=None):
    """Existing Go-order canonical Change keys, no identity/grant in the request."""
    return {'schema_version': 1, 'event_id': event,
            'expected_head': {'revision': '0', 'digest': ''} if head is None else head,
            'values': {'mode': 'local', 'local_socket_path': '/run/fixture/native-call.sock'},
            'reason': 'explicit synthetic native operator action'}


def packet(startup, operation, action=None, password=PASSWORD, proof=None, pin=None):
    """Independent exact framing; return public binding without hashing secrets."""
    body = b'' if action is None else json.dumps(action, ensure_ascii=False, separators=(',', ':')).encode()
    prefix = HEADER + (startup['bootstrap_pin'] if pin is None else pin).encode() + CHALLENGE + bytes([operation]) + struct.pack('>Q', len(body)) + body
    tail = (proof + password) if operation == 2 else password if operation >= 4 else b''
    return prefix + tail, hashlib.sha256(DOMAIN + prefix).hexdigest()


def argv(startup, extra=()):
    """Only trusted operator startup paths/pin are argv; no proof/password."""
    return [str(M.BINARY), 'identity', 'service-call', startup['bootstrap_pin'],
            startup['personal_seed_path'], '--input', startup['bootstrap_path'],
            '--json', '--lang', 'en', *extra]


def call(startup, operation, action=None, password=PASSWORD, proof=None, full=False):
    """Normal process; validate actual status/opcode/public binding and frame shape.

    Schema checks framing only; concrete history/preview/state assertions follow
    in each journey. Original stdout/stderr/exit retained by the shared driver.
    """
    raw, binding = packet(startup, operation, action, password, proof)
    result = M.run('call-' + str(operation), argv(startup), input_bytes=raw, full=full)
    if full:
        return result, None
    value = json.loads(result.stdout)
    VALIDATE.validate(value)
    assert value['operation'] == operation and value['request_digest'] == binding
    assert (result.returncode == 0) == value['completed'], result.stderr.decode()
    assert PASSWORD not in result.stderr and PASSWORD not in result.stdout
    return result, value


def install(metadata):
    """Public normal-binary install; independently prepared pin must match output."""
    value = M.report(M.command(metadata))
    assert value['publication'] == 'durable' and M.counts(metadata) == (1, 1)
    startup = value['startup']
    assert startup['bootstrap_pin'] == metadata['bootstrap_pin']
    return startup


def enroll(metadata, startup):
    """Actual new checkpoint and proof consumption to personal sequence3."""
    assert call(startup, 1)[1]['checkpoint'] == 'durable'
    proof = Path(metadata['proof_path']).read_bytes()
    assert len(proof) == 32
    value = call(startup, 2, proof=proof)[1]
    assert value['sequence'] == 3 and value['journal'] == 'acknowledged'
    assert M.counts(metadata) == (1, 3)


def witness():
    """Successful same-profile complete call before fault journeys."""
    with M.fixture('prepare-control') as m:
        s = install(m)
        enroll(m, s)
        assert len(call(s, 6, change())[1]['view']['snapshot']['history']) == 1


def all_operations():
    """All7normal operations; preview leaves config bytes intact, replay once."""
    with M.fixture('prepare-control') as m:
        s = install(m)
        enroll(m, s)
        assert call(s, 3)[1]['sequence'] == 3
        created = call(s, 6, change())[1]['view']
        assert len(created['snapshot']['history']) == 1
        replay = call(s, 6, change())[1]['view']
        assert replay == created
        config = Path(m['root']) / 'configuration/config.json'
        before = config.read_bytes()
        following = change('native-call-next', created['head'])
        preview = call(s, 5, following)[1]['projection']
        assert len(preview['proposed']['snapshot']['history']) == 2
        assert preview['current_event_replay'] is False and config.read_bytes() == before
        assert len(call(s, 4)[1]['view']['snapshot']['history']) == 1
        applied = call(s, 7, following)[1]['view']
        assert len(applied['snapshot']['history']) == 2
        assert call(s, 7, following)[1]['view'] == applied
        assert call(s, 3)[1]['sequence'] == 3 and M.counts(m) == (1, 3)


def early_refusals():
    """Target/frame/options and actual PTY refuse before absent startup is opened."""
    witness()
    with M.fixture('prepare-control') as m:
        s = {'bootstrap_pin': m['bootstrap_pin'],
             'bootstrap_path': str(Path(m['root']) / 'identity/bootstrap-v3.json'),
             'personal_seed_path': str(Path(m['root']) / 'identity/personal-authority.seed')}
        raw, _ = packet(s, 1, pin='00' * 32)
        r = M.run('wrong-target-before-startup', argv(s), input_bytes=raw)
        assert r.returncode and not r.stdout and b'service-call-target' in r.stderr
        for raw in [b'', HEADER, packet(s, 1)[0][:-1]]:
            r = M.run('invalid-packet-before-startup', argv(s), input_bytes=raw)
            assert r.returncode and not r.stdout and b'service-request-format' in r.stderr
        for flags in [('--root', '.'), ('--phase', 'delivery'), ('--json',), ('--input', s['bootstrap_path'])]:
            r = M.run('closed-call-options', argv(s, flags), input_bytes=b'')
            assert r.returncode and not r.stdout
        assert M.digest(M.BINARY.read_bytes()) == M.BINARY_SHA
        r = M.run('actual-call-pty', [sys.executable, str(ROOT / 'tests/fixtures/paea_personal_tty.py'), *argv(s)])
        tty = json.loads(r.stdout)
        assert r.returncode == 0 and tty['exit'] and not tty['stdout']
        assert 'service-call-stream-required' in tty['stderr']
        M.no_target(m)


def password_key_and_recovery():
    """Actual wrong password and private signing-key mode refuse; restore succeeds."""
    witness()
    with M.fixture('prepare-control') as m:
        s = install(m)
        enroll(m, s)
        assert call(s, 6, change())[1]['completed']
        config = Path(m['root']) / 'configuration/config.json'
        before = config.read_bytes()
        r, value = call(s, 4, password=b'wrong synthetic password')
        assert r.returncode and value['completed'] is False
        assert value['failure_layers'] == ['personal'] and value['view'] is None
        assert config.read_bytes() == before
        assert call(s, 4)[1]['view']['head']['revision'] == '1'
        key = Path(s['personal_seed_path'])
        key.chmod(0o644)
        raw, _ = packet(s, 4)
        r = M.run('unsafe-signing-key-call', argv(s), input_bytes=raw)
        assert r.returncode and not r.stdout and config.read_bytes() == before
        key.chmod(0o600)
        assert call(s, 4)[1]['view']['head']['revision'] == '1'
        assert call(s, 3)[1]['sequence'] == 3 and M.counts(m) == (1, 3)
        for private in [s['bootstrap_pin'], s['bootstrap_path'], s['personal_seed_path']]:
            assert private.encode() not in r.stderr


def stdout_loss():
    """Actual stdout ENOSPC after new mutation; exact replay never duplicates."""
    witness()
    with M.fixture('prepare-control') as m:
        s = install(m)
        enroll(m, s)
        config = Path(m['root']) / 'configuration/config.json'
        assert not config.exists()
        result, _ = call(s, 6, change(), full=True)
        assert result.returncode and b'completed=true' in result.stderr
        assert b'close=closed' in result.stderr and b'operation=6' in result.stderr
        assert len(json.loads(config.read_bytes())['history']) == 1
        assert len(call(s, 4)[1]['view']['snapshot']['history']) == 1
        assert len(call(s, 6, change())[1]['view']['snapshot']['history']) == 1
        assert M.counts(m) == (1, 3)
        for private in [s['bootstrap_pin'], s['bootstrap_path'], s['personal_seed_path']]:
            assert private.encode() not in result.stderr


def help_languages():
    """Both actual help variants document native call without starting a daemon."""
    for language in ['fr', 'en']:
        r = M.run('call-help-' + language, [str(M.BINARY), '--help', '--lang', language])
        assert r.returncode == 0 and b'identity service-call SHA256' in r.stdout


for name, case in [('all-seven-operations-and-exact-replay', all_operations),
                   ('target-frame-options-pty-before-startup', early_refusals),
                   ('password-private-key-refusal-and-recovery', password_key_and_recovery),
                   ('stdout-enospc-known-mutation', stdout_loss),
                   ('fr-en-service-call-help', help_languages)]:
    case()
    CASES.append(name)
    print('PASS', name, flush=True)
M.atomic_bytes(M.OUT / 'call-result.json', json.dumps({'status': 'PASS', 'cases': CASES,
               'count': len(CASES), 'commands': M.COUNTER, 'binary_sha256': M.BINARY_SHA,
               'scope': 'actual default CLI effects on own fixtures; no daemon/peer/OS/human/E2/E3 proof'}, indent=2).encode())
print('SERVICE_CALL_NATIVE_PASS cases=5', flush=True)
