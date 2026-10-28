"""Regress stale-source, duplicate-entry and empty-surface refusals in the measurement tool.

Uses retained inventory and qualified normal-profile paths from explicit env.
Mutates only temporary input JSON, never product/source/runtime state. The real
successful collection is a separate recipe step; these are negative safeguards,
not migration-parity or permission tests. No watchdog/provider/Go execution.
"""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest


class MatrixRefusals(unittest.TestCase):
    """Keep malformed measurement inputs from producing a successful matrix."""

    def setUp(self):
        """Load explicit known-good input; fixture cleanup owns only its temp root."""
        self.fixture = tempfile.TemporaryDirectory(prefix='migration-matrix-')
        self.addCleanup(self.fixture.cleanup)
        self.base = Path(self.fixture.name)
        self.inventory = json.loads(Path(os.environ['SWARM_MATRIX_INVENTORY']).read_bytes())
        self.normal = Path(os.environ['SWARM_MATRIX_NORMAL_DIRECTORY'])
        path = Path('tools/rust-migration/build_behavior_matrix.py')
        spec = importlib.util.spec_from_file_location('migration_matrix_under_test', path)
        self.tool = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.tool)

    def refuse(self, expected):
        """Require the specific refusal and absence of a published PASS matrix."""
        path = self.base / 'inventory.json'
        path.write_text(json.dumps(self.inventory))
        out = self.base / 'output'
        with self.assertRaisesRegex(ValueError, expected):
            self.tool.build(path, self.normal, out)
        self.assertFalse((out / 'matrix.json').exists())
        self.assertTrue((out / 'method.py').is_file())

    def test_stale_dispatch_source_refuses_before_classification(self):
        """A changed declared source hash must not reuse old entry classifications."""
        name = 'rust/crates/swarm-cli/src/lib.rs'
        self.inventory['current_source_sha256'][name] = '0' * 64
        self.refuse('inventoried source changed:')

    def test_duplicate_public_entry_refuses_instead_of_counting_twice(self):
        """Same total count cannot conceal a duplicate and a missing CLI entry."""
        names = self.inventory['cli']['same_name_present']
        self.assertGreater(len(names), 1)
        names[1] = names[0]
        self.refuse('duplicate matrix row')

    def test_empty_surface_refuses_instead_of_reporting_zero_success(self):
        """A zero-sized CLI inventory is missing coverage, never successful scope."""
        self.inventory['cli']['same_name_present'] = []
        self.inventory['cli']['same_name_absent'] = []
        self.inventory['cli']['reference_count'] = 0
        self.refuse('empty or inconsistent surface: cli')


if __name__ == '__main__':
    unittest.main()
