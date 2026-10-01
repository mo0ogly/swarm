//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
	"time"
)

func TestRecoveryContextIsBoundedScopedAndRedacted(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	for _, line := range []string{"Read · Lecture · web/cockpit.js", "Read · Lecture · web/cockpit.js", "Bash · token=private-value", "Résultat d'outil reçu · 1 reçus"} {
		if err := s.log(a.ID, "activity", line); err != nil {
			t.Fatal(err)
		}
	}
	if err := s.log(a.ID, "output", "secret raw tool output"); err != nil {
		t.Fatal(err)
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	got, err := workerRecoveryContext(tx, a, w.ID, a.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	if strings.Count(got, "web/cockpit.js") != 1 || strings.Contains(got, "private-value") || strings.Contains(got, "secret raw") || !strings.Contains(got, "ne prouvent pas") {
		t.Fatal(got)
	}
	if _, err = workerRecoveryContext(tx, a, "different-work", a.TaskID); err == nil {
		t.Fatal("cross-work context admitted")
	}
	if got, err = workerRecoveryContext(tx, Agent{}, w.ID, a.TaskID); err != nil || got != "" {
		t.Fatal(got, err)
	}
}

func TestLastAuthorizedToolFinishesButNextCallStillStops(t *testing.T) {
	limits, _ := (RunLimits{MaxToolCalls: 1, ToolSeconds: 1}).normalized()
	g := newLoopGuard(limits)
	at := time.Now()
	g.call("last", "Write", map[string]any{"file_path": "docs/report.md"}, at)
	if g.check(at) != "" {
		t.Fatal("last authorized tool interrupted at start")
	}
	if !strings.Contains(g.check(at.Add(2*time.Second)), "outil observable") {
		t.Fatal("pending tool timeout removed")
	}
	g.result("last", false)
	if g.completed != 1 || !strings.Contains(g.check(at), "appels") {
		t.Fatal("completion lost or cap removed")
	}
	g = newLoopGuard(limits)
	g.call("last", "Read", nil, at)
	g.call("extra", "Read", nil, at)
	if !strings.Contains(g.check(at), "appels") {
		t.Fatal("extra call not stopped")
	}
}

func TestBudgetMilestonesPreserveSmallAndLargeCaps(t *testing.T) {
	for _, n := range []int{1, 2, 40, 60, 100} {
		got := workerBudgetMilestones(RunLimits{MaxToolCalls: n})
		if !strings.Contains(got, "rapport") || !strings.Contains(got, "ne créer aucune tâche") {
			t.Fatal(got)
		}
	}
	got := workerBudgetMilestones(RunLimits{MaxToolCalls: 40})
	for _, want := range []string{"8 réservés", "10 appels pour l'exploration", "À l'appel 32", "reste 40"} {
		if !strings.Contains(got, want) {
			t.Fatal(got)
		}
	}
}

func TestRecoverySelectsPreviousRecordNotCurrentOrFuture(t *testing.T) {
	s, w, prior, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	if err := s.log(prior.ID, "activity", "Read · located-before.go"); err != nil {
		t.Fatal(err)
	}
	current := prior
	current.ID = "current-recovery"
	current.Attempt = "current-attempt"
	for _, a := range []Agent{current, func() Agent { v := prior; v.ID = "future-recovery"; return v }()} {
		body, _ := json.Marshal(a)
		if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, w.ID, a.TaskID, a.CWD, a.Status, body, []byte(`{}`)); err != nil {
			t.Fatal(err)
		}
		if err := s.log(a.ID, "activity", "Read · must-not-be-in-context.go"); err != nil {
			t.Fatal(err)
		}
	}
	got, err := s.workerRecoveryForAgent(current)
	if err != nil || !strings.Contains(got, "located-before.go") || strings.Contains(got, "must-not-be-in-context.go") {
		t.Fatal(got, err)
	}
}

func TestRecoveryFocusPreservesTaskAndAttributedReviewWithoutFreshness(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	task, _ := w.task(a.TaskID)
	task.Next = "Correct the failing control only"
	task.Criteria = []string{"Preserve CLI", "Repair the web path"}
	task.IndependentReview = &IndependentReview{ID: "review-owned", Attempt: a.Attempt, Producer: a.ID, Report: "docs/task-report.md", CandidateSHA: strings.Repeat("a", 40), Criteria: []ReviewCriterion{{Index: 1, Verdict: "pass"}, {Index: 2, Verdict: "fail"}}}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	f, err := recoveryTaskFocus(tx, a, w.ID, a.TaskID)
	if err != nil || f.Review != "review-owned" || len(f.Criteria) != 2 || f.Next != task.Next {
		t.Fatalf("missing focused history: %+v %v", f, err)
	}
	for _, c := range f.Criteria {
		if !c.VerificationRequired {
			t.Fatal("historical criterion became valid")
		}
	}
	other := a
	other.Attempt = "another-attempt"
	f, err = recoveryTaskFocus(tx, other, w.ID, a.TaskID)
	if err != nil || f.Review != "" || f.Criteria[0].HistoricalVerdict != "" {
		t.Fatal("unattributed evidence crossed attempts", f, err)
	}
}
func TestRecoveryContextUsesRecentOperationsInsteadOfRepeatedInitialInventory(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	for i := 0; i < 165; i++ {
		if err := s.log(a.ID, "activity", "Read · old-inventory.go"); err != nil {
			t.Fatal(err)
		}
	}
	if err := s.log(a.ID, "activity", "Test · latest-failing-check"); err != nil {
		t.Fatal(err)
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	got, err := workerRecoveryContext(tx, a, w.ID, a.TaskID)
	if err != nil || !strings.Contains(got, "latest-failing-check") || !strings.Contains(got, `"bounded_excerpt":true`) {
		t.Fatal("latest failure not carried into recovery", got, err)
	}
}
