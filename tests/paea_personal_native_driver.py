#!/usr/bin/env python3
"""Execute one declared, byte-bound personal CLI test driver against a normal ELF.

Required explicit variables with prefix SWARM_PAEA_PERSONAL_:
BINARY, BINARY_SHA256, DRIVER_META and EVIDENCE. Metadata keys binary/sha256/test_source_sha256 bind the
current maintained Rust test source. This fixture never qualifies the driver's
linked library as production, creates a human, or grants a peer. No retry/watchdog.
"""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'rust/crates/swarm-cli/tests/identity_personal.rs'
NAMES = [
    'personal_native_cold_enrollment_exact_once_and_recovery_fr_en',
    'personal_native_bad_frame_and_proof_refuse_without_consuming_then_recover',
    'personal_native_private_key_refusal_precedes_checkpoint_then_recovers',
    'personal_native_stdout_loss_keeps_actual_enrollment_and_safe_diagnostic',
    'personal_native_real_input_io_and_oversize_refuse_before_checkpoint',
    'personal_native_terminal_refuses_without_echo_or_bootstrap_use',
    'personal_native_observation_reports_match_closed_schema',
]


def digest(path):
    """Hash actual bytes through a file, without logging binary or private data."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def keep(out, name, raw):
    """Publish a new owned receipt atomically; failure preserves any original."""
    target = out / name
    assert not target.exists()
    fd, temporary = tempfile.mkstemp(prefix='.personal-receipt-', dir=out)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        assert Path(temporary).read_bytes() == raw
        os.rename(temporary, target)
        directory = os.open(out, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main():
    """Validate binding before launching, retain actual named results and reject
    zero/partial runs. The normal child and frozen driver must stay byte-identical.
    """
    binary = Path(os.environ['SWARM_PAEA_PERSONAL_BINARY']).resolve()
    expected = os.environ['SWARM_PAEA_PERSONAL_BINARY_SHA256']
    metadata = json.loads(Path(os.environ['SWARM_PAEA_PERSONAL_DRIVER_META']).read_text())
    driver = Path(metadata['binary']).resolve()
    if digest(binary) != expected:
        raise ValueError('personal-normal-binary-hash-mismatch')
    if digest(driver) != metadata['sha256']:
        raise ValueError('personal-driver-hash-mismatch')
    if digest(SOURCE) != metadata['test_source_sha256']:
        raise ValueError('personal-driver-maintained-source-mismatch')
    out = Path(os.environ['SWARM_PAEA_PERSONAL_EVIDENCE']).resolve()
    out.mkdir(parents=True, mode=0o700, exist_ok=False)
    argv = [str(driver), '--nocapture']
    environment = dict(os.environ)
    environment['SWARM_PAEA_PERSONAL_BINARY'] = str(binary)
    result = subprocess.run(argv, cwd=ROOT, env=environment, capture_output=True)
    keep(out, 'driver.stdout', result.stdout)
    keep(out, 'driver.stderr', result.stderr)
    keep(out, 'invocation.json', json.dumps({'argv': argv, 'exit': result.returncode,
         'normal_binary_sha256': expected, 'driver_sha256': metadata['sha256'],
         'test_source_sha256': metadata['test_source_sha256']}, indent=2).encode())
    assert digest(binary) == expected and digest(driver) == metadata['sha256']
    assert digest(SOURCE) == metadata['test_source_sha256']
    assert result.returncode == 0, 'personal-driver-test-failure'
    assert b'test result: ok. 7 passed;' in result.stdout, 'personal-driver-count-mismatch'
    for name in NAMES:
        assert (name + ' ... ok').encode() in result.stdout, 'personal-driver-named-test-missing'
        print('PASS', name, flush=True)
    keep(out, 'result.json', json.dumps({'status': 'PASS', 'tests': len(NAMES),
         'normal_binary_sha256': expected, 'driver_sha256': metadata['sha256'],
         'scope': 'Declared test driver and actual normal children only; no distinct UID/human/daemon/global acceptance'}, indent=2).encode())
    print('PERSONAL_NATIVE_DRIVER_PASS tests=7', flush=True)


if __name__ == '__main__':
    main()
