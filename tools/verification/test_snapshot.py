"""Isolated snapshot regressions on a fake repository; no compiler or live root."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('snapshot_tool', Path(__file__).with_name('snapshot.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class SnapshotTests(unittest.TestCase):
    def repo(self, tmp):
        """Fake repo: declared src/a.txt, undeclared src/secret.txt, deps dir, recipe."""
        root = Path(tmp) / 'repo'
        (root / 'src').mkdir(parents=True)
        (root / 'deps').mkdir()
        (root / 'tools/verification').mkdir(parents=True)
        (root / 'src/a.txt').write_text('declared')
        (root / 'src/secret.txt').write_text('undeclared')
        (root / 'deps/lib.js').write_text('dependency')
        (root / 'tools/verification/recipe.py').write_bytes((Path(__file__).with_name('recipe.py')).read_bytes())
        recipe = dict(schema_version=1, id='fake', inputs=['src/a.txt'], environment={},
                      steps=[dict(id='cat', command=['cat', 'src/a.txt'], expected_exit=0, required_stdout=['declared'], required_stderr=[])])
        path = root / 'tools/verification/recipes/fake.json'
        path.parent.mkdir()
        path.write_text(json.dumps(recipe))
        return root, path

    def test_witness_copies_only_declared_inputs_and_links_dependencies(self):
        with tempfile.TemporaryDirectory() as tmp:
            root, recipe = self.repo(tmp)
            snap = Path(tmp) / 'snap'
            inventory = module.freeze(root, recipe, snap, ['deps'])
            self.assertEqual((snap / 'src/a.txt').read_text(), 'declared')
            self.assertFalse((snap / 'src/secret.txt').exists(), 'undeclared input must not leak in')
            self.assertTrue((snap / 'deps').is_symlink())
            self.assertIn('tools/verification/recipe.py', inventory)
            self.assertIn('tools/verification/recipes/fake.json', inventory)
            self.assertTrue((snap / '.snapshot-inventory.json').exists())

    def test_refusals(self):
        with tempfile.TemporaryDirectory() as tmp:
            root, recipe = self.repo(tmp)
            existing = Path(tmp) / 'exists'
            existing.mkdir()
            with self.assertRaises(FileExistsError):
                module.freeze(root, recipe, existing, [])
            with self.assertRaisesRegex(ValueError, 'snapshot-link-invalid'):
                module.freeze(root, recipe, Path(tmp) / 's1', ['../outside'])
            with self.assertRaisesRegex(ValueError, 'snapshot-link-target-missing'):
                module.freeze(root, recipe, Path(tmp) / 's2', ['nope'])
            data = json.loads(recipe.read_text())
            data['inputs'] = ['src/missing-*.txt']
            recipe.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'recipe-input-pattern-empty'):
                module.freeze(root, recipe, Path(tmp) / 's3', [])

    def test_undeclared_embedded_input_is_refused_before_copy(self):
        with tempfile.TemporaryDirectory() as tmp:
            root, recipe = self.repo(tmp)
            (root / 'src/lib.rs').write_text('const A: &str = include_str!(\n    "a.txt"\n);\n'
                                             'const B: &[u8] = include_bytes!("../deps/lib.js");\n')
            data = json.loads(recipe.read_text())
            data['inputs'] = ['src/a.txt', 'src/lib.rs']
            recipe.write_text(json.dumps(data))
            snap = Path(tmp) / 'snap'
            with self.assertRaisesRegex(ValueError, r'snapshot-embedded-input-undeclared: deps/lib\.js$'):
                module.freeze(root, recipe, snap, ['deps'])
            self.assertFalse(snap.exists(), 'a refusal must leave no partial snapshot')
            # Witness with the same preconditions: once declared, the same source freezes.
            data['inputs'].append('deps/lib.js')
            recipe.write_text(json.dumps(data))
            module.freeze(root, recipe, snap, [])
            self.assertEqual((snap / 'deps/lib.js').read_text(), 'dependency')
            # A target escaping the repository can never be declared: refused by name.
            (root / 'src/lib.rs').write_text('const C: &str = include_str!("../../outside.txt");\n')
            with self.assertRaisesRegex(ValueError, r'snapshot-embedded-input-undeclared: \.\./outside\.txt$'):
                module.freeze(root, recipe, Path(tmp) / 'snap2', [])

    def test_end_to_end_run_passes_in_snapshot_and_fails_on_undeclared_read(self):
        with tempfile.TemporaryDirectory() as tmp:
            root, recipe = self.repo(tmp)
            tool = Path(__file__).with_name('snapshot.py')
            ok = subprocess.run([sys.executable, str(tool), '--root', str(root), '--recipe', str(recipe),
                                 '--snapshot', str(Path(tmp) / 'run1'), '--output', str(Path(tmp) / 'ev1')], capture_output=True, text=True)
            self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)
            data = json.loads(recipe.read_text())
            data['steps'][0]['command'] = ['cat', 'src/secret.txt']
            data['steps'][0]['required_stdout'] = ['undeclared']
            recipe.write_text(json.dumps(data))
            bad = subprocess.run([sys.executable, str(tool), '--root', str(root), '--recipe', str(recipe),
                                  '--snapshot', str(Path(tmp) / 'run2'), '--output', str(Path(tmp) / 'ev2')], capture_output=True, text=True)
            self.assertEqual(bad.returncode, 1, 'reading an undeclared input must FAIL in the snapshot')


if __name__ == '__main__':
    unittest.main()
