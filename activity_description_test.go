//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestOperationDescriptions(t *testing.T) {
	for _, tc := range []struct {
		name         string
		input        any
		want, detail string
	}{
		{"Read", map[string]any{"file_path": "src/main.go"}, "Lecture", "src/main.go"},
		{"Bash", map[string]any{"description": "Mesurer la consommation CPU", "command": "python3 probe.py --seconds 6"}, "Annonce de l’agent : Mesurer", "probe.py"},
		{"command_execution", "go test ./...", "Exécution", "go test"},
		{"Bash", map[string]any{"command": "API_TOKEN=abc curl x"}, "Exécution", "masqué"},
	} {
		a, d := describeOperation(tc.name, tc.input)
		if !strings.Contains(a, tc.want) || !strings.Contains(d, tc.detail) {
			t.Fatalf("%s: %s / %s", tc.name, a, d)
		}
	}
	if strings.Contains(operationText("x\x1b[31my", 80), "\x1b") {
		t.Fatal("terminal escape retained")
	}
}
func TestDetailsRefreshWhileOpen(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	c.dialog.mode = "detail"
	a.Progress.Action = "Mesurer la consommation CPU"
	a.Progress.Detail = "python3 probe.py"
	if e = s.saveAgent(a); e != nil {
		t.Fatal(e)
	}
	if e = s.log(a.ID, "activity", "Mesure démarrée"); e != nil {
		t.Fatal(e)
	}
	s.renderDashboard(w.ID, c, 100, 32, "")
	if c.dialog.agent.Progress.Action != a.Progress.Action || len(c.dialog.history) == 0 {
		t.Fatal("detail view stayed stale")
	}
	// Scroll down to reach every line, as an operator does.
	seen := ""
	for i := 0; i < 35; i++ {
		c.dialog.row = i
		seen += s.renderDashboard(w.ID, c, 80, 24, "")
	}
	for _, want := range []string{"Mesurer la consommation CPU", "python3 probe.py", "HISTORIQUE", "Mesure démarrée"} {
		if !strings.Contains(seen, want) {
			t.Fatal("missing detail", want)
		}
	}
}

func TestOperatorPreview(t *testing.T) {
	dir := os.Getenv("SWARM_PREVIEW_DIR")
	if dir == "" {
		t.Skip("optional visual artifact")
	}
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	a.Status = "running"
	a.Heartbeat = now()
	a.Progress = AgentProgress{Action: "Annonce de l’agent : Mesurer la consommation CPU", Detail: "python3 probe.py --seconds 6", LastTool: "Bash", ToolCalls: 8, ToolResults: 7, PendingTools: 1, LastResult: now()}
	if e = s.saveAgent(a); e != nil {
		t.Fatal(e)
	}
	s.log(a.ID, "activity", "Read · Lecture d’un fichier · agents_process.go")
	s.log(a.ID, "activity", "Bash · Mesurer la consommation CPU")
	c := &consoleState{}
	os.MkdirAll(dir, 0700)
	os.WriteFile(filepath.Join(dir, "dashboard.txt"), []byte(s.renderDashboard(w.ID, c, 80, 24, "")), 0600)
	s.openTaskDialog(w.ID, c)
	c.dialog.mode = "detail"
	c.dialog.row = 5
	os.WriteFile(filepath.Join(dir, "details.txt"), []byte(s.renderDashboard(w.ID, c, 80, 24, "")), 0600)
}

func TestBlockedWorkExplainsStopBeforeOpening(t *testing.T) {
	w := Work{Title: "APEX", Tasks: []Task{{ID: "SC-01", Status: "blocked", Blocker: "Limite d'appels d'outils atteinte"}}}
	frame := renderWorkPicker([]Work{w}, 0, 80, 24)
	for _, want := range []string{"AUCUNE TÂCHE EN COURS", "BLOCAGE SC-01", "Limite d'appels"} {
		if !strings.Contains(frame, want) {
			t.Fatal(frame)
		}
	}
	a := Agent{Status: "interrupted", Activity: "Limite d'appels d'outils atteinte", Limits: RunLimits{MaxToolCalls: 100}, Progress: AgentProgress{ToolCalls: 100, ToolResults: 99}}
	d := &taskDialog{task: w.Tasks[0], agent: &a, mode: "actions"}
	frame = renderTaskDialog(strings.Repeat("\r\n", 24), d, 80, 24)
	for _, want := range []string{"MOTIF : Limite", "100 / 100", "Relancer cette tentative", "Échap fermer"} {
		if !strings.Contains(frame, want) {
			t.Fatal(frame)
		}
	}
}
