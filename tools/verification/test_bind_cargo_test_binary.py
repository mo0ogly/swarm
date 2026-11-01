"""Typed counter-proof for the linked-output contract (paea-approval-form-rust).

The recipe contract now distinguishes declared immutable inputs (sources,
config, retained fixture) from the cargo test binary identity, which the run
itself produces and bind_cargo_test_binary.py retains as a LINKED OUTPUT.
These tests prove both directions of that distinction without any live root,
provider or real cargo call:

- binding the produced artifact succeeds, the retained record is exact and a
  failed resolution never creates or rewrites it; a second run rebinds the
  rebuilt binary atomically (the record is a cache, not append-only evidence);
- `--verify-stable` refuses a changed path or digest (a real concurrent
  rebuild of a compiled input cannot be masked) without rewriting the
  retained record; an ambiguous/failed cargo resolution refuses with exit 2;
- at recipe level, a step-created build artifact outside declared inputs
  passes (the run4 blocker is gone), while creating a file under a declared
  input pattern still fails `recipe-declared-inputs-changed` — the contract
  distinction does not hide the production of generated files, it removes
  them from the immutable inventory. Modification of a declared source is
  already refused by test_recipe.py::test_zero_tests_and_source_mutation_cannot_pass.
"""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest import mock

import sys as _fa_sys
from pathlib import Path as _fa_Path
_fa_sys.path.insert(0, str(_fa_Path(__file__).resolve().parents[2] / 'tests'))
from fixture_area import fixture_area

_REAL_TESTS = Path(__file__).resolve().parents[2] / 'tests'

spec = importlib.util.spec_from_file_location(
    'bind_cargo_test_binary', Path(__file__).with_name('bind_cargo_test_binary.py'))
binder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(binder)

recipe_spec = importlib.util.spec_from_file_location(
    'recipe', Path(__file__).with_name('recipe.py'))
recipe = importlib.util.module_from_spec(recipe_spec)
recipe_spec.loader.exec_module(recipe)


