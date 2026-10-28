"""Actual descriptor regression for observer setup refusal; no agent is executed."""
import errno
import os
import tempfile
import subprocess
import signal
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import rust_mission_watch_registered as probe


class ObserverResources(unittest.TestCase):
    """Healthy allocation precedes second-spool refusal and recovery, using real FDs."""
    def test_setup_refusal_closes_notify_and_already_opened_spool(self):
        """Keep the primary failure and close partial resources before propagating it.

        The synthetic Popen sentinel terminates the healthy setup without a real
        child; this is descriptor evidence only, not signal or lifecycle proof.
        """
        for fault in (False, True, False):
            with self.subTest(second_spool_refusal=fault), tempfile.TemporaryDirectory() as tmp:
                reader, writer = os.pipe()
                streams = []
                expected = 'fixture-second-spool' if fault else 'fixture-no-process'
                def spool(path):
                    """Open an owned spool or refuse its second open before creation."""
                    if fault and path.name == 'watch.stderr':
                        raise OSError(expected)
                    stream = path.open('wb'); streams.append(stream); return stream
                owned = SimpleNamespace(ok=lambda argv: {'agent': {}}, work='fixture-work', environment={}, root=Path(tmp))
                try:
                    with patch.object(probe.worker, 'watch_directory', return_value=reader), patch.object(probe.worker, 'spool', side_effect=spool), patch.object(probe.completion, 'one_turn', return_value={'state': 'acquired'}), patch.object(probe.completion, 'reservation', return_value=['reserved', 1]), patch.object(probe.relay, 'rows', return_value=[]), patch.object(probe.subprocess, 'Popen', side_effect=OSError(expected)):
                        with self.assertRaises(BaseException) as failure:
                            probe.observe(Path('/fixture/binary'), owned, Path(tmp) / 'observer', 'fixture-agent')
                    def causes(error):
                        """Walk only synthetic fixture exceptions to check the retained cause."""
                        return [str(error)] + [text for child in getattr(error, 'exceptions', ()) for text in causes(child)]
                    self.assertTrue(any(expected in text for text in causes(failure.exception)))
                    with self.assertRaises(OSError) as closed:
                        os.fstat(reader)
                    self.assertEqual(closed.exception.errno, errno.EBADF)
                    self.assertTrue(all(stream.closed for stream in streams))
                finally:
                    for stream in streams:
                        stream.close()
                    for fd in (reader, writer):
                        try:
                            os.close(fd)
                        except OSError as error:
                            if error.errno != errno.EBADF:
                                raise

    def test_pidfd_refusal_reaps_only_the_actual_owned_watch(self):
        """A real idle Python child witnesses pidfd support before its injected refusal.

        No Swarm or provider runs. Popen is the test's own child handle, never an
        identifier from storage; original error and its actual reap both matter.
        """
        with tempfile.TemporaryDirectory() as tmp:
            child = subprocess.Popen([sys.executable, '-c', "import signal; print('READY',flush=True); signal.pause()"], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            reader, writer = os.pipe()
            try:
                self.assertEqual(child.stdout.readline(), b'READY\n')
                witness = os.pidfd_open(child.pid); os.close(witness)
                owned = SimpleNamespace(ok=lambda argv: {'agent': {}}, work='fixture-work', environment={}, root=Path(tmp))
                with patch.object(probe.worker, 'watch_directory', return_value=reader), patch.object(probe.completion, 'one_turn', return_value={'state': 'acquired'}), patch.object(probe.completion, 'reservation', return_value=['reserved', 1]), patch.object(probe.relay, 'rows', return_value=[]), patch.object(probe.subprocess, 'Popen', return_value=child), patch.object(probe.os, 'pidfd_open', side_effect=OSError('fixture-pidfd')):
                    with self.assertRaises(BaseException) as failure:
                        probe.observe(Path('/fixture/binary'), owned, Path(tmp) / 'observer', 'fixture-agent')
                self.assertTrue(any('fixture-pidfd' in str(e) for e in failure.exception.exceptions))
                self.assertIsNotNone(child.poll(), 'owned child left alive after pidfd refusal')
                with self.assertRaises(ChildProcessError):
                    os.waitpid(child.pid, os.WNOHANG)
            finally:
                if child.poll() is None:
                    child.send_signal(signal.SIGTERM)
                child.wait(); child.stdout.close()
                for fd in (reader, writer):
                    try:
                        os.close(fd)
                    except OSError as error:
                        if error.errno != errno.EBADF:
                            raise
