"""Check live progress, refusal preservation and exclusive evidence creation."""
import os
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from recorded_process import capture


class RecordedProcessTests(unittest.TestCase):
    """Owned subprocess fixtures only; no server, provider or timing threshold."""

    def test_progress_is_visible_before_child_exit_and_nonzero_result_is_preserved(self):
        """A descriptor handshake proves observation precedes completion, without sleeping."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ready_read, ready_write = os.pipe()
            ack_read, ack_write = os.pipe()
            results, errors = [], []
            script = ("import os,sys;sys.stdout.buffer.write(b'progress\\n');sys.stdout.flush();"
                      "sys.stderr.buffer.write(b'original refusal\\n');sys.stderr.flush();"
                      f"os.write({ready_write},b'R');os.read({ack_read},1);"
                      "sys.stdout.buffer.write(sys.stdin.buffer.read());sys.stdout.flush();sys.exit(7)")

            def run():
                """Retain thread launch errors and close readiness so failure cannot hide as silence."""
                try:
                    results.append(capture([sys.executable, '-c', script], cwd=root, env=os.environ,
                                           data=b'original input\n', stdout_path=root/'stdout',
                                           stderr_path=root/'stderr', pass_fds=(ready_write, ack_read)))
                except BaseException as error:
                    errors.append(error)
                finally:
                    os.close(ready_write)
                    os.close(ack_read)

            thread = threading.Thread(target=run)
            thread.start()
            try:
                self.assertEqual(os.read(ready_read, 1), b'R', errors)
                self.assertEqual((root/'stdout').read_bytes(), b'progress\n')
                self.assertEqual((root/'stderr').read_bytes(), b'original refusal\n')
                self.assertTrue(thread.is_alive(), 'child must still await acknowledgement')
            finally:
                try:
                    os.write(ack_write, b'A')
                except BrokenPipeError:
                    pass
                os.close(ack_write)
                os.close(ready_read)
                thread.join()
            self.assertFalse(errors)
            self.assertEqual(results[0].returncode, 7)
            self.assertEqual(results[0].stdout, b'progress\noriginal input\n')
            self.assertEqual(results[0].stderr, b'original refusal\n')

    def test_previous_evidence_is_not_overwritten_and_fresh_capture_recovers(self):
        """Refuse a reused evidence name before child execution, then succeed with fresh names."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'stdout').write_bytes(b'original evidence')
            args = [sys.executable, '-c', "print('fresh result')"]
            with self.assertRaises(FileExistsError):
                capture(args, cwd=root, env=os.environ, data=None,
                        stdout_path=root/'stdout', stderr_path=root/'stderr')
            self.assertEqual((root/'stdout').read_bytes(), b'original evidence')
            self.assertFalse((root/'stderr').exists())
            result = capture(args, cwd=root, env=os.environ, data=None,
                             stdout_path=root/'fresh-stdout', stderr_path=root/'fresh-stderr')
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, b'fresh result\n')
            self.assertEqual(result.stderr, b'')


if __name__ == '__main__':
    unittest.main()