class LinkedOutputBinderTests(unittest.TestCase):
    """Exercise BOUND, STABLE and both refusal variants on finite fixtures."""

    def setUp(self):
        """Create a private fixture executable standing for the cargo artifact."""
        self.temporary = tempfile.TemporaryDirectory(dir=fixture_area())
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.executable = self.root / 'paea_git_call-460ea6e2a8fa6747'
        self.executable.write_text('#!/bin/sh\nexit 0\n')
        self.executable.chmod(0o700)
        self.deps = self.root / 'deps'
        self.deps.mkdir()
        self.output = self.root / 'binding.json'

    def bind(self, executable):
        """Bind once with the given resolution double; return (code, stdout)."""
        with mock.patch.object(binder, 'resolve', return_value=executable):
            with contextlib.redirect_stdout(io.StringIO()) as stream:
                code = binder.bind(self.deps, 'paea_git_call', self.output)
        return code, stream.getvalue()

    def verify(self, executable):
        """Verify stability with the given resolution double; return (code, out)."""
        with mock.patch.object(binder, 'resolve', return_value=executable):
            with contextlib.redirect_stdout(io.StringIO()) as stream:
                code = binder.verify_stable(self.deps, 'paea_git_call', self.output)
        return code, stream.getvalue()

    def test_bind_records_exact_identity_and_compact_status(self):
        """BOUND stdout carries the compact token and the record is byte-exact."""
        code, stdout = self.bind(self.executable)
        self.assertEqual(code, 0)
        # Compact separators are part of the contract: the recipe step asserts
        # the exact token "status":"BOUND" (no space) in required_stdout.
        self.assertIn('"status":"BOUND"', stdout)
        self.assertNotIn('"status": "BOUND"', stdout)
        record = json.loads(self.output.read_bytes())
        self.assertEqual(record['schema_version'], 1)
        self.assertEqual(record['target'], 'paea_git_call')
        self.assertEqual(record['artifact'], str(self.executable))
        self.assertEqual(record['sha256'], recipe.digest(self.executable.read_bytes()))
        self.assertEqual(record['bytes'], self.executable.stat().st_size)

    def test_rerun_rebinds_a_rebuilt_binary_atomically(self):
        """A second run replaces the retained record; a failed re-bind keeps it.

        Counter-witness for the run2 step-6 failure: the manifest binds to a
        fixed `{root}/.cache/...` path and every run legitimately rebuilds the
        binary (the build step forces it), so a re-bind must atomically
        replace the prior record instead of dying on
        `FileExistsError` from an exclusive link. A resolution refusal still
        leaves the retained record exactly as the previous run wrote it.
        """
        code, stdout = self.bind(self.executable)
        self.assertEqual(code, 0)
        first = self.output.read_bytes()
        # The next run rebuilds: cargo emits a new suffix and fresh bytes.
        rebuilt = self.root / 'paea_git_call-nextsuffix'
        rebuilt.write_text('#!/bin/sh\nexit 0\n# run 2\n')
        rebuilt.chmod(0o700)
        code, stdout = self.bind(rebuilt)
        self.assertEqual(code, 0)
        self.assertIn('"status":"BOUND"', stdout)
        replaced = json.loads(self.output.read_bytes())
        self.assertEqual(replaced['artifact'], str(rebuilt))
        self.assertEqual(replaced['sha256'], recipe.digest(rebuilt.read_bytes()))
        self.assertNotEqual(self.output.read_bytes(), first,
                            'the record must document the binary this run built')
        # A refused re-bind never damages the record the previous run wrote.
        with mock.patch.object(binder, 'resolve', return_value=None), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(binder.bind(self.deps, 'paea_git_call', self.output), 2)
        self.assertEqual(json.loads(self.output.read_bytes()), replaced)

    def test_failed_resolution_never_creates_or_rewrites_the_record(self):
        """A resolution refusal exits 2 and leaves any prior binding intact.

        With `resolve` itself stubbed out there is no stdout: the typed
        REFUSED emission belongs to the real resolver and is covered by
        test_resolve_typed_refusals; here only the record contract is proven.
        """
        self.assertEqual(self.bind(self.executable)[0], 0)
        original = self.output.read_bytes()
        fresh = self.root / 'binding-absent.json'
        with mock.patch.object(binder, 'resolve', return_value=None):
            with contextlib.redirect_stdout(io.StringIO()) as stream:
                self.assertEqual(binder.bind(self.deps, 'paea_git_call', fresh), 2)
        self.assertEqual(stream.getvalue(), '')
        self.assertFalse(fresh.exists(), 'no record may appear for a refused bind')
        self.assertEqual(self.output.read_bytes(), original)

    def test_resolve_typed_refusals(self):
        """The real resolver emits compact typed REFUSED on cargo failure/ambiguity."""
        def cargo(returncode, stdout, stderr):
            """Stub the cargo subprocess with a finite canned result."""
            completed = mock.Mock(returncode=returncode, stdout=stdout, stderr=stderr)
            return mock.patch.object(binder.subprocess, 'run', return_value=completed)
        # Cargo itself failed: the raw stderr tail is retained in the cause.
        with cargo(101, '', 'error: could not compile\n'), \
                contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertEqual(binder.resolve(self.deps, 'paea_git_call'), None)
        self.assertIn('"status":"REFUSED"', stream.getvalue())
        self.assertIn('"cause":"cargo-artifact-resolution-failed"', stream.getvalue())
        self.assertIn('could not compile', stream.getvalue())
        # Two executables match the target: cargo owns the suffix, ambiguity refuses.
        def artifact_line(executable):
            """One line of cargo's message-format=json artifact stream."""
            return json.dumps({'reason': 'compiler-artifact', 'executable': executable})
        ambiguous = '\n'.join(artifact_line(str(self.root / name))
                              for name in ('paea_git_call-a', 'paea_git_call-b'))
        with cargo(0, ambiguous + '\n', ''), contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertEqual(binder.resolve(self.deps, 'paea_git_call'), None)
        self.assertIn('"cause":"cargo-artifact-ambiguous"', stream.getvalue())
        # Exactly one existing executable resolves to its exact emitted path.
        self.executable.write_text('#!/bin/sh\nexit 0\n')
        self.executable.chmod(0o700)
        single = '\n'.join([artifact_line(str(self.executable)), artifact_line(None)])
        with cargo(0, single + '\n', ''):
            self.assertEqual(binder.resolve(self.deps, 'paea_git_call'), self.executable)

    def test_verify_stable_refuses_changed_digest_or_path_without_rewrite(self):
        """Any identity change of the consumed binary fails typed, record intact."""
        self.assertEqual(self.bind(self.executable)[0], 0)
        original = self.output.read_bytes()
        original_bytes = self.executable.read_bytes()
        # Digest change, same path: the artifact was rebuilt in place.
        rebuilt = self.root / 'paea_git_call-460ea6e2a8fa6747'
        rebuilt.write_text('#!/bin/sh\nexit 3\n')
        rebuilt.chmod(0o700)
        # Path change, identical bytes: cargo re-emitted under a new suffix.
        twin = self.root / 'paea_git_call-nextsuffix'
        twin.write_bytes(original_bytes)
        twin.chmod(0o700)
        for name, candidate in (('digest', rebuilt), ('path', twin)):
            with self.subTest(change=name):
                code, stdout = self.verify(candidate)
                self.assertEqual(code, 1)
                self.assertIn('"cause":"bound-binary-unstable"', stdout)
                self.assertIn('"retained"', stdout)
                self.assertIn('"observed"', stdout)
                self.assertEqual(self.output.read_bytes(), original,
                                 'verify-stable must never rewrite the record')
        # Restore the bound bytes, then the exact same artifact stays STABLE.
        rebuilt.write_bytes(original_bytes)
        rebuilt.chmod(0o700)
        code, stdout = self.verify(self.executable)
        self.assertEqual(code, 0)
        self.assertIn('"status":"STABLE"', stdout)
        self.assertEqual(self.output.read_bytes(), original)


