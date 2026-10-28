#!/usr/bin/env python3
"""Pure decoder, snapshot and control guards; no engine/SDK/provider is launched."""
import argparse
import copy
import io
import json
from pathlib import Path
import sqlite3
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

import rust_mission_legacy_intent as subject


class LegacyIntentGuards(unittest.TestCase):
    """Historical absence, exact refusal, owned cleanup and genuine-witness ordering."""

    @staticmethod
    def ready(schema=1):
        """Build only historical decoder inputs, not executable provenance or a grant."""
        binding = dict(schema_version=schema)
        if schema == 2:
            binding['parent_profile_sha256'] = 'a' * 64
        return dict(fixture_schema_version=2, ready=True, agent='agent', binding=binding)

    def test_historical_bindings_reject_new_identity_fifo_null_and_numeric_schema(self):
        """Both old shapes pass first; newly present metadata and cross-version coercion refuse."""
        for schema in (1, 2):
            state = self.ready(schema)
            subject.assert_binding(state, schema)
            for key, value in (('root_instance', None), ('root_instance', {}), ('workspace_turn_seq', None)):
                bad = copy.deepcopy(state)
                bad['binding'][key] = value
                with self.assertRaises((RuntimeError, AssertionError)):
                    subject.assert_binding(bad, schema)
            for value in (True, float(schema), 4):
                bad = copy.deepcopy(state)
                bad['binding']['schema_version'] = value
                with self.assertRaises((RuntimeError, AssertionError)):
                    subject.assert_binding(bad, schema)
        state = self.ready(2)
        state['binding']['parent_profile_sha256'] = 'A' * 64
        with self.assertRaises((RuntimeError, AssertionError)):
            subject.assert_binding(state, 2)

    def test_json_comparison_preserves_boolean_and_blob_types(self):
        """Protocol true is not integer1; SQLite blob/text representation remains distinct."""
        self.assertTrue(subject.json_equal(dict(valid=True), dict(valid=True)))
        self.assertFalse(subject.json_equal(dict(valid=True), dict(valid=1)))
        self.assertNotEqual(subject.sql_cell(b'root'), subject.sql_cell('root'))

    def test_optimized_python_refuses_before_historical_assert_helpers(self):
        """The baseline runtime passes; -O/-OO disables old asserts and must refuse before effects."""
        self.assertEqual(subject.python_runtime()['optimize'], 0)
        for value in (1, 2):
            with patch.object(subject.sys, 'flags', types.SimpleNamespace(optimize=value)), self.assertRaises(RuntimeError):
                subject.python_runtime()
        request = Path('/nonexistent-owned-request')
        with patch.object(subject.sys, 'flags', types.SimpleNamespace(optimize=1)), \
             patch.object(subject.importlib, 'import_module') as imports:
            with self.assertRaises(RuntimeError):
                subject.prepare_old_child(request)
            imports.assert_not_called()

    def test_business_snapshot_masks_only_owned_pulse_fields(self):
        """Actual readonly SQLite witness permits pulses but preserves stop/body/other Work."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / '.swarm').mkdir()
            db = sqlite3.connect(root / '.swarm/state.db')
            db.execute('CREATE TABLE mission_supervision(work_id,heartbeat_at,next_check_at,stopped_at)')
            db.execute('INSERT INTO mission_supervision VALUES(?,?,?,?)', ('owned', 'a', 'b', ''))
            db.execute('INSERT INTO mission_supervision VALUES(?,?,?,?)', ('other', 'a', 'b', ''))
            db.execute('CREATE TABLE agents(id,body)')
            db.execute('INSERT INTO agents VALUES(?,?)', ('agent', b'original'))
            db.commit()
            owned = types.SimpleNamespace(root=root, work='owned')
            before, _ = subject.business_snapshot(owned)
            db.execute('UPDATE mission_supervision SET heartbeat_at=?,next_check_at=? WHERE work_id=?', ('next', 'after', 'owned'))
            db.commit()
            self.assertTrue(subject.json_equal(subject.business_snapshot(owned)[0], before))
            for sql, values in [('UPDATE mission_supervision SET stopped_at=? WHERE work_id=?', ('stop', 'owned')),
                                ('UPDATE mission_supervision SET heartbeat_at=? WHERE work_id=?', ('otherchange', 'other')),
                                ('UPDATE agents SET body=? WHERE id=?', (b'changed', 'agent'))]:
                local_before, _ = subject.business_snapshot(owned)
                db.execute('BEGIN')
                db.execute(sql, values)
                db.commit()
                self.assertFalse(subject.json_equal(subject.business_snapshot(owned)[0], local_before))
            db.close()

    def test_history_preserves_duplicate_rows_and_typed_audit_bodies(self):
        """Appending valid rows passes; dropping a duplicate or changing text/BLOB content fails."""
        old = dict(tables=dict(events=dict(columns=['body'], rows=[[subject.sql_cell(b'old')], [subject.sql_cell(b'old')]])))
        valid = copy.deepcopy(old)
        valid['tables']['events']['rows'].append([subject.sql_cell(b'new')])
        subject.preserve_history(old, valid)
        for variant in ('duplicate', 'body', 'missing'):
            after = copy.deepcopy(valid)
            if variant == 'duplicate':
                after['tables']['events']['rows'].pop(0)
            elif variant == 'body':
                after['tables']['events']['rows'][0] = [subject.sql_cell('old')]
            else:
                after['tables'].clear()
            with self.subTest(variant=variant), self.assertRaises(RuntimeError):
                subject.preserve_history(old, after)

    def test_archived_bundle_requires_exact_old_binary_and_source_bytes(self):
        """A valid local inventory precedes tamper and root/path escape refusals."""
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            root = base / 'v1/root'
            (root / 'tests').mkdir(parents=True)
            helper = root / 'tests/rust_mission_task_profile.py'
            helper.write_bytes(b'# inert source never imported\n')
            binary = base / 'old'
            binary.write_bytes(b'old normal bytes never executed')
            manifest = dict(schema_version=1, variants=dict(v1=dict(binding_schema=1, helpers_root='v1/root',
                entrypoint='tests/rust_mission_task_profile.py:prepare', old_binary=dict(sha256=subject.digest(binary.read_bytes())),
                files=[dict(path='v1/root/tests/rust_mission_task_profile.py', sha256=subject.digest(helper.read_bytes()), bytes=helper.stat().st_size)],
                missing_local_imports=[])))
            path = base / 'manifest.json'
            path.write_text(json.dumps(manifest))
            _, files, sha = subject.validate_bundle(path, root, 1, binary)
            self.assertEqual(files, {helper.resolve(): subject.digest(helper.read_bytes())})
            self.assertEqual(sha, subject.digest(path.read_bytes()))
            helper.write_bytes(b'changed')
            with self.assertRaises(RuntimeError):
                subject.validate_bundle(path, root, 1, binary)
            helper.write_bytes(b'# inert source never imported\n')
            binary.write_bytes(b'wrong old normal')
            with self.assertRaises(RuntimeError):
                subject.validate_bundle(path, root, 1, binary)
            with self.assertRaises(RuntimeError):
                subject.validate_bundle(path, base, 1, binary)

    def test_loaded_historical_modules_cannot_fallback_to_current_checkout(self):
        """Exact module source set passes; missing or unlisted rust imports are refused."""
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'rust_mission_task_profile.py'
            path.write_bytes(b'inert source')
            files = {path.resolve(): subject.digest(path.read_bytes())}
            module = types.SimpleNamespace(__file__=str(path))
            isolated = {key:value for key,value in subject.sys.modules.items() if not key.startswith('rust_')}
            isolated['rust_mission_task_profile'] = module
            with patch.dict(subject.sys.modules, isolated, clear=True):
                self.assertIn('rust_mission_task_profile', subject.loaded_helpers(files))
                with patch.dict(subject.sys.modules, {'rust_unlisted_current': module}):
                    with self.assertRaises(RuntimeError):
                        subject.loaded_helpers(files)
            path.write_bytes(b'changed')
            with patch.dict(subject.sys.modules, isolated, clear=True):
                with self.assertRaises(RuntimeError):
                    subject.loaded_helpers(files)

    def test_fresh_current_witness_uses_real_probe_and_checked_subreaper_scope(self):
        """The witness invokes maintained probe v4/schema2; no imported PASS replaces it."""
        args = argparse.Namespace(binary=Path('/normal'), current_sdk=Path('/sdk'))
        probe = Mock()
        expected = dict(status='PASS_SAME_HOOK_WITNESS')
        probe.probe_case.return_value = expected
        probe.watch.subreaper_scope.side_effect = lambda operation, report, control: operation()
        with patch.object(subject, 'current', return_value=probe):
            self.assertEqual(subject.fresh_v4_witness(args, Path('/owned'), {'sha256':'observed'}), expected)
        probe.probe_case.assert_called_once_with(Path('/normal'), Path('/sdk'), {'sha256':'observed'},
            Path('/owned/current-v4-witness'), False, '', expected_binding_version=4, probe_schema_version=2)
        probe.watch.subreaper_scope.assert_called_once()

    def test_peer_finish_preserves_primary_transport_and_reap_failures(self):
        """A finish-channel error still closes/waits and retains independent nonzero owner failure."""
        peer = Mock()
        peer.poll.return_value = None
        peer.stdin.write.side_effect = OSError(5, 'owned simulated transport')
        peer.communicate.return_value = (b'tail', None)
        peer.returncode = 7
        with patch.object(subject, 'publish'):
            with self.assertRaises(BaseExceptionGroup) as captured:
                subject.finish_peer(peer, Mock(), b'ready', Path('/owned'))
        self.assertTrue(any(isinstance(error, OSError) for error in captured.exception.exceptions))
        self.assertTrue(any(isinstance(error, RuntimeError) for error in captured.exception.exceptions))
        peer.communicate.assert_called_once()
        self.assertIsNone(peer.stdin)

    def test_actual_refusal_consumer_cancels_only_after_exact_typed_denial(self):
        """Real Python flow under public doubles accepts denial/cancel/replay, rejects generic wrapper."""
        for valid in (True, False):
            with self.subTest(valid=valid), tempfile.TemporaryDirectory() as temporary:
                base = Path(temporary)
                root = base / 'owned-root'
                root.mkdir()
                (root / '.swarm').mkdir()
                (root / '.swarm/engine-settings.json').write_bytes(b'original')
                out = base / 'evidence'
                out.mkdir()
                binding = self.ready(2)['binding']
                agent = dict(id='agent', task_id='task', attempt_id='attempt', native_mission=binding,
                    status='queued', desired='run', stop_kind='', ended='')
                work = dict(tasks=[dict(id='task', status='running', attempts=[dict(id='old', status='failed'), dict(id='attempt', status='running')])])
                commands = []
                def ok(argv, value=None):
                    """Model exact desired-only stop then allowed queued cancellation without a provider."""
                    commands.append(argv)
                    if argv[:2] == ['agent', 'show']:
                        return dict(agent=copy.deepcopy(agent))
                    if argv[:2] == ['work', 'show']:
                        return dict(work=copy.deepcopy(work))
                    if argv[:2] == ['agent', 'stop']:
                        agent['desired'] = 'stop'
                        return dict(desired='stop', termination_confirmed=False)
                    if argv[:2] == ['agent', 'reconcile']:
                        agent.update(status='interrupted', desired='stop', stop_kind='operateur', ended='recorded')
                        work['tasks'][0]['status'] = 'blocked'
                        work['tasks'][0]['attempts'][-1]['status'] = 'interrupted'
                        return dict(cancelled_before_start=True, accepted_by_cancellation=False)
                    raise AssertionError('unexpected public operation')
                response = dict(schema_version=1, error='safe', failure=dict(code='agent-mission-provenance',
                    message='mission-agent-instance-required', retryable=False,
                    causes=['mission-agent-instance-required'] if valid else ['agent-mission-provenance']))
                owned = types.SimpleNamespace(root=root, work='work', ok=ok, call=Mock(return_value=(2, response)))
                def snap(value):
                    """Retain Agent/Work values under the same public-state double."""
                    return copy.deepcopy(dict(agent=agent, work=work, tables={})), []
                def resources(value, ident):
                    """Reservation only releases after actual modeled reconciliation."""
                    return dict(reservation=['released' if agent['status'] == 'interrupted' else 'reserved', 100.0], turns=[])
                state = dict(agent='agent', binding=binding, public_start_request={'event_id':'original'})
                with patch.object(subject, 'business_snapshot', side_effect=snap), patch.object(subject, 'live_owner'), \
                     patch.object(subject, 'replay_legacy_control') as replay, patch.object(subject, 'resources', side_effect=resources):
                    if valid:
                        after, _, _ = subject.refuse_and_cancel(owned, Mock(), state, out)
                        self.assertEqual(after['status'], 'interrupted')
                        self.assertEqual(sum(argv[:2] == ['agent', 'reconcile'] for argv in commands), 2)
                    else:
                        with self.assertRaises(RuntimeError):
                            subject.refuse_and_cancel(owned, Mock(), state, out)
                        self.assertFalse(any(argv[:2] == ['agent', 'stop'] for argv in commands))
                    replay.assert_called_once()

    def test_required_refusal_does_not_accept_generic_wrapper_or_message(self):
        """Actual CLI decoder distinguishes exact source cause from message-only lookalikes."""
        rootprobe = subject.current()
        expected = 'mission-agent-instance-required'
        value = dict(schema_version=1, error='safe', failure=dict(code='agent-mission-provenance',
            message='safe', retryable=False, causes=[expected]))
        self.assertIn(expected, rootprobe.refusal_codes(value))
        for causes in ([], [expected+'-suffix'], [dict(code=expected)]):
            bad = copy.deepcopy(value)
            bad['failure']['causes'] = causes
            bad['failure']['message'] = expected
            self.assertNotIn(expected, rootprobe.refusal_codes(bad))


if __name__ == '__main__':
    unittest.main()
