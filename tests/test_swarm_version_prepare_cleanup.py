"""Retain prepare qualification and cleanup causes using inert owned fixtures.

Actual prepare and fixture filesystem effects are exercised. Builder/profile,
source identity, verification, process/network boundaries are unit doubles;
no native build, real qualification, live Root or provider call is established.
Fixtures survive assertions and are cleaned by unittest even after failure.
"""
import contextlib
import errno
import hashlib
import io
import json
import os
import pathlib
import sys
import tempfile
import types
import unittest
from unittest.mock import patch
from fixture_area import fixture_area

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "tools"))
import swarm_version as version

class CleanupCauses(unittest.TestCase):

    def exercise(self, primary=None, cleanup=None):
        """Call real legacy prepare with inert build/probe boundaries in an owned fixture.

  Return original exceptions and paths while unittest cleanup keeps fixtures alive.
  Only fixture files are written; no compiler, process, network or provider is used.
  """
        temp = tempfile.TemporaryDirectory(prefix='prepare-causes-', dir=fixture_area())
        self.addCleanup(temp.cleanup)
        source = pathlib.Path(temp.name)
        binary = source / 'bin/swarm-local'
        binary.parent.mkdir()
        binary.write_bytes(b'INSTALLED_INERT')
        before = binary.read_bytes()
        actual = {'unit.rs': 'inert-source-hash'}
        (source / 'version_history.json').write_text(json.dumps({'releases': [{'version': 'v0.1.0'}], 'local_changes': {'summary': {'fr': 'Test', 'en': 'Test'}, 'items': [{'fr': 'Test', 'en': 'Test'}]}}))

        def identity(root, effective=None):
            """Return fixed candidate/delivery/effective fixture facts; no source or toolchain probe."""
            return ('candidate', {}) if effective is None else ('delivery', {}) if not effective else ('effective', dict(actual))

        def publish(path, data):
            """Write the supplied bytes or text only to a caller-owned fixture path; IO errors propagate."""
            pathlib.Path(path).write_bytes(data.encode() if isinstance(data, str) else data)

        def build(root, out, target, label):
            """Create an inert image and synthetic effective-input receipt in the owned evidence tree; execute nothing."""
            (out / 'profile').mkdir(parents=True)
            (out / 'profile/after.json').write_text(json.dumps(actual))
            image = out / 'frozen'
            image.write_bytes(b'INERT_CANDIDATE')
            return {'binary': str(image), 'metadata': {'version': label}}
        profile = types.SimpleNamespace(publish=publish, digest=lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest())
        builder = types.SimpleNamespace(profile=profile, build=build, release=lambda value: True)
        original_unlink = pathlib.Path.unlink

        def unlink(path, *args, **kwargs):
            """Inject failure only for the owned staged candidate; delegate every other unlink unchanged."""
            if cleanup is not None and path.parent == binary.parent and path.name.startswith(binary.name + '.candidate-'):
                raise cleanup
            return original_unlink(path, *args, **kwargs)
        result = None
        error = None
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.dict(os.environ, {k: v for k, v in os.environ.items() if k not in ['SWARM_VERSION', 'SWARM_RUST_TARGET_DIR']}, clear=True))
            stack.enter_context(patch.object(version, 'native_builder', return_value=builder))
            stack.enter_context(patch.object(version, 'source_identity', side_effect=identity))
            probe = stack.enter_context(patch.object(version, 'verify_candidate', side_effect=primary))
            stack.enter_context(patch.object(pathlib.Path, 'unlink', unlink))
            forbidden = [stack.enter_context(patch.object(owner, name, side_effect=AssertionError('forbidden ' + name))) for owner, name in [(version.subprocess, 'run'), (version.subprocess, 'Popen'), (version.subprocess, 'check_output'), (version.os, 'kill'), (version.urllib.request, 'urlopen')]]
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            try:
                result = version.prepare(source, binary)
            except BaseException as caught:
                error = caught
            for boundary in forbidden:
                boundary.assert_not_called()
        self.assertEqual(binary.read_bytes(), before)
        probe.assert_called_once()
        return (error, result, source)

    def control(self):
        """Require a healthy inert prepare and its staged image, without executing it."""
        error, result, source = self.exercise()
        self.assertIsNone(error)
        self.assertEqual(result[0].read_bytes(), b'INERT_CANDIDATE')
        self.assertTrue((source / 'deploy/local-build.json').is_file())

    def test_prepare_primary_and_cleanup_causes_survive_then_recover(self):
        """Control, dual failure with both original objects, then independent recovery."""
        self.control()
        primary = RuntimeError('inert qualification failed')
        cleanup = PermissionError(errno.EACCES, 'inert candidate cleanup denied')
        error, result, source = self.exercise(primary, cleanup)
        try:
            self.assertIsInstance(error, BaseExceptionGroup)
            self.assertEqual(error.exceptions, (primary, cleanup))
            self.assertIsNone(result)
            self.assertFalse((source / 'deploy/local-build.json').exists())
        finally:
            self.control()

    def test_prepare_primary_survives_successful_cleanup_then_recovers(self):
        """Control, original qualification failure after successful cleanup, recovery."""
        self.control()
        primary = RuntimeError('inert qualification failed')
        error, result, source = self.exercise(primary)
        try:
            self.assertIs(error, primary)
            self.assertIsNone(result)
            self.assertEqual(list((source / 'bin').glob('*.candidate-*')), [])
            self.assertFalse((source / 'deploy/local-build.json').exists())
        finally:
            self.control()

if __name__ == "__main__":
    unittest.main(verbosity=2)
