"""Inert main-path checks for fresh/preselected managed version refusals.

Build, live identity/version, pair check, stop, process, signal and HTTP boundaries
are doubled. Only owned temporary fixture files are touched; reaching the stop
marker proves ordering in this Python caller, not a real service replacement.
"""
import contextlib
import hashlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from fixture_area import fixture_area

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import swarm_local as local


class ReachedStop(Exception):
    """End a healthy inert path before any lifecycle mutation."""


class ReachedFront(Exception):
    """End an idempotent start after its gates, without an actual browser call."""


class ManagedVersionGate(unittest.TestCase):
    """Exercise the actual main caller using synthetic producer/server receipts."""

    def exercise(self, *, prepared=False, selected='v0.1.1-dev.12',
                 installed='v0.1.1-dev.11', identity_changed=False,
                 read_error=None, cached_current=False, cleanup_error=None,
                 action='restart', same_image=False):
        """Return the caught exception after checking all forbidden boundaries.

        Inputs describe inert version/identity faults. Each call owns a fresh
        fixture and intercepts stop with a sentinel. Original server-record and
        installed-image bytes must survive every path; process/network calls
        fail the test if reached. No result establishes kernel/process identity.
        """
        with tempfile.TemporaryDirectory(prefix='managed-version-', dir=fixture_area()) as area:
            source = Path(area)
            root = source / 'project'
            address = '127.0.0.1:18792'  # Valid parser input, never contacted.
            runtime = root / '.swarm/local-web' / hashlib.sha256(address.encode()).hexdigest()[:16]
            runtime.mkdir(parents=True)
            state_path = runtime / 'server.json'
            state_path.write_text(json.dumps({'pid': 7, 'identity': {'fixture': True}}))
            before_state = state_path.read_bytes()
            binary = source / 'bin/swarm-local'
            binary.parent.mkdir()
            binary.write_bytes(b'INERT_INSTALLED')
            before_binary = binary.read_bytes()
            candidate = binary if cached_current else binary.with_name('swarm-local.candidate-unit')
            if not cached_current:
                candidate.write_bytes(b'INERT_CANDIDATE')
            selected_sha = hashlib.sha256(candidate.read_bytes()).hexdigest()
            argv = ['swarm_local.py', action, '--root', str(root), '--address', address, '--no-open']
            if prepared:
                argv += ['--prepared-build', selected_sha]
            selected_state = {'version': selected, 'binary_sha256': selected_sha}
            observation = {'schema_version': 1, 'binary': {'version': installed}}
            error = None
            unlink = Path.unlink

            def injected_unlink(path, *args, **kwargs):
                """Fail only the owned candidate cleanup; leave fixture teardown usable."""
                if cleanup_error is not None and path == candidate:
                    raise cleanup_error
                return unlink(path, *args, **kwargs)

            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.object(local, 'SOURCE', source))
                stack.enter_context(patch.object(sys, 'argv', argv))
                stack.enter_context(patch.object(local.os, 'umask'))
                owner = stack.enter_context(patch.object(local, 'owned', side_effect=[True, False] if identity_changed else None, return_value=True))
                fresh = stack.enter_context(patch.object(local, 'prepare_versioned_build', return_value=(candidate, selected_state)))
                recorded = stack.enter_context(patch.object(local, 'prepare_recorded', return_value=(candidate, selected_state)))
                live = stack.enter_context(patch.object(local, 'running_version', return_value=(observation, selected_sha if same_image else 'different-installed-image'), side_effect=read_error))
                pair = stack.enter_context(patch.object(local, 'require_matching_assertion_pair'))
                stop = stack.enter_context(patch.object(local, 'stop', side_effect=ReachedStop()))
                stack.enter_context(patch.object(local, 'authority_front', side_effect=ReachedFront()))
                stack.enter_context(patch.object(Path, 'unlink', injected_unlink))
                forbidden = [stack.enter_context(patch.object(owner_obj, name, side_effect=AssertionError('forbidden ' + name)))
                             for owner_obj, name in [(local.subprocess, 'run'), (local.subprocess, 'Popen'),
                                                     (local.os, 'kill'), (local.urllib.request, 'urlopen'),
                                                     (local.socket, 'socket')]]
                stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
                try:
                    local.main()
                except BaseException as caught:
                    error = caught
                self.assertEqual(fresh.call_count, int(not prepared))
                self.assertEqual(recorded.call_count, int(prepared))
                for boundary in forbidden:
                    boundary.assert_not_called()
                if action == 'build':
                    live.assert_not_called()
                if isinstance(error, ReachedStop):
                    pair.assert_called_once()
                    stop.assert_called_once()
                elif isinstance(error, ReachedFront):
                    pair.assert_called_once()
                    stop.assert_not_called()
                else:
                    pair.assert_not_called()
                    stop.assert_not_called()
                self.assertEqual(state_path.read_bytes(), before_state)
                self.assertEqual(binary.read_bytes(), before_binary)
            return error

    def test_fresh_downgrade_and_unknown_refuse_then_recover(self):
        """Valid control first; fresh older/unorderable versions must not stop."""
        self.assertIsInstance(self.exercise(), ReachedStop)
        for selected in ['v0.1.1-dev.10', 'unknown']:
            with self.subTest(selected=selected):
                self.assertIsInstance(self.exercise(selected=selected), ValueError)
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_prepared_downgrade_and_unknown_still_refuse(self):
        """The prior preselected path retains its refusal and healthy control."""
        self.assertIsInstance(self.exercise(prepared=True), ReachedStop)
        for selected in ['v0.1.1-dev.10', 'unknown']:
            with self.subTest(selected=selected):
                self.assertIsInstance(self.exercise(prepared=True, selected=selected), ValueError)
        self.assertIsInstance(self.exercise(prepared=True), ReachedStop)

    def test_fresh_changed_identity_refuses_then_recovers(self):
        """Ownership change after version observation refuses before stop."""
        self.assertIsInstance(self.exercise(), ReachedStop)
        self.assertIsInstance(self.exercise(identity_changed=True), RuntimeError)
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_fresh_read_error_preserves_original_then_recovers(self):
        """Read failure stays the exact original object rather than a success."""
        self.assertIsInstance(self.exercise(), ReachedStop)
        cause = OSError('inert live version read failure')
        self.assertIs(self.exercise(read_error=cause), cause)
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_cached_current_binary_survives_refusal(self):
        """A producer may return the installed cache image; never delete it."""
        self.assertIsInstance(self.exercise(cached_current=True), ReachedStop)
        cause = OSError('inert read failure with cached current image')
        self.assertIs(self.exercise(cached_current=True, read_error=cause), cause)
        self.assertIsInstance(self.exercise(cached_current=True), ReachedStop)

    def test_cleanup_error_keeps_both_causes_then_recovers(self):
        """After a valid control, preserve the read and candidate cleanup causes."""
        self.assertIsInstance(self.exercise(), ReachedStop)
        primary = OSError('inert version read failure')
        cleanup = PermissionError('inert candidate cleanup refusal')
        result = self.exercise(read_error=primary, cleanup_error=cleanup)
        self.assertIsInstance(result, BaseExceptionGroup)
        self.assertEqual(len(result.exceptions), 2)
        self.assertIs(result.exceptions[0], primary)
        self.assertIs(result.exceptions[1], cleanup)
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_build_only_does_not_probe_or_stop(self):
        """A build-only request may prepare older bytes, never replace a live image."""
        self.assertIsInstance(self.exercise(), ReachedStop)
        self.assertIsNone(self.exercise(action='build', selected='v0.1.1-dev.10'))
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_prepared_start_remains_same_image_only(self):
        """Identical image starts stay idempotent; a different image needs restart."""
        self.assertIsInstance(self.exercise(prepared=True, action='start', same_image=True), ReachedFront)
        self.assertIsInstance(self.exercise(prepared=True, action='start'), RuntimeError)
        self.assertIsInstance(self.exercise(prepared=True, action='start', same_image=True), ReachedFront)


if __name__ == '__main__':
    unittest.main(verbosity=2)
