"""Qualify public native Work JSON listing against the actual Go CLI.

Private initialized roots only; no live service, providers or acceptance.
"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import sys
import tempfile
import time
from collections import Counter
from qualification_inputs import collect
from recorded_process import capture

ROOT = Path(__file__).resolve().parents[2]


def main():
    """Freeze effective inputs; require actual named tests, exact facts and recovery."""
    if not __debug__ or len(sys.argv) != 2:
        raise RuntimeError('active assertions and one fresh evidence directory required')
    output = Path(sys.argv[1]).resolve()
    output.mkdir(parents=True, exist_ok=False)
    work = Path(tempfile.mkdtemp(prefix='work-list-', dir=ROOT / '.cache'))
    env = dict(os.environ)
    commands = []

    def save(name, value):
        """Publish completed records atomically, preserving an existing record on failure."""
        path = output / name
        fd, temporary = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)

    def run(label, args, data=None, expected=0):
        """Stream original outputs with no global watchdog; reject an unexpected exit."""
        args = list(map(str, args))
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        start = time.monotonic()
        result = capture(args, cwd=ROOT, env=env, data=data,
                         stdout_path=output / (label + '.stdout'),
                         stderr_path=output / (label + '.stderr'))
        if data is not None:
            (output / (label + '.stdin')).write_bytes(data)
        commands.append(dict(label=label, command=args, start_utc=started,
                             seconds=time.monotonic()-start, exit=result.returncode,
                             expected_exit=expected))
        save('commands.json', commands)
        print(label, 'exit', result.returncode, flush=True)
        if result.returncode != expected:
            raise RuntimeError(label + ': original failure retained')
        return result.stdout

    try:
        before, snapshot, registry, close_graphs = collect(
            ROOT, output, env, run, ['./cmd/swarm', './internal/engine'], allow_root=True)
        cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
        run('revision', ['git', 'rev-parse', 'HEAD'])
        run('format', ['cargo', 'fmt', '--manifest-path', 'rust/Cargo.toml', '--all', '--', '--check'])
        run('compile', ['cargo', 'check', *cargo, '--workspace', '--all-features', '--all-targets'])
        run('clippy', ['cargo', 'clippy', *cargo, '--workspace', '--all-features', '--all-targets', '--', '-D', 'warnings'])
        tests = ['cargo', 'test', *cargo, '-p', 'swarm-cli', '--tests']
        actual_raw = run('cli-tests', tests).decode()
        inventory = run('cli-inventory', tests + ['--', '--list']).decode()
        actual = Counter(line[5:-7] for line in actual_raw.splitlines()
                         if line.startswith('test ') and line.endswith(' ... ok'))
        listed = Counter(line[:-6] for line in inventory.splitlines() if line.endswith(': test'))
        assert actual and actual == listed, (actual-listed, listed-actual)
        save('cli-test-names.json', list(actual.elements()))
        reference = work / 'go-cli'
        run('go-build', ['go', 'build', '-o', reference, './cmd/swarm'])
        run('cli-build', ['cargo', 'build', *cargo, '-p', 'swarm-cli'])
        run('fixture-probe-build', ['cargo', 'build', *cargo, '-p', 'swarm-engine',
                                  '--features', 'test-fixtures', '--example', 'root_store_probe'])
        target = Path(env['CARGO_TARGET_DIR']) / 'debug'
        native = target / 'swarm-rust'
        probe = target / 'examples/root_store_probe'
        fixture = Path(tempfile.mkdtemp(prefix='.swarm-rust-list-', dir=Path.home()))
        fixture.chmod(0o700)
        (fixture / '.rust-migration-fixture').touch()
        run('native-init', [native, 'init', '--root', fixture, '--json'])
        comparisons = []

        def compare(label, expected):
            """Compare complete actual public lists; verify no DB/policy byte mutation."""
            database = fixture / '.swarm/state.db'
            settings = fixture / '.swarm/engine-settings.json'
            original, policy = database.read_bytes(), settings.read_bytes()
            for language in ['fr', 'en']:
                args = ['work', 'list', '--json', '--lang', language, '--root', fixture]
                go = json.loads(run(label+'-'+language+'-go', [reference, *args]))
                rust = json.loads(run(label+'-'+language+'-native', [native, *args]))
                assert go == rust == expected, (label, go, rust)
                assert database.read_bytes() == original and settings.read_bytes() == policy
                comparisons.append(dict(name=label, language=language, works=len(expected), unchanged=True))
        compare('empty', [])
        expected = []
        for i, title in enumerate(['Premier <&> projet', 'Deuxième \"projet\" \u2028 fin']):
            frame = dict(work='', kind='work.create', request=dict(schema_version=1,
                         event_id='create-'+str(i), expected_revision=0, title=title,
                         objective='Objective', scope='Private fixture', criteria=['Proof']))
            result = json.loads(run('create-'+str(i), [probe, fixture], (json.dumps(frame)+'\n').encode()))
            assert result['code'] == 'ok' and not result['replayed']
            expected.insert(0, result['work'])
            compare('populated-'+str(i), expected)
        save('public-comparisons.json', comparisons)
        run('go-vet', ['go', 'vet', './cmd/swarm', './internal/engine'])
        run('rustdoc', ['cargo', 'doc', *cargo, '-p', 'swarm-cli', '--no-deps'])
        run('methods', ['python3', 'tools/agent-workflows/check.py'])
        run('diff-check', ['git', 'diff', '--check'])
        binaries = {}
        for name, source in [('go-cli', reference), ('native-cli', native), ('root-fixture-probe', probe)]:
            destination = output / (name+'.binary')
            shutil.copyfile(source, destination)
            destination.chmod(0o600)
            binaries[name] = hashlib.sha256(destination.read_bytes()).hexdigest()
        save('binaries.json', binaries)
        close_graphs()
        after = snapshot()
        assert before == after, 'effective inputs changed during verification'
        save('inputs-after.json', after)
        save('result.json', dict(status='PARTIAL', native_component_status='PASS',
             scope='R5b47 public work list --json only; other Work commands pending',
             cli_tests=sum(actual.values()), public_comparisons=len(comparisons),
             database_failure_and_recovery=True, output_error_and_recovery=True,
             inputs=len(before), registry_packages=len(registry), source_stable=True,
             global_watchdog=None, fixture_root=str(fixture), provider_calls=False,
             independent_review=False, deployment=False))
        print('PASS public Work listing; PARTIAL migration', flush=True)
    except BaseException as error:
        save('failure.json', dict(status='FAIL', cause=str(error), type=type(error).__name__))
        raise


if __name__ == '__main__':
    main()
