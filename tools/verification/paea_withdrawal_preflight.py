#!/usr/bin/env python3
"""Recheck retained PAEA inputs before an explicitly authorized fixture step.

Read-only by default. A supplied manifest binds a root admission receipt, the
actual compiled composition, retained ELF and original public fixture. Hashes
establish current byte agreement, not trusted authorship or execution authority.
The optional exact-test mode rechecks everything and holds storage admission
through the child. No retries, global watchdog, provider or live server.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

import build_storage
import cargo_space_guard


def require(value, reason):
    """Refuse even under Python -O; diagnostics never include receipt contents."""
    if not value:
        raise ValueError(reason)


def digest(path):
    """Hash full current file bytes; missing/unreadable inputs propagate IO errors."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_json(path):
    """Read strict JSON without accepting duplicate keys or nonfinite numbers."""
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'preflight-duplicate-key')
            result[key] = value
        return result
    return json.loads(Path(path).read_bytes(), object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'preflight-nonfinite'))


def hashes(items, base=None):
    """Check every declared file; relative maps stay inside their bound root."""
    require(isinstance(items, dict) and items, 'preflight-empty-input-map')
    for name, expected in items.items():
        path = Path(name)
        if base is not None:
            require(not path.is_absolute() and '..' not in path.parts,
                    'preflight-relative-path-invalid')
            path = Path(base) / path
            require(path.resolve().is_relative_to(Path(base).resolve()),
                    'preflight-input-escape')
        else:
            require(path.is_absolute(), 'preflight-absolute-path-required')
        require(digest(path) == expected, 'preflight-input-hash-mismatch')


def inspect(manifest):
    """Validate actual bytes and physical fixture, returning exact child context.

    Inputs are the explicit local manifest and retained root-issued admission.
    No files are written. Refusals/IO/JSON errors propagate; missing fields never
    become defaults. This is not same-UID confinement or a full toolchain proof.
    """
    require(manifest['schema_version'] == 1, 'preflight-version')
    hashes(manifest['receipts'])
    receipt = manifest['admission']
    require(digest(receipt['path']) == receipt['sha256'], 'preflight-admission-hash')
    admission = read_json(receipt['path'])
    require(admission['state'] == 'PREP_OUTPUTS_INDIVIDUALLY_ADMITTED_AFTER_CORRECTED_AUDIT',
            'preflight-not-admitted')
    compiled = Path(manifest['compiled'])
    result = read_json(compiled / 'result.json')
    require(result['exit'] == 0 and result['source_stable'] is True
            and result['registry_stable'] is True
            and result['build_finished'] == [{'reason': 'build-finished', 'success': True}],
            'preflight-compile-incomplete')
    freeze = read_json(compiled / 'freeze.json')
    inputs = compiled / 'source-inputs.json'
    require(digest(inputs) == freeze['source_inputs_sha256'] == result['source_inputs_sha256'],
            'preflight-source-manifest-hash')
    hashes(read_json(inputs), freeze['frozen'])
    hashes(read_json(compiled / 'actual-source-dependencies.json'))
    depfiles = read_json(compiled / 'depfile-proof.json')
    require(depfiles['workspace_all_declared'] is True, 'preflight-depfile-unbound')
    hashes(depfiles['depfiles'])
    require(read_json(compiled / 'depfile-missing.json') == []
            and read_json(compiled / 'depfile-unmatched.json') == {}, 'preflight-depfile-gap')
    elf = read_json(compiled / 'retained-elf.json')
    require(digest(inputs) == elf['source_inputs_sha256'], 'preflight-elf-source-binding')
    require(elf == admission['A']['elf'] and elf['features'] == ['test-fixtures']
            and elf['profile']['test'] is True and elf['body_executed'] is False,
            'preflight-elf-profile')
    require(digest(elf['path']) == elf['sha256'], 'preflight-elf-hash')
    fresh = Path(manifest['fresh'])
    qualified = read_json(fresh / 'qualified-fixture.json')
    require(qualified == admission['B']['fixture'], 'preflight-fresh-not-admitted')
    proof = Path(qualified['source_evidence'])
    require(proof == fresh / 'source', 'preflight-fresh-location')
    require(not (proof / 'failure.json').exists()
            and not (proof / 'qualification-failure.json').exists(), 'preflight-fresh-failed')
    terminal = read_json(proof / 'terminal.json')
    require(digest(proof / 'terminal.json') == qualified['terminal_sha256']
            and terminal['status'] == 'QUALIFIED_P4_FRESH_SOURCE'
            and terminal['exit'] == 0, 'preflight-terminal')
    require(digest(proof / 'process.json') == terminal['process_sha256']
            and read_json(proof / 'process.json')['exit'] == 0, 'preflight-child-exit')
    require(digest(proof / 'result.json') == terminal['result_sha256']
            == qualified['result_sha256'], 'preflight-fresh-result-hash')
    public = read_json(proof / 'result.json')
    require(public['status'] == 'PASS' and public['task_accepted'] is True
            and public['paid_provider'] is False, 'preflight-public-incomplete')
    hashes(public['artifacts'], proof)
    root = Path(qualified['root'])
    require(public['root'] == str(root) and root.resolve() == root, 'preflight-root-path')
    for name in public['consumed_originals']:
        require(Path(name).resolve().is_relative_to(root), 'preflight-original-escape')
    hashes(public['consumed_originals'])
    marker = public['review_marker']
    require(Path(marker['path']).resolve().is_relative_to(root), 'preflight-marker-escape')
    require(digest(marker['path']) == marker['sha256'], 'preflight-marker-hash')
    physical = (root / '.swarm').stat()
    require(public['physical'] == qualified['physical']
            == dict(device=physical.st_dev, inode=physical.st_ino), 'preflight-root-identity')
    hashes(read_json(fresh / 'source-inputs.json'), read_json(fresh / 'result.json')['helper_root'])
    require(digest(manifest['default_binary']) == qualified['binary_sha256']
            and digest(manifest['default_profile']) == qualified['profile_sha256'],
            'preflight-default-profile')
    hashes(manifest['current_code'])
    return dict(repository=manifest['repository'], target=freeze['fixture_target'],
                cwd=freeze['frozen'], elf=elf['path'], source=str(proof))


