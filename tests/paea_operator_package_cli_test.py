#!/usr/bin/env python3
"""Six normal-CLI operator/handoff journeys on synthetic private /tmp vaults.

Required SWARM_PAEA_OPERATOR_{BINARY,BINARY_SHA256,FIXTURE_HELPER,FIXTURE_META,
EVIDENCE}; separate frozen operator fixture intentionally stores test seeds/proof.
No live data, provider, watchdog/retry, human identity or actual UID separation.
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
PREFIX = 'SWARM_PAEA_OPERATOR_'
BINARY = Path(os.environ[PREFIX + 'BINARY']).resolve()
BINARY_SHA = os.environ[PREFIX + 'BINARY_SHA256']
HELPER = Path(os.environ[PREFIX + 'FIXTURE_HELPER']).resolve()
META = json.loads(Path(os.environ[PREFIX + 'FIXTURE_META']).read_text())
OUT = Path(os.environ[PREFIX + 'EVIDENCE']).resolve()
OUT.mkdir(parents=True, mode=0o700, exist_ok=False)
SCHEMA = json.loads((ROOT / 'schemas/paea/operator-package-cli-v1.schema.json').read_text())
Draft202012Validator.check_schema(SCHEMA)
VALIDATE = Draft202012Validator(SCHEMA)
COUNTER = 0
CASES = []


def digest(raw):
    """Exact fixture bytes digest only, never derive a trusted pin from received output."""
    return hashlib.sha256(raw).hexdigest()


def file_digest(path):
    """Bind executable bytes without printing their contents or private data."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def keep(path, raw, replace=False):
    """Atomic fsync/readback retention of owned fixture/log data; no live paths."""
    assert replace or not path.exists()
    fd, temporary = tempfile.mkstemp(prefix='.operator-fixture-', dir=path.parent)
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
    """One explicit child, actual hashes and original argv/output/exit; no stdin log.

    /dev/full is a real stdout ENOSPC after artifact publication. The fixture
    helper is separate from the supplied default CLI, never production-qualified.
    """
    global COUNTER
    COUNTER += 1
    if argv[0] == str(BINARY):
        assert file_digest(BINARY) == BINARY_SHA
    else:
        assert argv[0] == str(HELPER) and file_digest(HELPER) == META['sha256']
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
def fixture(mode='correct'):
    """Synthetic key material only in this helper's current-owned /tmp operator vault."""
    with tempfile.TemporaryDirectory(prefix='paea-operator-native-', dir='/tmp') as directory:
        result = run('fixture-' + mode, [str(HELPER), directory, mode])
        assert result.returncode == 0, result.stderr.decode()
        metadata = json.loads(result.stdout)
        assert metadata['fixture_kind'] == 'operator-package-v1'
        assert metadata['synthetic_secrets_written'] is True
        assert not metadata['human_verified'] and not metadata['separate_os_uid_verified']
        assert not Path(metadata['service_target']).exists()
        yield metadata


def command(metadata, operation='operator-check', pin=None, output=None, extra=(), lang='en', full=False):
    """Exact public grammar; only metadata pins/paths in argv, no seed/proof/actor override."""
    argv = [str(BINARY), 'identity', operation,
            metadata['operator_package_pin'] if pin is None else pin]
    if operation != 'operator-check':
        argv.append(metadata['output_path'] if output is None else str(output))
    argv += ['--input', metadata['operator_directory'], '--json', '--lang', lang, *extra]
    return run(operation, argv, full)


def safe(metadata, result):
    """No private fixture paths/pins/raw IO text in ordinary diagnostics or service root."""
    for field in ['operator_directory', 'operator_package_path', 'operator_package_pin',
                  'service_artifact_pin', 'output_path', 'service_target']:
        assert metadata[field].encode() not in result.stderr
    assert not Path(metadata['service_target']).exists()


def report(metadata, result, completed=True):
    """Closed meaningful semantic result, independent from source/target transport phases."""
    assert (result.returncode == 0) == completed, result.stderr.decode()
    value = json.loads(result.stdout)
    VALIDATE.validate(value)
    assert value['operation_result'] == ('completed' if completed else 'failed')
    assert value['operator_package_pin'] == metadata['operator_package_pin']
    assert value['service_artifact_pin'] == metadata['service_artifact_pin']
    assert value['controller_uid_matches_operator'] is True
    safe(metadata, result)
    return value


