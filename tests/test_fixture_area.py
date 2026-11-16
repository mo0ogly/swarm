"""fixture_area resolves the Admin setting `runtime_temporary_directory`.

Fake repositories live inside the configured fixture area itself (never /tmp).
Witness first (saved custom value honored), then the shipped default only when
an actual access proves the saved file absent (C-D01). A present document with
a missing or null field is not-configured (C-D03); unreadable, non-regular,
unparseable or wrongly typed saved data fails with its own cause (C-D02);
refusals mirror the Rust grammar, including 'x/' and '//x' (C-D04).
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
        """Check saved-path controls, missing/null refusal and invalid paths.

        Only the owned settings fixture is changed; unexpected results fail
        assertions rather than qualifying resolution or refusal.
        """
        self.save({'runtime_temporary_directory': 'work/scratch'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo / 'work/scratch')
        self.save({'runtime_temporary_directory': '/var/tmp/swarm-x'})
        self.assertEqual(fixture_area.configured_area(self.repo), Path('/var/tmp/swarm-x'))
        self.save({})
        with self.assertRaises(ValueError):
            fixture_area.configured_area(self.repo)
        self.save({'runtime_temporary_directory': None})
        with self.assertRaises(ValueError):
            fixture_area.configured_area(self.repo)
        for bad in ['/tmp', '/tmp/', '/tmp/x', '', '/', 'a/../b', './x', 'a//b', 'x/', '//x']:
            self.save({'runtime_temporary_directory': bad})
            with self.assertRaises(ValueError, msg=bad):
                fixture_area.configured_area(self.repo)

    def test_observation_uses_consumed_read_and_keeps_cache(self):
        """Observe the consumed buffer despite a subsequent file replacement."""
        self.save({'runtime_temporary_directory': 'first'})
        saved = self.repo / '.swarm/engine-settings.json'
        original_read = fixture_area._read_saved
        events = []
        reads = []
        cached = fixture_area._AREA
        def read(repo):
            """Replace owned saved bytes after the actual descriptor read."""
            value = original_read(repo)
            reads.append(saved)
            self.save({'runtime_temporary_directory': 'second'})
            return value
        with patch.object(fixture_area, '_read_saved', read):
            result = fixture_area.configured_area(self.repo, observer=events.append)
        self.assertEqual(reads, [saved])
        self.assertEqual(result, self.repo / 'first')
        self.assertEqual(events[-1]['kind'], 'resolved')
        self.assertEqual(events[-1]['path'], str(result))
        self.assertEqual(events[-1]['source'], 'saved')
        self.assertEqual(fixture_area._AREA, cached)
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo / 'second')

    def test_observation_preserves_current_default_and_refusals(self):
        """Valid observed control, then proven-absence default, then
        not-configured refusal and unchanged invalid-type/JSON/shape
        failures, all from the same access buffers (D1–D4)."""
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo, observer=lambda e: None), self.repo/'control')
        (self.repo/'.swarm/engine-settings.json').unlink()
        events = []
        self.assertEqual(fixture_area.configured_area(self.repo, observer=events.append), self.repo/'.swarm-fixtures')
        self.assertFalse(events[0]['is_file'])
        self.assertTrue(events[0]['absence_established'])
        self.assertEqual(events[0]['entry_kind'], 'absent')
        self.assertEqual(events[-1]['source'], 'default')
        for values in ({}, {'runtime_temporary_directory': None}):
            self.save(values)
            events = []
            with self.assertRaises(ValueError):
                fixture_area.configured_area(self.repo, observer=events.append)
            self.assertEqual(events[-1]['kind'], 'resolution_error')
            self.assertEqual(events[-1]['source'], 'saved')
            saved_field = next(e for e in events if e['kind']=='field' and e['source']=='saved')
            self.assertEqual(saved_field['present'], 'runtime_temporary_directory' in values)
            self.assertFalse(events[0]['absence_established'])
            self.assertEqual(events[0]['entry_kind'], 'file')
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
        with patch.object(fixture_area, '_read_saved', side_effect=PermissionError(13, 'fixture read refused')):
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



    def test_default_faults_fail_closed_after_proven_absence(self):
        """A valid default control precedes each missing/invalid default fault.

        Only owned fixture bytes change. Restore them after each refusal and
        resolve afresh; the original failure is never retried unchanged.
        """
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
        (self.repo/'.swarm/engine-settings.json').unlink()
        defaults = self.repo/'rust/crates/swarm-engine/config/engine-settings.json'
        original = defaults.read_bytes()
        cases = [(None, FileNotFoundError), (b'{invalid', json.JSONDecodeError),
                 (b'{}', KeyError),
                 (json.dumps({'runtime_temporary_directory': '/tmp/x'}).encode(), ValueError)]
        for raw, expected in cases:
            with self.subTest(expected=expected.__name__):
                self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'.swarm-fixtures')
                try:
                    if raw is None:
                        defaults.unlink()
                    else:
                        defaults.write_bytes(raw)
                    events = []
                    with self.assertRaises(expected):
                        fixture_area.configured_area(self.repo, observer=events.append)
                    self.assertTrue(events[0]['absence_established'])
                    self.assertEqual(events[-1]['source'], 'default')
                    self.assertFalse(any(e['kind']=='resolved' for e in events))
                finally:
                    defaults.write_bytes(original)
                self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'.swarm-fixtures')

    def test_present_directory_refused_with_saved_origin(self):
        """A settings directory is a present fault (C-D02), never absence."""
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
        saved = self.repo/'.swarm/engine-settings.json'
        saved.unlink()
        saved.mkdir()
        events = []
        try:
            with self.assertRaises(IsADirectoryError) as raised:
                fixture_area.configured_area(self.repo, observer=events.append)
            self.assertEqual(raised.exception.errno, 21)
            self.assertEqual(events[0]['entry_kind'], 'directory')
            self.assertFalse(events[0]['absence_established'])
            self.assertEqual(events[-1]['source'], 'saved')
            self.assertNotIn('default', [event['source'] for event in events])
        finally:
            saved.rmdir()
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')

    def test_symlink_and_missing_parent_classification(self):
        """Resolved link reads through; a dangling leaf or dangling parent
        symlink fails with its original cause; a missing entry under a held
        directory permits the default. Open file content can still change
        during a read; no atomic snapshot is claimed."""
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
        saved = self.repo/'.swarm/engine-settings.json'
        real = self.repo/'.swarm/real-settings.json'
        real.write_text(json.dumps({'values': {'runtime_temporary_directory': 'linked'}}))
        saved.unlink()
        saved.symlink_to(real)
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'linked')
        saved.unlink()
        real.unlink()
        saved.symlink_to(self.repo / 'no-such-target')
        events = []
        with self.assertRaises(FileNotFoundError) as dangling:
            fixture_area.configured_area(self.repo, observer=events.append)
        self.assertEqual(dangling.exception.errno, 2)
        self.assertEqual(events[0]['entry_kind'], 'uncertain')
        self.assertFalse(events[0]['absence_established'])
        self.assertEqual(events[-1]['source'], 'saved')
        self.assertNotIn('default', [event['source'] for event in events])
        saved.unlink()
        (self.repo/'.swarm').rmdir()
        events = []
        self.assertEqual(fixture_area.configured_area(self.repo, observer=events.append), self.repo/'.swarm-fixtures')
        self.assertTrue(events[0]['absence_established'])
        self.assertEqual(events[0]['entry_kind'], 'absent')
        self.assertEqual(events[-1]['source'], 'default')
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'.swarm-fixtures')
        (self.repo/'.swarm').symlink_to(self.repo / 'no-such-dir')
        events = []
        with self.assertRaises(FileNotFoundError):
            fixture_area.configured_area(self.repo, observer=events.append)
        self.assertEqual(events[-1]['source'], 'saved')
        self.assertNotIn('default', [event['source'] for event in events])
        (self.repo/'.swarm').unlink()
        (self.repo/'.swarm').mkdir()
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')


    def test_cd01_saved_shapes_types_paths_and_recovery(self):
        """Each invalid saved document follows a valid control and fresh recovery.

        No default read is allowed for a present saved entry. Resolution itself
        creates no directory; this is not a claim about later runner effects.
        """
        saved = self.repo/'.swarm/engine-settings.json'
        defaults = self.repo/'rust/crates/swarm-engine/config/engine-settings.json'
        cases = [(b'{invalid', json.JSONDecodeError), (b'[]', AttributeError),
                 (b'null', AttributeError), (b'{"values":[]}', AttributeError),
                 (b'{"values":null}', AttributeError), (b'{}', ValueError),
                 (b'{"values":{}}', ValueError),
                 (b'{"values":{"runtime_temporary_directory":null}}', ValueError)]
        bad_values = [False, 7, [], {}, '', '/', '/tmp', '/tmp/x',
                      'a//b', './x', 'a/../b', 'x/', '//x', 'a\0b']
        cases += [(json.dumps({'values':{'runtime_temporary_directory':v}}).encode(), ValueError)
                  for v in bad_values]
        original_read = Path.read_text
        def read_without_default(path, *args, **kwargs):
            """Refuse any default access to distinguish saved-side failure."""
            if path == defaults:
                self.fail('present saved input must not consult default')
            return original_read(path, *args, **kwargs)
        for index, (raw, expected) in enumerate(cases):
            with self.subTest(case=index):
                self.save({'runtime_temporary_directory': 'control'})
                self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
                saved.write_bytes(raw)
                events = []
                with patch.object(Path, 'read_text', read_without_default):
                    with self.assertRaises(expected):
                        fixture_area.configured_area(self.repo, observer=events.append)
                self.assertEqual(saved.read_bytes(), raw)
                self.assertFalse((self.repo/'control').exists())
                self.assertEqual(events[-1]['source'], 'saved')
                self.assertFalse(any(e['kind']=='resolved' for e in events))
                self.save({'runtime_temporary_directory': 'recovered'})
                self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'recovered')

    def test_cd01_non_directory_parent_refuses_then_recovers(self):
        """A non-directory parent remains a saved-side IO refusal, not absence."""
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
        parent = self.repo/'.swarm'
        (parent/'engine-settings.json').unlink()
        parent.rmdir()
        parent.write_text('owned non-directory')
        events = []
        try:
            with self.assertRaises(NotADirectoryError):
                fixture_area.configured_area(self.repo, observer=events.append)
            self.assertFalse(events[0]['absence_established'])
            self.assertEqual(events[-1]['source'], 'saved')
            self.assertNotIn('default', [e['source'] for e in events])
        finally:
            parent.unlink()
            parent.mkdir()
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')

    def test_cd01_held_parent_survives_ancestor_replacement(self):
        """Replacing the pathname parent cannot redirect an already held parent.

        The replacement is injected immediately before the real leaf open;
        no background actor or shared path is used. Restore before recovery.
        """
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
        parent = self.repo/'.swarm'
        retained = self.repo/'retained-parent'
        original_open = os.open
        changed = []
        def open_leaf(path, flags, *args, **kwargs):
            """Change the owned parent path before opening through its held FD."""
            if path == 'engine-settings.json' and not changed:
                parent.rename(retained)
                parent.symlink_to(self.repo/'missing-parent-target')
                changed.append(True)
            return original_open(path, flags, *args, **kwargs)
        try:
            events = []
            with patch.object(os, 'open', open_leaf):
                self.assertEqual(fixture_area.configured_area(self.repo, observer=events.append),
                                 self.repo/'control')
            self.assertEqual(changed, [True])
            self.assertEqual(events[-1]['source'], 'saved')
            self.assertFalse(events[0]['absence_established'])
        finally:
            if changed:
                parent.unlink()
                retained.rename(parent)
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')

    def test_cd01_fifo_refused_without_blocking_open(self):
        """An owned FIFO is opened only with NONBLOCK, then refused without read.

        Assert the IO flag before the real FIFO open; there is no global
        watchdog substituting for the mechanism. No writer process is needed.
        """
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
        saved = self.repo/'.swarm/engine-settings.json'
        saved.unlink()
        os.mkfifo(saved, mode=0o600)
        original_open = os.open
        def open_checked(path, flags, *args, **kwargs):
            """Check the nonblocking mechanism before the actual FIFO open."""
            if path == 'engine-settings.json':
                self.assertTrue(flags & os.O_NONBLOCK)
            return original_open(path, flags, *args, **kwargs)
        try:
            events = []
            with patch.object(os, 'open', open_checked):
                with self.assertRaises(OSError):
                    fixture_area.configured_area(self.repo, observer=events.append)
            self.assertEqual(events[0]['entry_kind'], 'special')
            self.assertEqual(events[-1]['source'], 'saved')
            self.assertNotIn('default', [e['source'] for e in events])
        finally:
            saved.unlink()
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')

    def test_cd01_primary_read_and_secondary_close_causes(self):
        """Injected read refusal survives secondary close diagnostics and recovery.

        The close double really releases each FD then raises a synthetic
        diagnostic; this does not reproduce a kernel close refusal.
        """
        self.save({'runtime_temporary_directory': 'control'})
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')
        original_close = os.close
        def close_then_diagnose(fd):
            """Release an owned descriptor before injecting a secondary error."""
            original_close(fd)
            raise OSError(5, 'synthetic close diagnostic after release')
        with patch.object(os, 'read', side_effect=PermissionError(13, 'synthetic read refusal')):
            with patch.object(os, 'close', close_then_diagnose):
                with self.assertRaises(PermissionError) as primary:
                    fixture_area.configured_area(self.repo)
        self.assertEqual(primary.exception.errno, 13)
        self.assertEqual(len(primary.exception.fixture_close_errors), 3)
        self.assertTrue(all(e.errno == 5 for e in primary.exception.fixture_close_errors))
        self.assertEqual(fixture_area.configured_area(self.repo), self.repo/'control')

if __name__ == '__main__':
    unittest.main()
