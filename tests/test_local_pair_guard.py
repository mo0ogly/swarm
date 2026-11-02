"""Launcher pair guard: authority protocol desynchronization refused before stop.

2026-10-31 incident prevention: an engine built speaking assertion v2 replaced a
running engine on a Root in authority mode whose installed daemon still spoke
v1; the daemon then refused every relayed request with `assertion-format` until
the daemon was refreshed. The guard (tools/swarm_local.py
require_matching_assertion_pair, wired into main() for start/restart) now
refuses the replacement before any stop.

Unit tests call the guard directly with a stub settings reader and fixture
daemon bytes (no sudo, no daemon, no compilation). Public-path tests run the
real launcher against a real isolated engine binary and Root, switching the
Root to authority mode through the public engine-settings CLI, with the
installed daemon observed only through SWARM_AUTHORITY_INSTALLED fixture bytes.
No live mission database, no service operations. Fixture area only; no /tmp.
"""
import hashlib
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

from fixture_area import fixture_area

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import swarm_local

# Real engine binary built in this tree (v311); both sides of the live pair
# carry the v2 domain marker (`rust/crates/swarm-authority/src/assertion.rs`).
BINARY = os.environ.get("SWARM_TEST_BINARY", str(ROOT / "bin" / "swarm"))
V2_MARKER = b"swarm-authority-assertion-v2\0"
V1_MARKER = b"swarm-authority-assertion-v1\0"
# Valid 32-byte base64 public key, as in test_swarm_local.py's mode switch.
PUBLIC_KEY = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8"


def protocol_bytes(*markers):
    """Inert fixture bytes carrying exactly the given domain markers."""
    return b"fixture " + b" ".join(markers)