def gated(manifest, submit, *, admission=build_storage.admitted, space=shutil.disk_usage):
    """Call submit once only after all checks, holding/rechecking storage FDs.

    submit(None) is never used: its exact context comes from inspected receipts.
    The target must exist before admission, preventing directory creation here.
    Space uses the persisted storage policy, without a hidden threshold. Original
    operation and cleanup errors are preserved by the existing admission owner.
    Tests may inject an inert admission/submit; no injection is exposed by CLI.
    """
    freeze = read_json(Path(manifest['compiled']) / 'freeze.json')
    target = Path(freeze['fixture_target'])
    require(target.is_dir(), 'preflight-existing-target-required')
    with admission(Path(manifest['repository']), target) as guard:
        guard.check()
        context = inspect(manifest)
        policy_path = Path(context['repository']) / '.cargo/storage-policy.json'
        policy = cargo_space_guard.load_policy(policy_path)
        minimum = policy['minimum_free_bytes']
        require(space(context['repository']).free >= minimum
                and space(context['target']).free >= minimum, 'preflight-space-reserve')
        primary = None
        try:
            result = submit(context)
        except BaseException as error:
            primary = error
        try:
            # Recheck only bound immutable inputs and physical identity. Trust
            # state and the fresh service fixtures may legitimately be mutated
            # by the test; neither is included in these immutable file maps.
            require(inspect(manifest) == context, 'preflight-context-changed')
            require(cargo_space_guard.load_policy(policy_path) == policy,
                    'preflight-storage-policy-changed')
            guard.check()
        except BaseException as secondary:
            if primary is not None:
                raise BaseExceptionGroup('Fixture step and post-verification failed',
                                         [primary, secondary])
            raise
        if primary is not None:
            raise primary
        return result


def main():
    """Read explicit manifest; check only unless separately authorized --execute.

    Execution is the one named ignored integration test, without shell/retry or
    watchdog. It inherits environment except the bound source fixture. The CLI
    does not confer authorization; recipe/root allocation must grant that step.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path)
    parser.add_argument('--manifest-sha256')
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    path = args.manifest or Path(os.environ['SWARM_PAEA_PREFLIGHT_MANIFEST'])
    expected = args.manifest_sha256 or os.environ['SWARM_PAEA_PREFLIGHT_SHA256']
    require(digest(path) == expected, 'preflight-manifest-hash')
    manifest = read_json(path)
    def submit(context):
        if not args.execute:
            print('P4_PREREQUISITES_VERIFIED', flush=True)
            return 0
        command = [context['elf'], '--ignored', '--exact',
                   'concurrent_withdrawal::promote_call_concurrent_withdrawal_waits_for_shared_section',
                   '--nocapture']
        env = dict(os.environ, SWARM_PAEA_SOURCE_FIXTURE=context['source'])
        child = subprocess.run(command, cwd=context['cwd'], env=env,
                               stdin=subprocess.DEVNULL)
        print(json.dumps({'event': 'P4_CHILD_EXIT', 'exit': child.returncode}), flush=True)
        if child.returncode != 0:
            raise subprocess.CalledProcessError(child.returncode, command)
        return child.returncode
    return gated(manifest, submit)


if __name__ == '__main__':
    raise SystemExit(main())
