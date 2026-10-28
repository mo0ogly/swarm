"""Regress operation measurement without running Go, Rust, providers or a Root.

Tests use retained source contracts and private temporary output only. They prove
collector success/refusal behavior, never implementation parity or acceptance.
"""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

TOOL = Path('tools/rust-migration/collect_cli_operations.py')
spec = importlib.util.spec_from_file_location('cli_operation_inventory', TOOL)
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)


class OperationInventory(unittest.TestCase):
    """Protect a finite reviewed denominator against silent omissions/stale input."""

    @classmethod
    def setUpClass(cls):
        """Read immutable Git reference and current sources; no application runs."""
        cls.go = {name: subprocess.check_output(['git', 'show', collector.REFERENCE + ':' + name]) for name in collector.GO_FILES}
        cls.rust = {name: Path(name).read_bytes() for name in collector.RUST_FILES}

    def test_valid_witness_routes_38_named_operations_in_eight_families(self):
        """Require real routed non-help agent cases and exact declared row counts."""
        collector.checked_sources(self.go | self.rust, collector.REVIEWED_SHA256)
        rows = collector.rows_for(self.go, self.rust)
        self.assertEqual(len(rows), 38)
        self.assertEqual(len({row['id'] for row in rows}), 38)
        self.assertEqual(set(row['family'] for row in rows), set(collector.EXPECTED))
        self.assertTrue(all(not row['parity_accepted'] for row in rows))
        by_id = {row['id']: row for row in rows}
        self.assertEqual(by_id['cli:agent:attach']['state'], 'NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN')
        self.assertEqual(by_id['cli:budget:preview']['state'], 'NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN')
        self.assertEqual(by_id['cli:agent:start']['state'], 'RENAMED_PARTIAL_CONTRACT_PARITY_OPEN')
        self.assertTrue(all(row['go_consumer_line'] > 0 for row in rows))

    def test_source_drift_refuses_even_when_operation_tokens_unchanged(self):
        """A source comment change still invalidates review bound to exact bytes."""
        sources = self.go | self.rust
        sources[collector.RUST_MAIN] += b'\n// changed bytes\n'
        with self.assertRaisesRegex(ValueError, 'source needs semantic reexamination:'):
            collector.checked_sources(sources, collector.REVIEWED_SHA256)

    def test_missing_source_refuses_instead_of_counting_zero(self):
        """An absent dispatcher invalidates collection rather than giving zero gaps."""
        sources = self.go | self.rust
        del sources[collector.RUST_MAIN]
        with self.assertRaisesRegex(ValueError, 'inventory missing or inconsistent'):
            collector.checked_sources(sources, collector.REVIEWED_SHA256)

    def test_malformed_digest_refuses(self):
        """No malformed digest can silently weaken the exact-byte review gate."""
        expected = dict(collector.REVIEWED_SHA256)
        expected[collector.RUST_MAIN] = 'not-a-digest'
        with self.assertRaisesRegex(ValueError, 'malformed reviewed digest:'):
            collector.checked_sources(self.go | self.rust, expected)

    def test_duplicate_operation_refuses(self):
        """Duplicated named paths cannot inflate a finite operation denominator."""
        with self.assertRaisesRegex(ValueError, 'duplicate operation projection:'):
            collector.unique_names(['relay show', 'relay show'], 'providers')

    def test_empty_and_malformed_operation_projections_refuse(self):
        """Missing or noncanonical paths remain failures, never an empty PASS."""
        for value in [[], [''], ['unknown?'], ['Show'], ['relay  show']]:
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, 'empty or malformed operation projection:'):
                collector.unique_names(value, 'providers')

    def test_unreachable_helper_refuses_without_main_route(self):
        """A helper and help text cannot substitute for a real main dispatcher edge."""
        go = dict(self.go)
        go[collector.GO_MAIN] = go[collector.GO_MAIN].replace(b'pos[0] == "pricing"', b'pos[0] == "retired-pricing"')
        with self.assertRaisesRegex(ValueError, 'historical main route missing: pricing'):
            collector.historical_operations(go)

    def test_new_historical_case_refuses_instead_of_disappearing(self):
        """Unknown dispatch branches require semantic review rather than old counts."""
        go = dict(self.go)
        name = 'internal/engine/console_cli.go'
        go[name] = go[name].replace(b'case "prepared":', b'case "new-action":\n\t\t\treturn nil\n\t\tcase "prepared":')
        with self.assertRaisesRegex(ValueError, 'historical operations require semantic reexamination'):
            collector.historical_operations(go)

    def test_stale_baseline_does_not_publish_operation_report(self):
        """Preserve partial evidence and refuse stale source-to-matrix continuity."""
        with tempfile.TemporaryDirectory(prefix='cli-operation-inventory-') as temporary:
            root = Path(temporary)
            baseline = root / 'baseline.json'
            baseline.write_text(json.dumps({'reference_commit': collector.REFERENCE, 'source_sha256': {collector.RUST_MAIN: '0' * 64}}))
            with self.assertRaisesRegex(ValueError, 'baseline reference or CLI source stale'):
                collector.collect(root / 'out', baseline)
            self.assertTrue((root / 'out' / 'collector.py').is_file())
            self.assertFalse((root / 'out' / 'operations.json').exists())


if __name__ == '__main__':
    unittest.main()