class PairGuardUnit(unittest.TestCase):
    """Direct guard calls: message contract and fail-closed cases, no server."""

    def setUp(self):
        self.area = Path(tempfile.mkdtemp(prefix="pair-guard-unit-", dir=fixture_area()))
        self.root = self.area / "Root"
        (self.root / ".swarm").mkdir(parents=True)
        (self.root / ".swarm" / "state.db").write_bytes(b"fixture state")
        self.daemon = self.area / "installed-daemon"
        self.candidate = self.area / "engine-candidate"
        self.current = self.area / "engine-current"
        self.candidate.write_bytes(protocol_bytes(V2_MARKER))

    def tearDown(self):
        shutil.rmtree(self.area, ignore_errors=True)

    def settings_stub(self, name, mode=None, fail=False):
        """Stub executable answering `engine-settings show --json` for the guard."""
        stub = self.area / name
        if fail:
            stub.write_text("#!/bin/sh\nexit 3\n")
        else:
            payload = json.dumps({"values": {"web_access_mode": mode}})
            stub.write_text("#!/bin/sh\nprintf '%s' '" + payload + "'\n")
        stub.chmod(0o700)
        return stub

    def guard(self, current, candidate=None, daemon=None, root=None):
        """Run the real guard with the installed-daemon path patched away from
        the operator's machine (module constant is read at import time)."""
        with mock.patch.object(swarm_local, "AUTHORITY_INSTALLED",
                               Path(daemon) if daemon else self.area / "no-such-daemon"):
            return swarm_local.require_matching_assertion_pair(
                str(current), str(candidate or self.candidate), str(root or self.root))

    def refusal(self, *args, **kwargs):
        """Assert refusal and return the message."""
        with self.assertRaises(RuntimeError) as caught:
            self.guard(*args, **kwargs)
        return str(caught.exception)

    def test_incident_shape_v1_daemon_v2_engine_refused_with_recovery(self):
        """The 2026-10-31 shape: installed daemon v1, engine candidate v2."""
        self.daemon.write_bytes(protocol_bytes(V1_MARKER))
        message = self.refusal(self.settings_stub("stub-authority", "authority"),
                               daemon=self.daemon)
        for fragment in ["Paire d’assertion incompatible", "v1", "v2",
                         "./swarm.sh authority install", "non arrêté"]:
            self.assertIn(fragment, message)
        # The recovery named is the daemon refresh, not `enable` (which would
        # activate the outdated daemon the guard just caught).
        self.assertNotIn("authority enable", message)
        # The guard reads bytes only: fixture files are untouched witnesses.
        self.assertEqual(self.daemon.read_bytes(), protocol_bytes(V1_MARKER))
        self.assertEqual(self.candidate.read_bytes(), protocol_bytes(V2_MARKER))

    def test_other_direction_v2_daemon_v1_engine_refused(self):
        self.daemon.write_bytes(protocol_bytes(V2_MARKER))
        self.candidate.write_bytes(protocol_bytes(V1_MARKER))
        stub = self.settings_stub("stub-authority-2", "authority")
        message = self.refusal(stub, daemon=self.daemon)
        self.assertIn("Paire d’assertion incompatible", message)
        self.assertIn("v1", message)
        self.assertIn("v2", message)

    def test_matching_v2_pair_passes(self):
        self.daemon.write_bytes(protocol_bytes(V2_MARKER))
        self.guard(self.settings_stub("stub-match", "authority"), daemon=self.daemon)

    def test_unknown_daemon_marker_refuses_fail_closed(self):
        self.daemon.write_bytes(b"daemon without any known marker")
        message = self.refusal(self.settings_stub("stub-opaque", "authority"),
                               daemon=self.daemon)
        self.assertIn("sans-protocole-reconnu", message)
        # Same fail-closed when the candidate carries no recognized marker.
        self.candidate.write_bytes(b"engine without any known marker")
        self.daemon.write_bytes(protocol_bytes(V2_MARKER))
        message = self.refusal(self.settings_stub("stub-opaque-2", "authority"),
                               daemon=self.daemon)
        self.assertIn("sans-protocole-reconnu", message)

    def test_owner_key_mode_skips_the_guard(self):
        # Mismatched pair on disk: irrelevant in owner-key mode.
        self.daemon.write_bytes(protocol_bytes(V1_MARKER))
        self.candidate.write_bytes(protocol_bytes(V2_MARKER))
        self.guard(self.settings_stub("stub-owner", "owner-key"), daemon=self.daemon)

    def test_uninitialized_root_skips_the_guard(self):
        # No state.db: nothing was ever switchable; the launcher inits later.
        # The stub would fail the settings read if the guard ever ran it.
        fresh = self.area / "FreshRoot"
        fresh.mkdir()
        broken = self.settings_stub("stub-broken", fail=True)
        self.daemon.write_bytes(protocol_bytes(V1_MARKER))
        self.guard(broken, candidate=self.candidate, daemon=self.daemon, root=fresh)

    def test_absent_installed_daemon_passes(self):
        # Nothing on disk to desynchronize with; refusing would lock the
        # operator out of a Root whose daemon was uninstalled.
        self.guard(self.settings_stub("stub-absent", "authority"), daemon=None)

    def test_unreadable_settings_refuse_fail_closed(self):
        self.daemon.write_bytes(protocol_bytes(V2_MARKER))
        message = self.refusal(self.settings_stub("stub-fail", fail=True),
                               daemon=self.daemon)
        self.assertIn("illisibles", message)
        self.assertIn("non arrêté", message)

    def test_absent_current_binary_passes_without_settings_read(self):
        # A current binary that disappeared mid-run has nothing to protect.
        self.daemon.write_bytes(protocol_bytes(V1_MARKER))
        self.guard(self.area / "gone-binary", daemon=self.daemon)


