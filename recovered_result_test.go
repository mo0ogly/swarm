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

func TestRecoveredResultRevisionPreservesRefusalAndChecksNewCandidate(t *testing.T) {
	s, w, a, r := recoveredResultFixture(t)
	managedReviewMode(t, s, "fail")
	first, e := s.submitRecoveredResult(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	prior, _ := first.task(a.TaskID)
	old := *prior.IndependentReview
	oldReceipt, e := os.ReadFile(filepath.Join(s.root, old.Receipt))
	if e != nil {
		t.Fatal(e)
	}
	oldResult := prior.RecoveredResult.Result
	repo, err := s.managedRepository(first)
	if err != nil {
		t.Fatal(err)
	}
	bare := filepath.Join(repo.Storage, "repository.git")
	unchangedRequest := r
	unchangedRequest.EventID = "unchanged-repair"
	unchangedRequest.Revision = first.Revision
	unchangedRequest.ReviewID = old.ID
	if _, err = s.planningChange(w.ID, "revise-recovered-result", unchangedRequest); err == nil {
		t.Fatal("unchanged result re-reviewed")
	}
	if managedReviewCalls(t, s) != 1 {
		t.Fatal("unchanged result consumed review")
	}
	os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("Corrected external report with complete evidence"), 0600)
	gitTest(t, a.CWD, "add", "-A")
	next := r
	next.EventID = "revised-repair"
	next.Revision = first.Revision
	next.ReviewID = old.ID
	next.ResultTree = gitTest(t, a.CWD, "write-tree")
	bad := next
	bad.ReviewID = "wrong"
	if _, e = s.planningChange(w.ID, "revise-recovered-result", bad); e == nil {
		t.Fatal("wrong review accepted")
	}
	unchanged := next
	unchanged.ResultTree = r.ResultTree
	// Restore the original tree in the index alone: the write-tree guard must still
	// examine actual current files and refuse this stale approval.
	if _, e = s.planningChange(w.ID, "revise-recovered-result", unchanged); e == nil {
		t.Fatal("stale tree accepted")
	}
	managedReviewMode(t, s, "pass")
	after, e := s.planningChange(w.ID, "revise-recovered-result", next)
	if e != nil {
		t.Fatal(e)
	}
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || task.RecoveredResult.ReplacesResult != oldResult || task.RecoveredResult.PriorReview != old.ID || task.IndependentReview.CandidateSHA == old.CandidateSHA {
		t.Fatal(task.Status, task.Blocker)
	}
	if got := gitTest(t, bare, "rev-parse", "refs/swarm/candidates/"+a.ID); got != old.CandidateSHA {
		t.Fatal("old candidate ref overwritten", got)
	}
	if got := gitTest(t, bare, "rev-parse", "refs/swarm/candidates/"+task.RecoveredResult.ProofKey); got != task.IndependentReview.CandidateSHA {
		t.Fatal("new candidate missing", got)
	}
	preserved, e := os.ReadFile(filepath.Join(s.root, old.Receipt))
	if e != nil || string(preserved) != string(oldReceipt) {
		t.Fatal("old receipt overwritten")
	}
	if len(task.Attempts) != 1 || task.Attempts[0].Status != "interrupted" || managedReviewCalls(t, s) != 2 {
		t.Fatal("history or calls changed")
	}
	if _, e = s.planningChange(w.ID, "revise-recovered-result", next); e != nil || managedReviewCalls(t, s) != 2 {
		t.Fatal("replay called review", e)
	}
	// An accepted repair cannot silently be replaced.
	next.EventID = "another"
	next.Revision = after.Revision
	next.ReviewID = task.IndependentReview.ID
	if _, e = s.planningChange(w.ID, "revise-recovered-result", next); e == nil {
		t.Fatal("accepted repair replaced")
	}
}

func TestRecoveredResultRevisionOfCompletedProducer(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "initial\n")
	managedReviewMode(t, s, "fail")
	if err := s.finishAgent(a, "completed", "", nil); err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	managedReviewMode(t, s, "fail")
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	prior := task.IndependentReview.ID
	os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("Corrected proof for completed producer"), 0600)
	body, _ := json.Marshal(completeDelivery(task, a))
	os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), body, 0600)
	gitTest(t, a.CWD, "add", "-A")
	req := PlanningRequest{Schema: 1, EventID: "completed-correction", Revision: w.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ConfirmRecovery: true, ReviewID: prior, ResultTree: gitTest(t, a.CWD, "write-tree"), Reason: "Correct rejected result with complete evidence"}
	managedReviewMode(t, s, "pass")
	after, err := s.planningChange(w.ID, "revise-recovered-result", req)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = after.task(a.TaskID)
	saved, _ := s.agent(a.ID)
	if task.Status != "accepted" || saved.Status != "completed" || task.RecoveredResult.PriorReview != prior || managedReviewCalls(t, s) != 2 {
		t.Fatal(task.Status, saved.Status)
	}
}

func TestRecoveredResultReviewRetryIsDrivenWithoutNewProducer(t *testing.T) {
	s, w, a, req := recoveredResultFixture(t)
	managedReviewMode(t, s, "exit")
	first, err := s.submitRecoveredResult(w.ID, req)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := first.task(a.TaskID)
	if task.IndependentReview.State != "error" {
		t.Fatal(task.IndependentReview.State)
	}
	candidate := task.IndependentReview.CandidateSHA
	managedReviewMode(t, s, "pass")
	_, err = s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "retry-fixed-provider", Revision: first.Revision, Task: a.TaskID, Reason: "Provider failure corrected; same candidate and no new producer"})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.reconcileKnownMissionResult(a, "test-conductor"); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	task, _ = after.task(a.TaskID)
	if task.Status != "accepted" || task.IndependentReview.CandidateSHA != candidate || len(task.Attempts) != 1 || task.Attempts[0].Status != "interrupted" || managedReviewCalls(t, s) != 2 {
		t.Fatal(task.Status, task.Blocker)
	}
	if err = s.reconcileKnownMissionResult(a, "test-conductor"); err != nil || managedReviewCalls(t, s) != 2 {
		t.Fatal("duplicate review", err)
	}
}
