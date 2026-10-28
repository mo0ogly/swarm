#!/usr/bin/env python3
"""Reproduce shutil copy fallback and verify rename-only owned cache transfer."""
from pathlib import Path
import errno,importlib.util,shutil,tempfile,unittest
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('move_target',Path(__file__).resolve().parents[1]/'tools/rust-migration/move_build_target.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


class MoveBuildTarget(unittest.TestCase):
    """Temporary owned directories only; no Cargo writer or repository data."""
    def test_missing_parent_copy_reproducer_and_rename_fix(self):
        """Missing parents trigger shutil copy; explicit parents preserve inode."""
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);old=root/'old';old.mkdir();(old/'proof').write_bytes(b'cache')
            before=old.stat().st_ino;dest=root/'missing'/'parent'/'target'
            shutil.move(old,dest)
            self.assertNotEqual(before,dest.stat().st_ino)
            newer=root/'next'/'target';before=dest.stat().st_ino
            observation=module.move_target(dest,newer)
            self.assertEqual(before,newer.stat().st_ino);self.assertFalse(observation['copied']);self.assertEqual((newer/'proof').read_bytes(),b'cache')

    def test_existing_destination_preserves_both(self):
        """A prior output is not replaced and original cache remains intact."""
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);source=root/'source';source.mkdir();dest=root/'dest';dest.mkdir()
            with self.assertRaises(FileExistsError):module.move_target(source,dest)
            self.assertTrue(source.is_dir());self.assertTrue(dest.is_dir())

    def test_cross_device_has_no_copy_fallback(self):
        """Injected EXDEV refuses without deleting or copying the original."""
        with tempfile.TemporaryDirectory() as temporary:
            source=Path(temporary)/'source';source.mkdir();(source/'proof').write_bytes(b'original');dest=Path(temporary)/'new'/'target'
            with patch.object(module.os,'rename',side_effect=OSError(errno.EXDEV,'fixture cross-device')):
                with self.assertRaises(OSError):module.move_target(source,dest)
            self.assertEqual((source/'proof').read_bytes(),b'original');self.assertFalse(dest.exists())

if __name__=='__main__':unittest.main(verbosity=2)
