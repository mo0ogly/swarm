//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"testing"
)

// Only the isolated fixture is made historical. Read paths must not rewrite it.
func TestEngineContractHistoryLegacyAcceptanceReadOnly(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if w.Tasks[0].Status != "accepted" {
		t.Fatal("fixture not accepted")
	}
	w.Planning.Reviewer = nil
	w.Planning.ReviewerRequired = false
	w.Tasks[0].IndependentReview = nil
	raw, err := json.Marshal(w)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	var before string
	if err = s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&before); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 2; i++ {
		current, err := s.get(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		if s.acceptedFresh(&current, &current.Tasks[0], map[string]bool{}) {
			t.Fatal("historical acceptance became fresh without review")
		}
		var out, errs bytes.Buffer
		if code := run([]string{"--root", s.root, "--json", "work", "show", w.ID}, &out, &errs); code != 0 {
			t.Fatal(code, errs.String())
		}
		var view struct {
			Work       Work `json:"work"`
			Validation struct {
				Tasks map[string]struct {
					Fresh bool `json:"fresh"`
				} `json:"tasks"`
			} `json:"validation"`
		}
		if err = json.Unmarshal(out.Bytes(), &view); err != nil {
			t.Fatal(err)
		}
		state, ok := view.Validation.Tasks["first"]
		if !ok || state.Fresh || view.Work.Tasks[0].Status != "accepted" {
			t.Fatal("CLI hides legacy status or grants current validity", out.String())
		}
	}
	var after string
	if err = s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&after); err != nil {
		t.Fatal(err)
	}
	if before != after {
		t.Fatal("reading historical evidence mutated work")
	}
}

func TestEngineContractHistoryRecoveryEvidence(t *testing.T) {
	t.Run("public_retry_preserves_result_and_reviews", TestIntegrationRetryPublicPreservesResultAndReviews)
	t.Run("missing_legacy_evidence_stays_explicit", TestIntegrationRetryLegacyKeepsMissingEvidenceExplicit)
	t.Run("new_candidate_invalidates_prior_acceptance", TestManagedIndependentReviewRechecksPreviouslyAcceptedTasksOnNewSHA)
	t.Run("historical_review_receipt_preserved", TestIncrementalReviewNewVerdictAndHistoricalEvidence)
}
