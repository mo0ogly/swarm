"""Deterministic process contention for cooperative PAEA laboratory sinks only.

Pause a real SQLite connection after its authority read, while BEGIN IMMEDIATE
still owns the write transaction. The native canonicalizer, sink SQL/Git and
separate oracle remain real. Only scheduling is instrumented in this test helper;
no hook, retry, timeout or protected authority is added to the delivered engine.
"""
from contextlib import contextmanager, redirect_stderr, redirect_stdout
import json
import hashlib
import multiprocessing
from multiprocessing.connection import wait
import os
from pathlib import Path
import sqlite3
import sys


def worker(channel, root, document, operation, pause, early_release, prefix):
    """Run one fixture operation in a fresh process, preserving output and outcome.

    Explicit parent pipe gates the authority read; EOF/abort raises before the
    effect and closes the transaction. A child exit wakes the parent through its
    process sentinel. early_release is an explicit negative-test substitution,
    never an environment-driven sink mode or production feature. No deadlines.
    """
    from tools.paea import pilot_lab as lab
    original_connect = lab.connect

    class HeldConnection:
        """Delegate SQLite calls and pause once after the selected authority read.

        The actual cursor, transaction, lock and database are retained. A rejected
        parent release propagates outside SQLite callbacks, so it cannot be
        swallowed by set_trace_callback. Closure rolls back uncommitted changes.
        """
        def __init__(self, db):
            self.db = db
            self.paused = False

        def execute(self, sql, *args):
            """Execute real SQL, then signal the exact held transaction boundary."""
            cursor = self.db.execute(sql, *args)
            selected = ('SELECT epoch FROM approvals WHERE digest=?' if operation == 'commit'
                        else 'SELECT epoch FROM metadata')
            if pause and not self.paused and sql == selected:
                # Fully consume/close the read cursor. In the negative control,
                # an active SELECT must not accidentally retain a shared lock.
                row = cursor.fetchone()
                cursor.close()
                self.paused = True
                if early_release:
                    # Deliberately reproduce a broken boundary for the recipe guard.
                    self.db.commit()
                channel.send({'event': 'authority-read', 'pid': os.getpid(),
                              'transaction_held': self.db.in_transaction})
                if channel.recv() != 'release':
                    raise RuntimeError('laboratory-test-gate-aborted')
                class ReadResult:
                    """Preserve the one actual fetched row across the scheduling gate."""
                    def fetchone(self):
                        """Return the already observed authority row, without another read."""
                        return row
                return ReadResult()
            return cursor

        def commit(self):
            """Commit the real transaction; preserve all SQLite errors."""
            return self.db.commit()

        def close(self):
            """Close the real connection, rolling back any pending transaction."""
            return self.db.close()

    def connect(path, write=False):
        """Instrument only the worker's real write connection, preserving ro reads."""
        db = original_connect(path, write)
        return HeldConnection(db) if write else db

    lab.connect = connect
    prefix = Path(prefix)
    with Path(str(prefix) + '.stdout').open('x') as out, Path(str(prefix) + '.stderr').open('x') as err:
        with redirect_stdout(out), redirect_stderr(err):
            try:
                if operation == 'commit':
                    sys.argv = ['pilot_lab.py', str(root), str(document)]
                    code = lab.main()
                elif operation == 'revoke':
                    epoch = lab.authority(root, 'revoke')
                    print(json.dumps({'epoch': epoch, 'scope': 'simulated-authority'}), flush=True)
                    code = 0
                else:
                    raise ValueError('laboratory-test-operation-invalid')
            except (OSError, sqlite3.Error, RuntimeError, ValueError, EOFError) as error:
                print(json.dumps({'state': 'UNKNOWN', 'cause': str(error),
                                  'type': type(error).__name__}), file=sys.stderr, flush=True)
                code = 1
            out.flush()
            err.flush()
            channel.send({'event': 'result', 'exit': code})
    channel.close()
    raise SystemExit(code)


def receive(channel, process):
    """Wait for a gate/result or child exit, with no silent pipe-only deadlock.

    Prefer a pending message when both are ready. Unexpected exit/EOF raises;
    the caller retains the original child output and handles fixture cleanup.
    No watchdog, sleep, retry or arbitrary concurrency bound is introduced.
    """
    ready = wait([channel, process.sentinel])
    if channel in ready:
        return channel.recv()
    raise RuntimeError('laboratory-test-child-exited-before-message: ' + str(process.exitcode))


