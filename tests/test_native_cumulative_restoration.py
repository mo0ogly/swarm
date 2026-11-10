"""Regression guards for retaining original and restoration failures together.

Pure exception fixtures only: no Root state, SQL, provider or accepted evidence.
"""
import unittest
from rust_native_cumulative_publication import restoration


class RestorationCauses(unittest.TestCase):
    """Verify ownership cleanup without disguising partial or failed recovery."""
    def test_success_runs_cleanup(self):
        """Successful body still executes its owned cleanup exactly once."""
        calls=[]
        with restoration(lambda:calls.append('restored')):calls.append('body')
        self.assertEqual(calls,['body','restored'])

    def test_primary_survives_successful_restoration(self):
        """A restored fixture does not turn its original failure into success."""
        original=ValueError('original fixture failure');calls=[]
        with self.assertRaises(ValueError) as caught:
            with restoration(lambda:calls.append('restored')):raise original
        self.assertIs(caught.exception,original);self.assertEqual(calls,['restored'])

    def test_late_restoration_failure_is_not_success(self):
        """A failure after successful body reports cleanup uncertainty directly."""
        failure=OSError('restore failed')
        def cleanup():
            """Inject the exact owned cleanup error, without effects."""
            raise failure
        with self.assertRaises(OSError) as caught:
            with restoration(cleanup):pass
        self.assertIs(caught.exception,failure)

    def test_compound_failure_preserves_both_original_objects(self):
        """Neither original failure nor failed restoration may mask the other."""
        original=ValueError('body');secondary=OSError('restore')
        def cleanup():
            """Inject a separate restore failure while the primary is active."""
            raise secondary
        with self.assertRaises(ExceptionGroup) as caught:
            with restoration(cleanup):raise original
        self.assertEqual(caught.exception.exceptions,(original,secondary))


if __name__=='__main__':unittest.main()
