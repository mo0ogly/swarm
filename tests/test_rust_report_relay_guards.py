"""Deterministic guards for the actual provider-entry/parent-record race."""
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch
import rust_mission_watch_worker as worker


class RunningBarrier(unittest.TestCase):
    """Keep starting distinct from running and preserve observation/close failures."""
    def test_starting_is_not_success_and_actual_running_is_required(self):
        """A valid running control precedes the delayed parent-record variant."""
        busy=(2,{'schema_version':1,'failure':{'code':'root-database-identity','causes':['root-database-identity','database-identity-operation:Prepare','sqlite-code:5']}})
        running=(0,{'agent':{'status':'running'}})
        for replies,expected_reads in [([running],0),([(0,{'agent':{'status':'starting'}}),running],1),([busy,running],1)]:
            owned = SimpleNamespace(root=Path('/owned'), call=Mock(side_effect=replies))
            with patch.object(worker, 'watch_directory', return_value=100) as watch, patch.object(worker.select, 'select', return_value=([100], [], [])) as wait, patch.object(worker.os, 'read') as read, patch.object(worker.os, 'close') as close:
                self.assertEqual(worker.wait_running(owned, 'agent', 200, 300), {'status': 'running'})
                watch.assert_called_once_with(Path('/owned/.swarm'), modifications=True)
                self.assertEqual(wait.call_count, expected_reads)
                self.assertEqual(read.call_count, expected_reads)
                close.assert_called_once_with(100)

    def test_observation_and_cleanup_causes_both_survive(self):
        """Failure never becomes an empty successful Agent even if close also fails."""
        owned = SimpleNamespace(root=Path('/owned'), call=Mock(side_effect=RuntimeError('owned-read-failed')))
        with patch.object(worker, 'watch_directory', return_value=100), patch.object(worker.os, 'close', side_effect=OSError('owned-close-failed')):
            with self.assertRaises(BaseExceptionGroup) as error:
                worker.wait_running(owned, 'agent', 200, 300)
        self.assertEqual([str(e) for e in error.exception.exceptions], ['owned-read-failed', 'owned-close-failed'])


if __name__ == '__main__': unittest.main()