@contextmanager
def operation(case, root, doc, kind, pause=True, early_release=False):
    """Start one owned worker, retaining argv-equivalent inputs, IPC and outputs.

    Yield (process, pipe, prefix) after start. Always abort/reap this child on
    assertion failure, without signalling another process. An unfinished gate is
    aborted rather than released, preserving no-effect failure. Original worker
    output/document hashes and the observed process exit survive failures. No global timeout.
    """
    case_type = type(case)
    case_type.sequence += 1
    prefix = case.out / ('call-%04d' % case.sequence)
    document = Path(str(prefix) + '.document.json')
    document.write_text(json.dumps(doc) + '\n')
    context = multiprocessing.get_context('spawn')
    parent, child = context.Pipe()
    process = context.Process(target=worker, args=(child, root, document, kind, pause, early_release, prefix))
    entry = {'recording_version': 2, 'id': case.sequence, 'fixture_worker': __file__, 'operation': kind,
             'document': str(document), 'pause': pause, 'early_release_fault': early_release,
             'events': []}
    case.calls.append(entry)
    started = False
    try:
        process.start()
        started = True
        entry['pid'] = process.pid
        child.close()
        yield process, parent, prefix, entry
    finally:
        child.close()
        if started:
            # One explicit abort is harmless if the worker already consumed release.
            try:
                parent.send('abort')
            except (BrokenPipeError, EOFError, OSError) as error:
                entry['cleanup_pipe_closed'] = type(error).__name__
            process.join()
            parent.close()
            entry['exit'] = process.exitcode
            process.close()
            entry['artifacts'] = {}
            for suffix in ('.stdout', '.stderr', '.document.json'):
                artifact = Path(str(prefix) + suffix)
                if artifact.is_file():
                    entry['artifacts'][artifact.name] = hashlib.sha256(artifact.read_bytes()).hexdigest()
                else:
                    entry.setdefault('artifact_errors', []).append('missing-' + suffix)
        else:
            parent.close()


def event(case, active, expected):
    """Retain the actual IPC event before asserting its kind; never infer success."""
    process, channel, _, entry = active
    value = receive(channel, process)
    entry['events'].append(value)
    case.assertEqual(value['event'], expected, value)
    return value


def exercise(case):
    """Exercise both lock owners, two domains and explicit recovery after busy.

    A matching unopposed worker control precedes every contention. A commit lock
    blocks revocation until publication; a revoke lock blocks the first commit,
    then the persisted new epoch denies its old grant. SQLite timeout=0 is the
    existing fixture contract: failed peers are not retried automatically.
    Separate oracles assert exact state/effects and SQL ordering. An explicit
    early-release fault must make the same normal assertion fail.
    """
    from tools.paea import pilot_lab as lab
    fault = os.environ.get('SWARM_PAEA_CONTENTION_FAULT', '')
    if fault not in ('', 'early-release'):
        raise ValueError('laboratory-contention-fault-invalid')
    for domain in ('payment', 'git'):
        for owner in ('commit', 'revoke'):
            with case.subTest(domain=domain, contention_owner=owner):
                # Same worker and held phase, no peer: prove the gate itself works.
                control, doc, report, _ = case.prepare(domain)
                with operation(case, control, doc, owner) as valid:
                    gate = event(case, valid, 'authority-read')
                    case.assertIs(gate['transaction_held'], True)
                    valid[1].send('release')
                    case.assertEqual(event(case, valid, 'result')['exit'], 0)
                case.assertEqual(valid[3]['exit'], 0)
                if owner == 'commit':
                    case.assert_effect(case.oracle(control, domain), domain, report)
                else:
                    denied = case.commit(control, doc)
                    case.assertEqual(denied.returncode, 2)
                    case.assertIn(b'laboratory-epoch-revoked', denied.stderr)

                root, doc, report, _ = case.prepare(domain)
                before = case.oracle(root, domain)
                peer_kind = 'revoke' if owner == 'commit' else 'commit'
                with operation(case, root, doc, owner, early_release=fault == 'early-release') as held:
                    gate = event(case, held, 'authority-read')
                    with operation(case, root, doc, peer_kind, pause=False) as peer:
                        result = event(case, peer, 'result')
                        # Assert the effect of the lock, not merely its flag.
                        case.assertEqual(result['exit'], 1, 'contention peer must fail before any competing publication')
                    case.assertEqual(peer[3]['exit'], 1)
                    error = json.loads(Path(str(peer[2]) + '.stderr').read_bytes())
                    case.assertEqual(error['state'], 'UNKNOWN')
                    case.assertIn('locked', error['cause'])
                    case.assertIs(gate['transaction_held'], True)
                    case.assertEqual(case.oracle(root, domain), before)
                    held[1].send('release')
                    case.assertEqual(event(case, held, 'result')['exit'], 0)
                case.assertEqual(held[3]['exit'], 0)
                if owner == 'revoke':
                    case.assertEqual(case.oracle(root, domain), before)
                    denied = case.commit(root, doc)
                    case.assertEqual(denied.returncode, 2)
                    case.assertIn(b'laboratory-epoch-revoked', denied.stderr)
                    case.assertEqual(case.oracle(root, domain), before)
                    case.assertEqual(lab.reconcile(root, report)['state'], 'RECONCILED_NOT_COMMITTED')
                    lab.authority(root, 'approve', report)
                    case.assertEqual(case.commit(root, doc).returncode, 0)
                    expected = [('approve', 1), ('revoke', 2), ('approve', 2), ('commit', 2)]
                else:
                    case.assert_effect(case.oracle(root, domain), domain, report)
                    lab.authority(root, 'revoke')
                    expected = [('approve', 1), ('commit', 1), ('revoke', 2)]
                case.assert_effect(case.oracle(root, domain), domain, report)
                case.assertEqual(lab.reconcile(root, report)['state'], 'RECONCILED_COMMITTED')
                db = lab.connect(root)
                try:
                    case.assertEqual(db.execute('SELECT event,epoch FROM ordering ORDER BY seq').fetchall(), expected)
                finally:
                    db.close()
                print('PAEA_CONTENTION_PASS ' + domain + ' owner=' + owner + ' explicit-recovery', flush=True)
