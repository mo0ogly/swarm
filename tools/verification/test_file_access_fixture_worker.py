"""Controlled metadata-drift refusal guard, without any child or strace.

SWARM_TRACE_TEST_PARENT is mandatory. Only the newly created caller-owned
fixture is opened; metadata differences are injected, not actual disk faults.
"""
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import file_access_fixture_worker as worker


class WorkerMetadataGuard(unittest.TestCase):
    """Healthy access, injected timestamp drift, fresh healthy recovery."""
    def test_metadata_drift_refuses_even_when_size_unchanged(self):
        """A changed snapshot must not become a successful open-only result."""
        parent = Path(os.environ['SWARM_TRACE_TEST_PARENT'])
        root = Path(tempfile.mkdtemp(prefix='file-metadata-guard-', dir=parent))
        fixture = root / 'input'
        fixture.write_bytes(b'OWNED_INERT_FILE_ACCESS_FIXTURE\n')
        os.chmod(fixture, 0o400)
        control = worker._run(str(fixture), 'open')
        self.assertEqual(control['before'], control['after'])
        self.assertIsNone(control['sha256'])
        st = fixture.stat()
        self.assertTrue(stat.S_ISREG(st.st_mode))
        before = {'dev': st.st_dev, 'ino': st.st_ino, 'size': st.st_size,
                  'mtime_ns': st.st_mtime_ns, 'ctime_ns': st.st_ctime_ns}
        after = dict(before, mtime_ns=st.st_mtime_ns + 1)
        caught = None
        with patch.object(worker, '_snapshot', side_effect=[(st, before), (st, after)]), \
             patch.object(subprocess, 'run', side_effect=AssertionError('NO_REAL_CHILD')) as native:
            try:
                worker._run(str(fixture), 'open')
            except ValueError as error:
                caught = error
            self.assertEqual(native.call_count, 0)
        recovery = worker._run(str(fixture), 'open')
        self.assertEqual(recovery['before'], recovery['after'])
        self.assertEqual(fixture.read_bytes(), b'OWNED_INERT_FILE_ACCESS_FIXTURE\n')
        self.assertIsNotNone(caught, 'Timestamp drift was published as a successful fixture access')


if __name__ == '__main__':
    unittest.main(verbosity=2)
