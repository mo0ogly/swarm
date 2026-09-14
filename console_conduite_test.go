//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestConduiteScreenShowsStateInboxAndPlan(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Workspace = s.root
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	a.Progress = AgentProgress{Action: "Lecture de fichier", Detail: "conductor.go", ToolCalls: 3}
	a.Status = "running"
	a.Heartbeat = now()
	if e = s.saveAgent(a); e != nil {
		t.Fatal(e)
	}
	c := &consoleState{conduite: true}
	frame := s.renderDashboard(w.ID, c, 100, 32, "")

	for _, want := range []string{"conduite", "créneau(x) occupé(s)", "Budget estimé", "À TRAITER", "PLAN ET AGENTS", "t1", "conductor.go", "3 appels"} {
		if !strings.Contains(frame, want) {
			t.Fatalf("écran de conduite sans %q :\n%s", want, frame)
		}
	}
	for _, unwanted := range []string{"SUIVI EN DIRECT", "Tab vers AGENTS"} {
		if strings.Contains(frame, unwanted) {
			t.Fatalf("l'écran de conduite ne montre pas les journaux bruts (%q)", unwanted)
		}
	}
}

func TestConduiteScreenSaysWhenNothingWaits(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	frame := s.renderDashboard(w.ID, &consoleState{conduite: true}, 100, 32, "")
	if !strings.Contains(frame, "Rien à traiter") {
		t.Fatalf("écran vide sans message d'orientation :\n%s", frame)
	}
}

func TestConsoleModeCommandTogglesScreens(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	c := &consoleState{}
	if _, e := s.consoleCommand(w.ID, "mode", c); e != nil {
		t.Fatal(e)
	}
	if !c.conduite || !strings.Contains(c.message, "conduite") {
		t.Fatalf("bascule en conduite non effectuée : %+v", c)
	}
	if _, e := s.consoleCommand(w.ID, "mode", c); e != nil {
		t.Fatal(e)
	}
	if c.conduite {
		t.Fatal("retour au mode expert non effectué")
	}
	frame := s.renderDashboard(w.ID, c, 100, 32, "")
	if !strings.Contains(frame, "SUIVI EN DIRECT") {
		t.Fatal("le mode expert conserve le tableau de bord complet")
	}
}

func TestConsoleAutonomyCommand(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	c := &consoleState{}
	if _, e := s.consoleCommand(w.ID, "autonomie assiste 4", c); e != nil {
		t.Fatal(e)
	}
	if s.autonomy(w.ID) != autonomyAssisted || s.slots(w.ID) != 4 {
		t.Fatalf("réglage non appliqué : %s / %d", s.autonomy(w.ID), s.slots(w.ID))
	}
	if _, e := s.consoleCommand(w.ID, "autonomie inconnu", c); e == nil {
		t.Fatal("un niveau inconnu doit être refusé")
	}
	if _, e := s.consoleCommand(w.ID, "autonomie", c); e != nil || !strings.Contains(c.message, "Assisté") {
		t.Fatalf("lecture du réglage : %q", c.message)
	}
}

func TestConduiteScreenFallsBackOnTinyTerminal(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	frame := s.renderDashboard(w.ID, &consoleState{conduite: true}, 40, 10, "")
	if !strings.Contains(frame, "terminal trop petit") {
		t.Fatalf("terminal minuscule : message attendu, obtenu :\n%s", frame)
	}
}

func TestConduiteScreenRespectsNoColor(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	t.Setenv("NO_COLOR", "1")
	frame := styleTerminalFrame(s.renderDashboard(w.ID, &consoleState{conduite: true}, 100, 32, ""))
	if strings.Contains(frame, "\x1b[") {
		t.Fatalf("NO_COLOR : aucune séquence de couleur attendue :\n%q", frame)
	}
	t.Setenv("NO_COLOR", "")
	t.Setenv("TERM", "dumb")
	frame = styleTerminalFrame(s.renderDashboard(w.ID, &consoleState{conduite: true}, 100, 32, ""))
	if strings.Contains(frame, "\x1b[") {
		t.Fatalf("TERM=dumb : aucune séquence de couleur attendue :\n%q", frame)
	}
}
