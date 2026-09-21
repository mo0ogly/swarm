//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Reconstruct the old unpaid-size boundary in a disposable Store. Production,
// candidate creation and controls use the real engine. Only the old preflight
// stop is injected; the subprocess reviewer is deterministic, not a real model.
func unpaidReviewFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", strings.Repeat("candidate source evidence\n", 2800))
	a.Ended = now()
	raw, _ := json.Marshal(a)
	if _, e := s.db.Exec("UPDATE agents SET body=? WHERE id=?", raw, a.ID); e != nil {
		t.Fatal(e)
	}
	if e := os.WriteFile(filepath.Join(a.CWD, "other.txt"), []byte(strings.Repeat("other evidence\n", 5500)), 0600); e != nil {
		t.Fatal(e)
	}
	if e := os.WriteFile(filepath.Join(a.CWD, "docs/first.review-context.json"), []byte(`{"version":1,"files":["value.txt"]}`), 0600); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	w.Planning.Reviewer.Failure = "fixture: pause before any review call"
	raw, _ = json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	w.Planning.Reviewer.Failure = ""
	raw, _ = json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	item, e := s.managedAttempt(a.ID)
	if e != nil || item.Result == "" {
		t.Fatal(item, e)
	}
	dir := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID)
	data, e := os.ReadFile(filepath.Join(dir, "candidate.json"))
	if e != nil {
		t.Fatal(e)
	}
	var cp managedReviewCheckpoint
	json.Unmarshal(data, &cp)
	receipt, e := os.ReadFile(filepath.Join(dir, "receipt.json"))
	if e != nil {
		t.Fatal(e)
	}
	c, e := s.managedReviewContext(w, a, cp.Candidate, receipt)
	if e != nil {
		t.Fatal(e)
	}
	owners, e := managedReviewSourceOwners(w, c)
	if e != nil {
		t.Fatal(e)
	}
	_, wp, e := agentWorkflow("reviewer")
	if e != nil {
		t.Fatal(e)
	}
	if plan, e := planManagedReviewBatchesTransport(managedReviewPrefix(wp), c, owners, false); e == nil || plan != nil {
		t.Fatal("old engine did not reject fixture")
	}
	t.Log("sizes", len(c.Diff), len(managedReviewPacket(c, true)), len(managedReviewPrefix(wp)), "parts", len(reviewSourceParts(c.Diff, c.Sources[0])))
	if e := s.managedFailure(a, managedReviewContextTooLarge+" : tâche first"); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	return s, w, a
}

func TestManagedPreflightRetryUsesRetainedCandidateWithoutProducer(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	before, _ := s.managedAttempt(a.ID)
	checkpoint, err := os.ReadFile(filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID, "candidate.json"))
	if err != nil {
		t.Fatal(err)
	}
	var candidate managedReviewCheckpoint
	if err = json.Unmarshal(checkpoint, &candidate); err != nil {
		t.Fatal(err)
	}
	r := PlanningRequest{Schema: 1, EventID: "retry-unpaid", Revision: w.Revision, Task: a.TaskID, Reason: "Exact duplicate evidence is now transported without loss."}
	after, e := s.planningChange(w.ID, "retry-review", r)
	if e != nil {
		t.Fatal(e)
	}
	item, _ := s.managedAttempt(a.ID)
	if item.State != "integrating" || item.Result != before.Result || len(after.Tasks[0].Attempts) != 1 || managedReviewCalls(t, s) != 0 {
		t.Fatal("retry changed production or consumed review")
	}
	if _, e = s.planningChange(w.ID, "retry-review", r); e != nil {
		t.Fatal("idempotent replay", e)
	}
	if e = s.reconcileKnownMissionResult(a, "fixture-conductor"); e != nil {
		t.Fatal(e)
	}
	after, _ = s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || task.IndependentReview == nil || task.IndependentReview.CandidateSHA != candidate.Candidate || managedReviewCalls(t, s) != 1 || len(task.Attempts) != 1 {
		t.Fatal(task.Status, task.Blocker)
	}
	if _, e = s.planningChange(w.ID, "retry-review", r); e != nil {
		t.Fatal("replay after paid verdict", e)
	}
	if e = s.reconcileKnownMissionResult(a, "fixture-conductor"); e != nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("duplicate paid review", e)
	}
	item, _ = s.managedAttempt(a.ID)
	if item.Result != before.Result {
		t.Fatal("producer result replaced")
	}
}

func TestManagedPreflightRetryRejectsMissingOrStaleProofs(t *testing.T) {
	for _, kind := range []string{"other-failure", "missing-candidate", "changed-contract", "changed-receipt", "exhausted-budget", "unconfirmed-end", "existing-review", "oversize-still"} {
		t.Run(kind, func(t *testing.T) {
			s, w, a := unpaidReviewFixture(t)
			dir := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID)
			switch kind {
			case "other-failure":
				s.managedFailure(a, "real failed control")
			case "missing-candidate":
				os.Remove(filepath.Join(dir, "candidate.json"))
			case "changed-contract":
				w.Tasks[0].Criteria = append(w.Tasks[0].Criteria, "new criterion")
			case "changed-receipt":
				os.WriteFile(filepath.Join(dir, "receipt.json"), []byte(`{"candidate_commit":"wrong"}`), 0600)
			case "exhausted-budget":
				w.Planning.Reviewer.Calls = w.Planning.Reviewer.MaxCalls
			case "unconfirmed-end":
				a.Ended = ""
				raw, _ := json.Marshal(a)
				s.db.Exec("UPDATE agents SET body=? WHERE id=?", raw, a.ID)
			case "existing-review":
				w.Tasks[0].IndependentReview = &IndependentReview{State: "changes_requested", Attempt: a.Attempt}
			case "oversize-still":
				// A valid candidate whose unchanged supplemental bytes cannot be
				// deduplicated still refuses before any reservation or mutation.
				w.Tasks[0].Criteria = append(w.Tasks[0].Criteria, strings.Repeat("criterion ", 22000))
				data, _ := os.ReadFile(filepath.Join(dir, "candidate.json"))
				var cp managedReviewCheckpoint
				json.Unmarshal(data, &cp)
				cp.Contract = managedReviewContract(w, a.TaskID)
				data, _ = json.Marshal(cp)
				os.WriteFile(filepath.Join(dir, "candidate.json"), data, 0600)
			}
			if kind != "other-failure" {
				raw, _ := json.Marshal(w)
				if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
					t.Fatal(e)
				}
			}
			w, _ = s.get(w.ID)
			before, _ := s.managedAttempt(a.ID)
			_, e := s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "refused", Revision: w.Revision, Task: a.TaskID, Reason: "Try bounded preflight again after maintenance"})
			after, _ := s.get(w.ID)
			item, _ := s.managedAttempt(a.ID)
			if e == nil || after.Revision != w.Revision || item.State != before.State || item.Result != before.Result || managedReviewCalls(t, s) != 0 {
				t.Fatal("unsafe retry changed state", e)
			}
		})
	}
}
