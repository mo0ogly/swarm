"""Regression controls for Cargo admission and atomic workstation policy writes.

All compiler execution and malformed-policy cases use isolated fixture roots;
no production policy, compiler cache, running process or service is modified.
"""
import importlib.util
import errno
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).with_name('cargo_space_guard.py')
spec = importlib.util.spec_from_file_location('guard', SOURCE)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


class GuardTests(unittest.TestCase):
    """Exercise actual CLI pass/refusal/recovery and targeted filesystem failures."""

    def setUp(self):
        """Create a valid isolated compiler-control before each fault variant."""
        area = guard.ROOT / '.swarm-fixtures'
        area.mkdir(exist_ok=True)
        self.area = tempfile.TemporaryDirectory(prefix='cargo-space-guard-', dir=area)
        self.addCleanup(self.area.cleanup)
        self.root = Path(self.area.name)
        script = self.root / 'tools/verification/cargo_space_guard.py'
        script.parent.mkdir(parents=True)
        script.write_bytes(SOURCE.read_bytes())
        self.script = script
        self.policy = self.root / '.cargo/storage-policy.json'
        self.policy.parent.mkdir()
        self.write_policy(1)
        self.compiler = self.root / 'compiler.py'
        self.marker = self.root / 'compiler-started'
        self.compiler.write_text(
            'from pathlib import Path\n'
            f'Path({str(self.marker)!r}).write_text("actual compiler control")\n'
            'raise SystemExit(7)\n')
        valid = self.invoke('--out-dir', str(self.root / 'output'))
        self.assertEqual(valid.returncode, 7, valid.stderr)
        self.assertEqual(self.marker.read_text(), 'actual compiler control')
        self.marker.unlink()

    def write_policy(self, minimum):
        """Publish an explicit test threshold in the disposable fixture only."""
        self.policy.write_text(json.dumps({'schema_version': 1,
                                           'minimum_free_bytes': minimum}))

    def invoke(self, *arguments):
        """Run the real wrapper with a child that records whether it started."""
        return subprocess.run([sys.executable, str(self.script), sys.executable,
                               str(self.compiler), *arguments],
                              capture_output=True, text=True)

    def test_refusal_no_compiler_then_recovery(self):
        """Insufficient space refuses before side effects; corrected policy resumes."""
        self.write_policy(10**30)
        denied = self.invoke('--out-dir', str(self.root / 'output'))
        self.assertEqual(denied.returncode, os.EX_CANTCREAT)
        self.assertIn('compiler not started', denied.stderr)
        self.assertFalse(self.marker.exists())
        self.write_policy(1)
        self.assertEqual(self.invoke('--out-dir', str(self.root)).returncode, 7)
        self.assertTrue(self.marker.exists())

    def test_exact_boundary_and_separate_output_volume(self):
        """Equality admits; a second output volume below the threshold refuses."""
        good = SimpleNamespace(f_bavail=5, f_frsize=2)
        bad = SimpleNamespace(f_bavail=4, f_frsize=2)
        self.assertEqual(guard.check_space([self.root], 10, lambda _: good)
                         [0]['available_bytes'], 10)
        with self.assertRaisesRegex(ValueError, 'insufficient'):
            with patch.object(guard.os, 'statvfs', side_effect=[good, bad]):
                guard.check_space([self.root, self.root], 10, guard.os.statvfs)

    def test_measurement_error_is_not_free_space(self):
        """Unreadable filesystem statistics propagate rather than silently admit."""
        with self.assertRaisesRegex(OSError, 'measurement failed'):
            guard.check_space([self.root], 1,
                              lambda _: (_ for _ in ()).throw(OSError('measurement failed')))

    def test_malformed_and_missing_policy_refuse(self):
        """Unknown, boolean, zero, negative and malformed thresholds refuse."""
        for value in [True, 0, -1, '20', 1.5]:
            self.write_policy(value)
            self.assertEqual(self.invoke('--out-dir', str(self.root)).returncode,
                             os.EX_CANTCREAT)
            self.assertFalse(self.marker.exists())
        self.policy.write_text('{')
        self.assertEqual(self.invoke('--out-dir', str(self.root)).returncode,
                         os.EX_CANTCREAT)
        self.policy.unlink()
        self.assertEqual(self.invoke('--out-dir', str(self.root)).returncode,
                         os.EX_CANTCREAT)

    def test_unknown_policy_field_refuses(self):
        """An accidental extra policy field cannot become an ignored fallback."""
        self.policy.write_text('{"schema_version":1,"minimum_free_bytes":1,"typo":2}')
        self.assertEqual(self.invoke('--out-dir', str(self.root)).returncode,
                         os.EX_CANTCREAT)
        self.assertFalse(self.marker.exists())

    def test_query_passes_without_writing_artifacts(self):
        """Compiler discovery works even when no compilation space is available."""
        self.write_policy(10**30)
        self.assertEqual(self.invoke('--version').returncode, 7)
        self.assertTrue(self.marker.exists())

    def test_equals_spelling_and_malformed_output(self):
        """Both Cargo argument spellings work; an empty output path refuses."""
        self.assertEqual(self.invoke('--out-dir=' + str(self.root)).returncode, 7)
        self.marker.unlink()
        for arguments in [('--out-dir',), ('--out-dir=',)]:
            self.assertEqual(self.invoke(*arguments).returncode, os.EX_CANTCREAT)
            self.assertFalse(self.marker.exists())

    def test_policy_configuration_and_status(self):
        """The public configuration CLI persists exactly the effective threshold."""
        result = subprocess.run([sys.executable, str(self.script),
                                 '--set-minimum-free-bytes', '123'],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['minimum_free_bytes'], 123)
        self.assertEqual(guard.load_policy(self.policy)['minimum_free_bytes'], 123)

    def test_failed_publication_preserves_original(self):
        """Injected fsync ENOSPC keeps the original policy byte exact and removes staging."""
        before = self.policy.read_bytes()
        with patch.object(guard, 'POLICY', self.policy), \
                patch.object(guard, 'load_policy', return_value=json.loads(before)), \
                patch.object(guard.os, 'fsync', side_effect=OSError(errno.ENOSPC, 'no space left')):
            with self.assertRaisesRegex(OSError, 'no space left') as caught:
                guard.configure(123)
            self.assertEqual(caught.exception.errno, errno.ENOSPC)
        self.assertEqual(self.policy.read_bytes(), before)
        self.assertEqual(list(self.policy.parent.iterdir()), [self.policy])

    def test_post_replace_sync_failure_does_not_claim_rollback(self):
        """A durability error after publication propagates with changed bytes visible."""
        with patch.object(guard, 'POLICY', self.policy), \
                patch.object(guard, 'load_policy', return_value=guard.load_policy(self.policy)), \
                patch.object(guard.os, 'fsync', side_effect=[None, OSError('directory sync failed')]):
            with self.assertRaisesRegex(OSError, 'directory sync failed'):
                guard.configure(123)
        self.assertEqual(json.loads(self.policy.read_text())['minimum_free_bytes'], 123)


