"""Synthetic known-result custody at both callers; all children forbidden.

Explicit SWARM_TRACE_TEST_PARENT selects the caller-admitted temporary parent.
Known0 and expected nonzero results precede later EIO/KeyboardInterrupt; valid
control and fresh recovery surround each fault. No real trace/mount/provider.
"""
import errno
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import trace_process_witness as process
import trace_signal_witness as sigterm
from trace_outcome_custody import raise_with_custody


class Admission:
    """Inert admission double; grants or proves no actual storage rights."""
    def check(self):
        """Return explicit synthetic scope without external effects."""
        return {'scope': 'SYNTHETIC_NO_VOLUME_ADMISSION'}


class CallerCustodyGuards(unittest.TestCase):
    """Known numeric/primary facts survive a later diagnostic failure."""
    def setUp(self):
        """Retain each fixture in an explicit private caller-selected parent."""
        self.parent = Path(tempfile.mkdtemp(prefix='caller-custody-',
                                         dir=Path(os.environ['SWARM_TRACE_TEST_PARENT'])))

    def observe(self, name, kind, nonzero, fault=None):
        """Use real caller validation over exact synthetic producer records.

        Native fake returns0 or the expected numeric error. Only a later hash
        can fail. A zero-path fault occurs after matched primary trace; nonzero
        faults occur at the earlier channel-hash guard before primary parsing.
        No executable or traced worker may actually launch.
        """
        out = self.parent / name
        real_sha = process.sha
        armed = False
        rc = (-int(signal.SIGTERM) if kind == 'signal' else 7) if nonzero else 0
        def token(value):
            """Encode known fixture bytes in strace -xx grammar, not live data."""
            return '"' + ''.join('\\x%02x' % b for b in os.fsencode(value)) + '"'
        def array(values):
            """Render only the supplied synthetic strings as a complete array."""
            return '[' + ', '.join(token(v) for v in values) + ']'
        def execute(argv, *, out, **kwargs):
            """Retain synthetic channels/receipts before returning or raising."""
            nonlocal armed
            primary = argv[argv.index('--') + 1:]
            env = kwargs['env']
            terminal = '+++ killed by SIGTERM +++' if rc < 0 else f'+++ exited with {rc} +++'
            trace = 'execve(' + token(primary[0]) + ', ' + array(primary) + ', ' + array(
                [f'{k}={v}' for k, v in env.items()]) + ') = 0\n' + terminal + '\n'
            process.publish(out / 'trace.123456', trace)
            process.publish(out / 'tracer.stdout', json.dumps({'pid': 123456, 'argv': primary[1:], 'env': env}))
            process.publish(out / 'tracer.stderr', '')
            receipt = {'returncode': rc, 'signal': -rc if rc < 0 else None, 'execution_attempted': True,
                       'issues': [{'type': 'ValueError', 'message': kwargs['failure_code']}] if rc else [],
                       'stdout_sha256': real_sha(out / 'tracer.stdout'),
                       'stderr_sha256': real_sha(out / 'tracer.stderr')}
            process.publish(out / 'tracer-result.json', json.dumps(receipt))
            armed = True
            if rc:
                error = ValueError(kwargs['failure_code'])
                error.native_outcome = {'subprocess': receipt, 'receipt_publication': {'ok': True},
                                        'admission_after': {'checked': True, 'ok': True}}
                raise error
            return subprocess.CompletedProcess(argv, 0)
        def digest(path):
            """Fail only after native completion at the selected real consumer."""
            target = 'tracer.stdout' if nonzero else 'trace.123456'
            if armed and fault and Path(path).name == target:
                if fault == 'interrupt':
                    raise KeyboardInterrupt('synthetic post-result interrupt')
                raise OSError(errno.EIO, 'synthetic post-result hash')
            return real_sha(path)
        with patch.object(process.outcomes, 'execute', side_effect=execute), \
             patch.object(process, 'sha', side_effect=digest), \
             patch.object(subprocess, 'run', side_effect=AssertionError('NO_REAL_CHILD')) as native:
            try:
                if kind == 'signal':
                    return sigterm.observe(out, Admission(), Path('/explicit/strace'), Path('/explicit/python'),
                                           'signal' if nonzero else 'control')
                return process.observe_case(out, Admission(), Path('/explicit/strace'), Path('/explicit/python'),
                                            rc, 'INERT_CALLER_CUSTODY')
            finally:
                self.assertEqual(native.call_count, 0)

    def exercise(self, kind, nonzero, fault):
        """Control→fault→fresh recovery, then assert retained consumer facts."""
        self.observe('control', kind, nonzero)
        captured = None
        try:
            self.observe('fault', kind, nonzero, fault)
        except BaseException as error:
            captured = error
        self.observe('fresh-recovery', kind, nonzero)
        self.assertIsNotNone(captured)
        expected = (-int(signal.SIGTERM) if kind == 'signal' else 7) if nonzero else 0
        carrier = getattr(captured, 'native_outcome', {})
        self.assertEqual(carrier.get('subprocess', {}).get('returncode'), expected,
                         'Known tracer result disappeared at the consumer')
        primary = carrier.get('primary_observation', {})
        if nonzero:
            self.assertEqual(primary.get('status'), 'UNKNOWN')
        else:
            self.assertEqual(primary.get('status'), 'OBSERVED')
            self.assertEqual(primary.get('exit_code'), 0)
            self.assertEqual(primary.get('pid'), 123456)
        seen = set()
        def contains(error):
            """Inspect typed causes/groups once; source metadata is not executed."""
            if id(error) in seen:
                return False
            seen.add(id(error))
            if fault == 'interrupt' and isinstance(error, KeyboardInterrupt):
                return True
            if fault == 'eio' and isinstance(error, OSError) and error.errno == errno.EIO:
                return True
            return any(contains(x) for x in getattr(error, 'exceptions', ())) or any(
                contains(x) for x in [error.__cause__, error.__context__] if x is not None)
        self.assertTrue(contains(captured), 'Original later failure category was lost')

    def test_process_known_seven_then_eio(self):
        """Retain expected7 while late EIO leaves primary unknown."""
        self.exercise('process', True, 'eio')

    def test_process_known_seven_then_interrupt(self):
        """Retain expected7 and late interruption without inventing primary."""
        self.exercise('process', True, 'interrupt')

    def test_process_known_zero_then_eio(self):
        """Retain known0 and matched primary when final hash fails."""
        self.exercise('process', False, 'eio')

    def test_process_known_zero_then_interrupt(self):
        """Retain known0/matched primary through late interruption."""
        self.exercise('process', False, 'interrupt')

    def test_signal_known_negative_then_eio(self):
        """Retain known negative tracer result before primary parsing."""
        self.exercise('signal', True, 'eio')

    def test_signal_known_negative_then_interrupt(self):
        """Retain native negative result and interruption independently."""
        self.exercise('signal', True, 'interrupt')

    def test_signal_known_zero_then_eio(self):
        """Retain known0/matched control at the signal caller's final hash."""
        self.exercise('signal', False, 'eio')

    def test_signal_known_zero_then_interrupt(self):
        """Retain known0/matched control despite late interruption."""
        self.exercise('signal', False, 'interrupt')

    def test_helper_unknown_same_object_does_not_invent_zero_or_self_cause(self):
        """Unknown facts stay unknown; reusing the error object creates no cycle."""
        control = KeyboardInterrupt('synthetic known control')
        with self.assertRaises(KeyboardInterrupt) as known:
            raise_with_custody(control, tracer_result={'returncode': 0}, primary_observation=None)
        self.assertIs(known.exception, control)
        self.assertEqual(control.native_outcome['subprocess']['returncode'], 0)
        fault = KeyboardInterrupt('synthetic unknown same object')
        with self.assertRaises(KeyboardInterrupt) as unknown:
            raise_with_custody(fault, tracer_result=None, earlier_failure=fault)
        self.assertIs(unknown.exception, fault)
        carrier = fault.native_outcome
        self.assertIsNone(carrier['subprocess'].get('returncode'))
        self.assertIsNone(carrier['subprocess'].get('execution_attempted'))
        self.assertFalse(carrier['primary_observation']['known'])
        self.assertIsNone(carrier['primary_observation'].get('pid'))
        self.assertIsNone(fault.__cause__)
        json.dumps(process.outcomes.failure_document(fault))
        recovery = KeyboardInterrupt('synthetic fresh control')
        with self.assertRaises(KeyboardInterrupt):
            raise_with_custody(recovery, tracer_result={'returncode': 0})
        self.assertIsNone(fault.native_outcome['subprocess'].get('returncode'))

    def test_helper_preserves_distinct_existing_cause_and_nested_native_record(self):
        """Grouping keeps existing cause, earlier numeric error and old metadata."""
        earlier = ValueError('synthetic earlier numeric')
        earlier.native_outcome = {'subprocess': {'returncode': 7}}
        control = ValueError('synthetic control')
        with self.assertRaises(ValueError) as healthy:
            raise_with_custody(control, tracer_result={'returncode': 7}, earlier_failure=earlier)
        self.assertIs(healthy.exception.__cause__, earlier)
        existing = OSError(errno.EIO, 'synthetic existing cause')
        later = ValueError('synthetic following validation')
        later.__cause__ = existing
        later.native_outcome = {'scope': 'synthetic-inner-record', 'subprocess': {'returncode': 9}}
        with self.assertRaises(ExceptionGroup) as caught:
            raise_with_custody(later, tracer_result={'returncode': 7},
                               native_receipt=earlier.native_outcome, earlier_failure=earlier)
        self.assertEqual(caught.exception.exceptions, (later, earlier))
        self.assertIs(later.__cause__, existing)
        carrier = caught.exception.native_outcome
        self.assertEqual(carrier['subprocess']['returncode'], 7)
        self.assertEqual(carrier['following_failure']['native_outcome']['subprocess']['returncode'], 9)
        self.assertEqual(carrier['earlier_failure']['native_outcome']['subprocess']['returncode'], 7)
        json.dumps(process.outcomes.failure_document(caught.exception))
        recovery = ValueError('synthetic fresh control')
        with self.assertRaises(ValueError):
            raise_with_custody(recovery, tracer_result={'returncode': 0})
        self.assertIs(later.__cause__, existing)


if __name__ == '__main__':
    unittest.main(verbosity=2)
