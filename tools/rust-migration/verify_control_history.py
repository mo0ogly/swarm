"""Compare the actual Go and Rust historical readers on owned read-only fixtures.

Valid data must match. Corruption/SQL failures deliberately refuse in Rust while
the legacy Go reader hides them. No live root, provider, acceptance or deployment.
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
    work = Path(tempfile.mkdtemp(prefix='control-history-', dir=ROOT / '.cache'))
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
            ROOT, output, env, run, ['./internal/engine'], allow_root=True)
        cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
        run('revision', ['git', 'rev-parse', 'HEAD'])
        run('format', ['cargo', 'fmt', '--manifest-path', 'rust/Cargo.toml', '--all', '--', '--check'])
        run('compile', ['cargo', 'check', *cargo, '--workspace', '--all-features', '--all-targets'])
        run('clippy', ['cargo', 'clippy', *cargo, '--workspace', '--all-features', '--all-targets', '--', '-D', 'warnings'])
        tests = ['cargo', 'test', *cargo, '-p', 'swarm-engine', '--lib', '--test', 'control_history',
                 '--test', 'root_control_history', '--test', 'recorded_event', '--test', 'recorded_work']
        actual_raw = run('native-tests', tests).decode()
        inventory = run('native-inventory', tests + ['--', '--list']).decode()
        actual = Counter(line[5:-7] for line in actual_raw.splitlines()
                         if line.startswith('test ') and line.endswith(' ... ok'))
        listed = Counter(line[:-6] for line in inventory.splitlines() if line.endswith(': test'))
        assert actual and actual == listed, (actual-listed, listed-actual)
        save('native-test-names.json', list(actual.elements()))
        reference = work / 'go-history'
        run('reference-build', ['go', 'test', '-c', '-o', reference, './internal/engine'])
        run('probe-build', ['cargo', 'build', *cargo, '-p', 'swarm-engine', '--features',
                            'test-fixtures', '--example', 'control_history_probe'])
        native = Path(env['CARGO_TARGET_DIR']) / 'debug/examples/control_history_probe'
        fixture = work / 'fixture'
        fixture.mkdir(mode=0o700)
        (fixture / '.rust-migration-fixture').touch()
        env['SWARM_RUST_HISTORY_FIXTURE'] = str(fixture)
        dbpath = fixture / 'history.db'
        results = []

        def prepare(rows, missing=False):
            """Replace only our closed fixture DB; never mutate application state."""
            if dbpath.exists():
                dbpath.unlink()
            db = sqlite3.connect(dbpath)
            try:
                db.execute('CREATE TABLE events(work_id TEXT,kind TEXT,revision INTEGER,payload BLOB)')
                db.executemany('INSERT INTO events VALUES(?,?,?,?)', rows)
                if missing:
                    db.execute('ALTER TABLE events RENAME TO retained_events')
                db.commit()
            finally:
                db.close()

        def compare(label, frame, expected_count, code=None, query_failed=False):
            """Require business content, exact native refusal and byte-preserving reads."""
            data = json.dumps(frame).encode()
            (fixture / 'history-probe.json').write_bytes(data)
            original = dbpath.read_bytes()
            go_raw = run(label+'-go', [reference, '-test.run=^TestRustMigrationControlHistoryProbe$',
                                     '-test.v', '-test.timeout=0']).decode()
            assert '--- PASS: TestRustMigrationControlHistoryProbe' in go_raw and '--- SKIP:' not in go_raw
            marked = [s.removeprefix('RUST_HISTORY_RESULT ') for s in go_raw.splitlines()
                      if s.startswith('RUST_HISTORY_RESULT ')]
            assert len(marked) == 1
            go = json.loads(marked[0])
            assert go['query_failed'] == query_failed
            rust = json.loads(run(label+'-native', [native, fixture], data, 2 if code else 0))
            if code:
                assert rust == dict(code=code, source_present=True), rust
                assert len(go['history']) == expected_count
            else:
                assert go['history'] == rust['history'] and len(rust['history']) == expected_count, (label, go, rust)
            assert original == dbpath.read_bytes(), 'read mutated fixture'
            results.append(dict(name=label, go=go, native=rust, unchanged=True))

        frame = dict(work='work', task=dict(attempts=[dict(id='attempt')]), current=None)
        receipt = dict(attempt_id='attempt', receipt='old', revision=7,
                       controls=[dict(id='control', executed=True, passed=True, exit_code=0,
                                      command=[], wall_duration_ms=12)])

        def row(revision, value, work_id='work', kind='task.auto-validation'):
            """Keep raw receipt bytes separate from the event revision and routing keys."""
            payload = value if isinstance(value, bytes) else json.dumps(value).encode()
            return (work_id, kind, revision, payload)

        base = [row(1, receipt)]
        cases = [('empty', [], frame, 0), ('one', base, frame, 1),
                 ('no-attempt', base, dict(frame, task={}), 0),
                 ('other-work', [row(1, receipt, work_id='other')], frame, 0),
                 ('other-kind', [row(1, receipt, kind='other')], frame, 0),
                 ('foreign-attempt', [row(1, dict(receipt, attempt_id='other'))], frame, 0),
                 ('duplicate', base+[row(2, receipt)], frame, 1),
                 ('current-excluded', base, dict(frame, current=receipt), 0),
                 ('nil-command', [row(1, dict(receipt, controls=[dict(id='nil')]))], frame, 1),
                 ('states-and-measurements', [row(1, dict(receipt, candidate_sha=' sha ', controls=[
                     dict(id='failed', executed=True, command=['cargo', 'test'], exit_code=3, cpu_duration_ms=0),
                     dict(id='started', started_at='start', finished_at='\u2003', exit_code=-1),
                     dict(id='unknown'), dict(id='passed', executed=True, passed=True)]))], frame, 4),
                 ('newest-first', base+[row(2, dict(receipt, receipt='new', controls=[dict(id='new')]))], frame, 2),
                 ('go-case-duplicate-codec', [row(1, b'{"ATTEMPT_ID":"other","attempt_id":"attempt","RECEIPT":"codec","controls":[{"ID":"upper"}]}')], frame, 1)]
        save('valid-vectors.json', [dict(name=n, rows=[dict(work_id=r[0], kind=r[1], revision=r[2], payload_hex=r[3].hex()) for r in rows], frame=f, count=c) for n, rows, f, c in cases])
        for name, rows, supplied, count in cases:
            prepare(base)
            compare(name+'-control', frame, 1)
            prepare(rows)
            compare(name, supplied, count)
            prepare(base)
            compare(name+'-recovery', frame, 1)
        for name, payload in [('syntax', b'{private-invalid'), ('type', b'[]'),
                              ('field-type', b'{"attempt_id":42}'), ('trailing', b'{}{}')]:
            prepare(base)
            compare(name+'-control', frame, 1)
            prepare(base+[row(2, payload)])
            compare(name+'-refusal', frame, 1, code='control-history-format')
            prepare(base)
            compare(name+'-recovery', frame, 1)
        prepare(base)
        compare('missing-table-control', frame, 1)
        prepare(base, missing=True)
        compare('missing-table-refusal', frame, 0, code='control-history-storage', query_failed=True)
        compare('missing-table-no-attempt', dict(frame, task={}), 0, query_failed=True)
        prepare(base)
        compare('missing-table-recovery', frame, 1)
        save('comparisons.json', results)
        run('go-vet', ['go', 'vet', './internal/engine'])
        run('rustdoc', ['cargo', 'doc', *cargo, '-p', 'swarm-engine', '--no-deps'])
        run('doctest', ['cargo', 'test', *cargo, '-p', 'swarm-engine', '--doc'])
        run('methods', ['python3', 'tools/agent-workflows/check.py'])
        run('diff-check', ['git', 'diff', '--check'])
        binaries = {}
        for name, source in [('go-history', reference), ('native-history', native)]:
            target = output / (name+'.binary')
            shutil.copyfile(source, target)
            target.chmod(0o600)
            binaries[name] = hashlib.sha256(target.read_bytes()).hexdigest()
        save('binaries.json', binaries)
        close_graphs()
        after = snapshot()
        assert before == after, 'effective inputs changed during verification'
        save('inputs-after.json', after)
        save('result.json', dict(status='PARTIAL', native_component_status='PASS',
             scope='R5a46 control history, not complete Work CLI projections',
             native_tests=sum(actual.values()), valid_scenarios=len(cases),
             comparisons=len(results), malformed_refusal_recovery=4, sql_refusal_recovery=1,
             fixture_reads_unchanged=True, inputs=len(before), registry_packages=len(registry),
             source_stable=True, global_watchdog=None, provider_calls=False,
             independent_review=False, deployment=False, go_error_silence_fixed=False))
        print('PASS history component; PARTIAL migration', flush=True)
    except BaseException as error:
        save('failure.json', dict(status='FAIL', cause=str(error), type=type(error).__name__))
        raise


if __name__ == '__main__':
    main()
