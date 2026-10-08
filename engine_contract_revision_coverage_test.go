//go:build linux

package main

import "testing"

func TestEngineContractRevisionCompleteCoverage(t *testing.T) {
	t.Run("same_sha_separate_reviewer", TestManagedIndependentReviewPublishesSameSHAAndSeparateProcess)
	t.Run("failure_unknown_wrong_sha", TestManagedIndependentReviewRejectsFailuresUnknownAndWrongSHA)
	t.Run("bound_report_contract_receipt", TestManagedIndependentReviewBindsReportsContractAndCandidate)
	t.Run("legacy_acceptance_requires_review", TestManagedIndependentReviewLegacyFlagCannotBypassAcceptance)
	t.Run("reopened_dependency", TestReopenedDependencyInvalidatesAcceptance)
	t.Run("new_sha_invalidates_acceptance", TestManagedIndependentReviewRechecksPreviouslyAcceptedTasksOnNewSHA)
	t.Run("context_refused_without_truncation", TestManagedIndependentReviewRefusesUnreviewableContextWithoutTruncation)
}
