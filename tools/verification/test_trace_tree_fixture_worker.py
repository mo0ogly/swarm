"""Synthetic consumer/cleanup failures after known child facts.

Requires SWARM_TRACE_TEST_PARENT. No real child or tracer is permitted. The
fresh caller-owned fixture is retained; metadata/output faults are injected.
"""
import errno
import hashlib
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import trace_tree_fixture_worker as worker


class TreeReceiptGuards(unittest.TestCase):
    """Retain known numeric child facts across output and unlink failures."""

    def _fixture(self):
        """Create one owned fixture and a synthetic successful child record."""
        root = Path(tempfile.mkdtemp(prefix='tree-custody-',
                                     dir=os.environ['SWARM_TRACE_TEST_PARENT']))
        content = b'OWNED_TREE_CUSTODY_FIXTURE\n'
        (root / 'seed').write_bytes(content)
        receipt = {'pid': 123, 'returncode': 0, 'source': 'synthetic child fixture'}
        child = {'receipt': receipt,
                 'report': {'file': {'sha256': hashlib.sha256(content).hexdigest()}}}
        return root, receipt, child

    def _healthy(self, role):
        """Run a fresh healthy control/recovery with no real subprocess."""
        root, receipt, child = self._fixture()
        cwd = Path.cwd()
        try:
            with patch.object(worker, '_spawn', return_value=child), \
                    patch.object(worker.subprocess, 'Popen', side_effect=AssertionError('NO_CHILD')), \
                    patch('builtins.print') as output:
                worker.main(['--root', str(root), '--python', '/synthetic/python',
                             '--role', role])
            result = json.loads(output.call_args.args[0])
            self.assertEqual(result['role'], role)
            self.assertEqual(result['child']['receipt'], receipt)
            self.assertTrue((root / 'seed').is_file())
            self.assertFalse((root / 'renamed').exists())
        finally:
            os.chdir(cwd)

    def test_nested_cleanup_preserves_spawn_receipt(self):
        """An added close failure must not hide the body's known spawn facts."""
        root, receipt, _ = self._fixture()
        with worker._owned_fd(os.open(root / 'seed', os.O_RDONLY)):
            pass
        original = OSError(errno.EINVAL, 'synthetic body')
        original.fixture_child_receipt = receipt
        close = os.close
        def close_then_fail(fd):
            """Release the owned fd before injecting a synthetic close error."""
            close(fd)
            raise OSError(errno.EIO, 'synthetic close')
        caught = None
        with patch.object(worker.os, 'close', side_effect=close_then_fail):
            try:
                with worker._owned_fd(os.open(root / 'seed', os.O_RDONLY)):
                    raise original
            except BaseExceptionGroup as error:
                caught = error
        with worker._owned_fd(os.open(root / 'seed', os.O_RDONLY)):
            pass
        self.assertIsNotNone(caught)
        self.assertIs(caught.exceptions[0], original)
        self.assertEqual(caught.exceptions[1].errno, errno.EIO)
        self.assertEqual(getattr(caught, 'fixture_child_receipt', None), receipt)

    def test_spawn_wait_error_preserves_already_known_exit(self):
        """A wait wrapper error after returncode becomes known cannot erase it."""
        def run(inject):
            """Use a local Popen double; no actual process or signal occurs."""
            root, _, _ = self._fixture()
            fake = SimpleNamespace(pid=456, returncode=None)
            def factory(*args, **kwargs):
                """Write a valid controlled marker before the synthetic wait."""
                kwargs['stdout'].write(json.dumps({'pid':456,'role':'grandchild'}).encode())
                return fake
            def wait():
                """Know zero before optionally injecting a later wrapper error."""
                fake.returncode = 0
                if inject:
                    raise OSError(errno.EIO, 'synthetic wait after known exit')
                return 0
            fake.wait = wait
            fake.kill = lambda: self.fail('already completed child must not be signalled')
            with patch.object(worker.subprocess, 'Popen', side_effect=factory):
                return worker._spawn(root, Path('/synthetic/python'), 'grandchild')
        self.assertEqual(run(False)['receipt']['returncode'], 0)
        caught = None
        try:
            run(True)
        except OSError as error:
            caught = error
        self.assertEqual(run(False)['receipt']['returncode'], 0)
        self.assertIsNotNone(caught)
        self.assertEqual(getattr(caught, 'fixture_child_receipt', {}).get('returncode'), 0)

    def test_deferred_stdout_flush_preserves_child_receipt(self):
        """Successful buffered writes followed by flush refusal retain facts."""
        class DeferredStream:
            """Owned in-memory stream; only flush can inject the synthetic fault."""
            def __init__(self, inject):
                """Start empty with an explicitly selected fixture fault."""
                self.text, self.inject = '', inject
            def write(self, value):
                """Accept all text, imitating successful buffered writes."""
                self.text += value
                return len(value)
            def flush(self):
                """Refuse only the selected flush; no real stdout is changed."""
                if self.inject:
                    raise OSError(errno.EIO, 'synthetic buffered flush')
        def run(inject):
            """Invoke parent using one fresh fixture and a Popen-free child double."""
            root, receipt, child = self._fixture()
            stream = DeferredStream(inject)
            with patch.object(worker, '_spawn', return_value=child), \
                    patch.object(worker.subprocess, 'Popen', side_effect=AssertionError('NO_CHILD')), \
                    patch.object(worker.sys, 'stdout', stream):
                worker.main(['--root', str(root), '--python', '/synthetic/python',
                             '--role', 'parent'])
            return json.loads(stream.text), receipt
        control, receipt = run(False)
        self.assertEqual(control['child']['receipt'], receipt)
        caught = None
        try:
            run(True)
        except OSError as error:
            caught = error
        recovery, _ = run(False)
        self.assertEqual(recovery['child']['receipt'], receipt)
        self.assertIsNotNone(caught, 'stdout flush escaped the protected receipt boundary')
        self.assertEqual(getattr(caught, 'fixture_role_receipt', {}).get('known_child'), receipt)

    def test_parent_print_failure_preserves_child_zero(self):
        """A late stdout failure must retain a returned child's known zero."""
        self._healthy('parent')
        root, receipt, child = self._fixture()
        caught = None
        with patch.object(worker, '_spawn', return_value=child), \
                patch.object(worker.subprocess, 'Popen', side_effect=AssertionError('NO_CHILD')), \
                patch('builtins.print', side_effect=OSError(errno.EIO, 'injected stdout')):
            try:
                worker.main(['--root', str(root), '--python', '/synthetic/python',
                             '--role', 'parent'])
            except OSError as error:
                caught = error
        self.assertIsNotNone(caught)
        self._healthy('parent')
        self.assertEqual(getattr(caught, 'fixture_role_receipt', {}).get('known_child'), receipt)

    def test_child_unlink_failure_preserves_grandchild_zero(self):
        """A late unlink refusal keeps the completed grandchild's known zero."""
        self._healthy('child')
        root, receipt, child = self._fixture()
        cwd = Path.cwd()
        caught = None
        try:
            with patch.object(worker, '_spawn', return_value=child), \
                    patch.object(worker.subprocess, 'Popen', side_effect=AssertionError('NO_CHILD')), \
                    patch.object(worker.os, 'unlink', side_effect=OSError(errno.EIO, 'injected unlink')):
                try:
                    worker.main(['--root', str(root), '--python', '/synthetic/python',
                                 '--role', 'child'])
                except OSError as error:
                    caught = error
        finally:
            os.chdir(cwd)
        self.assertIsNotNone(caught)
        self._healthy('child')
        self.assertEqual(getattr(caught, 'fixture_role_receipt', {}).get('known_child'), receipt)
        self.assertTrue((root / 'renamed').is_file(), 'partial evidence must remain')


if __name__ == '__main__':
    unittest.main(verbosity=2)
