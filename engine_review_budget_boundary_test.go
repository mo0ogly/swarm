//go:build linux

package main

import "testing"

// These disposable-store tests exercise the exact boundary disputed by the
// reviewer. The provider is a deterministic subprocess, not a real model.
func TestEngineReviewBudgetExactRemainingBatches(t *testing.T) {
	s, w, a := managedBatchRuntimeFixture(t)
	_, err := s.mutate(w.ID, "test.exact-budget", "test-exact-budget", w.Revision, []byte(`{}`), func(current *Work) error {
		current.Planning.Reviewer.MaxCalls = current.Planning.Reviewer.Calls + 2
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || after.Planning.Reviewer.Calls != after.Planning.Reviewer.MaxCalls || managedReviewCalls(t, s) != 3 {
		t.Fatalf("exact remaining budget refused: status=%s blocker=%s calls=%d/%d", task.Status, task.Blocker, after.Planning.Reviewer.Calls, after.Planning.Reviewer.MaxCalls)
	}
}
func TestEngineReviewBudgetExactPreflightRetry(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	w, err := s.mutate(w.ID, "test.exact-budget", "test-exact-budget", w.Revision, []byte(`{}`), func(current *Work) error {
		current.Planning.Reviewer.MaxCalls = current.Planning.Reviewer.Calls + 1
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	_, err = s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "exact-budget-retry", Revision: w.Revision, Task: a.TaskID, Reason: "Test a retained candidate with exactly one review call available"})
	if err != nil {
		t.Fatal(err)
	}
	if managedReviewCalls(t, s) != 0 {
		t.Fatal("preflight spent a call")
	}
	if err = s.reconcileKnownMissionResult(a, "test-conductor"); err != nil {
		t.Fatal(err)
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || after.Planning.Reviewer.Calls != after.Planning.Reviewer.MaxCalls || managedReviewCalls(t, s) != 1 {
		t.Fatalf("exact preflight budget refused: status=%s blocker=%s calls=%d/%d", task.Status, task.Blocker, after.Planning.Reviewer.Calls, after.Planning.Reviewer.MaxCalls)
	}
}
