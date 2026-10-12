//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func validationBudgetFixture(t *testing.T, s *Store, work string, seconds int, event string) RunLimitsConfigEntry {
	t.Helper()
	e, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: event, Scope: ScopeRole, Mission: work, Key: "validation", Revision: 0, Values: RunLimits{ToolSeconds: seconds}, Reason: "Explicit operator budget for deterministic validation controls"})
	if err != nil {
		t.Fatal(err)
	}
	return e
}
func TestControlRuntimeBudgetAppliesToAutomaticValidationWithoutChangingPolicy(t *testing.T) {
	p := automaticPolicy("python3", "-c", "import time; time.sleep(1.1)")
	p.Controls[0].Timeout = 1
	s, w, a, _ := automaticValidationFixture(t, p, false)
	validationBudgetFixture(t, s, w.ID, 2, "validation-budget")
	s.conduct(a, "completed")
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := got.task("t1")
	if task.Status != "accepted" {
		t.Fatalf("explicit validation runtime budget ignored: %s", task.Blocker)
	}
	if task.ValidationPolicy.Controls[0].Timeout != 1 || task.AutoValidation.Policy.Controls[0].Timeout != 1 {
		t.Fatal("historical authorized policy rewritten")
	}
}
func TestRecoveredControlRetryWithFreshValidationBudgetKeepsResultAndAttempt(t *testing.T) {
	s, w, a, r := recoveredResultFixture(t)
	var err error
	w, err = s.mutate(w.ID, "fixture", "short-control", w.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task(a.TaskID)
		task.ValidationPolicy = automaticPolicy("python3", "-c", "import time; time.sleep(1.1)")
		task.ValidationPolicy.Controls[0].Timeout = 1
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	taskBefore, _ := w.task(a.TaskID)
	delivery, _ := json.Marshal(completeDelivery(taskBefore, a))
	if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), delivery, 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	r.ResultTree = gitTest(t, a.CWD, "write-tree")
	r.Revision = w.Revision
	w, err = s.planningChange(w.ID, "submit-recovered-result", r)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	if task.Status != "blocked" || task.RecoveredResult == nil {
		t.Fatal("timeout fixture did not retain repaired result", task.Blocker)
	}
	result := task.RecoveredResult.Result
	oldFailure := task.Blocker
	calls := w.Planning.Reviewer.Calls
	validationBudgetFixture(t, s, w.ID, 2, "new-validation-budget")
	request := PlanningRequest{Schema: 1, EventID: "retry-retained-control", Revision: w.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ResultCommit: result, ExpectedCandidate: w.Planning.Repository.Candidate, ConfirmRecovery: true, Reason: "Operator corrected the explicit validation runtime budget after the retained timeout"}
	w, err = s.planningChange(w.ID, "retry-integration", request)
	if err != nil {
		t.Fatalf("repaired result cannot resume after fresh budget: %v", err)
	}
	task, _ = w.task(a.TaskID)
	if len(task.Attempts) != 1 || task.RecoveredResult.Result != result || w.Planning.Reviewer.Calls != calls || task.Status != "blocked" || len(task.IntegrationRetries) != 1 || task.IntegrationRetries[0].Failure != oldFailure {
		t.Fatal("retry spent or accepted before controls")
	}
	original, _ := json.Marshal(task.RecoveredResult)
	if err = s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ = w.task(a.TaskID)
	current, _ := json.Marshal(task.RecoveredResult)
	if task.Status != "accepted" || string(current) != string(original) || len(task.Attempts) != 1 {
		t.Fatal("recovery lost binding or never rechecked", task.Blocker)
	}
	if _, err = s.planningChange(w.ID, "retry-integration", request); err != nil {
		t.Fatal("replay", err)
	}
}

func TestRecoveredControlRuntimeRetryRefusesUnchangedOrUnconfirmedBudget(t *testing.T) {
	for _, mode := range []string{"missing-budget", "unconfirmed", "budget-before-failure", "changed-result", "changed-attempt"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, r := recoveredResultFixture(t)
			var err error
			w, err = s.mutate(w.ID, "fixture", "short-control", w.Revision, []byte(`{}`), func(w *Work) error {
				task, _ := w.task(a.TaskID)
				task.ValidationPolicy = automaticPolicy("python3", "-c", "import time;time.sleep(1.1)")
				task.ValidationPolicy.Controls[0].Timeout = 1
				return nil
			})
			if err != nil {
				t.Fatal(err)
			}
			if mode == "budget-before-failure" {
				validationBudgetFixture(t, s, w.ID, 1, "old-budget")
			}
			task, _ := w.task(a.TaskID)
			data, _ := json.Marshal(completeDelivery(task, a))
			if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), data, 0600); err != nil {
				t.Fatal(err)
			}
			gitTest(t, a.CWD, "add", "-A")
			r.ResultTree = gitTest(t, a.CWD, "write-tree")
			r.Revision = w.Revision
			w, err = s.planningChange(w.ID, "submit-recovered-result", r)
			if err != nil {
				t.Fatal(err)
			}
			task, _ = w.task(a.TaskID)
			if task.RecoveredResult == nil || task.Status != "blocked" {
				t.Fatal("fixture", task.Blocker)
			}
			if mode != "missing-budget" && mode != "budget-before-failure" {
				validationBudgetFixture(t, s, w.ID, 2, "fresh-budget")
			}
			request := PlanningRequest{Schema: 1, EventID: "forbidden-control-retry", Revision: w.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ResultCommit: task.RecoveredResult.Result, ExpectedCandidate: w.Planning.Repository.Candidate, ConfirmRecovery: mode != "unconfirmed", Reason: "Inspect controlled recovery without inventing acceptance"}
			if mode == "changed-result" {
				request.ResultCommit = "0000000000000000000000000000000000000000"
			}
			if mode == "changed-attempt" {
				request.Attempt = "other-attempt"
			}
			before, _ := s.get(w.ID)
			itemBefore, _ := s.managedAttempt(a.ID)
			if _, err = s.planningChange(w.ID, "retry-integration", request); err == nil {
				t.Fatal("unsafe recovery admitted")
			}
			after, _ := s.get(w.ID)
			itemAfter, _ := s.managedAttempt(a.ID)
			if after.Revision != before.Revision || after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls || itemAfter.State != itemBefore.State || itemAfter.Result != itemBefore.Result {
				t.Fatal("refused request mutated result/budget")
			}
		})
	}
}

func TestRecoveredControlRetryRejectsBudgetChangedAfterReservation(t *testing.T) {
	s, w, a, _ := recoveredResultFixture(t)
	cfg := validationBudgetFixture(t, s, w.ID, 2, "reserved-budget")
	_, digest, err := s.validationRuntimeLimits(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	item, _ := s.managedAttempt(a.ID)
	task.IntegrationRetries = []IntegrationRetry{{Agent: a.ID, Attempt: a.Attempt, Result: item.Result, Candidate: w.Planning.Repository.Candidate, Contract: managedReviewContract(w, task.ID), RuntimeBudgetDigest: digest}}
	if err := s.integrationRetryGuard(w, task, a, item); err != nil {
		t.Fatal(err)
	}
	_, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "budget-replaced", Scope: ScopeRole, Mission: w.ID, Key: "validation", Revision: cfg.Revision, Values: RunLimits{ToolSeconds: 3}, Reason: "Fresh operator update after reservation"})
	if err != nil {
		t.Fatal(err)
	}
	if err := s.integrationRetryGuard(w, task, a, item); err == nil {
		t.Fatal("changed reserved budget silently consumed")
	}
}
