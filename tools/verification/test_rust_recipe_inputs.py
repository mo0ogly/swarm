"""Check replay manifests cover actual literal Rust include inputs, read-only.

This is a coverage guard for declared recipes, not effective dependency discovery,
runtime execution, review or authority. Existing compiler qualification is broader.
"""
from pathlib import Path
import json
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/rust-migration'))
from qualification_inputs import rust_assets
from recipe import snapshot


class RustRecipeInputs(unittest.TestCase):
    """Every Rust recipe must archive SQL/config/context bytes it actually embeds."""
    def test_actual_literal_include_inputs_are_declared(self):
        assets = rust_assets(ROOT, set((ROOT / 'rust/crates').rglob('*.rs')))
        self.assertTrue(assets, 'empty include inventory is not a witness')
        recipes = []
        for path in sorted((ROOT / 'tools/verification/recipes').glob('*.json')):
            manifest = json.loads(path.read_bytes())
            if not any(pattern.startswith('rust/') for pattern in manifest['inputs']):
                continue
            recipes.append(path)
            with self.subTest(recipe=path.name), tempfile.TemporaryDirectory() as temporary:
                selected = snapshot(ROOT, manifest['inputs'], Path(temporary), path)
                missing = sorted(str(p.relative_to(ROOT)) for p in assets
                                 if str(p.relative_to(ROOT)) not in selected)
                self.assertEqual(missing, [], 'literal compiled bytes missing from replay archive')
        self.assertTrue(recipes, 'no selected Rust recipe is incomplete evidence')


if __name__ == '__main__':
    unittest.main()
