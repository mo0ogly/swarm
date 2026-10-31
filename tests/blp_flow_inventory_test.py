#!/usr/bin/env python3
"""Exercise inventory drift/refusal/recovery on project-owned source fixtures.

No provider, live datastore or runtime policy. A current witness precedes each
mutation; malformed JSON and escaping paths cannot turn into a fresh inventory.
"""
import copy
import hashlib
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tools.paea.check_blp_inventory import inspect, _object
from fixture_area import fixture_root


class InventoryChecks(unittest.TestCase):
    """Each fixture is retained for inspection; it contains synthetic source only."""

    def setUp(self):
        """Create one valid source/flow witness under the configured fixture area."""
        self.root = fixture_root('blp-inventory-')
        self.source = self.root/'source.rs'
        self.source.write_text('fn boundary() {}\n')
        self.document = {'schema_version': 1, 'scope': 'fixture only', 'runtime_enforced': False,
            'references': [{'id': 'source', 'path': 'source.rs',
                'sha256': hashlib.sha256(self.source.read_bytes()).hexdigest(),
                'line': 1, 'anchor': 'fn boundary()'}],
            'flows': [{'id': 'BF-01', 'name_fr': 'Fixture', 'name_en': 'Fixture',
                'producer': 'owned fixture', 'consumer': 'offline checker', 'references': ['source'],
                'material': 'synthetic text', 'current_controls': 'none claimed',
                'missing_blp': ['BLP-04'], 'owner': 'fixture owner',
                'required_observable': 'drift refused', 'status': 'OPEN'}],
            'exclusions': ['runtime'], 'remaining_evidence': ['all actual effects']}
        self.assertEqual(inspect(self.root, self.document), [])

    def test_current_is_readonly(self):
        """A valid inventory changes neither source bytes nor fixture entries."""
        before = self.source.read_bytes(), sorted(self.root.iterdir())
        self.assertEqual(inspect(self.root, self.document), [])
        self.assertEqual(before, (self.source.read_bytes(), sorted(self.root.iterdir())))

    def test_source_mutation_refuses_then_original_recovers(self):
        """An unreviewed byte change fails even when its anchor still exists."""
        original = self.source.read_bytes()
        self.source.write_bytes(original+b'// unreviewed destination\n')
        self.assertIn('source:source-changed', inspect(self.root, self.document))
        self.source.write_bytes(original)
        self.assertEqual(inspect(self.root, self.document), [])

    def test_wrong_anchor_cannot_be_hidden_by_current_hash(self):
        """A fresh digest alone never admits a missing reviewed boundary."""
        bad = copy.deepcopy(self.document); bad['references'][0]['anchor'] = 'absent_boundary'
        self.assertIn('source:anchor-changed', inspect(self.root, bad))
        self.assertEqual(inspect(self.root, self.document), [])

    def test_untrusted_graph_and_runtime_claim_refuse(self):
        """Missing contracts/refs, duplicate flow IDs and enforcement claims fail."""
        for mutate in [lambda d:d.update(runtime_enforced=True),
                       lambda d:d['flows'][0].update(references=['missing']),
                       lambda d:d['flows'][0].update(owner=''),
                       lambda d:d['flows'].append(d['flows'][0]),
                       lambda d:d.update(schema_version=True)]:
            bad=copy.deepcopy(self.document); mutate(bad)
            self.assertTrue(inspect(self.root,bad))
        with self.assertRaises(ValueError):
            json.loads('{"schema_version":1,"schema_version":2}',object_pairs_hook=_object)

    def test_escape_paths_refuse(self):
        """Relative traversal and an outside symlink never become reviewed IO."""
        bad=copy.deepcopy(self.document); bad['references'][0]['path']='../outside.rs'
        self.assertIn('source:path-invalid',inspect(self.root,bad))
        outside=fixture_root('blp-outside-')/'outside.rs';outside.write_text('fn boundary() {}\n')
        (self.root/'alias.rs').symlink_to(outside)
        bad['references'][0]['path']='alias.rs'
        self.assertIn('source:path-outside-root',inspect(self.root,bad))


if __name__ == '__main__':
    unittest.main(verbosity=2)
