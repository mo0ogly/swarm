#!/usr/bin/env python3
"""Admission guard for future Cargo compiler processes, never a cache cleaner.

Cargo passes the real compiler as argv[1]. Queries without an output directory
pass through; compilations check the persisted workstation policy against both
the checkout and output volumes before exec. No running process is interrupted.
This is a preflight check, not a quota or reservation: concurrent writes, build
scripts, rustdoc and a single large compilation can still cross the threshold.
"""
import argparse
import json
import os
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / '.cargo/storage-policy.json'


def load_policy(path=POLICY):
    """Read the sole persisted byte threshold; malformed/missing policy refuses.

    Accept exactly the v1 schema and a positive integer byte count (not bool).
    I/O/JSON/validation errors propagate to the CLI; no implicit fallback exists.
    """
    value = json.loads(Path(path).read_text())
    if (not isinstance(value, dict)
            or set(value) != {'schema_version', 'minimum_free_bytes'}
            or type(value['schema_version']) is not int
            or value['schema_version'] != 1
            or type(value['minimum_free_bytes']) is not int
            or value['minimum_free_bytes'] <= 0):
        raise ValueError('invalid storage-policy schema or minimum_free_bytes')
    return value


def existing_parent(path):
    """Resolve the nearest existing output ancestor without creating directories.

    Paths may be relative to the compiler cwd. Resolution/I/O failures propagate;
    an absent mount cannot be inferred from a pathname alone.
    """
    path = Path(path).resolve()
    while not path.exists():
        path = path.parent
    return path


def check_space(paths, minimum, statvfs=os.statvfs):
    """Return observed available bytes per path, refusing below the byte threshold.

    Uses caller-available f_bavail, not privileged free blocks. Equality passes.
    No disk reservation, deletion or asynchronous monitoring is performed.
    Measurement failures propagate rather than treating unknown space as free.
    """
    result = []
    for path in paths:
        volume = existing_parent(path)
        stats = statvfs(volume)
        available = stats.f_bavail * stats.f_frsize
        if available < minimum:
            raise ValueError(
                f'insufficient disk space at {volume}: available={available} bytes, '
                f'minimum_free_bytes={minimum}; compiler not started; '
                'free unused caches or use a suitable volume, then retry')
        result.append({'path': str(volume), 'available_bytes': available})
    return result


def output_directory(arguments):
    """Extract rustc's output directory, including --out-dir=PATH spelling.

    Return None for metadata/version probes; malformed output arguments refuse.
    Cargo compilation commands use --out-dir; this does not guard arbitrary
    direct rustc invocations using only -o or rustdoc/build-script subprocesses.
    """
    for index, arg in enumerate(arguments):
        if arg == '--out-dir':
            if index + 1 == len(arguments) or not arguments[index + 1]:
                raise ValueError('missing rustc --out-dir value')
            return arguments[index + 1]
        if arg.startswith('--out-dir='):
            if not arg.removeprefix('--out-dir='):
                raise ValueError('empty rustc --out-dir value')
            return arg.removeprefix('--out-dir=')
    return None


def configure(minimum):
    """Atomically replace only a valid existing policy with an explicit byte value.

    Writes, verifies and fsyncs an adjacent temporary file before replacement;
    Errors before replacement preserve the original and propagate. Directory
    sync failure after replacement reports uncertainty, never an inferred rollback.
    This local build policy is separate from Swarm runtime/Admin execution policy.
    """
    load_policy()
    if type(minimum) is not int or minimum <= 0:
        raise ValueError('minimum must be a positive integer byte count')
    raw = (json.dumps({'schema_version': 1, 'minimum_free_bytes': minimum},
                      indent=2) + '\n').encode()
    fd, name = tempfile.mkstemp(dir=POLICY.parent, prefix='.storage-policy-')
    temporary = Path(name)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        if temporary.read_bytes() != raw:
            raise OSError('storage policy publication verification failed')
        temporary.replace(POLICY)
        directory = os.open(POLICY.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        temporary.unlink(missing_ok=True)


def main():
    """Show/configure local policy, or check and exec the Cargo-supplied compiler.

    Refusals return EX_CANTCREAT with a diagnostic on stderr; actual compiler
    status/signals flow directly through exec. No environment/arguments logged.
    """
    try:
        if len(sys.argv) > 1 and sys.argv[1] in ('--status', '--set-minimum-free-bytes'):
            parser = argparse.ArgumentParser(description=__doc__)
            group = parser.add_mutually_exclusive_group(required=True)
            group.add_argument('--status', action='store_true')
            group.add_argument('--set-minimum-free-bytes', type=int)
            args = parser.parse_args()
            if args.set_minimum_free_bytes is not None:
                configure(args.set_minimum_free_bytes)
            policy = load_policy()
            stats = os.statvfs(ROOT)
            print(json.dumps({**policy, 'policy_path': str(POLICY),
                              'root_available_bytes': stats.f_bavail * stats.f_frsize,
                              'scope': 'compiler admission, not a quota'}))
            return 0
        if len(sys.argv) < 2:
            raise ValueError('expected compiler path or --status')
        compiler, arguments = sys.argv[1], sys.argv[2:]
        output = output_directory(arguments)
        if output is not None:
            check_space([ROOT, output], load_policy()['minimum_free_bytes'])
        os.execvp(compiler, [compiler, *arguments])
    except (OSError, ValueError) as error:
        print(f'cargo-space-guard: {error}', file=sys.stderr)
        return os.EX_CANTCREAT


if __name__ == '__main__':
    sys.exit(main())
