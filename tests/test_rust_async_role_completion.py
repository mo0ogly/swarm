"""Regression: dispatch synchronization alone cannot finish an async role fixture."""
import unittest
import rust_mission_report_relay as relay


class AsyncRoleCompletion(unittest.TestCase):
    """Guard the original premature-stop counterexample and recovery."""
    def test_requested_role_receipt_is_required_after_real_dispatch(self):
        """Valid receipt precedes marker-only/wrong-role refusal and recovery."""
        sync = {'event': 'mission-watch-blocked', 'synchronization_committed': True}
        controls = {'event': 'mission-controls-recorded'}
        review = {'event': 'mission-review-recorded'}
        target = 'mission-review-recorded'
        self.assertTrue(relay.cycle_complete([sync, controls, review], target))
        for early in [[sync], [sync, controls], [review], [{'event': 'mission-watch-blocked'}, review]]:
            self.assertFalse(relay.cycle_complete(early, target))
            self.assertTrue(relay.cycle_complete(early + [sync, review], target))
        self.assertTrue(relay.cycle_complete([sync]))
        self.assertFalse(relay.cycle_complete([{'event': 'mission-watch-blocked'}]))


if __name__ == '__main__': unittest.main()
