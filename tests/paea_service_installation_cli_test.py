#!/usr/bin/env python3
"""Nine real default-CLI installation journeys on disposable /tmp roots.

Required env: SWARM_PAEA_SERVICE_BINARY, SWARM_PAEA_SERVICE_BINARY_SHA256,
SWARM_PAEA_SERVICE_FIXTURE_HELPER and SWARM_PAEA_SERVICE_EVIDENCE. The separately
compiled test-only helper prepares material (and optional process73 crash stages);
only the supplied default CLI performs the public journeys. This does not prove
normal-binary crash injection, human identity, OS separation or confinement.
No global watchdog, provider, network, live datastore or secret stdin logging.
"""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import struct
import subprocess
import tempfile
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ['SWARM_PAEA_SERVICE_BINARY']).resolve()
BINARY_SHA = os.environ['SWARM_PAEA_SERVICE_BINARY_SHA256']
HELPER = Path(os.environ['SWARM_PAEA_SERVICE_FIXTURE_HELPER']).resolve()
OUT = Path(os.environ['SWARM_PAEA_SERVICE_EVIDENCE']).resolve()
OUT.mkdir(parents=True, exist_ok=False)
SCHEMA = json.loads((ROOT / 'schemas/paea/service-installation-cli-v1.schema.json').read_text())
Draft202012Validator.check_schema(SCHEMA)
VALIDATE = Draft202012Validator(SCHEMA)
COUNTER = 0
CASES = []


def digest(raw):
    """Exact bytes digest only, never learn a production pin from received data."""
    return hashlib.sha256(raw).hexdigest()


def atomic_bytes(path, raw, mode=0o600):
    """Replace only owned synthetic fixture/log bytes after fsync/readback.

    A failed temporary write leaves the previous file intact; no live path is
    supplied by this driver. Paths come solely from its own fresh fixture roots.
    """
    fd, temporary = tempfile.mkstemp(prefix='.paea-write-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        assert Path(temporary).read_bytes() == raw
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def run(name, argv, input_bytes=None, full=False):
    """Execute one real child with no watchdog; retain argv/output/exit, not stdin.

    /dev/full injects actual stdout ENOSPC. The supplied normal binary's exact
    hash is checked before each public command. No implicit command retry occurs.
    """
    global COUNTER
    COUNTER += 1
    if argv[0] == str(BINARY):
        assert digest(BINARY.read_bytes()) == BINARY_SHA
    if full:
        with open('/dev/full', 'wb', buffering=0) as sink:
            result = subprocess.run(argv, input=input_bytes, stdout=sink, stderr=subprocess.PIPE)
        result.stdout = b''
    else:
        result = subprocess.run(argv, input=input_bytes, capture_output=True)
    stem = OUT / f'{COUNTER:03d}-{name}'
    atomic_bytes(stem.with_suffix('.stdout'), result.stdout)
    atomic_bytes(stem.with_suffix('.stderr'), result.stderr)
    atomic_bytes(stem.with_suffix('.json'), json.dumps({'argv': argv, 'exit': result.returncode,
                 'stdin_bytes': None if input_bytes is None else len(input_bytes),
                 'stdout_dev_full': full}, indent=2).encode())
    return result


@contextlib.contextmanager
def fixture(mode='prepare'):
    """Private temporary parent passes actual ownership/nofollow checks.

    Metadata is emitted before the helper's intentional process73 exit. The
    helper writes no business seed. Cleanup removes only this driver's own root.
    """
    with tempfile.TemporaryDirectory(prefix='paea-service-cli-', dir='/tmp') as directory:
        parent = Path(directory)
        result = run('fixture-' + mode, [str(HELPER), str(parent / 'service'), str(parent / 'input'), mode])
        assert result.returncode == (0 if mode in ['prepare', 'prepare-control'] else 73), result.stderr.decode()
        metadata = json.loads(result.stdout)
        assert metadata['human_verified'] is False and metadata['separate_os_uid_verified'] is False
        yield metadata


def command(metadata, action='service-install', pin=None, extra=(), full=False):
    """Actual public route; no actor/key/head/root override or plaintext secret argv."""
    return run(action, [str(BINARY), 'identity', action, metadata['artifact_pin'] if pin is None else pin,
               '--input', metadata['artifact_path'], '--json', '--lang', 'en', *extra], full=full)


def report(result, completed=True):
    """Validate meaningful closed report and real success/failure, not exit alone."""
    assert (result.returncode == 0) == completed, result.stderr.decode()
    value = json.loads(result.stdout)
    VALIDATE.validate(value)
    assert value['operation_result'] == ('completed' if completed else 'failed')
    for field in ['human_presence_verified', 'service_started', 'confinement_verified']:
        assert value[field] is False
    return value


def counts(metadata):
    """Read only these isolated real journal fixtures; never a live mission DB."""
    values = []
    for domain in ['business', 'personal']:
        path = Path(metadata['root']) / domain / 'journal.sqlite'
        with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True) as connection:
            values.append(connection.execute('SELECT count(*) FROM enrollment_events').fetchone()[0])
    return tuple(values)


