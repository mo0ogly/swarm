"""Inert checks for the owned running-engine mode reader and its pair guard.

Every process, network, signal and stop boundary is doubled: the settings child
is answered by a recording stub behind a mocked subprocess.run, the "kernel
image" is a synthetic fixture file reached through a patched /proc/<pid>/exe
route, and stop raises a sentinel. No real engine binary, no state database and
no current kernel image is executed; healthy controls and recovery surround injected failures; no descriptor
outlives a completed test step (close-fault fixtures release their own fd).
"""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from fixture_area import fixture_area

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import swarm_local as local

_REAL_OPEN = os.open
_REAL_STAT = os.stat
_REAL_CLOSE = os.close


class ReachedStop(Exception):
    """Sentinel proving the guarded stop boundary was reached, in order."""


def protocol_bytes(name):
    """Synthetic image bytes carrying exactly the named assertion marker."""
    return b"fixture-boundary:" + dict(local.ASSERTION_PROTOCOL_DOMAINS)[name] + b":fixture-boundary"


class SyntheticEngine:
    """One fake owned engine: fixture image file, fake pid, matching identity.

    Routed boundaries answer only the exact /proc/<pid>/exe path and the
    `engine-settings` child; everything else passes through to the real os
    calls so fixture handling keeps working. `on_query` lets a test flip the
    synthetic identity or image between the reader's bracketing checks, the
    way a real re-exec, PID reuse or image replacement would.
    """

    def __init__(self, area):
        self.pid = 4242
        self.image = Path(area) / "owned-engine-image"
        self.image.write_bytes(b"SYNTHETIC_ENGINE_IMAGE_V1")
        self.replacement = Path(area) / "replacement-engine-image"
        self.replacement.write_bytes(b"SYNTHETIC_ENGINE_IMAGE_V2")
        self.identity_now = {"pid": self.pid, "start": "9001", "exe": str(self.image),
                             "cmd": b"swarm\0web\0".hex()}
        self.state = {"pid": self.pid, "identity": dict(self.identity_now)}
        self.replaced = False
        self.open_failure = None
        self.close_failure = None
        self.descriptor = None
        self.on_query = None
        self.settings_failure = None
        self.settings_returncode = 0
        self.settings_stdout = json.dumps({"values": {"web_access_mode": "owner-key"}})
        self.recorded = []
        self.order = []

    def reset(self):
        """Restore the owned inert image and healthy query after fault injection.

        Release only this fixture's retained descriptor, reset synthetic faults,
        identity and image observation, and preserve the fixture files. No real
        process, settings or kernel executable is changed.
        """
        self.release()
        self.identity_now = dict(self.state['identity'])
        self.replaced = False
        self.open_failure = self.close_failure = self.settings_failure = None
        self.on_query = None
        self.settings_returncode = 0
        self.settings_stdout = json.dumps({'values': {'web_access_mode': 'owner-key'}})

    def fake_identity(self, pid):
        """Stand in for swarm_local.identity on the synthetic image only."""
        if pid != self.pid:
            return None
        return dict(self.identity_now)

    def route_open(self, path, *args, **kwargs):
        """Serve the synthetic image for /proc/<pid>/exe; pass the rest through."""
        if str(path) == f"/proc/{self.pid}/exe":
            if self.open_failure is not None:
                raise self.open_failure
            self.descriptor = _REAL_OPEN(str(self.image), *args, **kwargs)
            return self.descriptor
        return _REAL_OPEN(path, *args, **kwargs)

    def route_stat(self, path, *args, **kwargs):
        """Stat the synthetic (possibly replaced) image for /proc/<pid>/exe."""
        if str(path) == f"/proc/{self.pid}/exe":
            target = self.replacement if self.replaced else self.image
            return _REAL_STAT(str(target), *args, **kwargs)
        return _REAL_STAT(path, *args, **kwargs)

    def route_close(self, fd):
        """Close for real unless a close refusal is armed on our descriptor."""
        if fd == self.descriptor and self.close_failure is not None:
            raise self.close_failure
        if fd == self.descriptor:
            self.descriptor = None
        return _REAL_CLOSE(fd)

    def route_run(self, command, **kwargs):
        """Answer the settings child; refuse any other subprocess use."""
        command = list(command)
        if "engine-settings" not in command:
            raise AssertionError("forbidden subprocess.run: " + repr(command))
        self.order.append("settings-query")
        self.recorded.append((command, dict(kwargs)))
        if self.on_query is not None:
            self.on_query()
        if self.settings_failure is not None:
            raise self.settings_failure
        return subprocess.CompletedProcess(args=command, returncode=self.settings_returncode,
                                           stdout=self.settings_stdout, stderr="")

    def enter(self, stack):
        """Install every routed boundary on an already-open ExitStack."""
        stack.enter_context(patch.object(local, "identity", side_effect=self.fake_identity))
        stack.enter_context(patch.object(local.os, "open", side_effect=self.route_open))
        stack.enter_context(patch.object(local.os, "stat", side_effect=self.route_stat))
        stack.enter_context(patch.object(local.os, "close", side_effect=self.route_close))
        stack.enter_context(patch.object(local.subprocess, "run", side_effect=self.route_run))

    def queried_descriptor(self):
        """The fd number the settings child inherited, from its own command."""
        command = self.recorded[-1][0]
        if not command[0].startswith("/proc/self/fd/"):
            raise AssertionError("settings child must exec the held descriptor path: " + repr(command))
        return int(command[0].rsplit("/", 1)[1])

    def assert_descriptor_closed(self, fd):
        """Fail unless the inherited fd is closed and its /proc path is gone."""
        try:
            os.fstat(fd)
        except OSError:
            pass
        else:
            raise AssertionError(f"descriptor {fd} is still open after the reader returned")
        if Path(f"/proc/self/fd/{fd}").exists():
            raise AssertionError(f"/proc/self/fd/{fd} still resolves after close")

    def release(self):
        """Really close any descriptor a refusal injection left open."""
        if self.descriptor is not None:
            fd, self.descriptor = self.descriptor, None
            _REAL_CLOSE(fd)


