//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestMissionWaitsUsesCurrentEvidence(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), true)
	s.conduct(a, "completed")
	w, _ = s.get(w.ID)
	child, _ := w.task("t2")
	if got := s.taskWaits(&w, child); len(got) != 0 {
		t.Fatal("fresh accepted parent retained child", got)
	}
	os.WriteFile(filepath.Join(s.root, report), []byte("changed evidence"), 0600)
	got := s.taskWaits(&w, child)
	if len(got) != 1 || got[0].Task != "t1" || got[0].State != "stale" {
		t.Fatal("stale prerequisite concealed", got)
	}
	child.Depends = append(child.Depends, "missing")
	got = s.taskWaits(&w, child)
	if len(got) != 2 || got[1].State != "missing" {
		t.Fatal(got)
	}
}
func TestMissionChangesBoundedAndReadOnly(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t1", Title: "User task", Criteria: []string{"proof"}, Deliverable: "docs/a.md", Next: "produce"})
	first, e := s.missionChanges(w, Visit{})
	if e != nil || !first.First || len(first.Items) != 0 {
		t.Fatal(first, e)
	}
	v := Visit{Revision: w.Revision, At: now()}
	for i := 1; i <= 205; i++ {
		raw, _ := json.Marshal(map[string]any{"id": "t1", "status": "blocked"})
		if _, e = s.db.Exec("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,?,?,?,?)", newID("fixture-"), w.ID, w.Revision+i, "task.update", now(), raw, raw); e != nil {
			t.Fatal(e)
		}
	}
	historical := w
	historical.Revision += 205
	changes, e := s.missionChanges(historical, v)
	if e != nil || !changes.More || len(changes.Items) != 200 || changes.Items[0].Revision != historical.Revision {
		t.Fatal(changes, e)
	}
	for _, item := range changes.Items {
		if item.Task != "t1" || item.Category != "block" {
			t.Fatal(item)
		}
	}
	still, _ := s.get(w.ID)
	if still.Revision != w.Revision {
		t.Fatal("read changed work")
	}
	other := createTest(t, s)
	empty, e := s.missionChanges(other, v)
	if e != nil || len(empty.Items) != 0 {
		t.Fatal("foreign history leaked", empty, e)
	}
}
func TestMissionSpendingSeparatesUnitsAndMissingCost(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	price := 1.25
	agents := []Agent{{TaskID: "t1", Progress: AgentProgress{ToolCalls: 7}, Usage: &Usage{Input: 100, Output: 20, ReportedCost: &price}}, {TaskID: "t1", Previous: "old", Mode: "terminal"}}
	for _, scope := range []string{"root", "reviewer"} {
		if _, e := s.db.Exec("INSERT INTO planning_calls(id,work_id,scope_id,estimate,state,created_at) VALUES(?,?,?,0,'estimated',?)", newID("fixture-"), w.ID, scope, now()); e != nil {
			t.Fatal(e)
		}
	}
	record, _ := json.Marshal(AutomaticValidation{Controls: []ValidationControlResult{{Executed: true}, {Executed: false}}})
	if _, e := s.db.Exec("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,'task.auto-validation',?,?,?)", newID("fixture-"), w.ID, w.Revision+1, now(), record, record); e != nil {
		t.Fatal(e)
	}
	got, e := s.missionSpending(w, agents)
	if e != nil || len(got.Rows) != 3 || got.Controls != 1 || got.Retries != 1 {
		t.Fatal(got, e)
	}
	for _, row := range got.Rows {
		if row.Kind == "worker" {
			if row.Calls != 2 || row.Tools != 7 || row.UnknownTools != 2 || row.Cost.Reported != price || row.Cost.WithCost != 1 || row.Cost.Silent != 1 || row.MissingUsage != 1 {
				t.Fatal(row)
			}
		} else if row.Calls != 1 || row.Tools != 0 || row.MissingUsage != 1 || row.Cost.WithCost != 0 {
			t.Fatal("unreported usage invented", row)
		}
	}
}

// REQ-QW7: reviews and retries are reported per task, so one task's rework
// does not inflate another task's row nor the engine-wide total.
func TestMissionSpendingTracksReviewsAndRetriesPerTask(t *testing.T) {
	s := storeTest(t)
	w := Work{ID: "w-fixture", Tasks: []Task{
		{ID: "t1", Title: "Reviewed twice", Status: "accepted",
			IndependentReview: &IndependentReview{ID: "cur"},
			PreviousReviews:   []IndependentReview{{ID: "old"}}},
		{ID: "t2", Title: "Never reviewed", Status: "ready"},
	}}
	agents := []Agent{
		{TaskID: "t1", Progress: AgentProgress{ToolCalls: 1}},
		{TaskID: "t1", Previous: "old-attempt", Progress: AgentProgress{ToolCalls: 1}},
		{TaskID: "t2", Progress: AgentProgress{ToolCalls: 1}},
	}
	got, e := s.missionSpending(w, agents)
	if e != nil {
		t.Fatal(e)
	}
	by := map[string]SpendingRow{}
	for _, row := range got.Rows {
		if row.Kind == "worker" {
			by[row.ID] = row
		}
	}
	if by["t1"].Reviews != 2 || by["t1"].TaskRetries != 1 {
		t.Fatal("reviewed task with one retry misreported", by["t1"])
	}
	if by["t2"].Reviews != 0 || by["t2"].TaskRetries != 0 {
		t.Fatal("never-reviewed, never-retried task must report real zeros, not t1's counts", by["t2"])
	}
	if got.Retries != 1 {
		t.Fatal("engine-wide retries must still equal the sum across tasks", got.Retries)
	}
	var out bytes.Buffer
	printMissionSpending(&out, got)
	text := out.String()
	if !strings.Contains(text, "2 revue(s) enregistrée(s) pour cette tâche · 1 reprise(s) pour cette tâche") {
		t.Fatalf("per-task review/retry line missing for t1: %s", text)
	}
	if !strings.Contains(text, "0 revue(s) enregistrée(s) pour cette tâche · 0 reprise(s) pour cette tâche") {
		t.Fatalf("per-task review/retry line missing for t2: %s", text)
	}
}
func TestRecoveryPreviewScopesAttemptAndPreservesState(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), true)
	before, _ := s.get(w.ID)
	old, _ := json.Marshal(before)
	p, e := s.recoveryPreview(w.ID, "t1", a.ID)
	if e != nil || p.Agent != a.ID || p.Task != "t1" || len(p.Criteria) != 1 || p.Correction != "exécuter" {
		t.Fatal(p, e)
	}
	if _, e = s.recoveryPreview(w.ID, "t2", a.ID); e == nil {
		t.Fatal("foreign attempt accepted")
	}
	after, _ := s.get(w.ID)
	new, _ := json.Marshal(after)
	if !bytes.Equal(old, new) {
		t.Fatal("preview mutated task")
	}
	var out bytes.Buffer
	if e = missionCLI(s, []string{"mission", "recovery", w.ID, "t1", a.ID}, "", false, &out); e != nil || !strings.Contains(out.String(), "Ce qui sera conservé") {
		t.Fatal(out.String(), e)
	}
}
func TestMissionChangeCategoryIgnoresProse(t *testing.T) {
	for _, kind := range []string{"checkpoint", "agent.activity", "unknown"} {
		if cat, _ := missionChangeCategory(kind, []byte(`{"message":"accepted blocked decision"}`)); cat != "" {
			t.Fatal("prose classified", cat)
		}
	}
	if cat, _ := missionChangeCategory("review.result", []byte(`{"state":"changes_requested"}`)); cat != "block" {
		t.Fatal(cat)
	}
}