def no_target(metadata):
    """Refusal leaves final/staging target absent; input technical lock is distinct."""
    root = Path(metadata['root'])
    assert not root.exists() and not root.with_name(root.name + '.paea-stage-v1').exists()


def input_control(metadata):
    """Valid same-input witness: reconcile absent target reaches kernel report.

    This proves input decoding succeeded without creating a target before an
    input fault. Its operation report distinguishes it from loader refusal.
    """
    value = report(command(metadata, 'service-reconcile'), completed=False)
    assert value['stage'] == 'not_attempted'
    no_target(metadata)


def private_stderr(result, metadata):
    """Diagnostics expose phases, never these synthetic private paths/pins."""
    for field in ['artifact_path', 'artifact_pin', 'proof_path', 'root']:
        assert metadata[field].encode() not in result.stderr


def cold_lifecycle():
    """Fresh normal CLI publishes, initializes/enrolls once and recovers seq3."""
    with fixture() as m:
        r = report(command(m))
        assert r['publication'] == 'durable' and counts(m) == (1, 1)
        startup = r['startup']
        assert startup['bootstrap_pin'] == m['bootstrap_pin']
        assert startup['bootstrap_path'] == str(Path(m['root']) / 'identity/bootstrap-v3.json')
        args = [startup['bootstrap_pin'], startup['personal_seed_path'], '--input', startup['bootstrap_path'], '--json']
        for action in ['personal-init', 'personal-enroll', 'personal-recover']:
            secret = None
            if action == 'personal-enroll':
                secret = b'swarm-personal-enrollment-v1\n' + Path(m['proof_path']).read_bytes() + b'synthetic-service-password'
            p = run(action, [str(BINARY), 'identity', action, *args], input_bytes=secret)
            assert p.returncode == 0, p.stderr.decode()
            value = json.loads(p.stdout)
            assert value['operation_result'] == 'completed' and value['human_presence_verified'] is False
            if action != 'personal-init':
                assert value['sequence'] == 3
        assert counts(m) == (1, 3)
        p = command(m, 'service-reconcile')
        report(p, completed=False)
        assert counts(m) == (1, 3)


