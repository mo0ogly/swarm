"""Synthetic diagnostic-failure retention; no tracer or other child executes.

SWARM_TRACE_TEST_PARENT is mandatory and supplied by the admitted caller.
Fake healthy/refusal/recovery records prove only error propagation, never a
real strace refusal, no-primary-execution claim or storage admission.
"""
import errno
import json
import os
from pathlib import Path
import tempfile
import subprocess
import unittest
from unittest.mock import patch

import trace_tracer_refusal_witness as candidate


class Admission:
    """Synthetic admission double, not a mount/space proof."""
    def check(self):
        """Return explicit fixture scope without external effects."""
        return {'scope': 'SYNTHETIC_NOT_STORAGE_ADMISSION'}


class RefusalDiagnosticGuard(unittest.TestCase):
    """Preserve earlier numeric refusal when a later channel hash raises EIO."""
    def setUp(self):
        """Create a private retained parent; missing explicit selection refuses."""
        self.parent = Path(tempfile.mkdtemp(prefix='tracer-diagnostic-',
                                         dir=Path(os.environ['SWARM_TRACE_TEST_PARENT'])))

    def run_case(self, name, broken_hash=False):
        """Run actual proposal with all child/positive observers replaced.

        Native fake writes controlled channels, numeric refusal1 and complete
        receipts. Only the later validation hash may raise; original receipts
        remain readable. Any actual subprocess execution is forbidden.
        """
        out = self.parent / name
        out.mkdir(mode=0o700)
        base = candidate.base
        real_sha = base.sha
        def positive(path, *args):
            """Create only a declared fake control, never execute the worker."""
            path.mkdir(mode=0o700)
            base.publish(path / 'worker.py', '# SYNTHETIC_NO_WORKER_EXECUTION\n')
            return {'scope': 'SYNTHETIC_POSITIVE_ONLY', 'case': path.name}
        def execute(argv, *, out, **kwargs):
            """Publish controlled refusal channels and attach the native receipt."""
            base.publish(out / 'tracer.stdout', '')
            base.publish(out / 'tracer.stderr', 'synthetic refusal\n')
            receipt = {'returncode': 1, 'signal': None, 'execution_attempted': True,
                       'issues': [{'type': 'ValueError', 'message': 'fixture-tracer-nonzero'}],
                       'stdout_sha256': real_sha(out / 'tracer.stdout'),
                       'stderr_sha256': real_sha(out / 'tracer.stderr')}
            base.publish(out / 'tracer-result.json', json.dumps(receipt))
            error = ValueError('fixture-tracer-nonzero')
            error.native_outcome = {'subprocess': receipt, 'admission_after': {'checked': True, 'ok': True},
                                    'receipt_publication': {'ok': True}}
            raise error
        def digest(path):
            """Inject EIO only at the consumer's later stdout-hash validation."""
            if broken_hash and Path(path).name == 'tracer.stdout':
                if broken_hash == 'interrupt':
                    raise KeyboardInterrupt('synthetic interruption after known refusal')
                raise OSError(errno.EIO, 'synthetic later channel hash')
            return real_sha(path)
        with patch.object(base, 'observe_case', side_effect=positive), \
             patch.object(base.outcomes, 'execute', side_effect=execute), \
             patch.object(base, 'sha', side_effect=digest), \
             patch.object(subprocess, 'run', side_effect=AssertionError('NO_REAL_CHILD')) as native:
            try:
                return candidate.run_tracer_refusal(out, Admission(), Path('/explicit/strace'),
                                                    Path('/explicit/python'))
            finally:
                self.assertEqual(native.call_count, 0)

    def test_later_hash_failure_retains_earlier_numeric_refusal(self):
        """Valid control before EIO, fresh recovery before asserting carrier."""
        control = self.run_case('healthy')
        self.assertEqual(control['refusal']['primary_outcome'], 'UNKNOWN')
        captured = None
        try:
            self.run_case('fault-hash-eio', broken_hash=True)
        except Exception as error:
            captured = error
        recovery = self.run_case('fresh-recovery')
        self.assertEqual(recovery['refusal']['primary_outcome'], 'UNKNOWN')
        self.assertIsNotNone(captured)
        carrier = getattr(captured, 'native_outcome', {})
        self.assertEqual(carrier.get('earlier_expected_tracer_refusal', {}).get('subprocess', {}).get('returncode'), 1,
                         'Later hash error lost earlier numeric tracer refusal from consumer carrier')
        errors = getattr(captured, 'exceptions', ())
        self.assertTrue(any(isinstance(error, OSError) and error.errno == errno.EIO for error in errors))
        retained = json.loads((self.parent / 'fault-hash-eio/refusal/tracer-error.json').read_bytes())
        self.assertEqual(retained['native_outcome']['subprocess']['returncode'], 1)

    def test_later_interrupt_retains_earlier_numeric_refusal(self):
        """Interruption remains a BaseExceptionGroup with earlier native data."""
        self.run_case('interrupt-healthy')
        captured = None
        try:
            self.run_case('fault-hash-interrupt', broken_hash='interrupt')
        except BaseException as error:
            captured = error
        self.run_case('interrupt-fresh-recovery')
        self.assertIsNotNone(captured)
        self.assertIsInstance(captured, BaseExceptionGroup)
        carrier = getattr(captured, 'native_outcome', {})
        self.assertEqual(carrier.get('earlier_expected_tracer_refusal', {}).get('subprocess', {}).get('returncode'), 1)
        self.assertTrue(any(isinstance(error, KeyboardInterrupt) for error in captured.exceptions))


if __name__ == '__main__':
    unittest.main(verbosity=2)
