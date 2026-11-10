#!/usr/bin/env python3
"""Measure real Root bootstrap under an owned SQLite exclusive lock, without a deadline.

Public init/show only, no provider, policy edits or authority SQL. Baseline/candidate
are explicit provenance labels, never a latency threshold. DELETE is observed, not
forced; another journal mode makes this particular experiment inconclusive.
"""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import time
import traceback


def require(condition, message):
    """Refuse a failed fixture invariant with a static diagnostic, without private values."""
    if not condition:
        raise RuntimeError(message)


def digest(raw):
    """Hash complete evidence bytes; the digest is not actor authentication."""
    return hashlib.sha256(raw).hexdigest()


def publish(path, value):
    """Publish fresh evidence exclusively after adjacent write/fsync; never replace proof.

    A link followed by fsync failure can leave visible evidence; errors propagate.
    Only owned fresh output is accepted; no live-state file is written here.
    """
    raw = value if isinstance(value, bytes) else (json.dumps(value, indent=2) + '\n').encode()
    descriptor, name = tempfile.mkstemp(dir=path.parent, prefix='.bootstrap-proof-')
    errors = []
    stream = None
    directory = None
    try:
        stream = os.fdopen(descriptor, 'wb')
        try:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        except BaseException as error:
            errors.append(error)
        try:
            stream.close()
        except BaseException as error:
            errors.append(error)
        if not errors:
            require(Path(name).read_bytes() == raw, 'evidence temporary bytes differ')
            os.link(name, path)
            directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
            os.fsync(directory)
    except BaseException as error:
        errors.append(error)
        if stream is None:
            try:
                os.close(descriptor)
            except BaseException as close:
                errors.append(close)
    finally:
        if directory is not None:
            try:
                os.close(directory)
            except BaseException as error:
                errors.append(error)
        try:
            Path(name).unlink(missing_ok=True)
        except BaseException as error:
            errors.append(error)
    if errors:
        raise BaseExceptionGroup('evidence publication and cleanup', errors)


def failure_codes(raw):
    """Read exact schema1 CLI typed codes only; malformed/text observations qualify nothing."""
    try:
        value = json.loads(raw)
    except (ValueError, UnicodeError):
        return []
    if not isinstance(value, dict) or type(value.get('schema_version')) is not int or value['schema_version'] != 1:
        return []
    failure = value.get('failure')
    if not isinstance(value.get('error'), str) or not isinstance(failure, dict) or set(failure) != {'code', 'message', 'retryable', 'causes'}:
        return []
    if not isinstance(failure['code'], str) or not isinstance(failure['message'], str) or failure['retryable'] is not False:
        return []
    causes = failure['causes']
    if not isinstance(causes, list) or any(type(code) is not str for code in causes):
        return []
    return causes


def typed_identity_operation(raw):
    """Observe one exact identity-operation label from a valid CLI typed cause vector.

    Only the inspected production enum vocabulary and root-database-identity family
    qualify. Missing, unknown or multiple distinct operations remain unknown; the
    flattened CLI vector cannot attribute a primary versus cleanup branch. This
    observation proves neither handler placement, elapsed time nor acceptance.
    """
    codes = failure_codes(raw)
    prefix = 'database-identity-operation:'
    operations = {code[len(prefix):] for code in codes if code.startswith(prefix)}
    known = {'Prepare', 'Query', 'Diagnostic', 'OpenWitness', 'InspectWitness', 'Finalize', 'Close'}
    if 'root-database-identity' in codes and len(operations) == 1 and operations <= known:
        return next(iter(operations))
    return None


def classify(exit_code, raw):
    """Classify public typed refusal without inferring a hidden handler or a time bound.

    Primary SQLite codes5/6 are protocol constants, not operator policies. Unknown,
    successful or unrelated refusals remain observations, never a fixed-handler PASS.
    Exact operation labels are observed separately when projected; older binaries
    without them remain unknown. No operation is inferred from message or duration.
    """
    codes = failure_codes(raw)
    busy = any(code in codes for code in ('sqlite-code:5', 'sqlite-code:6'))
    if type(exit_code) is int and exit_code == 2 and busy:
        if 'root-database-identity' in codes:
            return 'OBSERVED_CONNECTED_IDENTITY_BUSY'
        if 'root-storage' in codes:
            return 'OBSERVED_LATER_ROOT_STORAGE_BUSY'
    return 'INCONCLUSIVE_OTHER_LOCKED_OUTCOME'