def input_refusals():
    """Bad pin/frame/mode/link/parent refuses, restores exact input and recovers."""
    with fixture() as m:
        path = Path(m['artifact_path'])
        original = path.read_bytes()
        for variant in ['pin', 'frame', 'mode', 'symlink', 'hardlink', 'parent']:
            input_control(m)
            pin = None
            held = path.with_name('held.bin')
            if variant == 'pin':
                pin = '00' * 32
            elif variant == 'frame':
                atomic_bytes(path, b'not-an-artifact')
                pin = digest(b'not-an-artifact')
            elif variant == 'mode':
                path.chmod(0o644)
            elif variant == 'symlink':
                path.rename(held)
                path.symlink_to(held)
            elif variant == 'hardlink':
                os.link(path, held)
            else:
                path.parent.chmod(0o755)
            p = command(m, pin=pin)
            assert p.returncode != 0 and not p.stdout
            private_stderr(p, m)
            no_target(m)
            if variant == 'symlink':
                path.unlink()
                held.rename(path)
            elif variant == 'hardlink':
                held.unlink()
            elif variant == 'parent':
                path.parent.chmod(0o700)
            elif variant == 'mode':
                path.chmod(0o600)
            elif variant == 'frame':
                atomic_bytes(path, original)
            assert path.read_bytes() == original
        report(command(m))
        assert counts(m) == (1, 1)


def closed_options():
    """Forbidden default-valued/duplicate options refuse before input lock IO."""
    for extra in [('--root', '.'), ('--phase', 'delivery'), ('--output', ''), ('--task', ''),
                  ('--server', 'false'), ('--json',), ('--lang', 'fr'), ('--input', 'duplicate.bin')]:
        with fixture() as m:
            p = command(m, extra=extra)
            assert p.returncode != 0 and not p.stdout
            private_stderr(p, m)
            no_target(m)
            assert not (Path(m['artifact_path']).parent / 'private-documents.lock').exists()


def existing_destination():
    """No overwrite/adoption of a preexisting private destination and its bytes."""
    with fixture() as m:
        root = Path(m['root'])
        root.mkdir(mode=0o700)
        atomic_bytes(root / 'keep', b'EXISTING')
        identity = root.stat().st_ino
        value = report(command(m), completed=False)
        assert value['publication'] == 'not_attempted'
        assert root.stat().st_ino == identity and (root / 'keep').read_bytes() == b'EXISTING'
        assert not root.with_name(root.name + '.paea-stage-v1').exists()


def crash_resume():
    """Fixture producer really exits73; normal CLI resumes exact durable inputs."""
    for phase in ['crash-intent', 'crash-business', 'crash-personal', 'crash-visible']:
        with fixture(phase) as m:
            action = 'service-reconcile' if phase == 'crash-visible' else 'service-resume'
            value = report(command(m, action))
            assert value['publication'] == 'durable' and counts(m) == (1, 1)
            value = report(command(m, 'service-reconcile'))
            assert value['business_journal'] == 'observed' and value['personal_journal'] == 'observed'
            assert counts(m) == (1, 1)


def output_enospc():
    """Real stdout ENOSPC after root commit retains known effect then reconciles."""
    with fixture() as m:
        p = command(m, full=True)
        assert p.returncode != 0 and Path(m['root']).is_dir()
        assert b'service-reported-publication:durable' in p.stderr
        private_stderr(p, m)
        assert counts(m) == (1, 1)
        report(command(m, 'service-reconcile'))
        assert counts(m) == (1, 1)


def bad_signature():
    """Consistently pinned opaque corrupt signature reaches real normal journal.

    Python only mutates synthetic framing data; the native journal alone verifies
    cryptography. Assert exact canonical fixture roundtrip before fault injection.
    """
    with fixture() as m:
        input_control(m)
        path = Path(m['artifact_path'])
        raw = path.read_bytes()
        header = b'swarm-service-installation-v1\n'
        assert raw.startswith(header)
        seed = raw[len(header):len(header) + 32]
        offset = len(header) + 32
        pieces = []
        for _ in range(3):
            length = struct.unpack('>Q', raw[offset:offset + 8])[0]
            offset += 8
            pieces.append(raw[offset:offset + length])
            offset += length
        assert offset == len(raw)
        declaration = json.loads(pieces[0])
        encode = lambda v: json.dumps(v, ensure_ascii=False, separators=(',', ':')).encode()
        assert encode(declaration) == pieces[0]
        event = bytearray(pieces[1])
        at = event.index(b'"signature":"') + len(b'"signature":"')
        event[at] = ord('B') if event[at] == ord('A') else ord('A')
        declaration['control']['authority_head']['digest'] = digest(event)
        pieces = [encode(declaration), bytes(event), pieces[2]]
        bad = header + seed + b''.join(struct.pack('>Q', len(p)) + p for p in pieces)
        atomic_bytes(path, bad)
        p = command(m, pin=digest(bad))
        value = report(p, completed=False)
        assert value['business_journal'] == 'unknown' and value['publication'] == 'not_attempted'
        assert not Path(m['root']).exists()
        private_stderr(p, m)


