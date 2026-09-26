//go:build linux

package main

import "testing"

// The public delivery acceptance runner must execute these behavioral guards,
// rather than succeed on an empty test selection or the presence of a report.
func TestEngineContractDeliveryEvidenceAndPublication(t *testing.T) {
	t.Run("incomplete_delivery_stops_before_review", TestManagedDeliveryIncompleteStopsBeforePaidReview)
	t.Run("complete_report_still_requires_review", TestManagedCompleteDeliveryStillRequiresIndependentReview)
	t.Run("failed_control_prevents_delivery", TestManagedLocalDeliveryCannotBypassFailingEngineControl)
	t.Run("new_candidate_invalidates_old_acceptance", TestManagedIndependentReviewRechecksPreviouslyAcceptedTasksOnNewSHA)
	t.Run("root_waits_for_child_closure", TestEngineContractRealRootCannotCloseBeforeChildScope)
	t.Run("bundle_contains_delivered_branch", TestManagedBundleChecksOutDeliveredBranch)
}
