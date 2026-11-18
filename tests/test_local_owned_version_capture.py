"""Actual owned-reader checks for the launcher's version capture helper.

The real swarm_local.owned_running_version runs against one inert fixture
image held open through the routed /proc boundary. HTTP, session request,
stat/identity and close boundaries are explicit doubles; subprocess, socket
and signal boundaries are forbidden outright; the helper and the ownership
validator themselves are never mocked. No fixture file is executed and no
real process, provider or Root is contacted.
"""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from fixture_area import fixture_area
from test_local_live_engine_reader import SyntheticEngine

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import swarm_local as local

ADDRESS = '127.0.0.1:18792'
HEALTHY_BODY = {'schema_version': 1, 'binary': {'version': 'v0.1.1-dev.12'}}


class OwnedVersionCapture(unittest.TestCase):
    """Exercise the actual owned version capture on one synthetic engine."""

    def _capture_once(self, engine, *, body=None, on_http=None, query_failure=None,
                read_failure=None, delete_pathname=False):
        """Run the actual owned reader once and return (result, error, guard).

        Installs the engine's identity and close routes plus a guarded open
        that permits exactly one /proc/<pid>/exe open and nothing else
        (optionally deleting the fixture pathname afterwards so only the held
        descriptor can supply bytes), a stat route mimicking /proc for that
        deleted image, explicit session_request and urlopen doubles serving
        `body` under the one-second individual probe deadline, and forbidden
        subprocess, socket and kill boundaries. `on_http` fires mid-query the
        way a real image or identity replacement would. The guard records the
        open count, the held fd and whether the reader itself closed it; the
        fixture descriptor is released in every case and a deleted pathname
        is restored so later variants keep a usable fixture.
        """
        payload = HEALTHY_BODY if body is None else body
        image_bytes = engine.image.read_bytes()
        guard = {'opens': 0, 'requests': 0, 'fd': None, 'closed_by_reader': None,
                 'endpoint': None}

        def guarded_open(path, *args, **kwargs):
            """Permit exactly one open of the synthetic /proc image, none besides."""
            if str(path) != f"/proc/{engine.pid}/exe":
                raise AssertionError('owned reader opened a non-proc pathname: ' + str(path))
            guard['opens'] += 1
            if guard['opens'] > 1:
                raise AssertionError('owned reader reopened the process image pathname')
            fd = engine.route_open(path, *args, **kwargs)
            guard['fd'] = fd
            if delete_pathname:
                engine.image.unlink()
            return fd

        def routed_stat(path, *args, **kwargs):
            """Mimic /proc: a deleted image still stats through its held fd."""
            try:
                return engine.route_stat(path, *args, **kwargs)
            except FileNotFoundError:
                if delete_pathname and str(path) == f"/proc/{engine.pid}/exe" and engine.descriptor is not None:
                    return os.fstat(engine.descriptor)
                raise

        def fake_session_request(address, root, path):
            """Double the protected session request and pin the exact endpoint."""
            guard['requests'] += 1
            guard['endpoint'] = path
            if path != '/api/v1/native-version':
                raise AssertionError('unexpected endpoint: ' + str(path))
            if address != ADDRESS:
                raise AssertionError('unexpected address: ' + str(address))
            return ('session-request', path)

        def fake_urlopen(request, timeout=None):
            """Serve the scripted body under the existing one-second deadline."""
            if timeout != 1:
                raise AssertionError('owned reader must keep the individual probe deadline')
            if on_http is not None:
                on_http()
            if query_failure is not None:
                raise query_failure
            raw = payload if isinstance(payload, bytes) else json.dumps(payload).encode()
            return io.BytesIO(raw)

        result = error = None
        try:
            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.object(local, 'identity', side_effect=engine.fake_identity))
                stack.enter_context(patch.object(local.os, 'open', side_effect=guarded_open))
                stack.enter_context(patch.object(local.os, 'stat', side_effect=routed_stat))
                stack.enter_context(patch.object(local.os, 'close', side_effect=engine.route_close))
                stack.enter_context(patch.object(local.subprocess, 'run', side_effect=AssertionError('forbidden subprocess.run')))
                stack.enter_context(patch.object(local.subprocess, 'Popen', side_effect=AssertionError('forbidden subprocess.Popen')))
                stack.enter_context(patch.object(local.subprocess, 'check_output', side_effect=AssertionError('forbidden subprocess.check_output')))
                stack.enter_context(patch.object(local.os, 'kill', side_effect=AssertionError('forbidden os.kill')))
                stack.enter_context(patch.object(local.socket, 'socket', side_effect=AssertionError('forbidden socket.socket')))
                stack.enter_context(patch.object(local, 'session_request', side_effect=fake_session_request))
                stack.enter_context(patch.object(local.urllib.request, 'urlopen', side_effect=fake_urlopen))
                if read_failure is not None:
                    stack.enter_context(patch.object(local.os, 'read', side_effect=read_failure))
                result = local.owned_running_version(engine.state, ADDRESS, engine.image.parent)
        except BaseException as caught:
            error = caught
        finally:
            guard['closed_by_reader'] = engine.descriptor is None
            engine.release()
            if delete_pathname and not engine.image.exists():
                engine.image.write_bytes(image_bytes)
        return result, error, guard

    def healthy_control(self):
        """Verify a fresh valid fixture, exact DTO/hash and reader-owned close.

        Uses the same routed boundaries as each fault without inheriting its
        mutated identity or close failure; no process or network is contacted.
        """
        with tempfile.TemporaryDirectory(prefix='owned-capture-control-', dir=fixture_area()) as area:
            engine = SyntheticEngine(area)
            expected = hashlib.sha256(engine.image.read_bytes()).hexdigest()
            result, error, guard = self._capture_once(engine)
            self.assertIsNone(error)
            self.assertEqual(result, (HEALTHY_BODY, expected))
            self.assertEqual((guard['opens'], guard['requests']), (1, 1))
            self.assertTrue(guard['closed_by_reader'])
            engine.assert_descriptor_closed(guard['fd'])

    def capture(self, engine, **kwargs):
        """Bracket each scripted capture with independent valid and recovery controls.

        The fixture releases its descriptor even on failure. A deliberate close
        refusal remains an uncertain close in the reader, never a false success.
        """
        self.healthy_control()
        try:
            result, error, guard = self._capture_once(engine, **kwargs)
            if guard['fd'] is not None and engine.close_failure is None:
                self.assertTrue(guard['closed_by_reader'])
                engine.assert_descriptor_closed(guard['fd'])
            return result, error, guard
        finally:
            self.healthy_control()

    def test_healthy_capture_returns_dto_and_held_image_hash(self):
        """The reader returns the exact DTO plus the held descriptor's sha256."""
        with tempfile.TemporaryDirectory(prefix='owned-capture-', dir=fixture_area()) as area:
            engine = SyntheticEngine(area)
            expected = hashlib.sha256(engine.image.read_bytes()).hexdigest()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            self.assertEqual(result[0], HEALTHY_BODY)
            self.assertEqual(result[1], expected)
            self.assertEqual(guard['opens'], 1)
            self.assertEqual(guard['requests'], 1)
            self.assertEqual(guard['endpoint'], '/api/v1/native-version')
            self.assertTrue(guard['closed_by_reader'])
            engine.assert_descriptor_closed(guard['fd'])

    def test_open_query_read_failures_preserve_cause(self):
        """Open, query and read refusals keep the original OSError as __cause__."""
        with tempfile.TemporaryDirectory(prefix='owned-capture-', dir=fixture_area()) as area:
            engine = SyntheticEngine(area)
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            open_failure = OSError('inert image open refusal')
            engine.open_failure = open_failure
            try:
                result, error, guard = self.capture(engine)
                self.assertIsNone(result)
                self.assertIsInstance(error, RuntimeError)
                self.assertIs(error.__cause__, open_failure)
                self.assertIsNone(guard['fd'])
            finally:
                engine.reset()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            query_failure = OSError('inert native-version query refusal')
            result, error, guard = self.capture(engine, query_failure=query_failure)
            self.assertIsNone(result)
            self.assertIsInstance(error, RuntimeError)
            self.assertIs(error.__cause__, query_failure)
            self.assertTrue(guard['closed_by_reader'])
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            read_failure = OSError('inert descriptor read refusal')
            result, error, guard = self.capture(engine, read_failure=read_failure)
            self.assertIsNone(result)
            self.assertIsInstance(error, RuntimeError)
            self.assertIs(error.__cause__, read_failure)
            self.assertTrue(guard['closed_by_reader'])
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)

    def test_malformed_json_and_schema_refuse(self):
        """Unparseable or schema-invalid bodies refuse without a fabricated version."""
        with tempfile.TemporaryDirectory(prefix='owned-capture-', dir=fixture_area()) as area:
            engine = SyntheticEngine(area)
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            for label, body in [
                ('broken-json', b'{not-json'),
                ('wrong-schema', {'schema_version': 2, 'binary': {'version': 'v0.1.1-dev.12'}}),
                ('missing-binary', {'schema_version': 1}),
                ('empty-version', {'schema_version': 1, 'binary': {'version': ''}}),
                ('non-string-version', {'schema_version': 1, 'binary': {'version': 12}}),
            ]:
                with self.subTest(label=label):
                    result, error, guard = self.capture(engine, body=body)
                    self.assertIsNone(result)
                    self.assertIsInstance(error, ValueError)
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)

    def test_identity_and_inode_drift_refuse(self):
        """Identity or image change before or around the query refuses."""
        with tempfile.TemporaryDirectory(prefix='owned-capture-', dir=fixture_area()) as area:
            engine = SyntheticEngine(area)
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            engine.identity_now['start'] = '9003'
            result, error, guard = self.capture(engine)
            self.assertIsNone(result)
            self.assertIsInstance(error, RuntimeError)
            self.assertIn('changée', str(error))
            self.assertIsNone(guard['fd'])
            engine.reset()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)

            def swap_identity():
                """Flip the synthetic kernel start time the way PID reuse would."""
                engine.identity_now['start'] = '9002'
            result, error, guard = self.capture(engine, on_http=swap_identity)
            self.assertIsNone(result)
            self.assertIsInstance(error, RuntimeError)
            self.assertIn('changée', str(error))
            self.assertTrue(guard['closed_by_reader'])
            engine.reset()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)

            def swap_image():
                """Point the synthetic /proc image at the replacement inode."""
                engine.replaced = True
            result, error, guard = self.capture(engine, on_http=swap_image)
            self.assertIsNone(result)
            self.assertIsInstance(error, RuntimeError)
            self.assertIn('remplacée', str(error))
            self.assertTrue(guard['closed_by_reader'])
            engine.reset()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)

    def test_success_with_failed_close_refuses(self):
        """A close failure after a successful read refuses instead of succeeding."""
        with tempfile.TemporaryDirectory(prefix='owned-capture-', dir=fixture_area()) as area:
            engine = SyntheticEngine(area)
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            engine.close_failure = OSError('inert descriptor close refusal')
            result, error, guard = self.capture(engine)
            self.assertIsNone(result)
            self.assertIsInstance(error, RuntimeError)
            self.assertIn('non refermé', str(error))
            self.assertFalse(guard['closed_by_reader'])
            engine.reset()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)

    def test_body_error_with_failed_close_groups_both_causes(self):
        """A body refusal striking with a close failure keeps both causes, primary first."""
        with tempfile.TemporaryDirectory(prefix='owned-capture-', dir=fixture_area()) as area:
            engine = SyntheticEngine(area)
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            primary = OSError('inert descriptor read refusal')
            cleanup = OSError('inert descriptor close refusal')
            engine.close_failure = cleanup
            result, error, guard = self.capture(engine, read_failure=primary)
            self.assertIsNone(result)
            self.assertIsInstance(error, BaseExceptionGroup)
            self.assertEqual(len(error.exceptions), 2)
            self.assertIs(error.exceptions[0], primary)
            self.assertIs(error.exceptions[1], cleanup)
            engine.reset()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)

    def test_selected_pathname_different_or_absent_not_used(self):
        """A different or absent selected pathname never feeds the capture."""
        with tempfile.TemporaryDirectory(prefix='owned-capture-', dir=fixture_area()) as area:
            engine = SyntheticEngine(area)
            expected = hashlib.sha256(engine.image.read_bytes()).hexdigest()
            replacement = hashlib.sha256(engine.replacement.read_bytes()).hexdigest()
            decoy = Path(area) / 'selected-candidate-with-other-bytes'
            decoy.write_bytes(b'DECOY_SELECTED_BYTES_NEVER_OPENED')
            decoy_sha = hashlib.sha256(decoy.read_bytes()).hexdigest()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            self.assertEqual(result[1], expected)
            self.assertNotEqual(result[1], decoy_sha)
            self.assertNotEqual(result[1], replacement)
            self.assertEqual(guard['opens'], 1)
            decoy.unlink()
            result, error, guard = self.capture(engine)
            self.assertIsNone(error)
            self.assertEqual(result[1], expected)
            result, error, guard = self.capture(engine, delete_pathname=True)
            self.assertIsNone(error)
            self.assertEqual(result[0], HEALTHY_BODY)
            self.assertEqual(result[1], expected)
            self.assertEqual(guard['opens'], 1)
            self.assertTrue(guard['closed_by_reader'])
            engine.assert_descriptor_closed(guard['fd'])


if __name__ == '__main__':
    unittest.main(verbosity=2)