def reserved_uid_definition():
    """Each reserved role refuses canonical pinned input before target effects.

    First observe valid decoding through reconcile on the same protected input.
    Roundtrip the actual private frame before changing only the synthetic UID.
    Restore original bytes/pin and install normally after each refusal. No seed
    or declaration bytes are logged; root UID data grants no OS authority.
    """
    for field in ['service_uid', 'controller_uid']:
        with fixture() as m:
            input_control(m)
            path = Path(m['artifact_path'])
            raw = path.read_bytes()
            header = b'swarm-service-installation-v1\n'
            assert raw.startswith(header)
            offset = len(header) + 32
            prefix = raw[:offset]
            pieces = []
            for _ in range(3):
                length = struct.unpack('>Q', raw[offset:offset + 8])[0]
                offset += 8
                pieces.append(raw[offset:offset + length])
                offset += length
            assert offset == len(raw)
            declaration = json.loads(pieces[0])
            def encode(value):
                """Match canonical UTF-8 fixture JSON; no cryptographic approval."""
                return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode()
            assert encode(declaration) == pieces[0]
            assert prefix + b''.join(struct.pack('>Q', len(p)) + p for p in pieces) == raw
            declaration['control']['installation'][field] = (1 << 32) - 1
            pieces[0] = encode(declaration)
            bad = prefix + b''.join(struct.pack('>Q', len(p)) + p for p in pieces)
            atomic_bytes(path, bad)
            result = command(m, pin=digest(bad))
            assert result.returncode != 0 and not result.stdout
            assert b'service-artifact-document' in result.stderr
            private_stderr(result, m)
            no_target(m)
            atomic_bytes(path, raw)
            report(command(m))
            assert counts(m) == (1, 1)


def help_languages():
    """Both real help languages expose the exact three commands without effects."""
    for language in ['fr', 'en']:
        p = run('help-' + language, [str(BINARY), '--help', '--lang', language])
        assert p.returncode == 0
        for name in ['service-install', 'service-resume', 'service-reconcile']:
            assert name.encode() in p.stdout


def main():
    """Run all nine public installation journeys only on explicit script execution.

    Importing exposes existing fixture/log helpers without launching their cases;
    the module still acquires its fresh evidence output from the explicit env.
    """
    for name, case in [('cold-lifecycle', cold_lifecycle), ('input-refusals-and-recovery', input_refusals),
                       ('closed-options-before-io', closed_options), ('existing-destination', existing_destination),
                       ('actual-fixture-exit73-normal-resume', crash_resume), ('stdout-enospc-known-effect', output_enospc),
                       ('normal-signature-refusal', bad_signature), ('reserved-uid-definition-and-recovery', reserved_uid_definition), ('fr-en-public-help', help_languages)]:
        case()
        CASES.append(name)
        print('PASS', name, flush=True)
    assert digest(BINARY.read_bytes()) == BINARY_SHA
    atomic_bytes(OUT / 'result.json', json.dumps({'status': 'PASS', 'named_cases': CASES, 'count': len(CASES),
                 'commands': COUNTER, 'binary_sha256': BINARY_SHA,
                 'limits': 'fixture producer exits73; no normal-binary crash, human/OS/confine/E2/E3 acceptance'}, indent=2).encode())
    print('SERVICE_INSTALLATION_NATIVE_PASS cases=9', flush=True)


if __name__ == '__main__':
    main()
