"""Five prerequisite-reader contracts; no namespace/privileged operation runs."""
import importlib.util
import json
import os
from pathlib import Path
from types import SimpleNamespace
import stat
import tempfile
import unittest
from unittest.mock import patch
SPEC = importlib.util.spec_from_file_location('paea_linux_prerequisites', Path(__file__).resolve().parents[1] / 'tools/paea/linux_prerequisites.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

class Contracts(unittest.TestCase):
    """Metadata parsing/publishing witnesses never imply verified launch authority."""
    def test_ranges_valid_before_malformed_overflow_overlap_and_foreign_rows(self):
        """Refuse bad own metadata; return all own ranges without selecting a UID."""
        valid = 'other:10000:100\nfixture:165536:65536\n'
        expected = [{'start': 165536, 'count': 65536}]
        for bad in ['fixture:0:1', 'fixture:165536:0', 'fixture:4294967294:2',
                    'fixture:1', 'fixture:١:2', 'fixture:100:10\nfixture:105:10']:
            self.assertEqual(MODULE.parse_ranges(valid, 'fixture', 1001), expected)
            with self.assertRaises(ValueError):
                MODULE.parse_ranges(bad, 'fixture', 1001)
        self.assertEqual(MODULE.parse_ranges('other:1:2', 'fixture', 1001), [])

    def test_actual_map_representation_valid_before_empty_invalid_and_overflow(self):
        """Initial/full and self maps are observed, never a host separation proof."""
        valid = '0 0 4294967295\n'
        for bad in ['', '0 0 0', '0 0 -1', '0 0 4294967296', '0 0 1 2', '١ 0 1']:
            self.assertEqual(MODULE.parse_map(valid)[0]['count'], 4294967295)
            with self.assertRaises(ValueError):
                MODULE.parse_map(bad)
        self.assertEqual(MODULE.parse_map('0 1001 1')[0]['outside'], 1001)

    def test_helper_metadata_only_root_suid_and_safe_ancestors(self):
        """Pure metadata doubles, not installed/executed privileged helpers."""
        safe = SimpleNamespace(st_mode=stat.S_IFREG | 0o4755, st_uid=0)
        parent = SimpleNamespace(stat=lambda: SimpleNamespace(st_mode=stat.S_IFDIR | 0o755, st_uid=0))
        path = SimpleNamespace(stat=lambda: safe, parents=[parent])
        fake = SimpleNamespace(resolve=lambda strict: path)
        with patch.object(MODULE, 'Path', return_value=fake):
            self.assertEqual(MODULE.helper('/fixture/helper')['status'], 'metadata_safe')
            safe.st_uid = 1001
            self.assertEqual(MODULE.helper('/fixture/helper')['status'], 'unsafe')
            safe.st_uid = 0
            safe.st_mode |= 0o002
            self.assertEqual(MODULE.helper('/fixture/helper')['status'], 'unsafe')
        self.assertEqual(MODULE.helper(None)['status'], 'unavailable')

    def test_live_observation_never_claims_launch_confinement_or_human(self):
        """Read actual euid/maps; no present/missing metadata turns into a grant."""
        report = MODULE.observe()
        self.assertEqual(report['current_namespace']['euid'], os.geteuid())
        self.assertEqual(report['current_namespace']['maps']['uid_map']['status'], 'observed')
        self.assertTrue(report['read_only'])
        for key in ['launch_verified', 'separate_host_uid_verified', 'confinement_verified', 'human_verified']:
            self.assertIs(report[key], False)
        self.assertIsInstance(report['missing_prerequisites'], list)

    def test_publish_exact_json_then_existing_directory_refuses(self):
        """A successful observation file roundtrips and is never overwritten."""
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'new'
            expected = {'schema_version': 1, 'launch_verified': False}
            MODULE.publish(target, expected)
            self.assertEqual(json.loads((target / 'report.json').read_text()), expected)
            before = (target / 'report.json').read_bytes()
            with self.assertRaises(FileExistsError):
                MODULE.publish(target, {'changed': True})
            self.assertEqual((target / 'report.json').read_bytes(), before)

if __name__ == '__main__':
    unittest.main()