def input_refused(metadata, result):
    """Source/grammar refusal returns no usable report or artifact and no secret diagnostics."""
    assert result.returncode != 0 and not result.stdout
    safe(metadata, result)
    assert not Path(metadata['output_path']).exists()


def languages():
    """Actual FR/EN observation/help leaves private package untouched and exports nothing."""
    with fixture() as metadata:
        before = file_digest(Path(metadata['operator_package_path']))
        for lang in ['fr', 'en']:
            value = report(metadata, command(metadata, lang=lang))
            assert value['publication'] == 'not_attempted'
            assert not Path(metadata['output_path']).exists()
            assert file_digest(Path(metadata['operator_package_path'])) == before
            help_result = run('help-' + lang, [str(BINARY), '--help', '--lang', lang])
            assert help_result.returncode == 0
            for name in ['operator-check', 'service-export', 'service-export-reconcile']:
                assert name.encode() in help_result.stdout


def export_lifecycle():
    """Exact service-only private output, no-replace refusal and read-only reconciliation.

    Synthetic package raw bytes are read only to compare secret exclusion. They
    never enter logs; Python memory erasure is not claimed for these test copies.
    """
    with fixture() as metadata:
        report(metadata, command(metadata))
        before = file_digest(Path(metadata['operator_package_path']))
        value = report(metadata, command(metadata, 'service-export'))
        assert value['publication'] == 'durable'
        output = Path(metadata['output_path'])
        raw = output.read_bytes()
        assert digest(raw) == metadata['service_artifact_pin']
        assert output.stat().st_mode & 0o7777 == 0o600 and output.stat().st_nlink == 1
        package = Path(metadata['operator_package_path']).read_bytes()
        header = b'swarm-initial-provisioning-v1\n'
        assert package.startswith(header)
        offset = len(header)
        for secret in [package[offset:offset + 32], package[offset + 64:offset + 96]]:
            assert len(secret) == 32 and secret not in raw, 'operator-secret-in-service-artifact'
        report(metadata, command(metadata, 'service-export'), completed=False)
        assert output.read_bytes() == raw
        value = report(metadata, command(metadata, 'service-export-reconcile'))
        assert value['publication'] == 'not_attempted'
        assert value['output_validation'] == 'pin_matched'
        assert value['output_durability'] == 'directory_acknowledged'
        assert output.read_bytes() == raw
        assert file_digest(Path(metadata['operator_package_path'])) == before


def source_refusals():
    """Source witness then pin/mode/link/root/role refusal with restoration and recovery."""
    for variant in ['pin', 'file', 'parent', 'symlink', 'hardlink']:
        with fixture() as metadata:
            report(metadata, command(metadata))
            path = Path(metadata['operator_package_path'])
            before = path.read_bytes()
            held = path.with_suffix('.held')
            if variant == 'file':
                path.chmod(0o644)
            elif variant == 'parent':
                path.parent.chmod(0o755)
            elif variant == 'symlink':
                path.rename(held)
                path.symlink_to(held)
            elif variant == 'hardlink':
                os.link(path, held)
            input_refused(metadata, command(metadata, pin='00' * 32 if variant == 'pin' else None))
            if variant == 'file':
                path.chmod(0o600)
            elif variant == 'parent':
                path.parent.chmod(0o700)
            elif variant == 'hardlink':
                held.unlink()
            elif variant == 'symlink':
                path.unlink()
                held.rename(path)
            assert path.read_bytes() == before
            report(metadata, command(metadata))
    with fixture() as valid:
        report(valid, command(valid))
    with fixture('wrong-controller') as bad:
        result = command(bad)
        input_refused(bad, result)
        assert b'operator-controller-uid' in result.stderr
    with fixture() as recovered:
        report(recovered, command(recovered))


