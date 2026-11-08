"""P0 public launcher lock contract on isolated roots; no provider/live database."""
import fcntl
import hashlib
import json
import subprocess
import tempfile
import time
import unittest
from pathlib import Path
from fixture_area import fixture_area

SOURCE = Path(__file__).resolve().parent.parent


class StatusLock(unittest.TestCase):
    def setUp(self):
        """Allocate a private absent-server Root and owned lifecycle descriptor."""
        self.temp = tempfile.TemporaryDirectory(dir=fixture_area(), prefix="p0-status-")
        self.root = Path(self.temp.name)
        self.address = "127.0.0.1:19875"  # No listener used in these absent-server cases.
        self.runtime = self.root / ".swarm/local-web" / hashlib.sha256(self.address.encode()).hexdigest()[:16]
        self.runtime.mkdir(parents=True)
        self.args = [str(SOURCE / "swarm.sh"), "--root", str(self.root), "--address", self.address]
        self.lock = (self.runtime / "lifecycle.lock").open("a")
        self.addCleanup(self.temp.cleanup)
        self.addCleanup(self.lock.close)

    def status(self, code):
        """Run public read-only status and decode only the expected JSON exit."""
        result = subprocess.run([*self.args, "status"], capture_output=True, text=True)
        self.assertEqual(result.returncode, code, result.stderr)
        self.assertEqual(result.stderr, "")
        return json.loads(result.stdout)

    def test_exclusive_contention_unknown_and_recovery(self):
        """A valid control surrounds partial writer state, unknowns and recovery."""
        control = self.status(1)
        self.assertIs(control["running"], False)
        fcntl.flock(self.lock, fcntl.LOCK_EX)
        # Malformed unpublished state must never be read while a writer holds the lock.
        state = self.runtime / "server.json"
        state.write_bytes(b'{unfinished')
        before = state.read_bytes()
        result = self.status(2)
        self.assertEqual(result["status"], "concurrent-operation")
        for field in ("running", "ready", "pid", "binary", "binary_sha256"):
            self.assertIsNone(result[field])
        self.assertEqual(state.read_bytes(), before)
        state.unlink()  # Fixture writer completes its operation before unlock.
        fcntl.flock(self.lock, fcntl.LOCK_UN)
        self.assertEqual(self.status(1), control)

    def test_shared_readers_are_compatible(self):
        """A second reader can observe absent-server state under a shared lock."""
        control = self.status(1)
        fcntl.flock(self.lock, fcntl.LOCK_SH)
        self.assertEqual(self.status(1), control)

    def test_mutation_still_excluded_and_recovers(self):
        """Public stop waits for a fixture reader and resumes after its release."""
        self.status(1)  # Valid control before contention.
        fcntl.flock(self.lock, fcntl.LOCK_SH)
        p = subprocess.Popen([*self.args, "stop"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            # Observe the kernel's actual lock wait; no elapsed-time success assertion.
            while p.poll() is None:
                if Path(f"/proc/{p.pid}/wchan").read_text().strip() == "locks_lock_inode_wait":
                    break
                time.sleep(.01)
            self.assertIsNone(p.poll(), "mutation bypassed a shared reader")
        finally:
            fcntl.flock(self.lock, fcntl.LOCK_UN)
            out, err = p.communicate()
        self.assertEqual(p.returncode, 0, err)
        self.assertIn("Serveur arrêté", out)
        self.assertIs(self.status(1)["running"], False)

    def test_identity_change_during_readiness_refuses(self):
        """A process replacement during the final probe cannot publish stale success."""
        import sys
        from unittest.mock import patch
        sys.path.insert(0, str(SOURCE / "tools"))
        import swarm_local
        state = self.runtime / "server.json"
        state.write_text(json.dumps({"pid": 123, "identity": {}}))
        with patch.object(sys, "argv", ["swarm_local.py", "status", "--root", str(self.root), "--address", self.address]), \
             patch.object(swarm_local, "owned", side_effect=[True, False]), \
             patch.object(swarm_local, "running_version", return_value=({"binary": {}}, "fixture-sha")), \
             patch.object(swarm_local, "ready", return_value=True), \
             patch("builtins.print") as printed:
            with self.assertRaisesRegex(RuntimeError, "identity changed"):
                swarm_local.main()
            printed.assert_not_called()

    def test_invalid_state_refuses_without_rewrite(self):
        """Malformed released state fails visibly, remains intact, and recovers."""
        self.status(1)
        state = self.runtime / "server.json"
        state.write_bytes(b'{unfinished')
        result = subprocess.run([*self.args, "status"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertEqual(state.read_bytes(), b'{unfinished')
        state.unlink()
        self.assertIs(self.status(1)["ready"], False)


if __name__ == "__main__":
    unittest.main()
