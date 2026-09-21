//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func recoveredResultFixture(t *testing.T) (*Store, Work, Agent, PlanningRequest) {
	t.Helper()
	s, w := managedFixture(t)
	a, _, e := s.prepare(w.ID, Launch{Schema: 1, EventID: "interrupted-repair", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "work", Timeout: 60})
	if e != nil {
		t.Fatal(e)
	}
	if e = s.finishAgent(a, "interrupted", "Limite d'appels d'outils atteinte", nil); e != nil {
		t.Fatal(e)
	}
	a, _ = s.agent(a.ID)
	w, _ = s.get(w.ID)
	os.MkdirAll(filepath.Join(a.CWD, "docs"), 0700)
	os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("External repair: checked value, original process interrupted."), 0600)
	task, _ := w.task(a.TaskID)
	b, _ := json.Marshal(completeDelivery(task, a))
	os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), b, 0600)
	gitTest(t, a.CWD, "add", "-A")
	r := PlanningRequest{Schema: 1, EventID: "repair-result", Revision: w.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ConfirmRecovery: true, ResultTree: gitTest(t, a.CWD, "write-tree"), Reason: "External correction examined and checked; retain original interrupted process"}
	return s, w, a, r
}

func TestRecoveredResultChecksReviewAndPreservesInterruptedAttempt(t *testing.T) {
	s, w, a, r := recoveredResultFixture(t)
	after, e := s.planningChange(w.ID, "submit-recovered-result", r)
	if e != nil {
		t.Fatal(e)
	}
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || task.RecoveredResult == nil || !s.acceptedFresh(&after, task, map[string]bool{}) {
		t.Fatal(task.Status, task.Blocker)
	}
	saved, _ := s.agent(a.ID)
	if saved.Status != "interrupted" || len(task.Attempts) != 1 || task.Attempts[0].Status != "interrupted" || task.PlanMaxAttempts != w.Tasks[0].PlanMaxAttempts || managedReviewCalls(t, s) != 1 {
		t.Fatal("attempt history or limits rewritten")
	}
	receipt, e := os.ReadFile(filepath.Join(s.root, task.AutoValidation.Receipt))
	if e != nil {
		t.Fatal(e)
	}
	var v map[string]any
	json.Unmarshal(receipt, &v)
	if v["external_repair"] == nil {
		t.Fatal("external intervention hidden from review")
	}
	if task.IndependentReview.CandidateSHA != task.AutoValidation.CandidateSHA {
		t.Fatal("review did not cover tested commit")
	}
	if _, e = s.planningChange(w.ID, "submit-recovered-result", r); e != nil {
		t.Fatal(e)
	}
	if managedReviewCalls(t, s) != 1 {
		t.Fatal("replay launched a paid review")
	}
}

func TestRecoveredResultRefusesUnexaminedOrIncompleteInput(t *testing.T) {
	for _, kind := range []string{"confirmation", "revision", "attempt", "changed", "partial", "active", "reviewer-fail"} {
		t.Run(kind, func(t *testing.T) {
			s, w, a, r := recoveredResultFixture(t)
			switch kind {
			case "confirmation":
				r.ConfirmRecovery = false
			case "revision":
				r.Revision--
			case "attempt":
				r.Attempt = "other"
			case "changed":
				os.WriteFile(filepath.Join(a.CWD, "value.txt"), []byte("unexamined"), 0600)
			case "partial":
				os.Remove(filepath.Join(a.CWD, "docs/first.delivery.json"))
				gitTest(t, a.CWD, "add", "-A")
				r.ResultTree = gitTest(t, a.CWD, "write-tree")
			case "active":
				a.Status = "running"
				s.saveAgent(a)
			case "reviewer-fail":
				managedReviewMode(t, s, "fail")
			}
			_, e := s.planningChange(w.ID, "submit-recovered-result", r)
			if kind != "reviewer-fail" && e == nil {
				t.Fatal("unsafe submission accepted")
			}
			got, _ := s.get(w.ID)
			task, _ := got.task(a.TaskID)
			if task.Status == "accepted" || got.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
				t.Fatal("unverified result published")
			}
			calls := 0
			if kind == "reviewer-fail" {
				calls = 1
			}
			if managedReviewCalls(t, s) != calls {
				t.Fatal("unexpected review call")
			}
		})
	}
}

// Simulate a stopped host after immutable submission but before a review call.
// Only the context-size preflight may resume; the candidate and request stay bound.
func TestRecoveredResultReplayResumesContextPreflightOnly(t *testing.T) {
	s, w, a, r := recoveredResultFixture(t)
	item, e := s.managedAttempt(a.ID)
	if e != nil {
		t.Fatal(e)
	}
	result := gitTest(t, item.Path, "commit-tree", r.ResultTree, "-p", item.Base, "-m", "external fixture")
	bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
	gitTest(t, bare, "fetch", "--no-tags", item.Path, result)
	raw, _ := json.Marshal(r)
	_, e = s.mutate(w.ID, "fixture.preflight", "fixture-preflight", w.Revision, raw, func(w *Work) error {
		task, _ := w.task(a.TaskID)
		task.RecoveredResult = &RecoveredResult{Event: r.EventID, RequestDigest: hash(raw), Agent: a.ID, Attempt: a.Attempt, Result: result, Tree: r.ResultTree, Actor: "fixture", At: now(), Reason: r.Reason, ProcessStatus: a.Status}
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	_, e = s.db.Exec("UPDATE managed_attempts SET result_commit=?,state='conflict',detail=? WHERE agent_id=?", result, managedReviewContextTooLarge, a.ID)
	if e != nil {
		t.Fatal(e)
	}
	if managedReviewCalls(t, s) != 0 {
		t.Fatal("preflight consumed review")
	}
	after, e := s.submitRecoveredResult(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || managedReviewCalls(t, s) != 1 {
		t.Fatal(task.Status, task.Blocker)
	}
	if _, e = s.submitRecoveredResult(w.ID, r); e != nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("replay paid twice", e)
	}
}