class OwnedEngineModeReader(unittest.TestCase):
    """Direct owned_engine_mode calls against one synthetic proc image."""

    def setUp(self):
        self.area = Path(tempfile.mkdtemp(prefix="live-reader-unit-", dir=fixture_area()))
        self.engine = SyntheticEngine(self.area)
        self.assertEqual(self.read(), 'owner-key')
        self.engine.recorded.clear()
        self.engine.order.clear()

    def tearDown(self):
        try:
            self.engine.reset()
            self.assertEqual(self.read(), 'owner-key')
        finally:
            self.engine.release()
            shutil.rmtree(self.area)

    def read(self):
        """Run the real reader once; return the mode string or the exception."""
        result = None
        error = None
        with contextlib.ExitStack() as stack:
            self.engine.enter(stack)
            try:
                with local.owned_engine_mode(self.engine.state, self.area / "Root") as mode:
                    result = mode
            except BaseException as caught:
                error = caught
        return result if error is None else error

    def test_control_reads_image_mode_and_closes_descriptor(self):
        """Green control: mode from the held image, descriptor gone after."""
        self.assertEqual(self.read(), "owner-key")
        command, kwargs = self.engine.recorded[0]
        fd = self.engine.queried_descriptor()
        self.assertEqual(command[1:], ["--root", str(self.area / "Root"),
                                       "engine-settings", "show", "--json"])
        self.assertEqual(kwargs["pass_fds"], (fd,))
        self.assertEqual(kwargs["cwd"], local.SOURCE)
        self.engine.assert_descriptor_closed(fd)
        self.assertEqual(self.engine.image.read_bytes(), b"SYNTHETIC_ENGINE_IMAGE_V1")

    def test_authority_mode_is_returned_unchanged(self):
        """The reader is mode-agnostic; authority flows through untouched."""
        self.engine.settings_stdout = json.dumps({"values": {"web_access_mode": "authority"}})
        self.assertEqual(self.read(), "authority")
        self.engine.assert_descriptor_closed(self.engine.queried_descriptor())

    def test_changed_identity_after_query_refuses_and_closes(self):
        """A pid/start/exe/cmdline change between the two identity checks refuses."""
        changed = dict(self.engine.identity_now, start="1")
        self.engine.on_query = lambda: setattr(self.engine, "identity_now", changed)
        error = self.read()
        self.assertIsInstance(error, RuntimeError)
        self.assertIn("changée", str(error))
        self.assertEqual(len(self.engine.recorded), 1)
        self.engine.assert_descriptor_closed(self.engine.queried_descriptor())

    def test_replaced_image_inode_after_query_refuses_and_closes(self):
        """A dev/ino change of /proc/<pid>/exe between the two stats refuses."""
        self.engine.on_query = lambda: setattr(self.engine, "replaced", True)
        error = self.read()
        self.assertIsInstance(error, RuntimeError)
        self.assertIn("remplacée", str(error))
        self.engine.assert_descriptor_closed(self.engine.queried_descriptor())

    def test_failed_open_preserves_original_cause(self):
        """An unopenable image descriptor refuses with the OSError attached."""
        cause = OSError("inert image open refusal")
        self.engine.open_failure = cause
        error = self.read()
        self.assertIsInstance(error, RuntimeError)
        self.assertIn("indisponible", str(error))
        self.assertIs(error.__cause__, cause)
        self.assertEqual(self.engine.recorded, [])
        self.assertIsNone(self.engine.descriptor)

    def test_failed_query_preserves_original_cause_and_closes(self):
        """An unstartable settings child refuses with the OSError attached."""
        cause = OSError("inert settings exec refusal")
        self.engine.settings_failure = cause
        error = self.read()
        self.assertIsInstance(error, RuntimeError)
        self.assertIn("impossible", str(error))
        self.assertIs(error.__cause__, cause)
        self.engine.assert_descriptor_closed(self.engine.queried_descriptor())

    def test_failed_close_after_success_refuses_with_cause(self):
        """A failing close refuses loudly; the test releases the real fd."""
        cause = OSError("inert descriptor close refusal")
        self.engine.close_failure = cause
        error = self.read()
        self.assertIsInstance(error, RuntimeError)
        self.assertIn("non refermé", str(error))
        self.assertIs(error.__cause__, cause)
        self.engine.release()
        self.assertIsNone(self.engine.descriptor)

    def test_failed_query_and_failed_close_keep_both_causes(self):
        """The original read refusal is never dropped by a failing close."""
        primary = OSError("inert settings exec refusal")
        cleanup = OSError("inert descriptor close refusal")
        self.engine.settings_failure = primary
        self.engine.close_failure = cleanup
        error = self.read()
        self.assertIsInstance(error, BaseExceptionGroup)
        self.assertEqual(len(error.exceptions), 2)
        self.assertIs(error.exceptions[0], primary)
        self.assertIs(error.exceptions[1], cleanup)
        self.engine.release()

    def test_unreadable_settings_refuse_fail_closed_then_recover(self):
        """Nonzero exit and shapeless output refuse; a healthy read recovers."""
        for stdout, returncode in [("not-json", 0),
                                   (json.dumps({"values": {"web_access_mode": "authority"}}), 3)]:
            with self.subTest(stdout=stdout, returncode=returncode):
                self.engine.settings_stdout = stdout
                self.engine.settings_returncode = returncode
                error = self.read()
                self.assertIsInstance(error, RuntimeError)
                self.assertIn("illisibles", str(error))
                if returncode != 0:
                    self.assertIn(f"code {returncode}", str(error))
                self.engine.assert_descriptor_closed(self.engine.queried_descriptor())
        self.engine.settings_stdout = json.dumps({"values": {"web_access_mode": "owner-key"}})
        self.engine.settings_returncode = 0
        self.assertEqual(self.read(), "owner-key")


