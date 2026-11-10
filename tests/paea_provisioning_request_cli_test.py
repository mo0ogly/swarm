#!/usr/bin/env python3
"""Five actual default-CLI preflight journeys on disposable private /tmp roots.

Required SWARM_PAEA_PROVISIONING_{BINARY,BINARY_SHA256,FIXTURE_HELPER,
FIXTURE_SHA256,EVIDENCE}; fixture helper is independently compiled with explicit
 test-fixtures. It generates only canonical request data, no entropy/material.
No provider, live database, watchdog/retry, human or distinct-account claim.
"""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
PREFIX = 'SWARM_PAEA_PROVISIONING_'
BINARY = Path(os.environ[PREFIX + 'BINARY']).resolve()
BINARY_SHA = os.environ[PREFIX + 'BINARY_SHA256']
HELPER = Path(os.environ[PREFIX + 'FIXTURE_HELPER']).resolve()
HELPER_SHA = os.environ[PREFIX + 'FIXTURE_SHA256']
OUT = Path(os.environ[PREFIX + 'EVIDENCE']).resolve()
OUT.mkdir(parents=True, mode=0o700, exist_ok=False)
SCHEMA = json.loads((ROOT / 'schemas/paea/provisioning-request-cli-v1.schema.json').read_text())
Draft202012Validator.check_schema(SCHEMA)
VALIDATE = Draft202012Validator(SCHEMA)
COUNTER = 0
CASES = []


def digest(raw):
    """Hash synthetic bytes only; a digest learned from data is not authority."""
    return hashlib.sha256(raw).hexdigest()


