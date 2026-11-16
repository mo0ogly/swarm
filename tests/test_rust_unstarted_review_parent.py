"""Three PROPOSED behavioral regressions: controlled doubles, never OS proof.

DO NOT RUN without root allocation of interpreter and assembled source/import
closure (including fixture_area import effects). No campaign/CLI/provider is
called. Future comparison uses this same test file against exact parent12/13 and14
assemblies, retaining named RED/GREEN output and actual imported paths separately.
"""
import io
import json
import os
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch
import rust_unstarted_review_parent as parent
import rust_unstarted_review_process as process


class OwnershipRegression(unittest.TestCase):
    """Check consumer behavior with a healthy control before each fault."""

    def test_observer_stops_before_next_root(self):
        """A healthy W/F read precedes W handback; F must not start on failure.

        Doubled show/ledger/raw reads perform no public command or Root IO. A
        prior public W fact survives a later ledger handback; unread F is unknown.
        """
        campaign = parent.Campaign.__new__(parent.Campaign)
        campaign.phase, campaign.output = 'controlled-observation', Path('/unused')
        campaign.fixtures = {tag: dict(spec=dict(root='/unused/'+tag, work=tag), settings={}) for tag in ('W', 'F')}
        calls = []
        campaign.show = Mock(side_effect=lambda tag, name: (calls.append(tag) or dict(id=tag, revision=1)))
        with patch.object(parent, 'ledger'), patch.object(Path, 'read_bytes', return_value=b'{}'):
            healthy = campaign.failure_observations()
        self.assertEqual(calls, ['W', 'F'])
        self.assertEqual([v['fixture'] for v in healthy.facts['effects']], ['W', 'F'])
        owner = process.OwnedRun('/unused', [], {})
        owner.process = SimpleNamespace(pid=42, returncode=None)
        original = process.Failure('controlled-pending-owner', owner=owner)
        calls.clear()
        with patch.object(parent, 'ledger', side_effect=original), patch.object(Path, 'read_bytes', return_value=b'{}'):
            try:
                result = campaign.failure_observations()
            except process.Failure as error:
                result = error
        self.assertEqual(calls, ['W'], 'No next Root command before pending owner settlement')
        self.assertIn(owner, process.owners(result))
        self.assertIn(original, result.causes)
        self.assertEqual(result.facts['effects'][0]['observed_work'], 'W')
        self.assertIn('F', result.facts['unread_roots'])

    def test_close_refusal_is_not_closed_or_retried(self):
        """Healthy terminal precedes no-child and reaped-child close handback.

        The pause double records actual owner/cause and state BEFORE return,
        then models an explicit in-memory receiver resolving only fixture facts.
        No OS close/arbitration is inferred. No assertions raise inside pause:
        the controlled release makes this test finite without threads/timeouts.
        """
        for child_reaped in (False, True):
            with self.subTest(child_reaped=child_reaped), patch.object(process.os, 'fsync'), patch.object(Path, 'read_bytes', return_value=b''):
                healthy = process.OwnedRun('/unused', [], {})
                stream = Mock(closed=False)
                stream.close.side_effect = lambda: setattr(stream, 'closed', True)
                healthy.streams = [stream]
                if child_reaped:
                    healthy.process = SimpleNamespace(pid=42, returncode=0)
                    healthy.consumed_wait = SimpleNamespace(si_pid=42, si_code=os.CLD_EXITED, si_status=0)
                    healthy.reaped, healthy.wait_validated, healthy.status = True, True, 0
                healthy.finish_capture()
                healthy_terminal = io.StringIO()
                with patch.object(process.sys, 'stderr', healthy_terminal), patch.object(process.signal, 'pause') as healthy_pause:
                    status = process.report_terminal(process.Failure('controlled-settled', owner=healthy), None)
                self.assertEqual(status, 1)
                healthy_pause.assert_not_called()
                self.assertFalse(healthy.summary()['streams_held'])
                stream.close.assert_called_once()
                original = KeyboardInterrupt()
                failed = process.OwnedRun('/unused', [], {})
                if child_reaped:
                    failed.process = SimpleNamespace(pid=42, returncode=0)
                    failed.consumed_wait = SimpleNamespace(si_pid=42, si_code=os.CLD_EXITED, si_status=0)
                    failed.reaped, failed.wait_validated, failed.status = True, True, 0
                blocked = Mock(closed=False)
                blocked.close.side_effect = original
                failed.streams = [blocked]
                failed.finish_capture()
                self.assertIsNot(failed.summary()['streams_held'], False)
                self.assertFalse(failed.finalized)
                failed.finish_capture()
                observer, terminal, receipts, events = Mock(), io.StringIO(), [], []
                def receive_fixture_closure():
                    """Receive actual fixture owner before synthetic memory settlement.

                    Retain original exception, held stream and pre-decision facts;
                    this does not close or retry any OS handle.
                    """
                    events.append('held')
                    receipts.append((failed, original, failed.streams[0], failed.summary()))
                    blocked.closed = True
                    failed.stream_closures[0].update(closed_observed=True, state='closed')
                    failed.finalized = True
                with patch.object(process.sys, 'stderr', terminal), patch.object(process.signal, 'pause', side_effect=receive_fixture_closure) as pause:
                    status = process.report_terminal(process.Failure('controlled-close-failure', [original], failed), None, observer)
                    events.append('returned')
                self.assertEqual(events, ['held', 'returned'])
                pause.assert_called_once()
                self.assertEqual(status, 1)
                self.assertIs(receipts[0][0], failed)
                self.assertIs(receipts[0][1], original)
                self.assertIs(receipts[0][2], blocked)
                self.assertIsNot(receipts[0][3]['streams_held'], False)
                self.assertIn(original, failed.errors)
                observer.assert_not_called()
                blocked.close.assert_called_once()
                codes = [node.get('code') for node in json.loads(terminal.getvalue())['causes']]
                self.assertIn('parent-capture-closure-pending', codes)

    def test_consumption_survives_validation_refusal(self):
        """Healthy wait then mismatch/invalid identity/status never consume twice.

        waitid is a controlled double, not a child/kernel observation. Raw consumed
        identity/code/status must survive refusal, while invalid PID never reaps
        the owned child and unknown status never becomes success/returncode zero.
        """
        peek = SimpleNamespace(si_pid=42, si_code=os.CLD_EXITED, si_status=0)
        variants = [
            (SimpleNamespace(si_pid=42, si_code=os.CLD_EXITED, si_status=7), True, 7),
            (SimpleNamespace(si_pid=43, si_code=os.CLD_EXITED, si_status=0), False, None),
            (SimpleNamespace(si_pid=42, si_code=os.CLD_EXITED, si_status=-1), True, None),
        ]
        for receipt, owned_reap, known_status in variants:
            with self.subTest(pid=receipt.si_pid, status=receipt.si_status):
                healthy = process.OwnedRun('/unused', [], {})
                healthy.process = SimpleNamespace(pid=42, returncode=None)
                with patch.object(process.os, 'waitid', side_effect=[peek, peek]) as wait:
                    healthy.wait_kernel()
                    self.assertTrue(healthy.reaped)
                    self.assertEqual(healthy.status, 0)
                    self.assertEqual(wait.call_count, 2)
                with patch.object(Path, 'read_bytes', return_value=b''), patch.object(process.sys, 'stderr', io.StringIO()), patch.object(process.signal, 'pause') as healthy_pause:
                    status = process.report_terminal(process.Failure('controlled-valid-consumption', owner=healthy), None)
                self.assertEqual(status, 1)
                healthy_pause.assert_not_called()
                failed = process.OwnedRun('/unused', [], {})
                failed.process = SimpleNamespace(pid=42, returncode=None)
                with patch.object(process.os, 'waitid', side_effect=[peek, receipt]) as wait:
                    with self.assertRaises(process.Failure) as refused:
                        failed.wait_kernel()
                    # State/receipt must remain usable even after rejection.
                    with self.assertRaises(BaseException) as settlement:
                        failed.settle()
                    self.assertEqual(wait.call_count, 2, 'Rejected validation must not consume again')
                self.assertIs(failed.consumed_wait, receipt)
                self.assertEqual(failed.reaped, owned_reap)
                self.assertEqual(failed.status, known_status)
                self.assertFalse(failed.wait_validated)
                self.assertEqual(failed.summary()['consumed_wait']['status'], receipt.si_status)
                if receipt.si_pid == 42 and receipt.si_status == 7:
                    terminal, events, receipts = io.StringIO(), [], []
                    observer = Mock()
                    def receive_fixture_validation():
                        """Retain raw receipt before a synthetic receiver decision.

                        Only the doubled peek/consume disagreement is acknowledged
                        in memory. No wait, receipt replacement or OS arbitration.
                        """
                        events.append('held')
                        receipts.append((failed, failed.consumed_wait, failed.summary()))
                        failed.wait_validated = True
                    with patch.object(Path, 'read_bytes', return_value=b''), patch.object(process.sys, 'stderr', terminal), patch.object(process.os, 'waitid') as no_wait, patch.object(process.signal, 'pause', side_effect=receive_fixture_validation) as pause:
                        status = process.report_terminal(process.Failure('controlled-consume-refusal', [refused.exception, settlement.exception], failed), None, observer)
                        events.append('returned')
                    self.assertEqual(events, ['held', 'returned'])
                    pause.assert_called_once()
                    no_wait.assert_not_called()
                    observer.assert_not_called()
                    self.assertEqual(status, 1)
                    self.assertIs(receipts[0][0], failed)
                    self.assertIs(receipts[0][1], receipt)
                    self.assertEqual(receipts[0][2]['consumed_wait']['status'], 7)
                    self.assertFalse(receipts[0][2]['wait_validated'])
                    self.assertTrue(receipts[0][2]['reap_confirmed'])
                    self.assertIs(failed.consumed_wait, receipt)
                    codes = [node.get('code') for node in json.loads(terminal.getvalue())['causes']]
                    self.assertIn('parent-reap-observation-changed', codes)
                    self.assertIn('parent-consumed-wait-unvalidated', codes)


if __name__ == '__main__':
    print(json.dumps(dict(scope='CONTROLLED_DOUBLES_ONLY', modules={m.__name__: m.__file__ for m in (parent, process)})), flush=True)
    unittest.main(verbosity=2)