class PairGuardLiveReader(unittest.TestCase):
    """Guard routing: the owned running image decides, never a pathname."""

    def setUp(self):
        self.area = Path(tempfile.mkdtemp(prefix="live-reader-guard-", dir=fixture_area()))
        self.engine = SyntheticEngine(self.area)
        self.root = self.area / "Root"
        (self.root / ".swarm").mkdir(parents=True)
        (self.root / ".swarm" / "state.db").write_bytes(b"fixture state")
        self.daemon = self.area / "installed-daemon"
        self.candidate = self.area / "engine-candidate"
        self.selected = self.area / "selected-engine-path"
        self.candidate.write_bytes(protocol_bytes("v2"))
        self.assertIsNone(self.guard())
        self.engine.recorded.clear()
        self.engine.order.clear()

    def tearDown(self):
        try:
            self.engine.reset()
            self.assertIsNone(self.guard())
        finally:
            self.engine.release()
            shutil.rmtree(self.area)

    def guard(self, running="owned", current=None, candidate=None, daemon=None, root=None):
        """Run the real guard with boundaries routed and the daemon path patched.

        running="owned" passes the synthetic engine record; None exercises the
        historical no-server pathname path.
        """
        with contextlib.ExitStack() as stack:
            self.engine.enter(stack)
            stack.enter_context(patch.object(
                local, "AUTHORITY_INSTALLED",
                Path(daemon) if daemon else self.area / "no-such-daemon"))
            return local.require_matching_assertion_pair(
                str(self.selected if current is None else current),
                str(self.candidate if candidate is None else candidate),
                str(self.root if root is None else root),
                running=self.engine.state if running == "owned" else running)

    def refused(self, **kwargs):
        with self.assertRaises(RuntimeError) as caught:
            self.guard(**kwargs)
        return caught.exception

    def refusal(self, **kwargs):
        return str(self.refused(**kwargs))

    def test_live_image_decides_even_when_selected_pathname_is_missing(self):
        """Regression: an owned engine is read through /proc, never skipped."""
        self.assertIsNone(self.guard())
        self.assertEqual(len(self.engine.recorded), 1)
        fd = self.engine.queried_descriptor()
        self.engine.assert_descriptor_closed(fd)
        # Authority live mode against a v1 daemon: refused although the
        # selected pathname is absent — the old early-return would have passed.
        self.engine.settings_stdout = json.dumps({"values": {"web_access_mode": "authority"}})
        self.daemon.write_bytes(protocol_bytes("v1"))
        message = self.refusal(daemon=self.daemon)
        self.assertIn("Paire d’assertion incompatible", message)
        self.assertIn("authority install", message)
        self.engine.assert_descriptor_closed(self.engine.queried_descriptor())
        self.engine.settings_stdout = json.dumps({"values": {"web_access_mode": "owner-key"}})
        self.assertIsNone(self.guard())

    def test_present_but_different_selected_pathname_is_not_executed(self):
        """A decoy selected pathname is never consulted while owned."""
        decoy = self.area / "decoy-selected-engine"
        decoy.write_bytes(protocol_bytes("v1"))
        self.assertIsNone(self.guard(current=decoy))
        command = self.engine.recorded[0][0]
        self.assertTrue(command[0].startswith("/proc/self/fd/"))
        self.assertNotIn(str(decoy), command)

    def test_foreign_running_state_is_never_adopted(self):
        """A record that no longer matches /proc is refused before any query."""
        foreign = {"pid": self.engine.pid,
                   "identity": {"pid": self.engine.pid, "start": "1",
                                "exe": "elsewhere", "cmd": ""}}
        message = self.refusal(running=foreign)
        self.assertIn("changée", message)
        self.assertEqual(self.engine.recorded, [])
        self.assertIsNone(self.engine.descriptor)

    def test_initialized_root_without_server_and_missing_current_refuses(self):
        """Fail closed: an initialized Root cannot reveal its saved mode."""
        message = self.refusal(running=None)
        self.assertIn("absent", message)
        self.assertIn("inconnaissable", message)
        self.assertEqual(self.engine.recorded, [])

    def test_uninitialized_root_without_server_keeps_historical_skip(self):
        """No state.db: nothing was ever switchable; the skip stays."""
        fresh = self.area / "FreshRoot"
        fresh.mkdir()
        self.assertIsNone(self.guard(running=None, root=fresh))
        self.assertEqual(self.engine.recorded, [])

    def test_three_argument_callers_still_read_the_installed_pathname(self):
        """Existing direct calls keep working when no owned server exists."""
        stub = self.area / "installed-engine-stub"
        stub.write_bytes(b"fixture engine bytes")
        self.assertIsNone(self.guard(running=None, current=stub))
        command, kwargs = self.engine.recorded[0]
        self.assertEqual(command, [str(stub), "--root", str(self.root),
                                   "engine-settings", "show", "--json"])
        self.assertNotIn("pass_fds", kwargs)


