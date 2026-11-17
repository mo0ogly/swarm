"""Synthetic error-composition guards; no tracer or other child may execute.

The caller must explicitly set SWARM_TRACE_TEST_PARENT to an admitted temporary
parent with sufficient space. These tests exercise no storage/mount admission.
They prove only the named error contract, not real strace behavior or a build.
"""
import errno
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import trace_process_witness as witness


class Admission:
    """Inert check double: never evidence of a mounted or qualified volume."""

    def check(self):
        """Return an explicitly synthetic observation without any side effect."""
        return {'scope': 'SYNTHETIC_NO_STORAGE_ADMISSION'}


class ExpectedFailureGuards(unittest.TestCase):
    """Require a valid control, an extra-cause refusal and a fresh recovery."""

    def setUp(self):
        """Create a private fixture in the caller-selected parent; no fallback."""
        parent = Path(os.environ['SWARM_TRACE_TEST_PARENT'])
        self.root = Path(tempfile.mkdtemp(prefix='trace-error-guard-', dir=parent))

    def case(self, name, code, extra=False):
        """Emit exact synthetic no-fork trace/marker/receipt, with optional EIO.

        The actual native executor and all subprocess.run calls are replaced;
        an expected exit7 carries its normal error plus optional cleanup cause.
        Every synthetic byte remains in a distinct directory after completion.
        """
        def execute(argv, *, out, name, **kwargs):
            primary = argv[argv.index('--') + 1:]
            environment = kwargs['env']
            def token(value):
                return '"' + ''.join('\\x%02x' % b for b in os.fsencode(value)) + '"'
            def array(values):
                return '[' + ', '.join(token(v) for v in values) + ']'
            text = ('execve(' + token(primary[0]) + ', ' + array(primary) + ', ' +
                    array([f'{k}={v}' for k, v in environment.items()]) + ') = 0\n' +
                    f'exit_group({code}) = ?\n+++ exited with {code} +++\n')
            witness.publish(out / 'trace.123456', text)
            witness.publish(out / 'tracer.stdout', json.dumps({'pid': 123456, 'argv': primary[1:],
                                                            'env': environment}))
            witness.publish(out / 'tracer.stderr', '')
            issue = {'type': 'ValueError', 'message': 'fixture-tracer-nonzero'}
            receipt = {'returncode': code, 'signal': None, 'execution_attempted': True,
                       'issues': [issue] if code else [], 'stdout_sha256': witness.sha(out / 'tracer.stdout'),
                       'stderr_sha256': witness.sha(out / 'tracer.stderr')}
            if extra:
                receipt['issues'].append({'type': 'OSError', 'message': 'synthetic cleanup EIO'})
            witness.publish(out / 'tracer-result.json', json.dumps(receipt))
            if code:
                failure = ValueError('fixture-tracer-nonzero')
                error = ExceptionGroup('synthetic expected plus cleanup', [failure, OSError(errno.EIO, 'cleanup')]) if extra else failure
                error.native_outcome = {'subprocess': receipt, 'admission_after': {'checked': True, 'ok': True},
                                        'receipt_publication': {'ok': True}}
                raise error
            return subprocess.CompletedProcess(argv, code)
        with patch.object(witness.outcomes, 'execute', side_effect=execute), \
             patch.object(subprocess, 'run', side_effect=AssertionError('NO_REAL_CHILD')) as native:
            try:
                return witness.observe_case(self.root / name, Admission(), Path('/explicit/strace'),
                                            Path('/explicit/python'), code, 'DECLARED_INERT_NONCE')
            finally:
                self.assertEqual(native.call_count, 0)

    def test_expected_exit_does_not_mask_extra_cleanup(self):
        """A clean expected exit7 is valid; exit7 plus EIO must remain refused."""
        self.case('control-zero', 0)
        self.case('control-seven', 7)
        refused = None
        try:
            self.case('fault-seven-plus-cleanup', 7, extra=True)
        except ValueError as error:
            refused = error
        self.case('fresh-recovery', 0)
        self.assertIsNotNone(refused, 'Expected exit7 masked an additional cleanup error')
        self.assertIn('trace-fixture-unexpected-native-issues', str(refused))
        self.assertIsNotNone(refused.__cause__, 'Original native cause must remain chained')

    def test_receipt_write_and_close_keep_both_causes(self):
        """An ENOSPC write plus EIO close retains both and the partial file.

        This exercises the actual exclusive publisher with a controlled stream,
        no child. Healthy publication precedes injection; fresh recovery must
        preserve the original partial bytes and refuse overwriting that file.
        """
        healthy = self.root / 'healthy.json'
        witness.publish(healthy, '{"control": true}\n')
        self.assertEqual(json.loads(healthy.read_bytes()), {'control': True})
        fault = self.root / 'partial.json'

        class BrokenStream:
            """Own the real fd, write one byte, then model write/close failures."""

            def __init__(self, descriptor):
                self.descriptor = descriptor

            def write(self, value):
                """Leave a visible prefix then raise controlled ENOSPC."""
                os.write(self.descriptor, value.encode()[:1])
                raise OSError(errno.ENOSPC, 'synthetic write')

            def close(self):
                """Release the owned descriptor before reporting controlled EIO."""
                os.close(self.descriptor)
                raise OSError(errno.EIO, 'synthetic close after descriptor release')

        with patch.object(witness.os, 'fdopen', side_effect=lambda fd, *a, **k: BrokenStream(fd)), \
             patch.object(subprocess, 'run', side_effect=AssertionError('NO_REAL_CHILD')) as native:
            with self.assertRaises(ExceptionGroup) as caught:
                witness.publish(fault, '{"not_complete": true}\n')
            self.assertEqual(native.call_count, 0)
        self.assertEqual([error.errno for error in caught.exception.exceptions], [errno.ENOSPC, errno.EIO])
        original = fault.read_bytes()
        self.assertEqual(original, b'{')
        recovery = self.root / 'fresh-recovery.json'
        witness.publish(recovery, '{"recovered": true}\n')
        self.assertEqual(json.loads(recovery.read_bytes()), {'recovered': True})
        self.assertEqual(fault.read_bytes(), original)
        with self.assertRaises(FileExistsError):
            witness.publish(fault, '{"overwrite": true}\n')
        self.assertEqual(fault.read_bytes(), original)


if __name__ == '__main__':
    unittest.main(verbosity=2)
