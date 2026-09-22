//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestConductorReconcilesOrphanedReviewWithoutNewCall(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "result\n")
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture", "mode"), []byte("exit"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	review := task.IndependentReview
	if review == nil || review.State != "error" {
		t.Fatal("fixture review not interrupted")
	}
	// Reconstruct a crash after reserving the real context but before a durable
	// verdict, exclusively in this disposable Store.
	review.State = "running"
	review.Finished = ""
	task.Status = "blocked"
	w.Planning.Repository.Candidate = review.PreviousCandidate
	pending := []PlanningEvent{}
	for _, ev := range w.Planning.Inbox {
		if ev.ID != review.ID {
			pending = append(pending, ev)
		}
	}
	w.Planning.Inbox = pending
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("DELETE FROM events WHERE id=?", review.ID+"-result"); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("UPDATE managed_attempts SET state='conflict',detail='interrupted review' WHERE agent_id=?", a.ID); err != nil {
		t.Fatal(err)
	}
	calls := managedReviewCalls(t, s)
	unlock, err := managedLock(s.root, w.ID)
	if err != nil {
		t.Fatal(err)
	}
	s.conduct(a, "completed")
	locked, _ := s.get(w.ID)
	if locked.Tasks[0].IndependentReview.State != "running" {
		unlock()
		t.Fatal("live review disturbed")
	}
	unlock()
	releaseReview, err := managedReviewOwnershipLock(s.root, w.ID, a.ID)
	if err != nil {
		t.Fatal(err)
	}
	s.conduct(a, "completed")
	held, _ := s.get(w.ID)
	if held.Tasks[0].IndependentReview.State != "running" {
		releaseReview()
		t.Fatal("review ownership ignored with free Git lock")
	}
	releaseReview()
	s.conduct(a, "completed")
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	got, _ := after.task(a.TaskID)
	if got.IndependentReview.State != "error" {
		t.Fatalf("orphan remains %s", got.IndependentReview.State)
	}
	if got.IndependentReview.ID != review.ID || got.IndependentReview.CandidateSHA != review.CandidateSHA || managedReviewCalls(t, s) != calls {
		t.Fatal("candidate or paid reservation changed")
	}
	for i := 0; i < 3; i++ {
		s.conduct(a, "completed")
	}
	stable, _ := s.get(w.ID)
	if stable.Revision != after.Revision {
		t.Fatal("polling repeats orphan transition")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture", "mode"), []byte("pass"), 0600); err != nil {
		t.Fatal(err)
	}
	_, err = reopened.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "retry-orphan", Revision: stable.Revision, Task: a.TaskID, Reason: "Controller stopped; resume preserved candidate with unchanged evidence"})
	if err != nil {
		t.Fatal("public retry unavailable after reconciliation", err)
	}
	if err = reopened.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	final, _ := reopened.get(w.ID)
	result, _ := final.task(a.TaskID)
	if result.Status != "accepted" || result.IndependentReview.CandidateSHA != review.CandidateSHA || managedReviewCalls(t, reopened) != calls+1 {
		t.Fatalf("retry result status=%s review=%+v candidate=%s calls=%d want=%d blocker=%s", result.Status, result.IndependentReview, review.CandidateSHA, managedReviewCalls(t, reopened), calls+1, result.Blocker)
	}
}
