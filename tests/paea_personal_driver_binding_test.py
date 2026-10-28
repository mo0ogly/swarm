#!/usr/bin/env python3
"""Refuse stale personal driver bindings before a child or output directory exists.

Uses the same explicit SWARM_PAEA_PERSONAL_* environment as the maintained
positive seven-test journey. Only temporary synthetic metadata are changed;
actual normal/driver ELF, source and historical receipts stay immutable. No
watchdog/retry, live data, provider or human/OS authority is involved.
"""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    """Hash bytes without disclosing the supplied executable or private input."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    """Check a valid byte-binding witness before each isolated mismatched field.

    The wrapper must refuse with its static cause before making its fresh output
    directory; original metadata and executable/source bytes remain unchanged.
    The separate positive recipe establishes actual named test execution.
    """
    original = Path(os.environ['SWARM_PAEA_PERSONAL_DRIVER_META']).resolve()
    raw = original.read_bytes()
    metadata = json.loads(raw)
    binary = Path(os.environ['SWARM_PAEA_PERSONAL_BINARY']).resolve()
    driver = Path(metadata['binary']).resolve()
    source = ROOT / 'rust/crates/swarm-cli/tests/identity_personal.rs'
    for field, cause in [('normal', 'personal-normal-binary-hash-mismatch'),
                         ('sha256', 'personal-driver-hash-mismatch'),
                         ('test_source_sha256', 'personal-driver-maintained-source-mismatch')]:
        assert digest(binary) == os.environ['SWARM_PAEA_PERSONAL_BINARY_SHA256']
        assert digest(driver) == metadata['sha256']
        assert digest(source) == metadata['test_source_sha256']
        with tempfile.TemporaryDirectory(prefix='paea-driver-binding-', dir='/tmp') as directory:
            parent = Path(directory)
            changed = dict(metadata)
            environment = dict(os.environ)
            wrong = ('0' if metadata.get(field, '').startswith('1') else '1') * 64
            if field == 'normal':
                expected = environment['SWARM_PAEA_PERSONAL_BINARY_SHA256']
                environment['SWARM_PAEA_PERSONAL_BINARY_SHA256'] = ('0' if expected.startswith('1') else '1') * 64
            else:
                changed[field] = wrong
            synthetic = parent / 'metadata.json'
            synthetic.write_text(json.dumps(changed))
            output = parent / 'must-not-exist'
            environment['SWARM_PAEA_PERSONAL_DRIVER_META'] = str(synthetic)
            environment['SWARM_PAEA_PERSONAL_EVIDENCE'] = str(output)
            result = subprocess.run(['python3', str(ROOT / 'tests/paea_personal_native_driver.py')],
                                    cwd=ROOT, env=environment, capture_output=True)
            assert result.returncode != 0 and not result.stdout
            assert cause.encode() in result.stderr and not output.exists()
        assert original.read_bytes() == raw
        assert digest(binary) == os.environ['SWARM_PAEA_PERSONAL_BINARY_SHA256']
        assert digest(driver) == metadata['sha256'] and digest(source) == metadata['test_source_sha256']
        print('PASS binding-refusal-' + field, flush=True)
    print('PERSONAL_DRIVER_BINDING_PASS cases=3', flush=True)


if __name__ == '__main__':
    main()
