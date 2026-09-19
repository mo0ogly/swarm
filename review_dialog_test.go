//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestSubmissionAcceptanceDialogGateAndCancellation(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	dir := filepath.Join(s.root, "docs")
	os.MkdirAll(dir, 0700)
	report := "docs/t1-handoff with spaces.md"
	os.WriteFile(filepath.Join(s.root, report), []byte("review me"), 0600)
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	c.dialog.row = 7
	s.dialogKey(w.ID, c, "enter")
	if c.dialog.mode != "submit" || c.dialog.reportPath != report {
		t.Fatal("report not discoverable", c.dialog)
	}
	s.dialogKey(w.ID, c, "escape")
	current, _ := s.get(w.ID)
	if current.Tasks[0].Status != "todo" {
		t.Fatal("cancel mutated task")
	}
	s.openTaskDialog(w.ID, c)
	c.dialog.row = 7
	s.dialogKey(w.ID, c, "enter")
	s.dialogKey(w.ID, c, "enter")
	s.dialogKey(w.ID, c, "enter")
	current, _ = s.get(w.ID)
	if current.Tasks[0].Status != "submitted" {
		t.Fatal("submission missing", c.dialog)
	}
	s.openTaskDialog(w.ID, c)
	c.dialog.row = 9
	s.dialogKey(w.ID, c, "enter")
	c.dialog.row = 0
	s.dialogKey(w.ID, c, "enter")
	if c.dialog == nil || !strings.Contains(c.dialog.message, "gate") {
		t.Fatal("accepted without gate")
	}
	current, _ = s.get(w.ID)
	current = gateTest(t, s, current, fixture(t, s.root))
	// Tampering after gate evaluation must still prevent acceptance.
	os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600)
	s.dialogKey(w.ID, c, "enter")
	if c.dialog == nil {
		t.Fatal("accepted stale proof")
	}
	os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("proof"), 0600)
	s.dialogKey(w.ID, c, "enter")
	current, _ = s.get(w.ID)
	if c.dialog != nil || current.Tasks[0].Status != "accepted" {
		t.Fatal("valid reviewed task not accepted")
	}
}

func TestManualOverrideLifecycle(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	s.dialogKey(w.ID, c, "text:a")
	if c.dialog.row != 0 {
		t.Fatal("Enter must select confirmation")
	}
	s.dialogKey(w.ID, c, "enter")
	if c.dialog == nil || c.dialog.message == "" {
		t.Fatal("missing visible rejection")
	}
	s.dialogKey(w.ID, c, "text:f")
	s.dialogKey(w.ID, c, "enter")
	s.dialogKey(w.ID, c, "enter")
	if c.dialog == nil || !strings.Contains(c.dialog.message, "Motif") {
		t.Fatal("missing reason accepted")
	}
	s.dialogKey(w.ID, c, "escape")
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision {
		t.Fatal("cancel mutated state")
	}
	s.openTaskDialog(w.ID, c)
	s.dialogKey(w.ID, c, "text:f")
	s.dialogKey(w.ID, c, "text:Revue manuelle assumée pour cette livraison")
	s.dialogKey(w.ID, c, "enter")
	s.dialogKey(w.ID, c, "enter")
	if c.dialog != nil {
		t.Fatal(c.dialog.message)
	}
	current, _ = s.get(w.ID)
	task := &current.Tasks[0]
	if task.Status != "waived" || task.Override == nil || task.Override.Actor == "" || task.Override.At == "" || task.Override.PreviousStatus != "todo" || task.Gate != nil {
		t.Fatalf("bad override: %+v", task)
	}
	if !s.acceptedFresh(&current, task, map[string]bool{}) {
		t.Fatal("override does not unlock")
	}
	if state, _, _ := s.workStatus(current); state != "ACCEPTÉ AVEC DÉROGATION" {
		t.Fatal(state)
	}
	var events int
	if err := s.db.QueryRow("SELECT count(*) FROM events WHERE work_id=? AND kind='task.override'", w.ID).Scan(&events); err != nil || events != 1 {
		t.Fatal("audit missing", err, events)
	}
	if err := s.overrideReviewedTask(w.ID, "t1", "Nouvelle décision opérateur"); err == nil {
		t.Fatal("duplicate decision")
	}
	current = applyTest(t, s, current, "task.add", Request{ID: "t2", Title: "Downstream", Deliverable: "report", Criteria: []string{"review"}, Depends: []string{"t1"}})
	if err := s.apply(&current, "task.update", Request{ID: "t2", Status: "running"}); err != nil {
		t.Fatal("dependency still blocked", err)
	}
	if err := s.operatorTask(w.ID, Request{ID: "t1", Status: "todo"}); err != nil {
		t.Fatal(err)
	}
	current, _ = s.get(w.ID)
	if current.Tasks[0].Override != nil || s.acceptedFresh(&current, &current.Tasks[0], map[string]bool{}) {
		t.Fatal("reopen retained override")
	}
}

func TestOverridePreservesFailedGateAndBlocksLiveTask(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	if err := s.overrideReviewedTask(w.ID, "t1", "Acceptation manuelle justifiée"); err == nil {
		t.Fatal("live task overridden")
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	w = gateTest(t, s, w, fixture(t, s.root))
	os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600)
	if err := s.overrideReviewedTask(w.ID, "t1", "Preuve périmée examinée manuellement"); err != nil {
		t.Fatal(err)
	}
	current, _ := s.get(w.ID)
	if current.Tasks[0].Gate == nil || s.validGate(&current.Tasks[0]) {
		t.Fatal("gate fabricated")
	}
}

func TestOverrideCannotBypassUnacceptedDependency(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Dependent", Deliverable: "report", Criteria: []string{"review"}, Depends: []string{"t1"}})
	if err := s.overrideReviewedTask(w.ID, "t2", "Décision opérateur explicite"); err == nil || !strings.Contains(err.Error(), "t1") {
		t.Fatal("unaccepted dependency bypassed", err)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision {
		t.Fatal("refusal changed revision")
	}
}

func TestOverrideBlockedByPersistedAgent(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if _, _, err := s.prepare(w.ID, r); err != nil {
		t.Fatal(err)
	}
	// Even an inconsistent task row must not bypass the active agent check.
	w, _ = s.get(w.ID)
	if _, err := s.executeRequest(w.ID, "task.update", Request{Schema: 1, EventID: newID("test-"), Revision: w.Revision, ID: "t1", Status: "blocked", Blocker: "fixture inconsistency", Outcome: "interrupted"}); err == nil {
		t.Fatal("live task mutation permitted")
	}
	if err := s.overrideReviewedTask(w.ID, "t1", "Décision opérateur explicite"); err == nil {
		t.Fatal("active agent bypassed")
	}
}
