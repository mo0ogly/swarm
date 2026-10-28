#!/usr/bin/env python3
"""Actual matching fixture bindings, then swapped semantic-role refusal.

Uses explicit SERVICE/PROVISIONING/OPERATOR/INITIAL_REQUEST fixture namespaces. Source and ELF
metadata remain immutable; this check launches the verifier only, never fixtures.
"""
from pathlib import Path
import json
import os
import subprocess
import sys
import itertools

ROOT = Path(__file__).resolve().parents[1]


def verify(role, environment):
    """One guard invocation without watchdog/retry or any fixture execution."""
    return subprocess.run(['python3', str(ROOT / 'tests/paea_fixture_binding_check.py'), role],
                          cwd=ROOT, env=environment, capture_output=True)


def main():
    """A valid same-role witness precedes each swapped-role injection and recovery."""
    original = dict(os.environ)
    roles = sys.argv[1:]
    if len(roles) < 2 or len(set(roles)) != len(roles) or any(role not in ['service', 'provisioning', 'operator', 'initial_request'] for role in roles):
        raise ValueError('fixture-test-roles-required')
    count = 0
    for role, other in itertools.permutations(roles, 2):
        prefix = 'SWARM_PAEA_' + role.upper() + '_'
        opposite = 'SWARM_PAEA_' + other.upper() + '_'
        valid = verify(role, original)
        assert valid.returncode == 0 and ('FIXTURE_BINDING_PASS role=' + role).encode() in valid.stdout
        changed = dict(original)
        changed[prefix + 'FIXTURE_META'] = original[opposite + 'FIXTURE_META']
        changed[prefix + 'FIXTURE_HELPER'] = original[opposite + 'FIXTURE_HELPER']
        bad = verify(role, changed)
        assert bad.returncode != 0 and not bad.stdout and b'fixture-source-mismatch' in bad.stderr
        assert verify(role, original).returncode == 0
        count += 1
        print('PASS swapped-fixture-role-' + role + '-with-' + other, flush=True)
    print('FIXTURE_ROLE_GUARD_PASS cases=' + str(count), flush=True)


if __name__ == '__main__':
    main()
