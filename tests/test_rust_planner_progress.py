"""Closed trace-parser cases only; actual syscall proof is in the public recipe."""
import unittest
import rust_mission_watch_signal_probe as probe


class PlannerProgress(unittest.TestCase):
    """Keep old/partial/wrong-operation records out of the progress witness."""
    def test_complete_timed_main_waits_need_new_bytes(self):
        """A valid line precedes stale, incomplete and wrong-operation refusals."""
        line='futex(0x1, FUTEX_WAIT_BITSET_PRIVATE, 1, {tv_sec=3, tv_nsec=4}, FUTEX_BITSET_MATCH_ANY) = -1 ETIMEDOUT (Connection timed out)\n'
        self.assertEqual(probe.completed_main_waits(line,0),[line])
        for text,offset in [(line,len(line)),(line.rstrip('\n'),0),(line.replace('FUTEX_WAIT_BITSET_PRIVATE','FUTEX_LOCK_PI_PRIVATE'),0),(line.replace('{tv_sec=3, tv_nsec=4}','NULL'),0)]:
            self.assertEqual(probe.completed_main_waits(text,offset),[])
            self.assertEqual(probe.completed_main_waits(line,0),[line])
        self.assertEqual(probe.completed_main_waits(line+line,len(line)),[line])
