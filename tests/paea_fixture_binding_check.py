#!/usr/bin/env python3
"""Check one frozen fixture's semantic role/source/hash before any execution.

Explicit service/provisioning/operator/initial_request role selects a maintained fixture source and
its declared environment namespace. Metadata and ELF bytes prove local fixture
binding only, never production, human authority or confinement. No binary runs.
"""
from pathlib import Path
import hashlib
import json
import os
import sys

ROOT = Path(__file__).resolve().parents[1]
ROLES = {
    'service': ('SWARM_PAEA_SERVICE_', 'service_installation_fixture.rs'),
    'provisioning': ('SWARM_PAEA_PROVISIONING_', 'provisioning_request_fixture.rs'),
    'operator': ('SWARM_PAEA_OPERATOR_', 'operator_package_fixture.rs'),
    'initial_request': ('SWARM_PAEA_INITIAL_REQUEST_', 'initial_request_fixture.rs'),
}


def digest(path):
    """Read exact source/ELF bytes without exposing their contents or executing."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def check(role):
    """Require current source and declared ELF to match independently frozen metadata.

    Static errors disclose no paths, pins or metadata values. Legacy metadata
    may bind the source via source_inventory; current metadata uses the explicit
    test_source_sha256 field. Neither can silently omit the required source.
    """
    if role not in ROLES:
        raise ValueError('fixture-role-required')
    prefix, name = ROLES[role]
    source_name = 'rust/crates/swarm-enrollment-authority/examples/' + name
    source = ROOT / source_name
    metadata = json.loads(Path(os.environ[prefix + 'FIXTURE_META']).read_text())
    binary = Path(os.environ[prefix + 'FIXTURE_HELPER']).resolve()
    if Path(metadata['binary']).resolve() != binary:
        raise ValueError('fixture-binary-selection-mismatch')
    bindings = []
    if 'test_source_sha256' in metadata:
        bindings.append(metadata['test_source_sha256'])
    if source_name in metadata.get('source_inventory', {}):
        bindings.append(metadata['source_inventory'][source_name])
    if not bindings or any(value != digest(source) for value in bindings):
        raise ValueError('fixture-source-mismatch')
    if digest(binary) != metadata['sha256']:
        raise ValueError('fixture-binary-hash-mismatch')
    if prefix + 'FIXTURE_SHA256' in os.environ and os.environ[prefix + 'FIXTURE_SHA256'] != metadata['sha256']:
        raise ValueError('fixture-expected-hash-mismatch')
    return metadata


def main():
    """Validate one explicit role and print a static receipt; never launch a child."""
    if len(sys.argv) != 2:
        raise ValueError('fixture-role-required')
    check(sys.argv[1])
    print('FIXTURE_BINDING_PASS role=' + sys.argv[1], flush=True)


if __name__ == '__main__':
    main()
