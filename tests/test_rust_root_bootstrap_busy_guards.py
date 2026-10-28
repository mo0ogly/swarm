#!/usr/bin/env python3
"""Local guards for busy observation contracts; no engine/provider process is launched."""
import argparse
from contextlib import nullcontext
import io
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import Mock, patch

import rust_root_bootstrap_busy as subject


def diagnostic(codes):
    """Build an actual schema1 typed Failure shape with no private fixture data."""
    return json.dumps(dict(schema_version=1, error='safe', failure=dict(code='root-database-identity',
        message='safe', retryable=False, causes=codes))).encode()


class BootstrapBusyGuards(unittest.TestCase):
    """Witness-first typed observations, own lock release and refusal without timing policy."""

    def test_typed_codes_reject_messages_wrappers_and_malformed(self):
        """A valid vector precedes negatives; arbitrary text is never a SQLite observation."""
        self.assertEqual(subject.failure_codes(diagnostic(['sqlite-code:5'])), ['sqlite-code:5'])
        for document in (b'not json', b'[]', diagnostic([5]), diagnostic(['prefix-sqlite-code:5'])):
            self.assertNotIn('sqlite-code:5', subject.failure_codes(document))
        value = json.loads(diagnostic([]))
        value['error'] = 'sqlite-code:5'
        value['failure']['message'] = 'root-database-identity sqlite-code:5'
        value['failure']['code'] = 'sqlite-code:5'
        self.assertEqual(subject.failure_codes(json.dumps(value).encode()), [])
        value['schema_version'] = True
        self.assertEqual(subject.failure_codes(json.dumps(value).encode()), [])

    def test_typed_identity_operation_tracks_prepare_query_without_timing_claim(self):
        """Exact source labels qualify operation only; old bytes and malformed/ambiguous inputs do not."""
        for operation in ('Prepare', 'Query', 'Diagnostic', 'OpenWitness', 'InspectWitness', 'Finalize', 'Close'):
            raw = diagnostic(['root-database-identity', 'database-identity-operation:' + operation, 'sqlite-code:5'])
            self.assertEqual(subject.typed_identity_operation(raw), operation)
            self.assertEqual(subject.classify(2, raw), 'OBSERVED_CONNECTED_IDENTITY_BUSY')
        for codes in (['root-database-identity', 'sqlite-code:5'],
                      ['root-storage', 'database-identity-operation:Query', 'sqlite-code:5'],
                      ['root-database-identity', 'database-identity-operation:Query', 'database-identity-operation:Close'],
                      ['root-database-identity', 'database-identity-operation:Query', 'database-identity-operation:Unknown'],
                      ['root-database-identity', 'prefix-database-identity-operation:Query'],
                      ['root-database-identity', 'database-identity-operation:Query-suffix'],
                      ['root-database-identity', 'database-identity-operation:Duplicate'],
                      ['root-database-identity', {'code':'database-identity-operation:Query'}]):
            self.assertIsNone(subject.typed_identity_operation(diagnostic(codes)))
        value = json.loads(diagnostic(['root-database-identity']))
        value['failure']['message'] = 'database-identity-operation:Query'
        self.assertIsNone(subject.typed_identity_operation(json.dumps(value).encode()))

    def test_exercise_records_typed_capture_phase_in_actual_result(self):
        """Actual exercise consumer records typed Prepare; no subprocess or latency claim is used."""
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            binary = base / 'binary'
            binary.write_bytes(b'owned inert bytes never executed')
            settings = b'{"values":{"storage_busy_timeout_ms":"0"}}'
            raw = diagnostic(['root-database-identity', 'database-identity-operation:Prepare', 'sqlite-code:5'])
            calls = [(dict(exit=0), b'{}', b''), (dict(exit=0), settings, b''),
                     (dict(exit=2, codes=['root-database-identity', 'database-identity-operation:Prepare', 'sqlite-code:5']), b'', raw),
                     (dict(exit=0), settings, b'')]
            args = argparse.Namespace(output=base / 'output', binary=binary, initializer_binary=binary,
                profile=base / 'profile', initializer_profile=base / 'profile', role='candidate')
            with patch.object(subject, 'validate_profile', return_value=dict(sha256=subject.digest(binary.read_bytes()))), \
                 patch.object(subject, 'invoke', side_effect=calls), \
                 patch.object(subject, 'snapshot', return_value=dict(journal_mode='delete', immutable='same')), \
                 patch.object(subject, 'exclusive_lock', return_value=nullcontext()), \
                 patch.object(subject.sys, 'stdout', io.StringIO()):
                subject.exercise(args)
            result = json.loads((args.output / 'result.json').read_bytes())
            self.assertIs(result['locked']['capture_prepare_or_query_proven'], True)
            self.assertEqual(result['locked']['identity_operation'], 'Prepare')
            self.assertEqual(result['locked']['identity_operation_scope'], 'flattened-typed-cause-vector')
            self.assertIs(result['locked']['handler_timeout_proven'], False)
            self.assertIs(result['handler_zero_proven'], False)
            self.assertIsNone(result['latency_threshold'])

    def test_identity_and_later_storage_are_distinct_not_success_claims(self):
        """Exact root family and busy code classify phase; another error remains inconclusive."""
        self.assertEqual(subject.classify(2, diagnostic(['root-database-identity', 'sqlite-code:5'])), 'OBSERVED_CONNECTED_IDENTITY_BUSY')
        self.assertEqual(subject.classify(2, diagnostic(['root-storage', 'sqlite-code:6'])), 'OBSERVED_LATER_ROOT_STORAGE_BUSY')
        for code, labels in ((0, ['root-database-identity', 'sqlite-code:5']),
                             (True, ['root-database-identity', 'sqlite-code:5']),
                             (2, ['root-database-identity', 'sqlite-code:1032']),
                             (2, ['sqlite-code:5']), (2, ['root-storage', 'prefix-sqlite-code:5'])):
            self.assertEqual(subject.classify(code, diagnostic(labels)), 'INCONCLUSIVE_OTHER_LOCKED_OUTCOME')

    def test_snapshot_preserves_database_and_settings_bytes(self):
        """Readonly inspection after valid setup leaves main DB/settings unchanged."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = root / '.swarm'
            state.mkdir()
            db = sqlite3.connect(state / 'state.db')
            db.execute('CREATE TABLE witness(value TEXT)')
            db.execute('INSERT INTO witness VALUES (?)', ('owned',))
            db.commit()
            db.close()
            (state / 'engine-settings.json').write_bytes(b'{"storage_busy_timeout_ms":"0"}\n')
            before = {p.name:p.read_bytes() for p in state.iterdir()}
            a, b = subject.snapshot(root), subject.snapshot(root)
            self.assertEqual(a, b)
            self.assertEqual(a['journal_mode'], 'delete')
            self.assertEqual(before, {p.name:p.read_bytes() for p in state.iterdir()})

    def test_lock_child_eof_rolls_back_and_closes_without_dml(self):
        """Controller EOF releases the transaction; READY reflects the real lock call order."""
        db = Mock()
        db.execute.return_value.fetchone.return_value = ('delete',)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'state.db'
            path.touch()
            output = io.StringIO()
            with patch.object(subject.sqlite3, 'connect', return_value=db), patch.object(subject.sys, 'stdin', io.StringIO('')), patch.object(subject.sys, 'stdout', output):
                subject.lock_child(path)
            self.assertEqual([call.args[0] for call in db.execute.call_args_list], ['PRAGMA journal_mode', 'BEGIN EXCLUSIVE'])
            self.assertEqual(json.loads(output.getvalue())['status'], 'READY')
            db.rollback.assert_called_once()
            db.close.assert_called_once()

    def test_ready_publication_failure_still_releases_waits_and_closes(self):
        """Mock ENOSPC before yield cannot strand the owned child or hide its primary."""
        child = Mock(pid=123)
        child.stdout.readline.return_value = json.dumps(dict(status='READY', pid=123, journal_mode='delete', transaction='BEGIN EXCLUSIVE', dml=False)).encode()+b'\n'
        child.wait.return_value = 0
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            with patch.object(subject.subprocess, 'Popen', return_value=child), patch.object(subject, 'publish', side_effect=OSError(28, 'owned simulated full')):
                with self.assertRaises(BaseExceptionGroup) as captured:
                    with subject.exclusive_lock(output, output):
                        self.fail('publication failure must precede body')
            self.assertTrue(any(isinstance(e, OSError) and e.errno == 28 for e in captured.exception.exceptions))
            child.stdin.write.assert_called_once_with(b'release\n')
            child.stdin.close.assert_called_once()
            child.wait.assert_called_once_with()
            child.stdout.close.assert_called_once()

    def test_primary_and_release_failures_are_both_retained(self):
        """A body failure survives independent close/flush failure; neither becomes PASS."""
        child = Mock(pid=123)
        child.stdout.readline.return_value = json.dumps(dict(status='READY', pid=123, journal_mode='delete', transaction='BEGIN EXCLUSIVE', dml=False)).encode()+b'\n'
        child.wait.return_value = 0
        child.stdin.flush.side_effect = OSError(5, 'owned simulated flush')
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            with patch.object(subject.subprocess, 'Popen', return_value=child):
                with self.assertRaises(BaseExceptionGroup) as captured:
                    with subject.exclusive_lock(output, output):
                        raise ValueError('primary witness')
            self.assertTrue(any(isinstance(e, ValueError) for e in captured.exception.exceptions))
            self.assertTrue(any(isinstance(e, OSError) for e in captured.exception.exceptions))
            child.wait.assert_called_once_with()

    def test_preexisting_output_is_not_owned_or_overwritten(self):
        """Output collision refuses before init and cannot authorize failure.json publication."""
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            marker = output / 'original'
            marker.write_bytes(b'preserve')
            args = argparse.Namespace(output=output)
            with patch.object(subject.subprocess, 'run') as call:
                with self.assertRaises(FileExistsError):
                    subject.exercise(args)
                call.assert_not_called()
            self.assertFalse(getattr(args, '_owned_output', False))
            self.assertEqual(marker.read_bytes(), b'preserve')
            self.assertFalse((output / 'failure.json').exists())

    def test_publication_keeps_primary_and_directory_close_failure(self):
        """A visible link plus fsync/close errors preserves both causes, not rollback fiction."""
        real_close = subject.os.close
        def close_then_fail(descriptor):
            """Close the owned real descriptor before reporting a simulated independent error."""
            real_close(descriptor)
            raise OSError(5, 'owned close observation')
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'proof'
            with patch.object(subject.os, 'fsync', side_effect=[None, OSError(28, 'owned directory sync')]), patch.object(subject.os, 'close', side_effect=close_then_fail):
                with self.assertRaises(BaseExceptionGroup) as captured:
                    subject.publish(path, b'visible')
            self.assertEqual([error.errno for error in captured.exception.exceptions], [28, 5])
            self.assertEqual(path.read_bytes(), b'visible')

    def test_publication_keeps_primary_and_temporary_unlink_failure(self):
        """Collision preserves original bytes and cleanup failure without hiding FileExistsError."""
        real_unlink = Path.unlink
        def unlink_then_fail(path, **kwargs):
            """Remove only the actual fixture temporary, then simulate the secondary failure."""
            real_unlink(path, **kwargs)
            raise OSError(5, 'owned unlink observation')
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'proof'
            path.write_bytes(b'original')
            with patch.object(Path, 'unlink', unlink_then_fail):
                with self.assertRaises(BaseExceptionGroup) as captured:
                    subject.publish(path, b'other')
            self.assertIsInstance(captured.exception.exceptions[0], FileExistsError)
            self.assertIsInstance(captured.exception.exceptions[1], OSError)
            self.assertEqual(path.read_bytes(), b'original')

    def test_profiles_bind_normal_bytes_without_boolean_coercion(self):
        """Qualified profile true/hash are strict; fixture or numeric marker refuses."""
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / 'binary'
            binary.write_bytes(b'owned bytes, never executed')
            profile = Path(temporary) / 'profile.json'
            good = dict(no_fixture_feature=True, sha256=subject.digest(binary.read_bytes()))
            profile.write_text(json.dumps(good))
            self.assertEqual(subject.validate_profile(binary, profile), good)
            for value in (dict(good, no_fixture_feature=1), dict(good, no_fixture_feature=False), dict(good, sha256='other')):
                profile.write_text(json.dumps(value))
                with self.assertRaises(RuntimeError):
                    subject.validate_profile(binary, profile)


if __name__ == '__main__':
    unittest.main()
