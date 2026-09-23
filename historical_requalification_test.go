//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func historicalFixture(t *testing.T) (*Store, Work, Agent, PlanningRequest) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	w.Tasks[0].IndependentReview = nil
	raw, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	a.Ended = now()
	raw, _ = json.Marshal(a)
	if _, e := s.db.Exec("UPDATE agents SET body=? WHERE id=?", raw, a.ID); e != nil {
		t.Fatal(e)
	}
	item, _ := s.managedAttempt(a.ID)
	return s, w, a, PlanningRequest{Schema: 1, EventID: "historical-review", Revision: w.Revision, Task: "first", Agent: a.ID, Attempt: a.Attempt, ResultCommit: item.Result, ExpectedCandidate: w.Planning.Repository.Candidate, Reason: "Explicit review of retained historical candidate"}
}
func TestEngineContractHistoryPublicRequalification(t *testing.T) {
	s, w, a, r := historicalFixture(t)
	oldPath := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID, "receipt.json")
	old, e := os.ReadFile(oldPath)
	if e != nil {
		t.Fatal(e)
	}
	calls := w.Planning.Reviewer.Calls
	after, e := s.planningChange(w.ID, "requalify", r)
	if e != nil {
		t.Fatal(e)
	}
	if after.Tasks[0].Status != "blocked" || after.Tasks[0].Gate != nil || len(after.Tasks[0].Requalifications) != 1 || after.Planning.Reviewer.Calls != calls {
		t.Fatal("reservation granted validity or consumed review")
	}
	prior := after.Tasks[0].Requalifications[0]
	if prior.Status != "accepted" || prior.Gate == nil || prior.Validation == nil {
		t.Fatal("history lost")
	}
	if _, e = s.planningChange(w.ID, "requalify", r); e != nil {
		t.Fatal("replay", e)
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	if e = reopened.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	current, _ := reopened.get(w.ID)
	task := current.Tasks[0]
	if task.Status != "accepted" || task.IndependentReview == nil || task.IndependentReview.State != "passed" || task.IndependentReview.CandidateSHA != task.AutoValidation.CandidateSHA || current.Planning.Reviewer.Calls != calls+1 || len(task.Attempts) != 1 {
		t.Fatal("fresh controls/review missing", task.Blocker)
	}
	item, _ := reopened.managedAttempt(a.ID)
	if item.Result != r.ResultCommit {
		t.Fatal("result changed")
	}
	saved, _ := os.ReadFile(oldPath)
	if string(saved) != string(old) {
		t.Fatal("old receipt overwritten")
	}
	if _, e = reopened.planningChange(w.ID, "requalify", r); e != nil {
		t.Fatal(e)
	}
	final, _ := reopened.get(w.ID)
	if final.Revision != current.Revision || final.Planning.Reviewer.Calls != current.Planning.Reviewer.Calls {
		t.Fatal("replay repeats work")
	}
}
func TestEngineContractHistoryRequalificationRejectsDrift(t *testing.T) {
	for _, kind := range []string{"revision", "candidate", "result", "attempt", "reviewer", "budget", "review-existing"} {
		t.Run(kind, func(t *testing.T) {
			s, w, _, r := historicalFixture(t)
			switch kind {
			case "revision":
				r.Revision--
			case "candidate":
				r.ExpectedCandidate = "wrong"
			case "result":
				r.ResultCommit = "wrong"
			case "attempt":
				r.Attempt = "wrong"
			case "reviewer":
				w.Planning.Reviewer = nil
			case "budget":
				w.Planning.Reviewer.Calls = w.Planning.Reviewer.MaxCalls
			case "review-existing":
				w.Tasks[0].IndependentReview = &IndependentReview{State: "passed"}
			}
			raw, _ := json.Marshal(w)
			s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
			if _, e := s.planningChange(w.ID, "requalify", r); e == nil {
				t.Fatal("invalid requalification accepted")
			}
			current, _ := s.get(w.ID)
			if current.Revision != w.Revision || current.Tasks[0].Status != "accepted" || len(current.Tasks[0].Requalifications) != 0 {
				t.Fatal("rejection changed history")
			}
		})
	}
}
