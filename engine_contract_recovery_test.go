//go:build linux

package main

import "testing"

// E4 acceptance entry point. These named subtests execute the actual recovery
// guards on isolated Stores and deterministic subprocess fixtures. They are not
// evidence of autonomous decisions by a live AI provider.
func TestEngineContractRecoveryCompleteCoverage(t *testing.T) {
	// Stop after tests, before review reservation: restart must reuse the
	// checkpointed candidate and never rerun controls.
	t.Run("stop_after_tests_before_review_no_rerun", TestManagedIntegrationResumesAfterTestsWithoutRerunningControls)
	// Stop after review reservation and before publication (call interrupted
	// mid-inference): explicit bounded retry resumes without a duplicate paid
	// call, same producer, same SHA.
	t.Run("stop_after_review_reserved_before_publish", TestManagedIndependentReviewInterruptedCallRequiresExplicitBoundedRetry)
	// Stop after a passed verdict, before the publication transaction: restart
	// reuses the durable verdict atomically, never pays twice.
	t.Run("stop_after_verdict_before_publish", TestManagedIndependentReviewRestartAfterVerdictDoesNotPayTwice)
	// Restart: recovery budget and cause fingerprint survive a fresh Store.
	t.Run("restart_preserves_recovery_budget", TestRecoveryBudgetAndBoundPersistAcrossStoreRestart)
	// Double conductor: a second integration attempt on the same mission
	// while one is already in flight is refused immediately, never queued or
	// silently merged.
	t.Run("double_conductor_refused", TestManagedIntegrationWorkspaceLockOutranksMissingReport)
	// Quota / budget exhausted: neither the automatic recovery budget nor the
	// reviewer's call budget fabricate a retry or a publication once spent.
	t.Run("automatic_recovery_budget_exhausted", TestRecoveryTransientKeepsOperationIdentityAndStopsCommonCauseLoop)
	t.Run("reviewer_budget_exhausted_or_missing", TestManagedIndependentReviewMissingOrBudgetExhaustedCannotPublish)
	// Git conflict: two completed attempts merging onto the same base surface
	// a real merge conflict; the loser stays blocked, the source untouched.
	t.Run("git_merge_conflict_blocks_without_corrupting_source", TestManagedIntegrationAtomicAndConflict)
	// Stale reviews are acknowledged (rejected) without ever accepting a
	// different attempt's or a different SHA's verdict in their place.
	t.Run("stale_review_acknowledged_not_substituted", TestManagedIndependentReviewBindsReportsContractAndCandidate)
	t.Run("old_sha_review_invalid_on_new_candidate", TestManagedIndependentReviewRechecksPreviouslyAcceptedTasksOnNewSHA)
	// Retry requires a new, explicit decision; budgets/attempts stay bounded.
	t.Run("retry_requires_new_decision_stays_bounded", TestManagedRetryRequiresNewDecisionAndStaysBounded)
	// A barrier proves the Git lock is available during reviewer inference,
	// while a second conductor cannot overwrite the live review.
	t.Run("git_available_and_review_owned_during_inference", TestManagedLiveReviewSurvivesConcurrentConductor)
	// If another operation retakes the lock in that window, the already
	// obtained verdict survives and a later retry reuses it without a second
	// paid reviewer call.
	t.Run("lock_contention_after_inference_keeps_verdict", TestManagedLiveReviewKeepsVerdictWhenGitLockIsRetaken)
}