class DeclaredInputDistinctionTests(unittest.TestCase):
    """Recipe-level witness that the input/output split does not mask changes."""

    def setUp(self):
        """Rebuild the isolated fake repository used by test_recipe.py."""
        self.temporary = tempfile.TemporaryDirectory(dir=fixture_area())
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / 'tests').mkdir()
        shutil.copy2(_REAL_TESTS / 'fixture_area.py', self.root / 'tests' / 'fixture_area.py')
        (self.root / '.swarm').mkdir()
        (self.root / '.swarm' / 'engine-settings.json').write_text(json.dumps(
            {'values': {'runtime_temporary_directory': str(self.root / '.swarm-fixtures')}}))
        (self.root / 'source.txt').write_text('original\n')
        self.path = self.root / 'recipe.json'

    def step(self, code, name='witness'):
        """Build a finite subprocess assertion, with no shell or temporal deadline."""
        return dict(id=name, command=[sys.executable, '-c', code], expected_exit=0,
                    required_stdout=['named-test PASS'], required_stderr=[])

    def write(self, inputs, steps):
        """Persist an explicit recipe declaring exactly the given inputs."""
        self.path.write_text(json.dumps(dict(
            schema_version=1, id='fixture', inputs=inputs, environment={}, steps=steps)))

    def run_fixture(self, name):
        """Execute once into fresh evidence and return observed result plus location."""
        output = self.root / name
        return recipe.run_recipe(self.root, self.path, output), output

    def test_step_created_build_artifact_outside_declared_inputs_passes(self):
        """Producing the run4 fingerprint path passes once it is not declared.

        The created path mirrors the exact file that made run4 fail
        (`.fingerprint/swarm-web-*/test-integration-test-paea_git_call`): the
        fixed contract no longer declares it, so the recipe's own artifact
        production passes and `source_stable` stays true.
        """
        artifact = ('.cache/paea-verification-target/debug/.fingerprint/'
                    'swarm-web-460ea6e2a8fa6747/test-integration-test-paea_git_call')
        self.write(['source.txt'], [self.step(
            "from pathlib import Path\n"
            "target = Path(%r)\n"
            "target.parent.mkdir(parents=True, exist_ok=True)\n"
            "target.write_text('37b051\\n')\n"
            "print('named-test PASS')" % artifact)])
        result, output = self.run_fixture('artifact-production')
        self.assertEqual(result['status'], 'PASS',
                         (output / 'result.json').read_text())
        self.assertTrue(result['source_stable'])

    def test_creation_under_a_declared_pattern_still_refuses(self):
        """A step-created file matching a declared glob is never masked.

        Counter-witness for the contract change: generated files must live
        outside declared patterns. Creating one inside a declared directory
        changes the inventory between snapshots and reproduces the run4
        failure signature `recipe-declared-inputs-changed`.
        """
        (self.root / 'generated').mkdir()
        (self.root / 'generated' / 'seed.meta').write_text('seed\n')
        self.write(['generated/*.meta'], [self.step(
            "from pathlib import Path\n"
            "Path('generated/fingerprint.meta').write_text('37b051\\n')\n"
            "print('named-test PASS')")])
        result, output = self.run_fixture('generated-creation')
        self.assertEqual(result['failure']['cause'], 'recipe-declared-inputs-changed')
        self.assertFalse(result['source_stable'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
