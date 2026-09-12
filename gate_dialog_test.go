//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestGatePreviewNoMutationCancelAndFreshness(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	raw := fixture(t, s.root)
	os.Mkdir(filepath.Join(s.root, "docs"), 0700)
	os.WriteFile(filepath.Join(s.root, "docs/t1.evidence.json"), raw, 0600)
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	s.dialogKey(w.ID, c, "text:g")
	s.dialogKey(w.ID, c, "enter")
	s.dialogKey(w.ID, c, "enter")
	if c.dialog.mode != "gate-confirm" || !strings.Contains(c.dialog.review, "PASS") {
		t.Fatal(c.dialog)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision || current.Tasks[0].Gate != nil {
		t.Fatal("preview mutated state")
	}
	s.dialogKey(w.ID, c, "down")
	s.dialogKey(w.ID, c, "enter")
	if c.dialog.mode != "gate-load" {
		t.Fatal("cannot cancel")
	}
	s.dialogKey(w.ID, c, "enter")
	s.dialogKey(w.ID, c, "enter")
	os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600)
	s.dialogKey(w.ID, c, "enter")
	current, _ = s.get(w.ID)
	if s.validGate(&current.Tasks[0]) || current.Tasks[0].Gate == nil && c.dialog.message == "" {
		t.Fatal("changed proof accepted after preview")
	}
	if e := s.acceptReviewedTask(w.ID, "t1"); e == nil {
		t.Fatal("failed gate accepted")
	}
}
func TestGatePreviewRevisionConflictVisible(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	raw := fixture(t, s.root)
	os.WriteFile(filepath.Join(s.root, "gate.json"), raw, 0600)
	d := &taskDialog{task: w.Tasks[0], reportPath: "gate.json"}
	if e := s.previewGate(w.ID, d); e != nil {
		t.Fatal(e)
	}
	w = applyTest(t, s, w, "checkpoint", Request{Summary: "Concurrent change"})
	e := s.recordDialogGate(w.ID, d)
	if e == nil || commandFailure(e).Code != "revision_conflict" {
		t.Fatal(e)
	}
}
