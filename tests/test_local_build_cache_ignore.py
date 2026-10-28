"""Verify ordinary Git cache exclusions, not Codex app snapshot behavior.

Read the actual repository rules; a separate disposable repository demonstrates
that ignore never removes tracked files. No main index, history, app process,
provider or retained proof is changed. Failed disposable fixtures are preserved.
"""
from pathlib import Path
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def git(root, *args):
    """Run explicit local Git without inherited index/worktree overrides or hooks.

    No shell, commit, remote, timeout, retry or main-index mutation is added.
    Return the original status/output; each test validates its expected result.
    """
    env = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
    return subprocess.run(['git', '-c', 'core.hooksPath=/dev/null', '-c',
                           'core.fsmonitor=false', '-c', 'core.excludesFile=/dev/null', *args],
                          cwd=root, env=env, stdin=subprocess.DEVNULL, capture_output=True)


class LocalBuildCacheIgnore(unittest.TestCase):
    """Source eligibility and matcher behavior; no filesystem cleanup authority."""

    def test_root_and_nested_caches_are_ignored(self):
        """Parent and nested compiler caches match the actual saved rules."""
        for path in ['.cache/ignore-probe', '.cache/snap-probe/.cache/build/debug/deps/a.rlib', 'rust/target']:
            with self.subTest(path=path):
                result = git(ROOT, 'check-ignore', '--no-index', '-v', '--', path)
                self.assertEqual(result.returncode, 0, result.stderr.decode(errors='replace'))
                self.assertIn(path.encode(), result.stdout)

    def test_maintained_sources_and_recipes_remain_eligible(self):
        """Concrete current Rust/Python/recipe/docs inputs are not hidden by ignore."""
        for path in ['rust/crates/swarm-engine/src/root_store/reported_costs.rs',
                     'rust/crates/swarm-engine/tests/reported_costs.rs',
                     'rust/crates/swarm-cli/src/costs.rs',
                     'tests/test_local_build_cache_ignore.py',
                     'tools/verification/recipes/reported-costs.json',
                     'docs/architecture/REPORTED-COSTS.md']:
            with self.subTest(path=path):
                self.assertTrue((ROOT / path).is_file())
                result = git(ROOT, 'check-ignore', '--no-index', '--', path)
                self.assertEqual(result.returncode, 1, result.stderr.decode(errors='replace'))
                self.assertEqual(result.stdout, b'')

    def test_ignore_keeps_data_and_already_tracked_files(self):
        """A valid isolated index witness survives adopting the same ignore rules."""
        root = Path(tempfile.mkdtemp(prefix='swarm186-ignore-'))
        try:
            self.assertEqual(git(root, 'init', '--quiet').returncode, 0)
            cache = root / '.cache'
            cache.mkdir()
            value = cache / 'retained-data'
            original = b'owned retained cache data'
            value.write_bytes(original)
            # Counterexample: without a cache rule, ordinary Git includes it.
            self.assertEqual(git(root, 'check-ignore', '--no-index', '--', '.cache/retained-data').returncode, 1)
            self.assertEqual(git(root, 'add', '--', '.cache/retained-data').returncode, 0)
            tracked = git(root, 'ls-files', '--', '.cache/retained-data')
            self.assertEqual(tracked.stdout, b'.cache/retained-data\n')
            (root / '.gitignore').write_bytes((ROOT / '.gitignore').read_bytes())
            self.assertEqual(git(root, 'check-ignore', '--no-index', '--', '.cache/retained-data').returncode, 0)
            self.assertEqual(value.read_bytes(), original)
            self.assertEqual(git(root, 'ls-files', '--', '.cache/retained-data').stdout, tracked.stdout)
        except BaseException:
            print('Retained ignore fixture: ' + str(root), file=sys.stderr)
            raise
        else:
            shutil.rmtree(root)


if __name__ == '__main__':
    unittest.main()