class MainLiveGuardOrdering(unittest.TestCase):
    """main() must query the owned image and only then reach the stop."""

    def exercise(self, **kwargs):
        """Bracket every pair-guard scenario with valid main controls and recovery.

        Controls require the real guard to query the synthetic owned image and
        reach only the stop sentinel; no native image or live Root is executed.
        Recovery runs in finally, independently of unittest method ordering.
        """
        def control():
            """Require the owned settings query before the inert stop boundary."""
            error, seen = self._exercise_once()
            self.assertIsInstance(error, ReachedStop)
            self.assertEqual(seen['order'], ['settings-query', 'stop'])
            self.assertEqual(seen['state'], seen['state_before'])
            self.assertIsNone(seen['descriptor'])
        control()
        try:
            return self._exercise_once(**kwargs)
        finally:
            control()

    def _exercise_once(self, *, mode="owner-key", daemon_v1=False, foreign_state=False,
                 late_identity_change=False):
        """Run inert main+pair guard; double the pre/post owned-version snapshots.

        Records both captures and the exact installed DTO reaching the producer.
        Ownership/settings/protocol guard still use the synthetic engine routes;
        process, network and real stop stay forbidden. Returns error and receipts,
        preserving the owned fixture state and releasing its descriptor.
        """
        with tempfile.TemporaryDirectory(prefix="live-reader-main-", dir=fixture_area()) as area:
            source = Path(area)
            root = source / "project"
            address = "127.0.0.1:18792"
            runtime = root / ".swarm/local-web" / hashlib.sha256(address.encode()).hexdigest()[:16]
            runtime.mkdir(parents=True)
            (root / ".swarm" / "state.db").write_bytes(b"fixture state")
            engine = SyntheticEngine(source)
            engine.settings_stdout = json.dumps({"values": {"web_access_mode": mode}})
            if late_identity_change:
                original_identity = engine.fake_identity
                identity_reads = 0

                def change_at_guard(pid):
                    """Lose ownership only after main's earlier version/identity gate."""
                    nonlocal identity_reads
                    identity_reads += 1
                    value = original_identity(pid)
                    return dict(value, start='changed-at-pair-guard') if identity_reads >= 3 else value

                engine.fake_identity = change_at_guard
                # A present decoy makes the forbidden pathname fallback observable.
                (source / 'bin').mkdir(exist_ok=True)
                (source / 'bin/swarm-local').write_bytes(b'INERT_DECOY')
            state = ({"pid": engine.pid, "identity": {"foreign": True}}
                     if foreign_state else engine.state)
            state_path = runtime / "server.json"
            state_path.write_text(json.dumps(state))
            before_state = state_path.read_bytes()
            candidate = source / "bin/swarm-local.candidate-unit"
            candidate.parent.mkdir(exist_ok=True)
            candidate.write_bytes(protocol_bytes("v2"))
            candidate_sha = hashlib.sha256(candidate.read_bytes()).hexdigest()
            selected_state = {"version": "v0.1.1-dev.12", "binary_sha256": candidate_sha}
            observation = {"schema_version": 1, "binary": {"version": "v0.1.1-dev.11"}}
            daemon = source / "installed-daemon"
            daemon.write_bytes(protocol_bytes("v1" if daemon_v1 else "v2"))
            argv = ["swarm_local.py", "restart", "--root", str(root),
                    "--address", address, "--no-open"]
            order = []
            engine.order = order
            error = None

            def reached_stop(recorded_state, timeout, recorded_address):
                """Sentinel at the stop boundary, after the guard."""
                order.append("stop")
                raise ReachedStop()

            with contextlib.ExitStack() as stack:
                engine.enter(stack)
                stack.enter_context(patch.object(local, "SOURCE", source))
                stack.enter_context(patch.object(sys, "argv", argv))
                stack.enter_context(patch.object(local.os, "umask"))
                producer = stack.enter_context(patch.object(local, "prepare_versioned_build",
                                                 return_value=(candidate, selected_state)))
                stack.enter_context(patch.object(local, "prepare_recorded",
                                                 return_value=(candidate, selected_state)))
                observed_sha = hashlib.sha256(engine.image.read_bytes()).hexdigest()
                capture = stack.enter_context(patch.object(local, "owned_running_version",
                                                 return_value=(observation, observed_sha)))
                stack.enter_context(patch.object(local, "running_version",
                                                 side_effect=AssertionError("forbidden legacy running_version")))
                stack.enter_context(patch.object(local, "AUTHORITY_INSTALLED", daemon))
                stack.enter_context(patch.object(local, "stop", side_effect=reached_stop))
                stack.enter_context(patch.object(local, "authority_front",
                                                 side_effect=AssertionError("forbidden authority_front")))
                stack.enter_context(patch.object(local, "ready",
                                                 side_effect=AssertionError("forbidden ready")))
                for owner_obj, name in [(local.subprocess, "Popen"), (local.os, "kill"),
                                        (local.urllib.request, "urlopen"),
                                        (local.socket, "socket")]:
                    stack.enter_context(patch.object(owner_obj, name,
                                                     side_effect=AssertionError("forbidden " + name)))
                stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
                try:
                    local.main()
                except BaseException as caught:
                    error = caught
                if foreign_state:
                    capture.assert_not_called()
                    producer.assert_called_once_with(source, source / "bin/swarm-local")
                else:
                    self.assertEqual(capture.call_count, 2)
                    for call in capture.call_args_list:
                        self.assertEqual(call.args, (engine.state, address, root))
                        self.assertEqual(call.kwargs, {})
                    producer.assert_called_once_with(source, source / "bin/swarm-local",
                        installed={'version': observation['binary']['version'],
                                   'binary_sha256': observed_sha})
            engine.release()
            receipts = {
                "root": str(root), "source": str(source),
                "order": list(order), "settings": list(engine.recorded),
                "state": state_path.read_bytes(), "state_before": before_state,
                "candidate": candidate.read_bytes(),
                "selected_exists": (source / "bin/swarm-local").exists(),
                "descriptor": engine.descriptor,
            }
        return error, receipts

    def test_owner_key_control_queries_owned_image_then_stops(self):
        """The owned image is read, then — and only then — stop is reached."""
        for _ in range(2):
            error, seen = self.exercise()
            self.assertIsInstance(error, ReachedStop)
            self.assertEqual(seen["order"], ["settings-query", "stop"])
            self.assertEqual(len(seen["settings"]), 1)
            command, kwargs = seen["settings"][0]
            fd = int(command[0].rsplit("/", 1)[1])
            self.assertTrue(command[0].startswith("/proc/self/fd/"))
            self.assertEqual(command[1:], ["--root", seen["root"],
                                           "engine-settings", "show", "--json"])
            self.assertEqual(kwargs["pass_fds"], (fd,))
            self.assertEqual(kwargs["cwd"], Path(seen["source"]))
            self.assertFalse(seen["selected_exists"])
            self.assertIsNone(seen["descriptor"])
            self.assertFalse(Path(f"/proc/self/fd/{fd}").exists())
            self.assertEqual(seen["state"], seen["state_before"])

    def test_mismatched_pair_from_live_image_refuses_before_stop(self):
        """Authority live mode against a v1 daemon refuses before any stop."""
        self.assertIsInstance(self.exercise()[0], ReachedStop)
        error, seen = self.exercise(mode="authority", daemon_v1=True)
        self.assertIsInstance(error, RuntimeError)
        self.assertIn("Paire d’assertion incompatible", str(error))
        self.assertIn("authority install", str(error))
        self.assertEqual(seen["order"], ["settings-query"])
        self.assertEqual(seen["state"], seen["state_before"])
        self.assertEqual(seen["candidate"], protocol_bytes("v2"))
        self.assertIsNone(seen["descriptor"])
        self.assertIsInstance(self.exercise()[0], ReachedStop)

    def test_lost_identity_at_pair_entry_never_falls_back_to_path(self):
        """After a valid control, losing ownership refuses before query or stop."""
        self.assertIsInstance(self.exercise()[0], ReachedStop)
        error, seen = self.exercise(late_identity_change=True)
        self.assertIsInstance(error, RuntimeError)
        self.assertIn('changée', str(error))
        self.assertEqual(seen['order'], [])
        self.assertEqual(seen['settings'], [])
        self.assertEqual(seen['state'], seen['state_before'])
        self.assertIsNone(seen['descriptor'])
        self.assertIsInstance(self.exercise()[0], ReachedStop)

    def test_main_never_adopts_a_foreign_running_state(self):
        """A server.json we do not own is left alone and nothing is queried."""
        self.assertIsInstance(self.exercise()[0], ReachedStop)
        error, seen = self.exercise(foreign_state=True)
        self.assertIsInstance(error, RuntimeError)
        self.assertIn("absent", str(error))
        self.assertEqual(seen["order"], [])
        self.assertEqual(seen["settings"], [])
        self.assertEqual(seen["state"], seen["state_before"])
        self.assertIsInstance(self.exercise()[0], ReachedStop)



