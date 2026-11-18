"""Bounded refusal proof for tools/swarm_version.py::_fixture_area.

A failed 0700 adjustment of the delivery fixture area must propagate
unchanged (errno, exception object, cause) out of _fixture_area and
verify_candidate, leaving the existing area and its contents intact — no
cleanup, no retry, no fallback. Exactly two unittest branches: repository
checkout selection and installed-launcher fallback. No pytest, no new
dependency, no global environment rewrite, no native code at import, no /tmp:
all scratch lives under the repository's tests/fixture_area helper.
"""

import contextlib
import errno
import importlib.util
import io
import pathlib
import stat
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from fixture_area import fixture_area

_SPEC = importlib.util.spec_from_file_location(
    "swarm_version",
    Path(__file__).resolve().parents[1] / "tools/swarm_version.py")
version = importlib.util.module_from_spec(_SPEC)
# Definitions only run here: no native code, provider call or area creation.
_SPEC.loader.exec_module(version)


def _prepare_layout(scratch, kind):
    """Non-test helper: build a fake owner layout inside `scratch` (a directory
    handed out by the tests/fixture_area helper) and return
    (module_file, patched_home, expected_base).

    kind='repo' builds a checkout root whose `tools/` sibling makes
    _fixture_area pick `<checkout>/.swarm-fixtures/`. kind='installed' builds
    a layout without any `tools/` sibling so the function must consult
    pathlib.Path.home, which the caller patches to `patched_home`, a directory
    of the same fixture; the producer home lookup stays in the fixture; HOME is not modified. The `tools/`
    marker is mirrored onto the resolve() spelling of the layout so the
    expectation stays exact even on symlinked scratch paths.
    """
    if kind == 'repo':
        (scratch / 'checkout' / 'tools').mkdir(parents=True)
        module_file = scratch / 'checkout' / 'tools' / 'swarm_version.py'
        patched_home = scratch / 'unused-home'
    elif kind == 'installed':
        (scratch / 'installed' / 'lib').mkdir(parents=True)
        module_file = scratch / 'installed' / 'lib' / 'swarm_version.py'
        patched_home = scratch / 'patched-home'
    else:
        raise AssertionError('unknown layout kind: %r' % (kind,))
    patched_home.mkdir()
    resolved_base = Path(module_file).resolve().parent.parent
    if kind == 'repo':
        (resolved_base / 'tools').mkdir(parents=True, exist_ok=True)
    expected_base = resolved_base if kind == 'repo' else patched_home
    return module_file, patched_home, expected_base


def _refusing_chmod(area, injected):
    """Non-test helper: return a pathlib.Path.chmod stand-in that raises
    `injected` only when the target is the selected `area`, delegating every
    other target to the real method."""
    real_chmod = pathlib.Path.chmod

    def chmod(target, mode):
        if target == area:
            raise injected
        return real_chmod(target, mode)

    return chmod


def _assert_witness_intact(test, area):
    """Non-test helper: the witness area still exists 0700 with its contents."""
    test.assertTrue(area.is_dir())
    test.assertEqual(stat.S_IMODE(area.stat().st_mode), 0o700)
    test.assertEqual(
        (area / 'witness.txt').read_text(encoding='utf-8'), 'witness')
    test.assertEqual(
        (area / 'witness-nested' / 'inner.txt').read_text(encoding='utf-8'),
        'inner')


def _run_refusal_scenario(test, kind):
    """Non-test helper: drive one bounded chmod-refusal scenario for one
    selection branch of the loaded production module, under a scratch
    directory taken from the tests/fixture_area helper (never /tmp).

    (1) Establish a valid witness: exact area path, mode 0700, contents on
    disk. (2) Inject PermissionError(errno.EACCES) only for the selected
    area's chmod; the original exception object must leave _fixture_area
    unchanged (identity, errno, no cause) while the area and its contents
    survive with no cleanup and no fallback. (3) The same refusal must reach
    verify_candidate before any subprocess.run/Popen or HTTP use; the parent
    invents no candidate success. (4) Remove only the chmod stand-in and
    prove healthy recovery under the same selection conditions (the
    pathlib.Path.home and module __file__ patches stay active throughout).
    """
    with tempfile.TemporaryDirectory(dir=fixture_area()) as scratch:
        scratch = Path(scratch)
        module_file, patched_home, expected_base = _prepare_layout(scratch, kind)
        area = expected_base / '.swarm-fixtures'
        area.mkdir(mode=0o700, parents=True)
        (area / 'witness.txt').write_text('witness', encoding='utf-8')
        (area / 'witness-nested').mkdir()
        (area / 'witness-nested' / 'inner.txt').write_text('inner', encoding='utf-8')
        with mock.patch.object(pathlib.Path, 'home',
                               mock.Mock(return_value=patched_home)), \
                mock.patch.object(version, '__file__', str(module_file)):
            witness = version._fixture_area()
            test.assertEqual(witness, area)
            _assert_witness_intact(test, area)
            injected = PermissionError(
                errno.EACCES, 'Permission denied', str(area))
            with mock.patch.object(pathlib.Path, 'chmod',
                                   _refusing_chmod(area, injected)), \
                    mock.patch.object(version, 'subprocess') as subprocess_mock, \
                    mock.patch.object(version, 'urllib') as urllib_mock:
                with test.assertRaises(PermissionError) as direct:
                    version._fixture_area()
                test.assertIs(direct.exception, injected)
                test.assertEqual(direct.exception.errno, errno.EACCES)
                test.assertIsNone(direct.exception.__cause__)
                _assert_witness_intact(test, area)
                if kind == 'repo':
                    test.assertFalse(
                        (patched_home / '.swarm-fixtures').exists())
                with contextlib.redirect_stdout(io.StringIO()):
                    with test.assertRaises(PermissionError) as guarded:
                        version.verify_candidate(
                            area / 'no-such-native-binary',
                            'v0.0.0', 'unused notes')
                test.assertIs(guarded.exception, injected)
                test.assertEqual(guarded.exception.errno, errno.EACCES)
                subprocess_mock.run.assert_not_called()
                subprocess_mock.Popen.assert_not_called()
                urllib_mock.request.urlopen.assert_not_called()
                _assert_witness_intact(test, area)
            recovered = version._fixture_area()
            test.assertEqual(recovered, area)
            _assert_witness_intact(test, area)


class FixtureAreaChmodRefusal(unittest.TestCase):
    """A failed 0700 chmod is a preparation-boundary refusal, never success."""

    def test_repo_selection_refuses_failed_chmod(self):
        """Checkout selection: EACCES on the area chmod propagates unchanged
        through _fixture_area and verify_candidate; area and contents stay."""
        _run_refusal_scenario(self, 'repo')

    def test_installed_fallback_refuses_failed_chmod(self):
        """Installed fallback: EACCES on the area chmod propagates unchanged
        through _fixture_area and verify_candidate; area and contents stay."""
        _run_refusal_scenario(self, 'installed')


if __name__ == '__main__':
    unittest.main()