class CargoIntegration(unittest.TestCase):
    """Use installed Cargo/rustc on an owned dependency-free fixture, without doubles."""

    def test_compact_profile_actual_compiler_and_named_test(self):
        """Cargo consumes the wrapper/default flags, preserves assertions and executes a test."""
        area = guard.ROOT / '.swarm-fixtures'
        area.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='cargo-compact-control-', dir=area) as name:
            root = Path(name)
            (root/'src').mkdir()
            (root/'src/lib.rs').write_text(
                '#[test] fn actual_debug_assertions_retained() { assert!(cfg!(debug_assertions)); }\n')
            (root/'Cargo.toml').write_text(
                '[package]\nname="swarm-compact-profile-control"\nversion="0.0.0"\nedition="2021"\n[workspace]\n')
            (root/'.cargo').mkdir()
            # Explicit fixture policy: test data, never a production threshold override.
            (root/'.cargo/storage-policy.json').write_text(
                '{"schema_version":1,"minimum_free_bytes":1}\n')
            (root/'.cargo/config.toml').write_bytes((guard.ROOT/'.cargo/config.toml').read_bytes())
            script = root/'tools/verification/cargo_space_guard.py'
            script.parent.mkdir(parents=True)
            script.write_bytes(SOURCE.read_bytes())
            script.chmod(0o700)
            environment = dict(os.environ)
            for key in list(environment):
                if key.startswith(('CARGO_PROFILE_', 'CARGO_TARGET_', 'CARGO_BUILD_')) or key in {
                        'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                        'RUSTC_WORKSPACE_WRAPPER', 'CARGO_INCREMENTAL', 'CARGO_TARGET_DIR'}:
                    environment.pop(key)
            run = subprocess.run(['cargo', 'test', '--offline', '--target-dir',
                                   str(root/'target'), '-v'], cwd=root, env=environment,
                                  capture_output=True, text=True)
            self.assertEqual(run.returncode, 0, run.stderr)
            self.assertIn('actual_debug_assertions_retained ... ok', run.stdout)
            self.assertIn('1 passed', run.stdout)
            self.assertIn(str(script), run.stderr)
            self.assertNotIn('-C debuginfo=2', run.stderr)
            self.assertNotIn('-C incremental=', run.stderr)
            # Cargo may create an empty directory even when rustc incremental
            # output is disabled. Assert content and the actual compiler argv.
            self.assertFalse(any((root/'target/debug/incremental').rglob('*')))


if __name__ == '__main__':
    unittest.main(verbosity=2)
