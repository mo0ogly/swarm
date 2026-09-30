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
