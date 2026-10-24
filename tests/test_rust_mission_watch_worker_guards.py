#!/usr/bin/env python3
"""Targeted watch harness guards; no provider, process, real prctl or B1 run.

Synthetic kernel observations establish ordering, not OS authentication. The
actual run_case failure path is exercised with owned temp files and a fake
listener; faults preserve exception objects and prove resource close is reached.
"""
import ast
import copy
import errno
import inspect
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_mission_watch_worker as subject


def leaves(error):
    """Flatten error groups for identity checks without rendering private data."""
    if isinstance(error, BaseExceptionGroup):
        return [item for nested in error.exceptions for item in leaves(nested)]
    return [error]


class WatchWorkerGuards(unittest.TestCase):
    """Witness/refusal/recovery checks for peer ordering and cleanup failures."""

    def peer(self, parent=7):
        """Construct a nonblocking mock peer; recv records every attempted read."""
        connection = Mock()
        connection.getsockopt.return_value = struct.pack('3i', 11, os.getuid(), 0)
        connection.recv.side_effect = [bytes([byte]) for byte in b'{"nonce":"owned"}\n']
        return connection, dict(pid=11, stamp='owned-stamp', ppid=parent)

    def test_valid_peer_attached_before_frame(self):
        """Valid witness holds fd before decoding the nonce frame."""
        connection, identity = self.peer()
        with patch.object(subject, 'observation', return_value=identity), patch.object(subject, 'attach', return_value=19) as attach:
            actual, fd = subject.owned_provider_peer(connection, dict(pid=7))
            connection.recv.assert_not_called()
            attach.assert_called_once_with(identity)
            self.assertEqual((actual, fd), (identity, 19))
            self.assertEqual(subject.frame(connection), dict(nonce='owned'))

    def test_wrong_parent_refused_before_frame_or_attach(self):
        """Same-UID wrong parent cannot enter blocking recv or acquire pidfd."""
        connection, identity = self.peer(parent=99)
        with patch.object(subject, 'observation', return_value=identity), patch.object(subject, 'attach') as attach:
            with self.assertRaisesRegex(RuntimeError, 'parent is not supervisor'):
                subject.owned_provider_peer(connection, dict(pid=7))
            connection.recv.assert_not_called()
            attach.assert_not_called()

    def test_changed_parent_after_attach_closes_before_frame(self):
        """Post-attach drift refuses and closes the held descriptor."""
        connection, identity = self.peer()
        with patch.object(subject, 'observation', side_effect=[identity, dict(identity, ppid=99)]), patch.object(subject, 'attach', return_value=19), patch.object(subject.os, 'close') as close:
            with self.assertRaisesRegex(RuntimeError, 'identity changed'):
                subject.owned_provider_peer(connection, dict(pid=7))
            connection.recv.assert_not_called()
            close.assert_called_once_with(19)

    def test_run_case_uses_owned_peer_before_frame(self):
        """Keep the tested peer guard on the actual blocking call path."""
        tree = ast.parse(inspect.getsource(subject.run_case))
        calls = {name: [node.lineno for node in ast.walk(tree)
                       if isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
                       and node.func.id == name]
                 for name in ('owned_provider_peer', 'frame')}
        self.assertTrue(calls['owned_provider_peer'])
        self.assertTrue(calls['frame'])
        self.assertLess(min(calls['owned_provider_peer']), min(calls['frame']))

    def test_subreaper_witness_and_primary_restore(self):
        """Success and primary-only witnesses restore the original fake flag."""
        control = Mock(side_effect=[False, None, None])
        report = Mock()
        self.assertEqual(subject.subreaper_scope(lambda: 'result', report, control), 'result')
        self.assertEqual(control.call_args_list[-1].args, (False,))
        report.assert_not_called()
        primary = RuntimeError('primary')
        control = Mock(side_effect=[True, None, None])
        with self.assertRaises(BaseExceptionGroup) as raised:
            subject.subreaper_scope(Mock(side_effect=primary), report, control)
        self.assertEqual(leaves(raised.exception), [primary])
        self.assertEqual(control.call_args_list[-1].args, (True,))

    def test_subreaper_primary_publication_restore_failures_retained(self):
        """ENOSPC reporting cannot bypass restoration or hide any cause."""
        primary = RuntimeError('primary')
        publication = OSError(errno.ENOSPC, 'synthetic publication')
        restoration = OSError(errno.EIO, 'synthetic restoration')
        control = Mock(side_effect=[False, None, restoration])
        with self.assertRaises(BaseExceptionGroup) as raised:
            subject.subreaper_scope(Mock(side_effect=primary), Mock(side_effect=publication), control)
        self.assertEqual(leaves(raised.exception), [primary, publication, restoration])
        self.assertEqual(control.call_args_list[-1].args, (False,))

    def test_enable_failure_also_attempts_restore(self):
        """A failed enable still triggers restoration of the observed flag."""
        primary = OSError(errno.EPERM, 'synthetic enable')
        control = Mock(side_effect=[False, primary, None])
        operation = Mock()
        with self.assertRaises(BaseExceptionGroup) as raised:
            subject.subreaper_scope(operation, Mock(), control)
        self.assertEqual(leaves(raised.exception), [primary])
        operation.assert_not_called()
        self.assertEqual(control.call_args_list[-1].args, (False,))

    def test_descriptor_finally_preserves_pending_operation(self):
        """Witness close, then failed finally-close retains the active cause."""
        primary = RuntimeError('synthetic operation')
        secondary = OSError(errno.EIO, 'synthetic descriptor close')
        for inject in (False, True):
            with self.subTest(inject=inject), patch.object(subject.os, 'close', side_effect=secondary if inject else None) as close:
                try:
                    try:
                        raise primary
                    finally:
                        subject.close_descriptor(19, sys.exception())
                except BaseException as error:
                    self.assertEqual(leaves(error), [primary, secondary] if inject else [primary])
                close.assert_called_once_with(19)
        self.assertIn('close_descriptor(descriptor, sys.exception())', inspect.getsource(subject.run_case))

    def test_attach_failure_and_close_failure_both_retained(self):
        """Actual attach path retains stamp refusal plus close failure."""
        identity = dict(pid=11, stamp='owned')
        secondary = OSError(errno.EIO, 'synthetic pidfd close')
        for inject in (False, True):
            with self.subTest(inject=inject), patch.object(subject, 'observation', side_effect=[identity, dict(identity, stamp='changed')]), patch.object(subject.os, 'pidfd_open', return_value=19), patch.object(subject.os, 'close', side_effect=secondary if inject else None) as close:
                with self.assertRaises(BaseException) as raised:
                    subject.attach(identity)
                errors = leaves(raised.exception)
                self.assertIsInstance(errors[0], RuntimeError)
                self.assertIn('stamp changed after', str(errors[0]))
                self.assertEqual(errors[1:], [secondary] if inject else [])
                close.assert_called_once_with(19)

    def test_completion_requires_native_facts_and_exact_receipt(self):
        """Valid witness then each missing/incorrect fact and identity refuses."""
        facts = dict(kernel_reaped=True, output_complete=True, recovery_required=False,
                     workspace_release_confirmed=True)
        running = dict(id='owned', work_id='work', supervisor_pid=7, child_pid=11,
                       task_id='task', attempt_id='attempt', supervisor_identity={'stamp': 's'},
                       child_identity={'stamp': 'c'}, native_mission={'schema_version': 3})
        final = dict(agent=dict(running, status='completed', native_execution_result=facts), task_accepted=False)
        journal = dict(schema_version=1, directory=[1, 2], expected=running,
                       details=facts, incomplete_output=False, exit_code=0, failed=False,
                       interrupted=False, may_have_started=True, reserved=['work', 1.0, 'reserved'])
        def verify(public, receipt, reservation=None):
            """Validate synthetic decoded values without touching any fixture."""
            subject.verify_completion(public, receipt, running, 'work', dict(pid=7),
                dict(pid=11), reservation or ['work', 1.0, 'estimated'], [1, 2])
        verify(final, journal)
        for key, value in facts.items():
            for invalid in ('missing', not value, int(value)):
                with self.subTest(key=key, invalid=invalid):
                    changed = copy.deepcopy(final)
                    if invalid == 'missing':
                        del changed['agent']['native_execution_result'][key]
                    else:
                        changed['agent']['native_execution_result'][key] = invalid
                    with self.assertRaises(RuntimeError):
                        verify(changed, journal)
        for key, value in (('id', 'other'), ('work_id', 'other'), ('supervisor_pid', 99)):
            changed = copy.deepcopy(journal)
            changed['expected'][key] = value
            with self.subTest(identity=key), self.assertRaises(RuntimeError):
                verify(final, changed)
        changed = copy.deepcopy(journal)
        changed['details']['extra'] = 'receipt-only'
        with self.assertRaises(RuntimeError):
            verify(final, changed)
        with self.assertRaises(RuntimeError):
            verify(final, journal, ['other', 1.0, 'estimated'])

    def test_receipt_details_preserve_json_types(self):
        """Witness first; numeric receipt flags and nested type drift refuse.

        Python equality equates booleans and 0/1; JSON-typed comparisons must not.
        This pure guard does not read a production receipt or exercise Rust.
        """
        facts = dict(kernel_reaped=True, output_complete=True, recovery_required=False,
                     workspace_release_confirmed=True, other={'nested': [1, False]})
        running = dict(id='owned', work_id='work', supervisor_pid=7, child_pid=11)
        final = dict(agent=dict(running, status='completed', native_execution_result=facts), task_accepted=False)
        journal = dict(schema_version=1, directory=[1, 2], expected=running,
                       details=copy.deepcopy(facts), incomplete_output=False, exit_code=0,
                       failed=False, interrupted=False, may_have_started=True,
                       reserved=['work', 1.0, 'reserved'])
        def verify(receipt):
            """Compare decoded synthetic JSON using the real completion guard."""
            subject.verify_completion(final, receipt, running, 'work', dict(pid=7),
                dict(pid=11), ['work', 1.0, 'estimated'], [1, 2])
        verify(journal)
        for key in ('kernel_reaped', 'output_complete', 'recovery_required', 'workspace_release_confirmed'):
            with self.subTest(numeric_receipt_flag=key):
                changed = copy.deepcopy(journal)
                changed['details'][key] = int(facts[key])
                with self.assertRaises(RuntimeError):
                    verify(changed)
        for index, invalid in ((0, True), (1, 0)):
            with self.subTest(other_nested_type=index):
                changed = copy.deepcopy(journal)
                changed['details']['other']['nested'][index] = invalid
                with self.assertRaises(RuntimeError):
                    verify(changed)

    def test_supervisor_pending_then_independent_requires_same_stamp(self):
        """Inherited spawn is allowed, but provider-ready needs same new session."""
        initial = dict(pid=11, stamp='owned', ppid=7, sid=5, pgid=5)
        conductor = dict(pid=7, sid=5)
        established = dict(initial, sid=11, pgid=11)
        with patch.object(subject, 'observation', side_effect=[initial, established]):
            self.assertEqual(subject.supervisor_session(initial, conductor, False), initial)
            self.assertEqual(subject.supervisor_session(initial, conductor, True), established)
        for current, phase in ((initial, True), (dict(established, stamp='reused'), True),
                               (dict(initial, stamp='reused'), False), (dict(established, ppid=99), True)):
            with self.subTest(current=current, established=phase), patch.object(subject, 'observation', return_value=current):
                with self.assertRaises(RuntimeError):
                    subject.supervisor_session(initial, conductor, phase)

    def test_helper_refusals_use_only_invalid_isolated_subprocesses(self):
        """Guard the four malformed argv and refuse unexpected file mutation.

        subprocess is mocked: this proves harness assertions, not CLI behavior.
        Valid helper entry/setsid is never invoked in this test process.
        """
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 'refusals'
            with patch.object(subject.subprocess, 'run', return_value=types.SimpleNamespace(returncode=2, stdout=b'', stderr=b'usage')) as run:
                subject.helper_refusals(Path('/owned-normal'), out)
            self.assertEqual(run.call_count, 4)
            for call in run.call_args_list:
                argv = call.args[0]
                self.assertEqual(argv[:2], ['/owned-normal', '_mission_worker_entry'])
                self.assertEqual(call.kwargs['cwd'], out / 'cwd')
            self.assertEqual(json.loads((out / 'result.json').read_text())['refused'],
                ['missing-agent', 'help', 'relative-root', 'without-json'])
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / 'refusals'
            def mutate(argv, cwd, capture_output):
                """Simulate a refused CLI incorrectly creating a Root directory."""
                (cwd / 'unopened-root').mkdir()
                return types.SimpleNamespace(returncode=2, stdout=b'', stderr=b'usage')
            with patch.object(subject.subprocess, 'run', side_effect=mutate):
                with self.assertRaisesRegex(RuntimeError, 'changed owned files'):
                    subject.helper_refusals(Path('/owned-normal'), out)

    def test_actual_case_publication_failure_still_closes_listener(self):
        """Actual case cleanup survives ENOSPC, retaining primary and close fault.

        First run is the publication/close witness; second injects both failures
        with identical setup. No process is created and no socket syscall runs.
        """
        for inject in (False, True):
            with self.subTest(inject=inject), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary) / 'root'
                (root / '.swarm').mkdir(parents=True)
                out = Path(temporary) / 'case'
                owned = types.SimpleNamespace(root=root)
                primary = RuntimeError('synthetic bind')
                publication = OSError(errno.ENOSPC, 'synthetic publication')
                close = OSError(errno.EIO, 'synthetic listener close')
                listener = Mock()
                listener.bind.side_effect = primary
                if inject:
                    listener.close.side_effect = close
                publish = subject.fixture.publish
                def publication_operation(path, value):
                    """Inject only failure sidecar writes; preserve owned setup."""
                    if Path(path).name == 'failure.json' and inject:
                        raise publication
                    return publish(path, value)
                with patch.object(subject.planning, 'flow', return_value=(dict(work_id='owned'), owned)), patch.object(subject.socket, 'socket', return_value=listener), patch.object(subject.fixture, 'publish', side_effect=publication_operation), patch.object(subject.subprocess, 'Popen') as spawn:
                    with self.assertRaises(BaseExceptionGroup) as raised:
                        subject.run_case(Path('/unused'), Path('/unused-sdk'), dict(sha256='owned'), {}, out, 'graceful')
                self.assertEqual(leaves(raised.exception), [primary, publication, close] if inject else [primary])
                listener.close.assert_called_once()
                spawn.assert_not_called()
                if not inject:
                    self.assertEqual(json.loads((out / 'failure.json').read_text())['status'], 'FAIL')


    def test_continuation_requires_typed_nonempty_scoped_queued_evidence(self):
        """A valid witness precedes malformed, overclaimed and changed variants."""
        facts = dict(agent_sha256='a' * 64, work_sha256='b' * 64,
                     receipt_count=1, fifo_turn_count=1, fifo_blocker_count=0)
        valid = dict(status='QUEUED_CONTINUATION_VERIFIED', target_agent_id='agent',
                     work='work', work_revision=1, settings_revision='1', generation=1,
                     target_count=1, entries=[dict(agent_id='agent', task_id='task',
                         observation=dict(state='queued-unregistered-observed', generation=1,
                                          current_lease_generation=True))],
                     before=facts, after=dict(facts), public_projections_unchanged=True,
                     agent_work_receipts_fifo_verified=True, reservation_rows_verified=False,
                     scope='public-projections-before-spawn', process_started_by_observation=False,
                     observation_is_permission=False)
        subject.verify_continuation_observation(valid, 'agent', 'work')
        variants = [None, {}, {**valid, 'target_count': True}, {**valid, 'entries': []},
                    {**valid, 'settings_revision': 1}, {**valid, 'settings_revision': '01'},
                    {**valid, 'generation': True}, {**valid, 'target_agent_id': 'other'},
                    {**valid, 'reservation_rows_verified': True},
                    {**valid, 'observation_is_permission': True},
                    {**valid, 'process_started_by_observation': True},
                    {**valid, 'before': {**facts, 'fifo_turn_count': True}},
                    {**valid, 'after': {**facts, 'receipt_count': 2}}]
        for value in variants:
            with self.subTest(value=value):
                with self.assertRaises(RuntimeError):
                    subject.verify_continuation_observation(value, 'agent', 'work')
        bad = copy.deepcopy(valid)
        bad['entries'][0]['observation']['state'] = 'registered-observed'
        with self.assertRaises(RuntimeError):
            subject.verify_continuation_observation(bad, 'agent', 'work')
        subject.verify_continuation_observation(valid, 'agent', 'work')


if __name__ == '__main__':
    unittest.main(verbosity=2)
