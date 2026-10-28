"""Owned fixture restoration must survive both observation and retention failure."""
from pathlib import Path
import os,tempfile,unittest
from unittest.mock import patch
import rust_native_http_mission as mission


class OwnedDatabaseRestore(unittest.TestCase):
    """Valid original/copy witness precedes the shared restoration fault."""
    def test_identical_copy_has_distinct_inode_then_original_is_restored(self):
        """Exercise actual owned file substitution and exact physical recovery."""
        with tempfile.TemporaryDirectory() as directory:
            database=Path(directory)/'state.db';database.write_bytes(b'owned fixture')
            identity=database.stat().st_ino
            with mission.foreign_inode(database):
                self.assertEqual(database.read_bytes(),b'owned fixture')
                self.assertNotEqual(database.stat().st_ino,identity)
            self.assertEqual(database.stat().st_ino,identity)
            self.assertEqual(database.read_bytes(),b'owned fixture')

    def test_observation_and_copy_retention_failures_still_restore_original(self):
        """Retaining a refused copy must not prevent the original restoration."""
        # Same complete successful witness before this fault injection.
        self.test_identical_copy_has_distinct_inode_then_original_is_restored()
        with tempfile.TemporaryDirectory() as directory:
            database=Path(directory)/'state.db';database.write_bytes(b'owned fixture')
            identity=database.stat().st_ino;replace=os.replace
            def failing(source,target):
                if Path(source)==database and Path(target).name=='state.replacement-owned.db':raise OSError('fixture-copy-retention')
                return replace(source,target)
            with patch.object(mission.os,'replace',side_effect=failing):
                with self.assertRaises(BaseExceptionGroup) as caught:
                    with mission.foreign_inode(database):raise RuntimeError('fixture-observation')
            self.assertEqual([str(e) for e in caught.exception.exceptions],['fixture-observation','fixture-copy-retention'])
            self.assertEqual(database.stat().st_ino,identity)
            self.assertEqual(database.read_bytes(),b'owned fixture')

    def test_failed_original_restoration_preserves_source_and_explicit_recovery(self):
        """A failed restore remains failed, with original retained for owned recovery."""
        self.test_identical_copy_has_distinct_inode_then_original_is_restored()
        with tempfile.TemporaryDirectory() as directory:
            database=Path(directory)/'state.db';database.write_bytes(b'owned fixture')
            identity=database.stat().st_ino;replace=os.replace
            def failing(source,target):
                if Path(source).name=='state.original-owned.db' and Path(target)==database:raise PermissionError('fixture-restore')
                return replace(source,target)
            with patch.object(mission.os,'replace',side_effect=failing):
                with self.assertRaises(BaseExceptionGroup) as caught:
                    with mission.foreign_inode(database):raise RuntimeError('fixture-observation')
            self.assertEqual([str(e) for e in caught.exception.exceptions],['fixture-observation','fixture-restore'])
            original=database.with_name('state.original-owned.db')
            self.assertFalse(database.exists());self.assertEqual(original.stat().st_ino,identity)
            self.assertEqual(original.read_bytes(),b'owned fixture')
            # The injected refusal is now removed: this is a changed precondition.
            os.replace(original,database)
            self.assertEqual(database.stat().st_ino,identity)


class StatusComparison(unittest.TestCase):
    """Independent host samples never justify discarding state or permission."""
    def test_status_comparison_keeps_every_nonmeasurement_field(self):
        import copy
        value={'spending':{'observed_at':'2026-10-24T00:00:00Z','calls':0},'enabled':False,'runtime':{'observed_at':'2026-10-24T00:00:00Z','launch_allowed':True,'volumes':[{'kind':'store','state':'ready','available_bytes':1,'available_inodes':2}]}}
        other=copy.deepcopy(value);other['runtime']['observed_at']='2026-10-24T00:00:01Z';other['runtime']['volumes'][0]['available_bytes']=3
        self.assertEqual(mission.comparable_status(value)[0],mission.comparable_status(other)[0])
        self.assertEqual(value['runtime']['volumes'][0]['available_bytes'],1)
        for field in ['enabled','runtime','spending']:
            changed=copy.deepcopy(other)
            if field=='enabled': changed['enabled']=True
            elif field=='runtime': changed['runtime']['launch_allowed']=False
            else: changed['spending']['calls']=1
            self.assertNotEqual(mission.comparable_status(value)[0],mission.comparable_status(changed)[0])


if __name__=='__main__':unittest.main()