class PairGuardPublicPath(unittest.TestCase):
    """Real launcher restart on a real isolated Root in authority mode."""

    def setUp(self):
        self.area = Path(tempfile.mkdtemp(prefix="pair-guard-public-", dir=fixture_area()))
        self.root = self.area / "project with spaces"
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            self.address = f"127.0.0.1:{sock.getsockname()[1]}"
        self.daemon = self.area / "installed-daemon"
        self.unit = self.area / "swarm-authority.service"
        # The launcher reads both paths from its environment at import time.
        self.env = {**os.environ,
                    "SWARM_AUTHORITY_INSTALLED": str(self.daemon),
                    "SWARM_AUTHORITY_UNIT": str(self.unit)}
        self.args = [str(ROOT / "swarm.sh"), "--root", str(self.root),
                     "--address", self.address, "--binary", BINARY,
                     "--no-open", "--no-mask-cifs"]

    def tearDown(self):
        subprocess.run([*self.args, "stop"], env=self.env, capture_output=True, timeout=60)
        shutil.rmtree(self.area, ignore_errors=True)

    def runtime_dir(self):
        tag = hashlib.sha256(self.address.encode()).hexdigest()[:16]
        return self.root / ".swarm" / "local-web" / tag

    def state(self):
        return json.loads((self.runtime_dir() / "server.json").read_text())

    def server_alive(self, state):
        """Witness that the exact owned server process is still running."""
        try:
            stat = Path(f"/proc/{state['pid']}/stat").read_text().rsplit(")", 1)[1].split()
        except OSError:
            return False
        return stat[0] != "Z" and stat[19] == state["identity"]["start"]

    def cli(self, *arguments):
        result = subprocess.run([BINARY, "--root", str(self.root), *arguments],
                                text=True, capture_output=True, check=True)
        return json.loads(result.stdout)

    def switch_to_authority(self):
        """Adopt `authority` access mode on the isolated Root via the CLI."""
        shown = self.cli("engine-settings", "show", "--json")
        values = {**shown["values"], "web_access_mode": "authority",
                  "web_authority_public_key": PUBLIC_KEY}
        change = {"schema_version": shown["schema_version"], "event_id": "pair-guard-test",
                  "expected_revision": shown["revision"], "values": values,
                  "reason": "Pair guard test", "preview_token": ""}
        request = self.area / "authority-change.json"
        request.write_text(json.dumps(change))
        change["preview_token"] = self.cli("engine-settings", "preview",
                                           "--input", str(request), "--json")["preview_token"]
        request.write_text(json.dumps(change))
        self.cli("engine-settings", "apply", "--input", str(request), "--json")

    def start(self, extra_env=None):
        result = subprocess.run([*self.args, "start"],
                                env={**self.env, **(extra_env or {})},
                                text=True, capture_output=True, timeout=120)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def restart(self, binary, code=None):
        result = subprocess.run([*self.args, "restart", "--binary", str(binary)],
                                env=self.env, text=True, capture_output=True, timeout=120)
        if code is not None:
            self.assertEqual(result.returncode, code,
                             f"stdout={result.stdout}\nstderr={result.stderr}")
        return result

    def test_restart_refusal_keeps_running_server_and_names_recovery(self):
        """Incident shape on the public path: daemon v1 on disk, engine v2
        candidate — restart refuses with exit 2 and the server keeps running."""
        self.start()
        self.switch_to_authority()
        before = self.state()
        self.assertTrue(self.server_alive(before))
        self.daemon.write_bytes(protocol_bytes(V1_MARKER))
        result = self.restart(BINARY)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        for fragment in ["Paire d’assertion incompatible", "v1", "v2",
                         "./swarm.sh authority install", "non arrêté"]:
            self.assertIn(fragment, result.stderr)
        self.assertNotIn("Paire d’assertion incompatible", result.stdout)
        # Refusal happened before any stop: same pid, same start time, port
        # still served by the engine that was running before the attempt.
        after = self.state()
        self.assertEqual(after["pid"], before["pid"])
        self.assertEqual(after["identity"]["start"], before["identity"]["start"])
        self.assertTrue(self.server_alive(after))

    def test_restart_matching_pair_proceeds_and_replaces_the_server(self):
        """Control: aligned v2/v2 pair passes; the restart completes and the
        running server is replaced (pid changes), proving the guard is not a
        blanket refusal."""
        self.start()
        self.switch_to_authority()
        before = self.state()
        self.daemon.write_bytes(protocol_bytes(V2_MARKER))
        result = self.restart(BINARY, code=0)
        self.assertNotIn("Paire d’assertion incompatible", result.stderr)
        after = self.state()
        self.assertNotEqual(after["pid"], before["pid"])
        self.assertTrue(self.server_alive(after))

    def test_start_in_authority_mode_with_compatible_pair_succeeds(self):
        """First start on a Root already in authority mode also passes the
        guard when the pair is compatible (start is guarded like restart)."""
        # Root pre-switched to authority before its first launcher start. The
        # launcher creates the root after os.umask(0o077); reproducing that
        # pre-condition here (0700) keeps the engine's service-owned storage
        # check satisfied exactly as on the real first start.
        self.root.mkdir(mode=0o700)
        init = subprocess.run([BINARY, "--root", str(self.root), "init"],
                              cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(init.returncode, 0, init.stderr)
        self.switch_to_authority()
        self.daemon.write_bytes(protocol_bytes(V2_MARKER))
        self.start()


if __name__ == "__main__":
    unittest.main()
