//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestTaskDialogDoesNotRequireAgentID(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	if c.dialog == nil || c.dialog.agent == nil || c.dialog.agent.ID != a.ID {
		t.Fatal("task did not resolve its agent")
	}
	c.dialog.row = 1
	s.dialogKey(w.ID, c, "enter")
	if c.dialog.mode != "actions" || !strings.Contains(c.dialog.message, "Agent actif") {
		t.Fatal("live retry enabled")
	}
	c.dialog.row = 2
	s.dialogKey(w.ID, c, "enter")
	desired, _ := s.desired(a.ID)
	if desired == "stop" {
		t.Fatal("stop before confirmation")
	}
	s.dialogKey(w.ID, c, "escape")
	desired, _ = s.desired(a.ID)
	if desired == "stop" {
		t.Fatal("cancel stopped agent")
	}
	s.openTaskDialog(w.ID, c)
	c.dialog.row = 2
	s.dialogKey(w.ID, c, "enter")
	s.dialogKey(w.ID, c, "enter")
	desired, _ = s.desired(a.ID)
	if desired != "stop" {
		t.Fatal("confirmation did not stop")
	}
}
func TestDialogProvidersLimitsAndBounds(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	if len(c.dialog.providers) != 1 || c.dialog.providers[0] != "fixture" {
		t.Fatal("hardcoded providers")
	}
	for _, size := range [][2]int{{80, 24}, {140, 40}, {64, 20}} {
		frame := s.renderDashboard(w.ID, c, size[0], size[1], "")
		lines := strings.Split(frame, "\r\n")
		if len(lines) > size[1] {
			t.Fatal("height overflow")
		}
		for _, line := range lines {
			if len([]rune(line)) > size[0] {
				t.Fatal("width overflow")
			}
		}
	}
	s.dialogKey(w.ID, c, "enter")
	if c.dialog.mode != "start" || c.dialog.workspace != s.root {
		t.Fatal("missing defaults")
	}
	s.dialogKey(w.ID, c, "tab")
	s.dialogKey(w.ID, c, "tab")
	s.dialogKey(w.ID, c, "clear")
	s.dialogKey(w.ID, c, "text:/missing/workspace")
	s.dialogKey(w.ID, c, "tab")
	s.dialogKey(w.ID, c, "tab")
	s.dialogKey(w.ID, c, "tab")
	s.dialogKey(w.ID, c, "tab")
	s.dialogKey(w.ID, c, "enter")
	if c.dialog == nil || c.dialog.message == "" {
		t.Fatal("invalid workspace not explained")
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 0 {
		t.Fatal("invalid form started process")
	}
}

func TestLaunchRefusalIsVisibleAndReopeningExplicit(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	if err := s.overrideReviewedTask(w.ID, "t1", "Décision de recette explicite"); err != nil {
		t.Fatal(err)
	}
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	c.dialog.row = 0
	s.dialogKey(w.ID, c, "enter")
	if c.dialog.row != 5 {
		t.Fatal("confirmation not initially selected")
	}
	s.dialogKey(w.ID, c, "enter")
	if c.dialog.mode != "launch-error" {
		t.Fatal("refusal not prominent")
	}
	for _, size := range [][2]int{{80, 24}, {167, 44}} {
		frame := s.renderDashboard(w.ID, c, size[0], size[1], "")
		for _, want := range []string{"LANCEMENT REFUSÉ", "déjà Dérogation", "Rouvrir"} {
			if !strings.Contains(frame, want) {
				t.Fatalf("hidden %q at %v", want, size)
			}
		}
	}
	var count int
	s.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='launch.refused'", w.ID).Scan(&count)
	if count != 1 {
		t.Fatal("refusal not recorded")
	}
	s.dialogKey(w.ID, c, "text:o")
	s.dialogKey(w.ID, c, "escape")
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "waived" {
		t.Fatal("reopen without confirmation")
	}
	s.openTaskDialog(w.ID, c)
	s.dialogKey(w.ID, c, "text:o")
	s.dialogKey(w.ID, c, "enter")
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "todo" {
		t.Fatal("reopen failed")
	}
}

func TestLaunchRefusalLinksToBlockingDependency(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Downstream", Deliverable: "report", Criteria: []string{"review"}, Depends: []string{"t1"}})
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	c.dialog.task = w.Tasks[1]
	c.dialog.row = 0
	s.dialogKey(w.ID, c, "enter")
	s.dialogKey(w.ID, c, "enter")
	if c.dialog.mode != "launch-error" || c.dialog.blockedTask != "t1" {
		t.Fatalf("bad refusal: %+v", c.dialog)
	}
	s.dialogKey(w.ID, c, "text:v")
	if c.dialog.mode != "review" || c.dialog.task.ID != "t1" || !strings.Contains(c.dialog.review, "aucune gate") {
		t.Fatal("cannot inspect dependency")
	}
}
