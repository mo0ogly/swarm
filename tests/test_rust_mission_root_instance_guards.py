#!/usr/bin/env python3
"""Owned SQLite/filesystem and pure oracle guards; no engine/provider process."""
import json
import ast
import inspect
import copy
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest
import types
from unittest.mock import patch

import rust_mission_root_instance as subject


class RootInstanceGuards(unittest.TestCase):
    """Witness-first clone, input refusal, conservative restore and exact oracle."""

    def test_wal_backup_is_coherent_and_files_have_new_inodes(self):
        """Committed data still in WAL survives backup without hardlink copying."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'source'
            source.mkdir(mode=0o700)
            database = source / 'state.db'
            db = sqlite3.connect(database)
            database.chmod(0o600)
            try:
                self.assertEqual(db.execute('PRAGMA journal_mode=WAL').fetchone()[0], 'wal')
                db.execute('CREATE TABLE fixture(value TEXT)')
                db.execute('INSERT INTO fixture VALUES (?)', ('committed-wal-value',))
                db.commit()
                self.assertTrue((source / 'state.db-wal').stat().st_size > 0)
                subject.fixture.publish(source / 'private.json', dict(opaque='preserved'))
                (source / 'nested').mkdir(mode=0o700)
                subject.fixture.publish(source / 'nested/child', b'owned opaque file')
                original_lstat = Path.lstat
                def volatile_sqlite_metadata(path):
                    """A volatile SQLite sibling must never be byte-copy metadata."""
                    if path.parent == source and path.name in subject.SQLITE_FAMILY:
                        raise FileNotFoundError('simulated volatile SQLite name')
                    return original_lstat(path)
                with patch.object(Path, 'lstat', new=volatile_sqlite_metadata):
                    self.assertIn('private.json', subject.private_files(source))
                before = database.read_bytes()
                receipt = subject.coherent_clone(source, root / 'clone', root)
                self.assertEqual(database.read_bytes(), before)
                self.assertNotEqual(receipt['original_identity'], receipt['clone']['identity'])
                self.assertEqual((source / 'private.json').read_bytes(), (root / 'clone/private.json').read_bytes())
                self.assertNotEqual((source / 'private.json').stat().st_ino, (root / 'clone/private.json').stat().st_ino)
                copied = sqlite3.connect('file:' + str(root / 'clone/state.db') + '?mode=ro', uri=True)
                try:
                    self.assertEqual(copied.execute('SELECT value FROM fixture').fetchall(), [('committed-wal-value',)])
                finally:
                    copied.close()
                self.assertIn('state.db-wal', receipt['excluded_sqlite_files'])
            finally:
                db.close()

    def test_private_copy_rejects_hardlinks_after_valid_witness(self):
        """Valid regular private file passes; same bytes with nlink2 refuse."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subject.fixture.publish(root / 'private', b'owned')
            self.assertIn('private', subject.private_files(root))
            original_lstat = Path.lstat
            def missing_private_metadata(path):
                """A missing non-SQLite private file must remain an error."""
                if path == root / 'private':
                    raise FileNotFoundError('simulated private file disappearance')
                return original_lstat(path)
            with patch.object(Path, 'lstat', new=missing_private_metadata):
                with self.assertRaises(FileNotFoundError):
                    subject.private_files(root)
            os.link(root / 'private', root / 'second')
            with self.assertRaisesRegex(RuntimeError, 'type/link'):
                subject.private_files(root)

    def test_restore_refuses_unknown_process_closure_then_preserves_instances(self):
        """Unknown closure makes no rename; known closure preserves both dirs."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            current = root / '.swarm'
            original = root / 'original'
            retained = root / 'retained-clone'
            current.mkdir(mode=0o700)
            original.mkdir(mode=0o700)
            current_inode, original_inode = current.stat().st_ino, original.stat().st_ino
            with self.assertRaisesRegex(RuntimeError, 'closure is unknown'):
                subject.restore_instances(root, original, retained, False)
            self.assertEqual(current.stat().st_ino, current_inode)
            self.assertEqual(original.stat().st_ino, original_inode)
            subject.restore_instances(root, original, retained, True)
            self.assertEqual(current.stat().st_ino, original_inode)
            self.assertEqual(retained.stat().st_ino, current_inode)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = root / 'original'
            original.mkdir(mode=0o700)
            inode = original.stat().st_ino
            subject.restore_instances(root, original, root / 'unused-retained', True)
            self.assertEqual((root / '.swarm').stat().st_ino, inode)

    def test_database_only_swap_and_private_restitution_preserve_both_histories(self):
        """Same directory/new DB witness; preserve mutated files and original DB.

        SQL contains fixture data only, no engine/permission rows or processes.
        Unknown closure must refuse first; known closed state permits restitution.
        """
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            local = root / '.swarm'
            local.mkdir(mode=0o700)
            db = sqlite3.connect(local / 'state.db')
            (local / 'state.db').chmod(0o600)
            self.assertEqual(db.execute('PRAGMA journal_mode=WAL').fetchone()[0], 'wal')
            db.execute('CREATE TABLE fixture(value TEXT)')
            db.execute('INSERT INTO fixture VALUES (?)', ('original',))
            db.commit()
            subject.fixture.publish(local / 'opaque.json', dict(original=True))
            initial_dir = local.stat().st_ino
            initial_db = (local / 'state.db').stat().st_ino
            initial_private = (local / 'opaque.json').read_bytes()
            state = {}
            original, staging, retained = root / 'original', root / 'staging', root / 'retained'
            try:
                receipt = subject.prepare_database_variant(local, staging, original, root, state)
                subject.install_database_variant(local, staging, original, root, state)
                self.assertEqual(local.stat().st_ino, initial_dir)
                self.assertNotEqual((local / 'state.db').stat().st_ino, initial_db)
                self.assertEqual((local / 'opaque.json').read_bytes(), initial_private)
                self.assertEqual(subject.instance_snapshot(local)[0]['sql_sha256'], receipt['source_sql_sha256'])
                clone = sqlite3.connect(local / 'state.db')
                try:
                    clone.execute('INSERT INTO fixture VALUES (?)', ('clone-write',))
                    clone.commit()
                finally:
                    clone.close()
                subject.fixture.publish(local / 'opaque.json', dict(clone=True))
                (local / 'new-receipts').mkdir(mode=0o700)
                subject.fixture.publish(local / 'new-receipts/receipt.json', dict(clone_receipt=True))
                with self.assertRaisesRegex(RuntimeError, 'closure is unknown'):
                    subject.restore_database_variant(local, original, retained, state, False)
                self.assertFalse(retained.exists())
            finally:
                db.close()
            subject.restore_database_variant(local, original, retained, state, True)
            self.assertEqual(local.stat().st_ino, initial_dir)
            self.assertEqual((local / 'state.db').stat().st_ino, initial_db)
            self.assertEqual((local / 'opaque.json').read_bytes(), initial_private)
            self.assertEqual(json.loads((retained / 'private-mutations/opaque.json').read_text()), dict(clone=True))
            self.assertTrue((retained / 'private-mutations/new-receipts/receipt.json').exists())
            self.assertEqual(subject.instance_snapshot(retained / 'database')[1].count(b'clone-write'), 1)
            self.assertNotIn(b'clone-write', subject.instance_snapshot(local)[1])

    def test_database_partial_install_retains_effects_and_restores_after_known_close(self):
        """Injected rename failure preserves original main file and partial state."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            local = root / '.swarm'
            local.mkdir(mode=0o700)
            db = sqlite3.connect(local / 'state.db')
            (local / 'state.db').chmod(0o600)
            db.execute('CREATE TABLE fixture(value TEXT)')
            db.commit()
            db.close()
            inode = (local / 'state.db').stat().st_ino
            state = {}
            original, staging, retained = root / 'original', root / 'staging', root / 'retained'
            subject.prepare_database_variant(local, staging, original, root, state)
            rename = os.rename
            def fail_install(source, target):
                """Fail only installing fresh main DB, after old DB preservation."""
                if source == staging / 'state.db':
                    raise OSError(5, 'synthetic backup install refusal')
                return rename(source, target)
            with patch.object(subject.os, 'rename', side_effect=fail_install):
                with self.assertRaisesRegex(OSError, 'synthetic backup install'):
                    subject.install_database_variant(local, staging, original, root, state)
            self.assertIn('state.db', state['original_moved'])
            self.assertFalse(state['installation_complete'])
            self.assertTrue((original / 'database/state.db').exists())
            subject.restore_database_variant(local, original, retained, state, True)
            self.assertEqual((local / 'state.db').stat().st_ino, inode)
            self.assertTrue((staging / 'state.db').exists())

    def test_current_binding4_observes_directory_and_database_not_only_version(self):
        """Fresh witness then legacy/type/shape/directory/DB mismatches refuse."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / '.swarm').mkdir(mode=0o700)
            subject.fixture.publish(root / '.swarm/state.db', b'metadata-only fixture')
            local, database = (root / '.swarm').stat(), (root / '.swarm/state.db').stat()
            identity = dict(directory=[local.st_dev, local.st_ino], database=[database.st_dev, database.st_ino])
            binding = dict(schema_version=4, root_instance=identity)
            subject.watch.verify_current_root_binding(binding, root)
            variants = [dict(binding, schema_version=3), dict(binding, root_instance=None),
                        dict(binding, root_instance=dict(identity, unknown=True)),
                        dict(binding, root_instance=dict(identity, directory=[local.st_dev, local.st_ino + 1])),
                        dict(binding, root_instance=dict(identity, database=[database.st_dev, database.st_ino + 1])),
                        dict(binding, root_instance=dict(identity, database=[True, database.st_ino]))]
            for invalid in variants:
                with self.assertRaises(RuntimeError):
                    subject.watch.verify_current_root_binding(invalid, root)

    def test_expected_binding_version_is_explicit_never_detected(self):
        """Historical3 passes only declared3; default4 and malformed3 refuse."""
        legacy = dict(schema_version=3, parent_profile_sha256='a' * 64, workspace_turn_seq='1')
        root = Path('/unused-no-physical-identity-claim')
        subject.watch.verify_current_root_binding(legacy, root, expected_version=3)
        with self.assertRaises(RuntimeError):
            subject.watch.verify_current_root_binding(legacy, root)
        variants = [dict(legacy, schema_version=4), dict(legacy, root_instance=None),
                    dict(legacy, root_instance={}), dict(legacy, parent_profile_sha256='A' * 64),
                    dict(legacy, workspace_turn_seq='01'), dict(legacy, workspace_turn_seq='0'),
                    dict(legacy, workspace_turn_seq=1)]
        for invalid in variants:
            with self.assertRaises(RuntimeError):
                subject.watch.verify_current_root_binding(invalid, root, expected_version=3)
        for invalid in (1, True, '3'):
            with self.assertRaises(RuntimeError):
                subject.watch.verify_current_root_binding(legacy, root, expected_version=invalid)

    def test_probe_zombie_is_kernel_owned_and_never_claims_reobserved_executable(self):
        """Known zombie witness; wrong parent/stamp/reused PID/readiness refuse."""
        initial = dict(pid=11, stamp='owned', ppid=7, sid=11, pgid=11, state='Z')
        conductor = dict(pid=7, stamp='parent', sid=5)
        with patch.object(subject.watch, 'observation', return_value=initial), patch.object(subject.os, 'pidfd_open', return_value=19), patch.object(subject, 'select_ready', return_value=True):
            fd, actual, proof = subject.attach_probe_child(initial, conductor, Path('/unused'), Path('/unused'))
            self.assertEqual((fd, actual), (19, initial))
            self.assertFalse(proof['executable_hash_reobserved'])
            self.assertFalse(proof['argv_reobserved'])
            self.assertEqual(proof['executable_and_argv_source'], 'qualified-sdk-spawn')
        for observations, ready in (([dict(initial, ppid=99)], True), ([dict(initial, stamp='other')], True),
                                    ([initial, dict(initial, stamp='reused')], True), ([initial, initial], False)):
            with patch.object(subject.watch, 'observation', side_effect=observations), patch.object(subject.os, 'pidfd_open', return_value=19), patch.object(subject, 'select_ready', return_value=ready), patch.object(subject.os, 'close'):
                with self.assertRaises(RuntimeError):
                    subject.attach_probe_child(initial, conductor, Path('/unused'), Path('/unused'))

    def sdk_projection(self, variant):
        """Build the inspected source projection independently of classifier helpers."""
        empty = dict(labels=[], sqlite_code=None, io_errno=None)
        root = dict(labels=['mission-root-unavailable'], sqlite_code=None, io_errno=None)
        specific = dict(labels=['root-database-instance-changed'], sqlite_code=None, io_errno=None)
        reason = [root, specific]
        if variant == 'directory':
            reason = [dict(labels=['mission-storage-failed'], sqlite_code=None, io_errno=None),
                      dict(labels=['sqlite-code:1032'], sqlite_code=1032, io_errno=None), empty]
        supervisor = dict(pid=11, stamp='owned', ppid=7)
        conductor = dict(pid=7, stamp='parent')
        spawned = dict(supervisor=supervisor, conductor=conductor)
        exit_fact = dict(code=2, signal=None, scope='probe-final-try-wait')
        observed = dict(worker_exit_observed=True, child_reaped_by_sdk=True, supervisor_pid=11, exit_observation=exit_fact)
        trace = dict(pid=11, phase='publish-probe-observation', supervisor=supervisor, initial_supervisor=supervisor,
                     session_state='pending', conductor=conductor, early_exit=exit_fact,
                     spawned_publication_attempted=True, spawned_publication_confirmed=True)
        platform = dict(labels=['runtime-platform'], sqlite_code=None, io_errno=None)
        failure = dict(status='FAIL', worker_not_stopped_by_fixture=True, child_ownership=trace,
            causes=[[empty, dict(child_ownership=trace), platform, platform, empty,
                     dict(fixture_phase='watch-fixture-cycle'), empty] + reason, reason])
        rows = [dict(row, phase=phase, path=['root'] + ['source'] * depth, recorded_work=None)
                for phase in ('supervision', 'journal') for depth, row in enumerate(reason)]
        cycle = dict(diagnostics=dict(action_started=True, action_succeeded=True, causes=rows), causes=[empty] + reason)
        return failure, cycle, spawned, observed

    def test_sdk_context_classification_requires_every_exact_branch_and_phase(self):
        """Both source-defined witnesses pass; unknown/close/IO/wrong types fail."""
        for variant in ('directory', 'database'):
            data = self.sdk_projection(variant)
            result = subject.classify_sdk_refusal(*data, variant, 1, True)
            self.assertEqual(result['status'], 'EXPECTED_SUBSTITUTED_CONTEXT_REFUSAL')
            self.assertFalse(result['lease_stop_recorded'])
            self.assertFalse(result['sdk_checked_close_receipt'])
            for fault in ('extra_branch', 'unknown_label', 'io', 'phase', 'action', 'missing_stop', 'no_reap', 'wrong_pid'):
                failure, cycle, spawned, observed = copy.deepcopy(data)
                if fault == 'extra_branch': failure['causes'].append([dict(labels=[], sqlite_code=None, io_errno=None)])
                elif fault == 'unknown_label': failure['causes'][-1][0]['labels'] = ['unknown-close-error']
                elif fault == 'io': failure['causes'][-1][0]['io_errno'] = 5
                elif fault == 'phase': cycle['diagnostics']['causes'][0]['phase'] = 'action'
                elif fault == 'action': cycle['diagnostics']['action_succeeded'] = 1
                elif fault == 'missing_stop': failure['causes'].pop()
                elif fault == 'no_reap': observed['child_reaped_by_sdk'] = False
                else: failure['child_ownership']['pid'] = 99
                with self.subTest(variant=variant, fault=fault):
                    self.assertEqual(subject.classify_sdk_refusal(failure, cycle, spawned, observed, variant, 1, True)['status'],
                                     'INCONCLUSIVE_SDK_FAILURE_UNCLASSIFIED')
            self.assertEqual(subject.classify_sdk_refusal(*data, variant, 1, False)['status'], 'INCONCLUSIVE_SDK_FAILURE_UNCLASSIFIED')
            stop_only = copy.deepcopy(data[0])
            stop_only['causes'] = [stop_only['causes'][-1]]
            self.assertEqual(subject.classify_sdk_refusal(stop_only, None, data[2], data[3], variant, 1, True)['status'],
                             'EXPECTED_SUBSTITUTED_CONTEXT_REFUSAL')
            self.assertEqual(subject.classify_sdk_refusal(*data, variant, True, True)['status'], 'INCONCLUSIVE_SDK_FAILURE_UNCLASSIFIED')

    def test_public_recovery_separates_desired_stop_from_reconciled_release(self):
        """Scripted public contract witness, then premature stop-release refuses."""
        for faulty_stop in (False, True):
            with tempfile.TemporaryDirectory() as temporary:
                out = Path(temporary)
                agent = dict(id='owned', status='queued', native_mission={'schema_version': 4}, supervisor_pid=0)
                turn = dict(seq='9', work='work', task='task', workspace='owned', state='acquired', agent='owned',
                            requested_at='request', acquired_at='acquired', released_at='')
                state = dict(phase='before', reconciles=0)
                def current_agent():
                    """Return controlled public Agent projection for the current stage."""
                    if state['phase'] == 'before': return agent.copy()
                    return dict(agent, desired='stop', status='interrupted' if state['phase'] == 'done' else 'queued')
                def current_turn():
                    """Only reconciliation changes the timestamp/state of this exact turn."""
                    return dict(turn, state='released', released_at='actual-release') if state['phase'] == 'done' else turn.copy()
                def operation(argv):
                    """Model only actual allowed public stop/reconcile/read operations."""
                    if argv[:2] == ['agent', 'show']: return dict(agent=current_agent())
                    if argv[:2] == ['mission', 'queue']: return dict(turns=[current_turn()])
                    if argv[:2] == ['agent', 'stop']:
                        state['phase'] = 'stop'
                        return dict(desired='stop', termination_confirmed=False)
                    self.assertEqual(argv[:2], ['agent', 'reconcile'])
                    state['phase'] = 'done'; state['reconciles'] += 1
                    return dict(cancelled_before_start=True, accepted_by_cancellation=False)
                def money(_owned, _agent):
                    """Inject an erroneous release on stop in the negative fixture only."""
                    return ['released', 1.0] if state['phase'] == 'done' or (faulty_stop and state['phase'] == 'stop') else ['reserved', 1.0]
                owned = types.SimpleNamespace(root=out, work='work', ok=operation)
                with patch.object(subject, 'business_snapshot', return_value={}), patch.object(subject.watch.completion, 'reservation', side_effect=money), patch.object(subject.watch.completion, 'provider_files', return_value={}), patch.object(subject.watch.diagnostic, 'snapshot', return_value=('same',)):
                    if faulty_stop:
                        with self.assertRaisesRegex(RuntimeError, 'desired-only stop settled'):
                            subject.recover_by_public_cancellation(owned, out, agent, {})
                        self.assertEqual(state['reconciles'], 0)
                    else:
                        result = subject.recover_by_public_cancellation(owned, out, agent, {})
                        self.assertEqual(result['status'], 'PASS_RECOVERY_BY_CANCELLATION')
                        self.assertFalse(result['new_admission_proven'])
                        self.assertEqual(state['reconciles'], 2)

    def diagnostic(self, causes, code='agent-mission-provenance', message='safe diagnostic'):
        """Build the actual CLI schema1 Failure shape, without executing a CLI."""
        return dict(schema_version=1, error=message,
                    failure=dict(code=code, message=message, retryable=False, causes=causes))

    def test_refusal_codes_are_exact_structured_fields(self):
        """Only actual schema1 Failure typed cause strings qualify."""
        document = self.diagnostic(['agent-mission-provenance', 'mission-agent-instance-mismatch'])
        self.assertEqual(subject.refusal_codes(document), {'agent-mission-provenance', 'mission-agent-instance-mismatch'})
        self.assertEqual(subject.refusal_codes(dict(error='mission-agent-instance-mismatch')), set())
        self.assertEqual(subject.refusal_codes(dict(failure=dict(code='mission-agent-instance-mismatch'))), set())

    def test_clone_oracle_cannot_promote_provider_or_other_failure(self):
        """Exact typed vector witness; entry/other/changed/malformed stay failed."""
        expected = 'mission-agent-instance-mismatch'
        stderr = json.dumps(self.diagnostic([expected]))
        self.assertEqual(subject.clone_oracle(False, 2, expected, stderr, True), 'PASS_INSTANCE_REFUSAL')
        self.assertEqual(subject.clone_oracle(True, 0, expected, stderr, True), 'FAIL_PROVIDER_ENTERED_CLONE')
        for code, status, raw, stable in [('', 2, stderr, True), ('other', 2, stderr, True),
                                        (expected, 0, stderr, True), (expected, 2, stderr, False),
                                        (expected, 2, 'not JSON', True)]:
            self.assertEqual(subject.clone_oracle(False, status, code, raw, stable), 'INCONCLUSIVE_OTHER_REFUSAL')

    @staticmethod
    def late_observation():
        """Construct the exact source schema2 witness with owned synthetic process identity."""
        conductor = dict(pid=7, stamp='owned-parent', ppid=3, pgid=3, sid=3, state='S')
        document = dict(status='ROOT_INSTANCE_PROBE_SPAWN_READY', schema_version=2, work='work', agent_id='agent',
            original_binding_matches_open_store=True, mission_cycle_monitor_running=True,
            original_binding_retained=True, supervisor_spawned=False, observation_is_authority=False,
            conductor=conductor.copy())
        return document, conductor

    def test_late_probe_ready_exact_schema_checks_and_conductor(self):
        """Valid real-producer shape precedes numeric booleans, unknown fields and changed identity."""
        document, conductor = self.late_observation()
        subject.verify_spawn_ready(document, 'work', 'agent', conductor)
        variants = []
        for key in ('original_binding_matches_open_store', 'mission_cycle_monitor_running', 'original_binding_retained'):
            variants.extend([dict(document, **{key: False}), dict(document, **{key: 1})])
        variants.extend([dict(document, schema_version=True), dict(document, schema_version=1),
            dict(document, schema_version=2.0), dict(document, extra=True), dict(document, work='other'),
            dict(document, agent_id='other'), dict(document, supervisor_spawned=0),
            dict(document, observation_is_authority=0)])
        for key, value in (('pid', 8), ('stamp', 'reused'), ('ppid', 9), ('pgid', 7), ('sid', 7), ('pid', True)):
            variants.append(dict(document, conductor=dict(document['conductor'], **{key: value})))
        missing = dict(document)
        missing.pop('mission_cycle_monitor_running')
        variants.append(missing)
        for variant in variants:
            with self.subTest(variant=variant):
                with self.assertRaises(RuntimeError):
                    subject.verify_spawn_ready(variant, 'work', 'agent', conductor)

    def test_late_probe_resume_validate_then_inject_and_spawn_order(self):
        """Mock handshake requires READY before injection; AST guard preserves historical branch order."""
        document, conductor = self.late_observation()
        events = []
        with tempfile.TemporaryDirectory() as temporary:
            with patch.object(subject.watch, 'send_command', side_effect=lambda *args: events.append(args[1])), \
                 patch.object(subject.watch, 'wait_json', side_effect=lambda *args: (events.append('ready'), document)[1]), \
                 patch.object(subject.watch, 'observation', return_value=conductor), \
                 patch.object(subject.fixture, 'publish', side_effect=lambda *args: events.append('retained')):
                subject.late_probe_barrier(object(), Path(temporary), 1, 2, 'work', 'agent', conductor)
        self.assertEqual(events, ['resume\n', 'ready', 'retained'])
        document['mission_cycle_monitor_running'] = False
        with patch.object(subject.watch, 'send_command') as send, patch.object(subject.watch, 'wait_json', return_value=document), patch.object(subject.fixture, 'publish') as publish:
            with self.assertRaises(RuntimeError):
                subject.late_probe_barrier(object(), Path('/owned'), 1, 2, 'work', 'agent', conductor)
            send.assert_called_once()
            self.assertEqual(send.call_args.args[1], 'resume\n')
            publish.assert_not_called()
        tree = ast.parse(inspect.getsource(subject.probe_case))
        calls = [node for node in ast.walk(tree) if isinstance(node, ast.Call)]
        def positions(name):
            """Keep every call position; never let the last same-name call hide an early call."""
            return [node.lineno for node in calls if isinstance(node.func, ast.Name) and node.func.id == name]
        late = positions('late_probe_barrier')
        injections = positions('coherent_clone') + positions('install_database_variant')
        sends = [node for node in calls if isinstance(node.func, ast.Attribute) and node.func.attr == 'send_command']
        first = min(sends, key=lambda node: node.lineno)
        self.assertTrue(late and injections)
        self.assertLess(max(late), min(injections))
        self.assertLess(max(injections), first.lineno)
        self.assertEqual(ast.unparse(first.args[1]), "'spawn\\n' if probe_schema_version == 2 else 'resume\\n'")

    def test_unknown_connected_identity_operation_is_not_sdk_context_success(self):
        """New typed diagnostic vocabulary cannot silently widen the exact Option B allowlist."""
        for variant in ('directory', 'database'):
            for operation in ('Prepare', 'Query', 'Diagnostic', 'Close', 'unknown'):
                failure, cycle, spawned, observed = copy.deepcopy(self.sdk_projection(variant))
                failure['causes'][-1][0]['labels'].append('database-identity-operation:' + operation)
                self.assertEqual(subject.classify_sdk_refusal(failure, cycle, spawned, observed, variant, 1, True)['status'],
                                 'INCONCLUSIVE_SDK_FAILURE_UNCLASSIFIED')

    def test_cli_typed_cause_vector_rejects_substrings_wrappers_and_malformed(self):
        """Real safe Vec<String> witness; text/shape/coercion never qualify."""
        expected = 'mission-agent-instance-mismatch'
        witness = self.diagnostic([expected])
        self.assertEqual(subject.clone_oracle(False, 2, expected, json.dumps(witness), True), 'PASS_INSTANCE_REFUSAL')
        variants = [self.diagnostic(['prefix:' + expected]), self.diagnostic([expected + ':suffix']),
                    self.diagnostic([], message=expected), self.diagnostic([dict(code=expected)]),
                    self.diagnostic(expected), self.diagnostic([expected, 1]), self.diagnostic([expected, None]),
                    self.diagnostic([], code=expected),
                    dict(witness, schema_version=True), dict(witness, schema_version=2), dict(witness, error=1),
                    dict(self.diagnostic([]), causes=[expected]), self.diagnostic([])]
        variants[-1]['arbitrary'] = dict(code=expected, causes=[expected])
        for document in variants:
            with self.subTest(document=document):
                self.assertEqual(subject.clone_oracle(False, 2, expected, json.dumps(document), True),
                                 'INCONCLUSIVE_OTHER_REFUSAL')
        for wrapper in ('agent-mission-provenance', 'mission-agent-instance-required', 'database-identity-unavailable'):
            self.assertEqual(subject.clone_oracle(False, 2, wrapper,
                json.dumps(self.diagnostic([wrapper], code=wrapper)), True), 'INCONCLUSIVE_OTHER_REFUSAL')
        root_guard = 'root-database-instance-changed'
        self.assertEqual(subject.clone_oracle(False, 2, root_guard,
            json.dumps(self.diagnostic([root_guard])), True), 'PASS_INSTANCE_REFUSAL')


if __name__ == '__main__':
    unittest.main(verbosity=2)
