#!/usr/bin/env python3
"""Replay owned material-path refusal/recovery cases on a qualified default core.

Consumes retained native/SDK profile manifests and actual binary bytes; it does
not build, install, accept a caller Root or use paid providers. The SDK creates
its own inactive Roots through public APIs and checks exact state preservation.
No global watchdog is imposed. Failed SDK fixtures are retained by the example;
argv, timestamps and hashes are retained as JSON objects, alongside complete
streams in a fresh output directory.
Profile records are declared evidence, not independent compiler attestation.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess

import rust_agent_model_route as fixture


def require(value, code):
    """Refuse missing/wrong evidence with static diagnostics, without private data."""
    if not value:
        raise RuntimeError(code)


def digest(path):
    """Hash the complete actual input bytes, never trust a supplied filename alone."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(binary, normal_profile, sdk_profile, output):
    """Verify declared profiles then execute the fixed two-case SDK exactly once.

    Requires a fresh output directory and matching ordinary engine manifest.
    Refusals in the cases are positive assertions only when valid controls,
    no-effect observations and restoration/recovery also pass. Nonzero exit or
    absent/unexpected markers fails and preserves all original evidence.
    """
    normal = json.loads(normal_profile.read_bytes())
    sdk = json.loads(sdk_profile.read_bytes())
    require(normal.get('status') == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
            and normal.get('no_fixture_feature') is True
            and normal.get('all_core_features_empty') is True
            and normal.get('all_core_test_false') is True, 'native profile not default core')
    require(sdk.get('status') == 'VERIFIED_SDK_EXAMPLE_DEFAULT_CORE'
            and sdk.get('example') == 'mission_material_path_refusals'
            and sdk.get('engine_features_empty') is True
            and sdk.get('shared_workspace_features_match_normal') is True
            and sdk.get('test_profile') is False
            and sdk.get('normal_profile_sha256') == normal.get('sha256')
            and sdk.get('binary_sha256') == digest(binary), 'material SDK profile or binary mismatch')
    output.mkdir(parents=True, exist_ok=False)
    inputs = {str(p): digest(p) for p in (binary, normal_profile, sdk_profile)}
    argv = [str(binary)]
    fixture.publish(output / 'command.json', dict(
        argv=argv, input_sha256=inputs, global_watchdog=None,
        started_at=datetime.now(timezone.utc).isoformat()))
    with (output / 'stdout').open('xb') as out, (output / 'stderr').open('xb') as err:
        result = subprocess.run(argv, stdin=subprocess.DEVNULL, stdout=out, stderr=err)
    stable = inputs == {str(p): digest(p) for p in (binary, normal_profile, sdk_profile)}
    passed = (result.returncode == 0 and stable
              and (output / 'stdout').read_bytes() == b'CASE writer PASS\nCASE reader PASS\n'
              and not (output / 'stderr').read_bytes())
    fixture.publish(output / 'result.json', dict(
        status='PASS' if passed else 'FAIL', exit_code=result.returncode,
        input_stable=stable, finished_at=datetime.now(timezone.utc).isoformat(),
        cases=2 if passed else None, installed=False, global_acceptance=False))
    require(passed, 'material-path public SDK assertions failed; original evidence retained')
    print('PASS material path writer and reader refusal no-effect recovery', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'normal-profile', 'sdk-profile', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.normal_profile.resolve(), args.sdk_profile.resolve(), args.output.resolve())