def file_digest(path):
    """Bind the actual executable bytes, without logging their contents."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def keep(path, raw, replace=False):
    """Atomically retain owned fixture/evidence bytes after fsync/readback.

    Only request mutations may replace existing owned data; original test input
    stays in the caller until recovery. New evidence paths refuse collisions.
    """
    assert replace or not path.exists()
    fd, temporary = tempfile.mkstemp(prefix='.preflight-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        assert Path(temporary).read_bytes() == raw
        os.chmod(temporary, 0o600)
        os.rename(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def run(name, argv, full=False):
    """Run exactly once with hash guards and original argv/stdout/stderr/exit.

    /dev/full is an actual public stdout ENOSPC injection, not a fake success or
    ordinary child timeout. Input request bytes never enter argv or these logs.
    """
    global COUNTER
    COUNTER += 1
    if argv[0] == str(BINARY):
        assert file_digest(BINARY) == BINARY_SHA
    else:
        assert argv[0] == str(HELPER) and file_digest(HELPER) == HELPER_SHA
    if full:
        with open('/dev/full', 'wb', buffering=0) as sink:
            result = subprocess.run(argv, stdout=sink, stderr=subprocess.PIPE)
        result.stdout = b''
    else:
        result = subprocess.run(argv, capture_output=True)
    stem = OUT / f'{COUNTER:03d}-{name}'
    keep(stem.with_suffix('.stdout'), result.stdout)
    keep(stem.with_suffix('.stderr'), result.stderr)
    keep(stem.with_suffix('.json'), json.dumps({'argv': argv, 'exit': result.returncode,
         'stdout_dev_full': full}, indent=2).encode())
    return result


@contextlib.contextmanager
def fixture():
    """Canonical Rust producer before each journey; no service or key created."""
    with tempfile.TemporaryDirectory(prefix='paea-provisioning-native-', dir='/tmp') as directory:
        result = run('request-fixture', [str(HELPER), directory])
        assert result.returncode == 0
        metadata = json.loads(result.stdout)
        assert not metadata['human_verified'] and not metadata['material_generated']
        assert not metadata['separate_os_uid_verified']
        assert not Path(metadata['target']).exists()
        yield metadata


def command(metadata, pin=None, lang='en', extra=(), full=False):
    """Invoke the public closed route without a clock/actor/head/project override."""
    return run('provisioning-check', [str(BINARY), 'identity', 'provisioning-check',
        metadata['request_pin'] if pin is None else pin, '--input', metadata['request_path'],
        '--json', '--lang', lang, *extra], full)


def valid(metadata, lang='en'):
    """Require actual success, closed meaningful facts and unchanged source/target."""
    before = Path(metadata['request_path']).read_bytes()
    result = command(metadata, lang=lang)
    assert result.returncode == 0, result.stderr.decode()
    value = json.loads(result.stdout)
    VALIDATE.validate(value)
    assert value['request_digest'] == metadata['request_pin']
    original = json.loads(before)
    assert value['service_uid'] == original['service_uid']
    assert value['controller_uid'] == original['controller_uid']
    assert value['request_validated'] and value['input_durability'] == 'directory_acknowledged'
    assert Path(metadata['request_path']).read_bytes() == before
    assert not Path(metadata['target']).exists()


def refused(metadata, result):
    """No empty success or secret/path/pin diagnostics and no material target."""
    assert result.returncode != 0 and not result.stdout
    for field in ['request_path', 'request_pin', 'target']:
        assert metadata[field].encode() not in result.stderr
    assert not Path(metadata['target']).exists()


def languages():
    """Real FR/EN check and help expose the route and keep all authority claims false."""
    with fixture() as metadata:
        for lang in ['fr', 'en']:
            valid(metadata, lang)
            result = run('help-' + lang, [str(BINARY), '--help', '--lang', lang])
            assert result.returncode == 0 and b'identity provisioning-check' in result.stdout


def bad_inputs():
    """Valid canonical witness before each pin/role/encoding fault, then exact recovery."""
    with fixture() as metadata:
        path = Path(metadata['request_path'])
        raw = path.read_bytes()
        encode = lambda value: json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode()
        original = json.loads(raw)
        assert encode(original) == raw
        for variant in ['pin', 'trailing', 'service_uid', 'controller_uid', 'unknown', 'null']:
            valid(metadata)
            bad = raw
            if variant == 'trailing':
                bad += b'\n'
            elif variant in ['service_uid', 'controller_uid', 'unknown', 'null']:
                value = json.loads(raw)
                if variant in ['service_uid', 'controller_uid']:
                    value[variant] = (1 << 32) - 1
                elif variant == 'unknown':
                    value['unexpected'] = True
                else:
                    value['schema_version'] = None
                bad = encode(value)
            keep(path, bad, replace=True)
            refused(metadata, command(metadata, pin=('00' * 32 if variant == 'pin' else digest(bad))))
            keep(path, raw, replace=True)
            valid(metadata)


def unsafe_inputs():
    """Actual metadata/link refusals after valid controls, restoration and success."""
    for variant in ['mode', 'parent', 'symlink', 'hardlink', 'basename', 'reserved-name']:
        with fixture() as metadata:
            valid(metadata)
            path = Path(metadata['request_path'])
            held = path.with_suffix('.held')
            original_path = path
            if variant in ['basename', 'reserved-name']:
                if variant == 'reserved-name':
                    (path.parent / 'private-documents.lock').unlink()
                changed = path.with_name('_invalid.json' if variant == 'basename' else 'private-documents.lock')
                path.rename(changed)
                metadata['request_path'] = str(changed)
            elif variant == 'mode':
                path.chmod(0o644)
            elif variant == 'parent':
                path.parent.chmod(0o755)
            elif variant == 'hardlink':
                os.link(path, held)
            else:
                path.rename(held)
                path.symlink_to(held)
            refused(metadata, command(metadata))
            if variant in ['basename', 'reserved-name']:
                Path(metadata['request_path']).rename(original_path)
                metadata['request_path'] = str(original_path)
            elif variant == 'mode':
                path.chmod(0o600)
            elif variant == 'parent':
                path.parent.chmod(0o700)
            elif variant == 'hardlink':
                held.unlink()
            else:
                path.unlink()
                held.rename(path)
            valid(metadata)
            if variant == 'basename':
                print('PASS private-basename-refusal-and-recovery', flush=True)
            elif variant == 'reserved-name':
                print('PASS reserved-lock-basename-refusal-and-recovery', flush=True)


def flags():
    """Default-valued aliases/duplicates refuse before reopening the private input."""
    for extra in [('--root', '.'), ('--phase', 'delivery'), ('--server', 'false'),
                  ('--output', ''), ('--task', ''), ('--json',), ('--lang', 'en'), ('--input', 'duplicate')]:
        with fixture() as metadata:
            valid(metadata)
            lock = Path(metadata['request_path']).parent / 'private-documents.lock'
            assert lock.is_file()
            lock.unlink()  # this fixture's closed lock only, never live state
            refused(metadata, command(metadata, extra=extra))
            assert not lock.exists()
            valid(metadata)


def output_full():
    """Real normal ELF stdout ENOSPC retains validation; no target, explicit recovery."""
    with fixture() as metadata:
        valid(metadata)
        result = command(metadata, full=True)
        refused(metadata, result)
        assert b'provisioning-request-validated:true' in result.stderr
        assert b'provisioning-output-phase:' in result.stderr
        valid(metadata)


def main():
    """Execute only named public assertions and retain actual counts and exact ELF."""
    for name, case in [('fr-en-preflight-and-help', languages),
                       ('canonical-pin-uid-refusal-and-recovery', bad_inputs),
                       ('private-file-link-parent-refusal-and-recovery', unsafe_inputs),
                       ('closed-flags-before-input-io', flags),
                       ('stdout-enospc-keeps-known-validation', output_full)]:
        case()
        CASES.append(name)
        print('PASS', name, flush=True)
    assert file_digest(BINARY) == BINARY_SHA and file_digest(HELPER) == HELPER_SHA
    keep(OUT / 'result.json', json.dumps({'status': 'PASS', 'cases': CASES, 'count': len(CASES),
         'commands': COUNTER, 'binary_sha256': BINARY_SHA, 'fixture_sha256': HELPER_SHA,
         'scope': 'default CLI read/validation only; no material generation/install/UID/human/daemon/E2/E3 acceptance'}, indent=2).encode())
    print('PROVISIONING_CHECK_NATIVE_PASS cases=5', flush=True)


if __name__ == '__main__':
    main()
