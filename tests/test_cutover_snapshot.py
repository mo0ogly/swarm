"""Offline fixture snapshot refusal/recovery; synthetic bytes are not a real DB."""
from pathlib import Path
import importlib.util,tempfile,unittest,os,json,errno
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('cutover',ROOT/'tools/rust-migration/cutover_snapshot.py')
snapshot=importlib.util.module_from_spec(spec);spec.loader.exec_module(snapshot)

class FixtureSnapshot(unittest.TestCase):
    """Valid capture/restore precedes each corruption or publication fault."""
    def setUp(self):
        """Use one owned disposable tree; no Root SQL/process/provider exists."""
        t=tempfile.TemporaryDirectory(prefix='cutover-synthetic-');self.addCleanup(t.cleanup)
        self.base=Path(t.name);self.root=self.base/'root';self.root.mkdir(mode=0o700)
        (self.root/'.swarm').mkdir(mode=0o700);(self.root/'.swarm/state.db').write_bytes(b'opaque fixture DB stand-in')
        (self.root/'empty').mkdir(mode=0o700);(self.root/'config').write_bytes(b'PRIVATE inert fixture config')
        (self.root/'config').chmod(0o600);self.before=snapshot.inventory(self.root);self.archive=self.base/'snapshot';snapshot.capture(self.root,self.archive)
        self.assertEqual(snapshot.verify(self.archive)['entries'],self.before)
    def test_exact_tree_modes_and_original_retained(self):
        """Public helper restores data/modes only and retains original DB inode."""
        inode=(self.root/'.swarm/state.db').stat().st_ino
        result=snapshot.restore(self.archive,self.base/'restored')
        self.assertFalse(result['execution_authority_transferred']);self.assertEqual(snapshot.inventory(self.base/'restored'),self.before)
        self.assertEqual((self.root/'.swarm/state.db').stat().st_ino,inode)
    def test_existing_destination_and_recovery(self):
        """Refuse an existing tree without mutation; a fresh destination succeeds."""
        with self.assertRaisesRegex(ValueError,'snapshot-destination-exists'):snapshot.restore(self.archive,self.root)
        self.assertEqual(snapshot.inventory(self.root),self.before);snapshot.restore(self.archive,self.base/'fresh')
    def test_corrupted_snapshot_refusal_before_destination_then_recovery(self):
        """A valid archive precedes one corrupt byte; original remains intact."""
        p=self.archive/'data/config';old=p.read_bytes();p.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'snapshot-retained-tree-mismatch'):snapshot.restore(self.archive,self.base/'bad')
        self.assertFalse((self.base/'bad').exists());self.assertEqual(snapshot.inventory(self.root),self.before)
        p.write_bytes(old);snapshot.restore(self.archive,self.base/'recovered')
    def test_manifest_symlink_refuses_before_reading(self):
        """A linked manifest is a special input, even if target bytes are valid."""
        p=self.archive/'manifest.json';saved=self.base/'original-manifest.json';p.rename(saved);p.symlink_to(saved)
        with self.assertRaisesRegex(ValueError,'snapshot-manifest-not-owned-regular'):snapshot.verify(self.archive)
        p.unlink();saved.rename(p);self.assertEqual(snapshot.verify(self.archive)['entries'],self.before)
    def test_wal_symlink_and_overlap_refuse_without_source_changes(self):
        """Valid source precedes one WAL, one link and overlapping output."""
        wal=self.root/'.swarm/state.db-wal';wal.write_bytes(b'')
        with self.assertRaisesRegex(ValueError,'snapshot-wal-or-shm-present'):snapshot.capture(self.root,self.base/'wal')
        wal.unlink();link=self.root/'linked';link.symlink_to(self.root/'config')
        with self.assertRaisesRegex(ValueError,'snapshot-link-or-special-entry'):snapshot.capture(self.root,self.base/'link')
        link.unlink()
        with self.assertRaisesRegex(ValueError,'snapshot-overlapping-trees'):snapshot.capture(self.root,self.root/'inside')
        self.assertEqual(snapshot.inventory(self.root),self.before)
    def test_enospc_retains_original_and_new_destination_absent(self):
        """Inject storage failure only after valid witness; preserve typed errno."""
        with patch.object(snapshot.io,'publish',side_effect=OSError(errno.ENOSPC,'fixture full')):
            with self.assertRaises(OSError) as error:snapshot.restore(self.archive,self.base/'full')
        self.assertEqual(error.exception.errno,errno.ENOSPC);self.assertFalse((self.base/'full').exists());self.assertEqual(snapshot.inventory(self.root),self.before)
        self.assertTrue(list(self.base.glob('.full-restore-*')));snapshot.restore(self.archive,self.base/'retry-new')

if __name__=='__main__':unittest.main(verbosity=2)
