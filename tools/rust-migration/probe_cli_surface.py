"""Verify absent same-name CLI entries against an explicitly hashed frozen binary.

Inputs: inventory JSON, binary path, required SHA-256, expected refusal code and fresh output. Executes only
inventory entries classified absent by source inspection, in an empty temporary
fixture, with French diagnostics and a caller-declared exact refusal code from the actual source contract. No timeout/watchdog, Go or provider call is
added. Records original outputs; nonempty fixture, wrong diagnostic/exit/hash or
empty collection fails. This does not test a valid form of any supported command,
subcommand parity, renamed replacements, or the current checkout build.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    """Run checked refusal cases and atomically retain evidence, including failures.

    IO/process/JSON failures propagate instead of becoming PASS. Binary hash is
    checked before and after; fixture mutations and unexpected output fail the
    result. Temporary fixture cleanup is scoped to the root created here. A sync
    failure after rename is reported, with no promise that rename was undone.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--binary-sha256', required=True)
    parser.add_argument('--expected-code', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError('fresh output required; preserve original failures')
    if not args.expected_code:
        raise ValueError('expected refusal code required')
    inventory = json.loads(args.inventory.read_text())
    binary = args.binary.resolve()
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    if digest != args.binary_sha256:
        raise ValueError('binary hash does not match declared qualified artifact')
    commands = inventory['cli']['same_name_absent']
    if not commands or any(not isinstance(c, str) or c.startswith('-') for c in commands):
        raise ValueError('invalid or empty absent-command collection')
    cases = []
    with tempfile.TemporaryDirectory(prefix='swarm-coverage-cli-') as fixture:
        for command in commands:
            completed = subprocess.run([str(binary), command, '--root', fixture, '--json'], capture_output=True, text=True, env={**os.environ, 'SWARM_LANG': 'fr'})
            payload = json.loads(completed.stderr)
            expected = 'Commande non encore portée dans ce binaire Rust de développement.'
            passed = completed.returncode == 2 and completed.stdout == '' and payload.get('failure', {}).get('code') == args.expected_code and payload.get('error') == expected
            cases.append({'command': command, 'exit_code': completed.returncode, 'stdout': completed.stdout, 'stderr': completed.stderr, 'status': 'PASS' if passed else 'FAIL'})
        unchanged = not list(Path(fixture).iterdir())
    stable = hashlib.sha256(binary.read_bytes()).hexdigest() == digest
    passed = unchanged and stable and all(c['status'] == 'PASS' for c in cases)
    result = {'status': 'PASS' if passed else 'FAIL', 'scope': __doc__, 'binary': str(binary), 'binary_sha256': digest, 'binary_stable': stable, 'inventory_sha256': hashlib.sha256(args.inventory.read_bytes()).hexdigest(), 'recipe_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'empty_fixture_unchanged': unchanged, 'global_watchdog': None, 'expected_refusal_code': args.expected_code, 'cases': cases}
    fd, temporary = tempfile.mkstemp(dir=args.output.parent, prefix='.' + args.output.name + '.')
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(result, stream, ensure_ascii=False, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, args.output)
        directory = os.open(args.output.parent, os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    print(json.dumps({'status': result['status'], 'cases': len(cases), 'fixture_unchanged': unchanged}))
    if not passed:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
