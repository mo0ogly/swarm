"""fixture_area resolves the Admin setting `runtime_temporary_directory`.

Fake repositories live inside the configured fixture area itself (never /tmp).
Witness first (saved custom value honored), then installation default when
the Root has not adopted v51, then refusals of /tmp and traversal values.
"""
import json
import os
import shutil
import unittest
from pathlib import Path
from unittest.mock import patch

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

    def test_observation_uses_consumed_read_and_keeps_cache(self):
        """Observe the consumed buffer despite a subsequent file replacement."""
        self.save({'runtime_temporary_directory': 'first'})
        saved = self.repo / '.swarm/engine-settings.json'
        original_read = Path.read_text
        events, reads = [], []
        cached = fixture_area._AREA
        def read(path, *args, **kwargs):
            """Replace only owned saved bytes after returning the original buffer."""
            value = original_read(path, *args, **kwargs)
            if path == saved:
                reads.append(path)
                self.save({'runtime_temporary_directory': 'second'})
            return value
        with patch.object(Path, 'read_text', read):
            result = fixture_area.configured_area(self.repo, observer=events.append)
        self.assertEqual(reads, [saved])
        self.assertEqual(result, self.repo / 'first')
        self.assertEqual(events[-1]['kind'], 'resolved')
        self.assertEqual(events[-1]['path'], str(result))
        self.assertEqual(events[-1]['source'], 'saved')
        self.assertEqual(fixture_area._AREA, cached)
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo / 'second')

    def test_observation_preserves_current_default_and_refusals(self):
        """A valid observed control precedes unchanged missing/type/JSON behavior."""
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo, observer=lambda e: None), self.repo/'control')
        (self.repo/'.swarm/engine-settings.json').unlink()
        events = []
        self.assertEqual(fixture_area.configured_area(self.repo, observer=events.append), self.repo/'.swarm-fixtures')
        self.assertFalse(events[0]['is_file'])
        self.assertFalse(events[0]['absence_established'])
        self.assertEqual(events[-1]['source'], 'default')
        for values in ({}, {'runtime_temporary_directory': None}):
            self.save(values)
            events = []
            self.assertEqual(fixture_area.configured_area(self.repo, observer=events.append), self.repo/'.swarm-fixtures')
            self.assertEqual(events[-1]['source'], 'default')
            saved_field = next(e for e in events if e['kind']=='field' and e['source']=='saved')
            self.assertEqual(saved_field['present'], 'runtime_temporary_directory' in values)
            self.assertFalse(events[0].get('absence_established', True))
        for value in (False, 7, [], {}, '/tmp', 'a/../b'):
            self.save({'runtime_temporary_directory': value})
            events = []
            with self.assertRaises(ValueError):
                fixture_area.configured_area(self.repo, observer=events.append)
            self.assertEqual(events[-1]['kind'], 'resolution_error')
        (self.repo/'.swarm/engine-settings.json').write_text('{private-invalid')
        events = []
        with self.assertRaises(json.JSONDecodeError):
            fixture_area.configured_area(self.repo, observer=events.append)
        self.assertNotIn('private-invalid', json.dumps(events))
        (self.repo/'.swarm/engine-settings.json').write_text(json.dumps({'values': []}))
        with self.assertRaises(AttributeError):
            fixture_area.configured_area(self.repo, observer=events.append)
        self.assertEqual(events[-1]['error_type'], 'AttributeError')
        self.save({'runtime_temporary_directory': 'recovered'})
        self.assertEqual(fixture_area.configured_area(self.repo, observer=events.append), self.repo/'recovered')

    def test_read_and_observer_errors_preserve_primary(self):
        """Injected read refusal preserves its cause even when the observer fails."""
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')

        def observer(event):
            """Fail observation without replacing the independent resolution error."""
            raise RuntimeError('observer unavailable')
        with patch.object(Path, 'read_text', side_effect=PermissionError(13, 'fixture read refused')):
            with self.assertRaises(PermissionError) as refused:
                fixture_area.configured_area(self.repo, observer=observer)
        self.assertTrue(refused.exception.fixture_observation_errors)
        self.assertEqual(refused.exception.errno, 13)
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')

    @unittest.skipIf(os.geteuid() == 0, 'root cannot witness an actual mode-based read denial')
    def test_actual_read_denial_then_recovery(self):
        """Same identity reads the valid control, then really receives EACCES."""
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
        saved = self.repo/'.swarm/engine-settings.json'
        original_mode = saved.stat().st_mode & 0o777
        saved.chmod(0)
        events = []
        try:
            with self.assertRaises(PermissionError) as refused:
                fixture_area.configured_area(self.repo, observer=events.append)
            self.assertEqual(refused.exception.errno, 13)
            self.assertEqual(events[-1]['error_type'], 'PermissionError')
        finally:
            saved.chmod(original_mode)
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')



if __name__ == '__main__':
    unittest.main()
