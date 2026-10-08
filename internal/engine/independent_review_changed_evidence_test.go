//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestRefusedReviewRecoveryRequiresChangedBoundEvidence(t *testing.T) {
	testRefusedRecovery(t, false, "changes_requested")
}

func TestArchivedRefusedReviewRecoveryRequiresChangedEvidence(t *testing.T) {
	testRefusedRecovery(t, true, "changes_requested")
}

func TestArchivedErrorReviewRecoveryRequiresChangedEvidence(t *testing.T) {
	testRefusedRecovery(t, true, "error")
}

func testRefusedRecovery(t *testing.T, archived bool, reviewState string) {
	t.Helper()
	s, p := preparedTeam(t)
	w, _ := s.get(p.WorkID)
	task := &w.Tasks[0]
	task.Status = "blocked"
	task.Attempts = []Attempt{{ID: "existing-attempt", Status: "completed"}}
	name := "docs/recovery-proof.md"
	if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(s.root, name)
	if err := os.WriteFile(path, []byte("original evidence"), 0600); err != nil {
		t.Fatal(err)
	}
	task.IndependentReview = &IndependentReview{ID: "refused", State: reviewState, Finished: now(), Attempt: "existing-attempt", Contract: reviewContract(task), Report: name, Digest: hash([]byte("original evidence"))}
	if archived {
		task.PreviousReviews = append(task.PreviousReviews, *task.IndependentReview)
		task.IndependentReview = nil
	}
	w.Planning.Reviewer.Calls = 1
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	request := PlanningRequest{Schema: 1, EventID: "changed-evidence-retry", Revision: w.Revision, Task: task.ID, Reason: "Le livrable corrigé contient maintenant les preuves manquantes"}
	if s.refusedReviewEvidenceChanged(&w, task) {
		t.Fatal("unchanged refusal marked stale")
	}
	if _, err := s.planningChange(w.ID, "retry-review", request); err == nil {
		t.Fatal("unchanged refusal retried")
	}
	os.Remove(path)
	if s.refusedReviewEvidenceChanged(&w, task) {
		t.Fatal("deleted proof authorizes retry")
	}
	os.WriteFile(path, []byte("corrected evidence"), 0600)
	if !s.refusedReviewEvidenceChanged(&w, task) {
		t.Fatal("changed bound proof not detected")
	}
	if archived {
		snapshot, err := s.cockpitSnapshot(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		reviews := snapshot["independent_reviews"].(map[string]any)
		if reviews[task.ID].(map[string]any)["archived_review"] == nil {
			t.Fatal("archived recovery absent from public snapshot")
		}
	}
	next, err := s.planningChange(w.ID, "retry-review", request)
	if err != nil {
		t.Fatal(err)
	}
	got := next.Tasks[0]
	if got.IndependentReview != nil || len(got.PreviousReviews) != 1 || got.PreviousReviews[0].ID != "refused" || len(got.Attempts) != 1 || got.Status != "submitted" || next.Planning.Reviewer.Calls != 1 {
		t.Fatal("history, status, attempts or spent calls changed incorrectly")
	}
}

func TestAcceptedReviewRevalidationRetainsAttemptAndBudget(t *testing.T) {
	s, p := preparedTeam(t)
	w, _ := s.get(p.WorkID)
	task := &w.Tasks[0]
	task.Status = "accepted"
	task.Attempts = []Attempt{{ID: "completed-producer", Status: "completed"}}
	name := "docs/accepted-proof.md"
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	path := filepath.Join(s.root, name)
	os.WriteFile(path, []byte("accepted evidence"), 0600)
	task.IndependentReview = &IndependentReview{ID: "old-acceptance-review", State: "passed", Finished: now(), Attempt: "completed-producer", Contract: reviewContract(task), Report: name, Digest: hash([]byte("accepted evidence"))}
	w.Planning.Reviewer.Calls = 7
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	r := PlanningRequest{Schema: 1, EventID: "revalidate-stale-acceptance", Revision: w.Revision, Task: task.ID, Reason: "Inputs changed; recheck existing result without production"}
	if s.acceptedReviewRevalidationAvailable(&w, task) {
		t.Fatal("fresh acceptance marked for revalidation")
	}
	if _, err := s.planningChange(w.ID, "retry-review", r); err == nil {
		t.Fatal("fresh acceptance reopened through retry")
	}
	os.Remove(path)
	if s.acceptedReviewRevalidationAvailable(&w, task) {
		t.Fatal("missing evidence enables retry")
	}
	os.WriteFile(path, []byte("changed evidence"), 0600)
	if !s.acceptedReviewRevalidationAvailable(&w, task) {
		t.Fatal("changed bound evidence ignored")
	}
	w.Planning.Reviewer.Calls = w.Planning.Reviewer.MaxCalls
	raw, _ = json.Marshal(w)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	if _, err := s.planningChange(w.ID, "retry-review", r); err == nil {
		t.Fatal("exhausted reviewer allowed revalidation")
	}
	w.Planning.Reviewer.Calls = 7
	raw, _ = json.Marshal(w)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	next, err := s.planningChange(w.ID, "retry-review", r)
	if err != nil {
		t.Fatal(err)
	}
	got := next.Tasks[0]
	if got.Status != "submitted" || got.IndependentReview != nil || len(got.PreviousReviews) != 1 || len(got.Attempts) != 1 || got.Attempts[0].Status != "completed" || next.Planning.Reviewer.Calls != 7 || got.Gate != nil {
		t.Fatal("revalidation changed production or reused acceptance", got)
	}
}
