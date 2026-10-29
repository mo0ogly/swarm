"""Guard the differential harness against false identity/type/key comparisons.

Pure comparator fixtures only: these do not exercise or accept the Rust product.
The separate public recipe obtains actual documents from both frozen binaries.
"""
import copy
import hashlib
import os
from pathlib import Path
import tempfile
import unittest
from datetime import datetime, timezone
import rust_cockpit_agent_list as bench


class Comparator(unittest.TestCase):
    def document(self, root):
        """Make an explicit comparison fixture; no production state is inferred."""
        value = {key: None for key in ['agents', 'autonomy', 'autonomy_label', 'cost',
            'cost_text', 'independent_reviews', 'paused', 'pilotage', 'post_delivery',
            'priority', 'slots', 'task_actions', 'validation', 'work']}
        value['pilotage'] = dict(snapshot_as_of='2026-01-01T00:00:00Z',
            project_view_key=hashlib.sha256(os.fsencode(root)).hexdigest(),
            summary=dict(confirmed=0, loaded=0, complete=True))
        return value

    def compare(self, left, right, root):
        """Use a fixed clock fixture window, without a verification deadline."""
        moment = datetime(2026, 1, 1, tzinfo=timezone.utc).timestamp()
        return bench.compare_lists(left, right, 'left', 'right',
            (moment, moment), (moment, moment), 'comparator-only', root)

    def test_project_identity_is_canonical_filesystem_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'réel'
            root.mkdir()
            alias = Path(directory) / 'alias'
            alias.symlink_to(root, target_is_directory=True)
            self.assertEqual(bench.canonical_root_bytes(alias), os.fsencode(root))
            value = self.document(root)
            self.compare(value, copy.deepcopy(value), root)
            wrong = copy.deepcopy(value)
            wrong['pilotage']['project_view_key'] = hashlib.sha256(b'{"work":"not a Root pathname"}').hexdigest()
            with self.assertRaises(AssertionError):
                self.compare(wrong, wrong, root)

    def test_exact_keys_refuse_an_invented_section_with_the_same_count(self):
        value = self.document(Path('/owned-fixture'))
        bench.require_document_shape(value, 'valid-control')
        del value['autonomy']
        value['labels'] = None
        self.assertEqual(len(value), 14)
        with self.assertRaises(AssertionError):
            bench.require_document_shape(value, 'invented-section')

    def test_boolean_does_not_equal_a_zero_counter(self):
        root = Path('/owned-fixture')
        left = self.document(root)
        self.compare(left, copy.deepcopy(left), root)
        wrong = copy.deepcopy(left)
        wrong['pilotage']['summary']['confirmed'] = False
        with self.assertRaises(AssertionError):
            self.compare(left, wrong, root)

    def test_other_changes_and_invalid_timestamp_offsets_refuse(self):
        root = Path('/owned-fixture')
        left = self.document(root)
        self.compare(left, copy.deepcopy(left), root)
        wrong = copy.deepcopy(left)
        wrong['cost_text'] = 'a different fact'
        with self.assertRaises(AssertionError):
            self.compare(left, wrong, root)
        with self.assertRaises(AssertionError):
            bench.rfc3339_utc('2026-01-01T00:00:00+00:99', 'invalid-offset')

    def test_dependency_clocks_are_validated_without_hiding_graph_facts(self):
        """A real edge clock may vary inside its call; graph facts may not."""
        root = Path('/owned-fixture')
        left = self.document(root)
        left['pilotage']['edges'] = [dict(from_task_id='a', to_task_id='b',
            satisfied_now=False, evaluated_at='2026-01-01T00:00:00Z')]
        self.compare(left, copy.deepcopy(left), root)
        for key, value in [('evaluated_at', '2027-01-01T00:00:00Z'),
                           ('satisfied_now', True), ('from_task_id', 'invented')]:
            wrong = copy.deepcopy(left)
            wrong['pilotage']['edges'][0][key] = value
            with self.assertRaises(AssertionError):
                self.compare(left, wrong, root)
        missing = copy.deepcopy(left)
        del missing['pilotage']['edges'][0]['evaluated_at']
        with self.assertRaises(AssertionError):
            self.compare(missing, copy.deepcopy(missing), root)


if __name__ == '__main__':
    unittest.main()