def target_refusals():
    """Valid export/read before mutation, exact input/source remains protected on refusal."""
    for variant in ['content', 'mode', 'parent', 'symlink', 'hardlink', 'basename', 'reserved']:
        with fixture() as metadata:
            report(metadata, command(metadata))
            report(metadata, command(metadata, 'service-export'))
            report(metadata, command(metadata, 'service-export-reconcile'))
            path = Path(metadata['output_path'])
            raw = path.read_bytes()
            held = path.with_suffix('.held')
            selected = path
            if variant == 'content':
                keep(path, b'EXISTING', replace=True)
            elif variant == 'mode':
                path.chmod(0o644)
            elif variant == 'parent':
                path.parent.chmod(0o755)
            elif variant == 'hardlink':
                os.link(path, held)
            elif variant == 'symlink':
                path.rename(held)
                path.symlink_to(held)
            else:
                if variant == 'reserved':
                    (path.parent / 'private-documents.lock').unlink()
                selected = path.with_name('_invalid.bin' if variant == 'basename' else 'private-documents.lock')
                path.rename(selected)
            report(metadata, command(metadata, 'service-export-reconcile', output=selected), completed=False)
            if variant == 'content':
                assert path.read_bytes() == b'EXISTING'
                keep(path, raw, replace=True)
            elif variant == 'mode':
                path.chmod(0o600)
            elif variant == 'parent':
                path.parent.chmod(0o700)
            elif variant == 'hardlink':
                held.unlink()
            elif variant == 'symlink':
                path.unlink()
                held.rename(path)
            else:
                selected.rename(path)
            assert path.read_bytes() == raw
            report(metadata, command(metadata, 'service-export-reconcile'))


def closed_flags():
    """Default aliases and duplicates refuse before source/output lock IO."""
    for operation in ['operator-check', 'service-export', 'service-export-reconcile']:
        for flags in [('--root', '.'), ('--phase', 'delivery'), ('--output', ''),
                      ('--server', 'false'), ('--json',), ('--input', 'duplicate')]:
            with fixture() as metadata:
                report(metadata, command(metadata))
                lock = Path(metadata['operator_directory']) / 'private-documents.lock'
                assert lock.is_file()
                lock.unlink()  # owned synthetic closed lock only
                before = file_digest(Path(metadata['operator_package_path']))
                input_refused(metadata, command(metadata, operation, extra=flags))
                assert not lock.exists()
                assert not (Path(metadata['output_path']).parent / 'private-documents.lock').exists()
                assert file_digest(Path(metadata['operator_package_path'])) == before
                report(metadata, command(metadata))


def stdout_full():
    """Real ENOSPC after publication keeps its effect; independently derived reconcile recovers."""
    with fixture() as metadata:
        report(metadata, command(metadata))
        result = command(metadata, 'service-export', full=True)
        assert result.returncode != 0 and not result.stdout
        safe(metadata, result)
        assert b'operator-output-publication:durable' in result.stderr
        output = Path(metadata['output_path'])
        assert file_digest(output) == metadata['service_artifact_pin']
        value = report(metadata, command(metadata, 'service-export-reconcile'))
        assert value['output_durability'] == 'directory_acknowledged'
        assert file_digest(output) == metadata['service_artifact_pin']


def main():
    """Require each named actual journey; retain real child count and byte-bound default ELF."""
    for name, case in [('fr-en-operator-observation-and-help', languages),
                       ('service-only-export-no-replace-and-reconcile', export_lifecycle),
                       ('source-pin-role-permissions-and-recovery', source_refusals),
                       ('target-pin-name-permissions-and-recovery', target_refusals),
                       ('closed-flags-before-source-io', closed_flags),
                       ('stdout-enospc-retains-artifact-and-reconciles', stdout_full)]:
        case()
        CASES.append(name)
        print('PASS', name, flush=True)
    assert file_digest(BINARY) == BINARY_SHA and file_digest(HELPER) == META['sha256']
    keep(OUT / 'result.json', json.dumps({'status': 'PASS', 'cases': CASES, 'count': len(CASES),
         'commands': COUNTER, 'binary_sha256': BINARY_SHA,
         'scope': 'actual default metadata and private artifact only; no initial package generator, real UID/human/service/admission/E2/E3 acceptance'}, indent=2).encode())
    print('OPERATOR_PACKAGE_NATIVE_PASS cases=6', flush=True)


if __name__ == '__main__':
    main()
