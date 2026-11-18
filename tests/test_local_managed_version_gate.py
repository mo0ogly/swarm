"""Inert main-path checks for fresh/preselected managed version refusals.

Build, owned version capture, identity, pair check, stop, process, signal and HTTP
boundaries are doubled. Only owned temporary fixture files are touched; reaching the
stop marker proves ordering in this Python caller, not a real service replacement.
"""
import contextlib
import hashlib
import io
import json
import pathlib
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import MagicMock, patch

from fixture_area import fixture_area

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import swarm_local as local


class ReachedStop(Exception):
    """End a healthy inert path before any lifecycle mutation."""


class ReachedFront(Exception):
    """End an idempotent start after its gates, without an actual browser call."""


class ManagedVersionGate(unittest.TestCase):
    """Exercise the actual main caller using synthetic producer/server receipts."""

    def exercise(self, **kwargs):
        """Bracket every caller scenario with a valid matching-operation control.

        Controls use the same managed/preselected and running/stopped path,
        with stable synthetic observations and no fault. The fixture checks
        exact producer dispatch, unchanged state/image and forbidden boundaries.
        Recovery runs in finally even when an injected result assertion fails.
        """
        prepared = kwargs.get('prepared', False)
        action = kwargs.get('action', 'restart')
        control = {'prepared': prepared, 'action': action,
                   'not_running': kwargs.get('not_running', False),
                   'same_image': prepared and action == 'start'}
        expected = None if action == 'build' else ReachedFront if prepared and action == 'start' else ReachedStop
        def check_control():
            """Require the exact healthy caller path for this operation."""
            result = self._exercise_once(**control)
            if expected is None:
                self.assertIsNone(result)
            else:
                self.assertIsInstance(result, expected)
        check_control()
        try:
            return self._exercise_once(**kwargs)
        finally:
            check_control()

    def _exercise_once(self, *, prepared=False, selected='v0.1.1-dev.12',
                 installed='v0.1.1-dev.11', identity_changed=False,
                 read_error=None, cached_current=False, cleanup_error=None,
                 action='restart', same_image=False, not_running=False,
                 pre_read_error=None, post_drift=False, post_version=None):
        """Return the caught exception after checking all forbidden boundaries.

        Inputs describe inert version/identity faults. Each call owns a fresh
        fixture and intercepts stop with a sentinel. The owned reader is
        doubled with scripted captures: an active fresh run must capture
        before and after the producer and hand it the exact pre DTO, a
        preselected run keeps one post capture and a stopped run none.
        Original server-record bytes and active-image bytes must survive
        every path; a stopped build may publish its inert candidate through
        the existing legacy path; process/network calls fail the test if reached. No result
        establishes kernel/process identity.
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
            expected_binary = candidate.read_bytes() if not_running and action == 'build' else before_binary
            selected_sha = hashlib.sha256(candidate.read_bytes()).hexdigest()
            argv = ['swarm_local.py', action, '--root', str(root), '--address', address, '--no-open']
            if prepared:
                argv += ['--prepared-build', selected_sha]
            selected_state = {'version': installed if same_image or cached_current else selected, 'binary_sha256': selected_sha}
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
                owner = stack.enter_context(patch.object(local, 'owned', side_effect=[False] if not_running else [True, False] if identity_changed else None, return_value=not not_running))
                fresh = stack.enter_context(patch.object(local, 'prepare_versioned_build', return_value=(candidate, selected_state)))
                recorded = stack.enter_context(patch.object(local, 'prepare_recorded', return_value=(candidate, selected_state)))
                active = (observation, selected_sha if same_image else hashlib.sha256(before_binary).hexdigest())
                readings = []
                if not_running:
                    readings = []
                elif pre_read_error is not None:
                    readings = [pre_read_error]
                elif prepared:
                    readings = [read_error if read_error is not None else active]
                else:
                    post = ({'schema_version': 1, 'binary': {'version': post_version or installed}}, hashlib.sha256(b'INERT_DRIFT').hexdigest() if post_drift else active[1])
                    readings = [active, read_error if read_error is not None else post]
                live = stack.enter_context(patch.object(local, 'owned_running_version', side_effect=readings))
                stack.enter_context(patch.object(local, 'running_version', side_effect=AssertionError('forbidden running_version')))
                hub = MagicMock()
                hub.attach_mock(live, 'live')
                hub.attach_mock(fresh, 'fresh')
                hub.attach_mock(recorded, 'recorded')
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
                self.assertEqual(fresh.call_count, 0 if pre_read_error is not None else int(not prepared))
                self.assertEqual(recorded.call_count, int(prepared))
                for boundary in forbidden:
                    boundary.assert_not_called()
                order = [call[0] for call in hub.mock_calls]
                if not_running:
                    self.assertEqual(order, ['fresh'])
                    live.assert_not_called()
                    fresh.assert_called_once_with(source, binary)
                elif pre_read_error is not None:
                    self.assertEqual(order, ['live'])
                    self.assertTrue(candidate.exists(), 'pre-capture failure must not delete any candidate')
                elif prepared:
                    self.assertEqual(order, ['recorded', 'live'])
                else:
                    self.assertEqual(order, ['live', 'fresh', 'live'])
                    fresh.assert_called_once_with(source, binary, installed={'version': installed, 'binary_sha256': active[1]})
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
                self.assertEqual(binary.read_bytes(), expected_binary)
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

    def test_active_build_observes_without_stop(self):
        """An active build captures pre/post around the producer, refuses older, never stops."""
        self.assertIsInstance(self.exercise(), ReachedStop)
        self.assertIsNone(self.exercise(action='build'))
        self.assertIsInstance(self.exercise(action='build', selected='v0.1.1-dev.10'), ValueError)
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_prepared_start_remains_same_image_only(self):
        """Identical image starts stay idempotent; a different image needs restart."""
        self.assertIsInstance(self.exercise(prepared=True, action='start', same_image=True), ReachedFront)
        self.assertIsInstance(self.exercise(prepared=True, action='start'), RuntimeError)
        self.assertIsInstance(self.exercise(prepared=True, action='start', same_image=True), ReachedFront)

    def test_pre_capture_failure_skips_producer_and_cleanup(self):
        """A failed pre-capture never starts the producer nor deletes any candidate."""
        self.assertIsInstance(self.exercise(), ReachedStop)
        cause = OSError('inert pre-capture refusal')
        self.assertIs(self.exercise(pre_read_error=cause), cause)
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_fresh_active_image_drift_refuses(self):
        """An active image changed between the fresh captures refuses without stop."""
        self.assertIsInstance(self.exercise(), ReachedStop)
        for variant in ({'post_drift': True}, {'post_version': 'v0.1.1-dev.10'}):
            with self.subTest(variant=variant):
                self.assertIsInstance(self.exercise(), ReachedStop)
                try:
                    self.assertIsInstance(self.exercise(**variant), RuntimeError)
                finally:
                    self.assertIsInstance(self.exercise(), ReachedStop)
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_stopped_build_keeps_legacy_no_probe(self):
        """A stopped build probes nothing, keeps legacy dispatch and may publish the candidate."""
        self.assertIsNone(self.exercise(not_running=True, action='build', cached_current=True))
        self.assertIsNone(self.exercise(not_running=True, action='build'))
        self.assertIsInstance(self.exercise(), ReachedStop)

    def test_preselected_keeps_single_post_capture(self):
        """The preselected path performs one post capture and no pre-snapshot."""
        self.assertIsInstance(self.exercise(prepared=True), ReachedStop)
        self.assertIsInstance(self.exercise(prepared=True, selected='v0.1.1-dev.10'), ValueError)
        self.assertIsInstance(self.exercise(prepared=True), ReachedStop)


class MainOwnedSnapshotWiring(unittest.TestCase):
 def test_active_build_passes_observation_to_producer(self):
  """Reach a healthy active build, then require the new producer DTO/capture order.

  Actual main and fixture locks/files run; ownership, version observation,
  producer and every process/network/stop boundary are explicit inert doubles.
  This tests receiver dispatch, not actual process ownership or native build.
  """
  with tempfile.TemporaryDirectory(prefix='snapshot-caller-',dir=fixture_area()) as area:
   source=pathlib.Path(area);root=source/'Root';address='127.0.0.1:18792';runtime=root/'.swarm/local-web'/hashlib.sha256(address.encode()).hexdigest()[:16];runtime.mkdir(parents=True);state=runtime/'server.json';state.write_text(json.dumps({'pid':7,'identity':{'fixture':True}}));before_state=state.read_bytes();binary=source/'bin/swarm-local';binary.parent.mkdir();binary.write_bytes(b'INERT_INSTALLED');before_binary=binary.read_bytes();candidate=binary.with_name(binary.name+'.candidate-unit');candidate.write_bytes(b'INERT_CANDIDATE');image_sha=hashlib.sha256(before_binary).hexdigest();observation={'schema_version':1,'binary':{'version':'v0.1.1-dev.11'}};selected={'version':'v0.1.1-dev.12','binary_sha256':hashlib.sha256(candidate.read_bytes()).hexdigest()};order=[]
   def observe(*args,**kwargs):
    """Record one synthetic capture and return the same stable image facts."""
    order.append('capture');return observation,image_sha
   def produce(*args,**kwargs):
    """Record producer dispatch, returning only an inert owned candidate."""
    order.append('producer');return candidate,selected
   with contextlib.ExitStack() as stack:
    stack.enter_context(patch.object(local,'SOURCE',source));stack.enter_context(patch.object(sys,'argv',['swarm_local.py','build','--root',str(root),'--address',address,'--no-open']));stack.enter_context(patch.object(local.os,'umask'));stack.enter_context(patch.object(local,'owned',return_value=True));capture=stack.enter_context(patch.object(local,'owned_running_version',side_effect=observe,create=True));fresh=stack.enter_context(patch.object(local,'prepare_versioned_build',side_effect=produce));forbidden=[stack.enter_context(patch.object(obj,name,side_effect=AssertionError('forbidden '+name))) for obj,name in [(local,'running_version'),(local,'prepare_recorded'),(local,'require_matching_assertion_pair'),(local,'stop'),(local.subprocess,'run'),(local.subprocess,'Popen'),(local.os,'kill'),(local.urllib.request,'urlopen'),(local.socket,'socket')]]
    result=local.main()
    # Valid control of the exact old active-build path precedes the new contract
    # assertion: no native/mutation failure is allowed to masquerade as wiring.
    self.assertEqual(result,0);fresh.assert_called_once();self.assertEqual(binary.read_bytes(),before_binary);self.assertEqual(state.read_bytes(),before_state)
    for boundary in forbidden:boundary.assert_not_called()
    self.assertEqual(fresh.call_args.kwargs.get('installed'),{'version':'v0.1.1-dev.11','binary_sha256':image_sha});self.assertEqual(capture.call_count,2);self.assertEqual(order,['capture','producer','capture'])

if __name__ == '__main__':
    unittest.main(verbosity=2)
