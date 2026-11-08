"""fixture_area resolves the Admin setting `runtime_temporary_directory`.

Fake repositories live inside the configured fixture area itself (never /tmp).
Witness first (saved custom value honored), then installation default when
the Root has not adopted v51, then refusals of /tmp and traversal values.
"""
import json
import shutil
import unittest
from pathlib import Path

import fixture_area


class ConfiguredArea(unittest.TestCase):
    def setUp(self):
        self.repo = fixture_area.fixture_root('fixture-area-test-')
        (self.repo / '.swarm').mkdir()
        defaults = self.repo / 'rust/crates/swarm-engine/config'
        defaults.mkdir(parents=True)
        (defaults / 'engine-settings.json').write_text(json.dumps({'runtime_temporary_directory': '.swarm-fixtures'}))

    def tearDown(self):
        shutil.rmtree(self.repo)

    def save(self, values):
        (self.repo / '.swarm/engine-settings.json').write_text(json.dumps({'values': values}))

    def test_saved_value_then_default_then_refusals(self):
        self.save({'runtime_temporary_directory': 'work/scratch'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo / 'work/scratch')
        self.save({'runtime_temporary_directory': '/var/tmp/swarm-x'})
        self.assertEqual(fixture_area.configured_area(self.repo), Path('/var/tmp/swarm-x'))
        self.save({})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo / '.swarm-fixtures')
        for bad in ['/tmp', '/tmp/', '/tmp/x', '', '/', 'a/../b', './x', 'a//b']:
            self.save({'runtime_temporary_directory': bad})
            with self.assertRaises(ValueError, msg=bad):
                fixture_area.configured_area(self.repo)


if __name__ == '__main__':
    unittest.main()
