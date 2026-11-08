#!/usr/bin/env python3
"""Build and execute the isolated Rust POC-06, retaining actual Cargo artifacts.

Requires a fresh evidence directory and the existing offline Cargo dependencies.
Uses the source-managed build lock without waiting; never changes a live Root,
policy or server. Captures original build/run streams and exact binary identity.
No retry or global watchdog. Source inventory is declared, not a qualification of
all compiler inputs, native agent authorization or production Biba integration.
Failures retain their original artifacts and propagate with a nonzero exit.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import traceback


def digest(path):
    """Hash retained bytes, propagating filesystem errors without a fallback."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    """Atomically retain UTF-8 JSON or bytes beside the destination, with fsync."""
    raw = value if isinstance(value, bytes) else (json.dumps(value, indent=2) + '\n').encode()
    temporary = path.with_name(path.name + '.pending')
    with temporary.open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(path)


def run(argv, output, stem, environment):
    """Execute one explicit command without timeout/retry; retain both streams.

    A failed command raises after saving its real exit and original output.
    Callers must not continue after a failed build prerequisite.
    """
    result = subprocess.run(argv, env=environment, capture_output=True)
    save(output / (stem + '.stdout'), result.stdout)
    save(output / (stem + '.stderr'), result.stderr)
    save(output / (stem + '.command.json'), {'argv': argv, 'exit': result.returncode})
    if result.returncode:
        raise RuntimeError(f'{stem} exited {result.returncode}; original streams retained')
    return result


def sources(root):
    """Inventory Rust source/config bytes for change detection, not a compiler graph."""
    paths = set(root.glob('rust/crates/*/src/**/*.rs'))
    paths.update(root.glob('rust/crates/*/Cargo.toml'))
    for name in ('rust/Cargo.toml', 'rust/Cargo.lock', '.cargo/config.toml',
                 'rust/crates/swarm-engine/examples/poc06_task_mandate.rs'):
        paths.add(root / name)
    return {str(path.relative_to(root)): digest(path) for path in sorted(paths)}


def main():
    """Build default-feature example, execute its scenarios, publish scoped proof."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(exist_ok=False)
    root = Path(__file__).resolve().parents[1]
    if Path.cwd().resolve() != root:
        raise RuntimeError('Run from the repository root')
    try:
        environment = dict(os.environ)
        environment['CARGO_TARGET_DIR'] = str(root / '.cache/rust-build-local')
        with (root / 'deploy/.local-build.lock').open('a+b') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            before = sources(root)
            save(output / 'sources-before.json', before)
            result = run(['cargo', 'build', '--offline', '--locked', '--manifest-path',
                          'rust/Cargo.toml', '-p', 'swarm-engine', '--example',
                          'poc06_task_mandate', '--message-format=json'],
                         output, 'build', environment)
            artifacts = [json.loads(line) for line in result.stdout.splitlines() if line]
            selected = [item for item in artifacts
                        if item.get('reason') == 'compiler-artifact'
                        and item.get('target', {}).get('name') == 'poc06_task_mandate'
                        and item.get('executable')]
            assert len(selected) == 1, 'Expected exactly one example executable'
            artifact = selected[0]
            assert artifact['profile']['test'] is False, 'Unexpected test profile'
            assert 'test-fixtures' not in artifact['features'], 'Fixture feature refused'
            engine = [item for item in artifacts if item.get('reason') == 'compiler-artifact'
                      and item.get('target', {}).get('name') == 'swarm_engine']
            assert engine and all('test-fixtures' not in item['features'] for item in engine)
            binary = Path(artifact['executable'])
            binary_hash = digest(binary)
            execution = run([str(binary)], output, 'journey', environment)
            records = [json.loads(line) for line in execution.stdout.splitlines() if line]
            assert records, 'Empty output is not success'
            # The exact report contract is checked against the implemented
            # producer before qualification; unknown JSON cannot imply PASS.
            summary = records[-1]
            assert summary.get('status') == 'PASS', 'Missing explicit passing summary'
            after = sources(root)
            save(output / 'sources-after.json', after)
            assert before == after, 'Sources changed during build/execution'
            assert digest(binary) == binary_hash, 'Example executable changed during execution'
            save(output / 'result.json', {'status': 'PASS', 'binary': str(binary),
                 'binary_sha256': binary_hash, 'cargo_artifact': artifact,
                 'engine_artifacts': engine, 'records': records,
                 'scope': 'isolated reference PoC; no native agent enforcement',
                 'global_watchdog': None, 'retries': 0})
        print('POC06_TASK_MANDATE_VERIFIED ' + str(output))
    except BaseException:
        save(output / 'failure.txt', traceback.format_exc().encode())
        raise


if __name__ == '__main__':
    main()