class MissingCurrentGuard(unittest.TestCase):
    """Reproduce the initialized-Root missing-current refusal through the old API."""

    def test_missing_current_refuses_after_valid_control_then_recovers(self):
        """A known readable inert image is the control; missing current must refuse.

        The public settings child is doubled. No native executable, process,
        socket or live database is used; the synthetic state.db is an inert file.
        """
        with tempfile.TemporaryDirectory(prefix='missing-current-', dir=fixture_area()) as area:
            root = Path(area)
            (root / '.swarm').mkdir()
            (root / '.swarm/state.db').write_bytes(b'INERT_STATE')
            current = root / 'current'
            current.write_bytes(b'INERT_CURRENT')
            candidate = root / 'candidate'
            candidate.write_bytes(b'INERT_CANDIDATE')
            shown = subprocess.CompletedProcess([], 0, stdout=json.dumps({'values': {'web_access_mode': 'owner-key'}}), stderr='')
            with patch.object(local.subprocess, 'run', return_value=shown) as query, \
                 patch.object(local.subprocess, 'Popen', side_effect=AssertionError('forbidden Popen')), \
                 patch.object(local.os, 'kill', side_effect=AssertionError('forbidden kill')), \
                 patch.object(local.urllib.request, 'urlopen', side_effect=AssertionError('forbidden HTTP')):
                self.assertIsNone(local.require_matching_assertion_pair(current, candidate, root))
                query.assert_called_once()
                query.reset_mock()
                with self.assertRaisesRegex(RuntimeError, 'inconnaissable'):
                    local.require_matching_assertion_pair(root / 'missing', candidate, root)
                query.assert_not_called()
                self.assertIsNone(local.require_matching_assertion_pair(current, candidate, root))
                self.assertEqual((root / '.swarm/state.db').read_bytes(), b'INERT_STATE')

    def test_legacy_read_oserror_keeps_original_cause_then_recovers(self):
        """The no-server reader keeps its OS cause as well as a safe public error."""
        with tempfile.TemporaryDirectory(prefix='legacy-reader-error-', dir=fixture_area()) as area:
            root = Path(area)
            (root / '.swarm').mkdir()
            (root / '.swarm/state.db').write_bytes(b'INERT_STATE')
            current = root / 'current'
            current.write_bytes(b'INERT_CURRENT')
            shown = subprocess.CompletedProcess([], 0, stdout=json.dumps({'values': {'web_access_mode': 'owner-key'}}), stderr='')
            cause = OSError('inert legacy settings read refusal')
            with patch.object(local.subprocess, 'run', return_value=shown) as query:
                self.assertIsNone(local.require_matching_assertion_pair(current, current, root))
                query.side_effect = cause
                with self.assertRaises(RuntimeError) as caught:
                    local.require_matching_assertion_pair(current, current, root)
                self.assertIs(caught.exception.__cause__, cause)
                query.side_effect = None
                self.assertIsNone(local.require_matching_assertion_pair(current, current, root))


if __name__ == '__main__':
    unittest.main(verbosity=2)