def snapshot(root):
    """Read owned DB/settings bytes and coherent SQL outside the injected lock.

    No PRAGMA setter, DML or policy mutation. Readonly connection explicitly closes;
    its failure remains grouped with a preceding observation failure.
    """
    dbpath = root / '.swarm/state.db'
    database = sqlite3.connect(dbpath.as_uri() + '?mode=ro', uri=True, timeout=0)
    primary = None
    try:
        database.execute('BEGIN')
        mode = database.execute('PRAGMA journal_mode').fetchone()[0]
        require(database.execute('PRAGMA integrity_check').fetchall() == [('ok',)], 'owned DB integrity differs')
        raw = '\n'.join(database.iterdump()).encode()
        return dict(journal_mode=mode, sql_sha256=digest(raw), database_sha256=digest(dbpath.read_bytes()),
                    settings_sha256=digest((root / '.swarm/engine-settings.json').read_bytes()))
    except BaseException as error:
        primary = error
        raise
    finally:
        try:
            database.close()
        except BaseException as error:
            if primary is not None:
                raise BaseExceptionGroup('snapshot and close', [primary, error])
            raise


def invoke(binary, root, output, label, command):
    """Run actual public CLI once with EOF stdin, no watchdog; retain raw channels and argv.

    Failures and partial evidence propagate. Measurement includes process startup,
    bootstrap and reporting, not exclusively SQLite handler time.
    """
    argv = [str(binary), *command, '--root', str(root), '--json']
    publish(output / (label + '.argv.json'), argv)
    start = time.monotonic_ns()
    process = subprocess.run(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    elapsed = time.monotonic_ns() - start
    publish(output / (label + '.stdout'), process.stdout)
    publish(output / (label + '.stderr'), process.stderr)
    result = dict(argv=argv, exit=process.returncode, elapsed_monotonic_ns=elapsed,
                  codes=failure_codes(process.stderr), global_watchdog=None)
    publish(output / (label + '.json'), result)
    return result, process.stdout, process.stderr


def lock_child(path):
    """Hold only this owned existing main DB EXCLUSIVE until release or controller EOF.

    BEGIN EXCLUSIVE contains no DML. READY is emitted only after the actual lock;
    EOF/errors also rollback and close. Unknown control input refuses after cleanup.
    """
    require(path.is_file(), 'lock database absent')
    db = sqlite3.connect(path, timeout=0)
    primary = None
    try:
        mode = db.execute('PRAGMA journal_mode').fetchone()[0]
        require(mode == 'delete', 'lock child journal mode differs')
        db.execute('BEGIN EXCLUSIVE')
        sys.stdout.write(json.dumps(dict(status='READY', pid=os.getpid(), journal_mode=mode, transaction='BEGIN EXCLUSIVE', dml=False)) + '\n')
        sys.stdout.flush()
        command = sys.stdin.readline()
        require(command in ('release\n', ''), 'unknown lock control input')
    except BaseException as error:
        primary = error
    errors = [primary] if primary is not None else []
    for operation in (db.rollback, db.close):
        try:
            operation()
        except BaseException as error:
            errors.append(error)
    if errors:
        raise BaseExceptionGroup('owned lock operation and cleanup', errors)


@contextmanager
def exclusive_lock(root, output):
    """Acquire a real subprocess lock by READY pipe; always release, wait and close.

    Publication/test failures cannot skip controller EOF/reap. Cleanup failures are
    grouped with the primary, never transformed into an unchanged-state claim.
    No PID signal, polling cap or timeout is introduced.
    """
    argv = [sys.executable, str(Path(__file__).resolve()), '--lock-child', str(root / '.swarm/state.db')]
    errors = []
    child = None
    stderr = (output / 'lock-child.stderr').open('xb')
    try:
        try:
            child = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
            raw = child.stdout.readline()
            publish(output / 'lock-child.ready', raw)
            ready = json.loads(raw)
            require(ready == dict(status='READY', pid=child.pid, journal_mode='delete', transaction='BEGIN EXCLUSIVE', dml=False), 'exclusive lock READY differs')
            yield ready
        except BaseException as error:
            errors.append(error)
        finally:
            if child is not None:
                for operation in (lambda: child.stdin.write(b'release\n'), child.stdin.flush, child.stdin.close):
                    try:
                        operation()
                    except BaseException as error:
                        errors.append(error)
                try:
                    code = child.wait()
                    require(code == 0, 'owned lock process failed')
                    publish(output / 'lock-child.exit.json', dict(exit=code, actual_wait=True))
                except BaseException as error:
                    errors.append(error)
                try:
                    child.stdout.close()
                except BaseException as error:
                    errors.append(error)
    finally:
        try:
            stderr.close()
        except BaseException as error:
            errors.append(error)
    if errors:
        raise BaseExceptionGroup('locked observation and release', errors)


def validate_profile(binary, path):
    """Require explicitly qualified normal bytes; no test/fixture profile is accepted."""
    profile = json.loads(path.read_bytes())
    require(profile.get('no_fixture_feature') is True and profile.get('sha256') == digest(binary.read_bytes()), 'normal profile or binary hash differs')
    return profile


def exercise(args):
    """Public witness, locked observation, IPC release, then exact unchanged-state recovery.

    Both roles record measurements without a threshold. DELETE incompatibility is
    INCONCLUSIVE and exits nonzero; baseline/candidate labels do not prove a fix.
    Fixture is retained for parent inspection, never a user-supplied live Root.
    """
    output = args.output.resolve()
    output.mkdir(mode=0o700, exist_ok=False)
    args._owned_output = True
    root = output / 'owned-root'
    root.mkdir(mode=0o700)
    binary, initializer = args.binary.resolve(), args.initializer_binary.resolve()
    sources = {str(Path(__file__).resolve()): digest(Path(__file__).read_bytes())}
    inputs = dict(binary=str(binary), binary_profile=validate_profile(binary, args.profile),
                  initializer=str(initializer), initializer_profile=validate_profile(initializer, args.initializer_profile),
                  role=args.role, root=str(root), sources=sources, global_watchdog=None)
    publish(output / 'inputs.json', inputs)
    first, _, _ = invoke(initializer, root, output, 'initialize', ['init'])
    require(first['exit'] == 0, 'public Root initialization failed')
    witness, raw, _ = invoke(binary, root, output, 'witness', ['engine-settings', 'show'])
    require(witness['exit'] == 0 and isinstance(json.loads(raw).get('values'), dict), 'public settings witness failed')
    before = snapshot(root)
    publish(output / 'snapshot-before.json', before)
    if before['journal_mode'] != 'delete':
        publish(output / 'result.json', dict(status='INCONCLUSIVE_JOURNAL_MODE', observed=before['journal_mode'], role=args.role))
        raise RuntimeError('exclusive-lock journal contract unavailable')
    with exclusive_lock(root, output):
        locked, _, error = invoke(binary, root, output, 'locked', ['engine-settings', 'show'])
        operation = typed_identity_operation(error)
        observation = dict(status=classify(locked['exit'], error), call=locked,
                           identity_operation=operation, identity_operation_scope='flattened-typed-cause-vector',
                           handler_timeout_proven=False, capture_prepare_or_query_proven=operation in ('Prepare', 'Query'))
        publish(output / 'locked-observation.json', observation)
    recovered, recovered_raw, _ = invoke(binary, root, output, 'recovery', ['engine-settings', 'show'])
    require(recovered['exit'] == 0 and json.loads(recovered_raw) == json.loads(raw), 'public settings recovery differs')
    after = snapshot(root)
    publish(output / 'snapshot-after.json', after)
    require(before == after, 'Root bytes settings or SQL changed under read lock probe')
    require(all(digest(Path(path).read_bytes()) == sha for path, sha in sources.items()), 'probe source changed')
    require(digest(binary.read_bytes()) == inputs['binary_profile']['sha256'] and digest(initializer.read_bytes()) == inputs['initializer_profile']['sha256'], 'probe binary changed')
    publish(output / 'result.json', dict(status='PASS_OBSERVATION_AND_RECOVERY', role=args.role,
        locked=observation, state_unchanged=True, policy_unchanged=True, latency_threshold=None,
        handler_zero_proven=False, telemetry_covered=False))
    sys.stdout.write('PASS public bootstrap lock observation retained; unchanged Root and settings reopen\n')
    sys.stdout.flush()


def main():
    """Parse explicit normal baseline/candidate inputs or the isolated internal lock child.

    Errors return nonzero through Python with original/grouped causes; this script
    makes no acceptance, performance or TelemetryStore claim and launches no model.
    """
    if len(sys.argv) == 3 and sys.argv[1] == '--lock-child':
        lock_child(Path(sys.argv[2]))
        return
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'initializer-binary', 'initializer-profile', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--role', choices=('baseline', 'candidate'), required=True)
    args = parser.parse_args()
    try:
        exercise(args)
    except BaseException as primary:
        try:
            if getattr(args, '_owned_output', False):
                publish(args.output / 'failure.json', dict(status='FAIL', traceback=''.join(traceback.format_exception(primary))))
        except BaseException as publication:
            raise BaseExceptionGroup('probe failure and evidence publication', [primary, publication])
        raise


if __name__ == '__main__':
    main()
