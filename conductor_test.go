//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func conductorAgent(t *testing.T, s *Store) (Work, Agent) {
	t.Helper()
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	return w, a
}

func writeReport(t *testing.T, s *Store, name, body string) string {
	t.Helper()
	dir := filepath.Join(s.root, "docs")
	if e := os.MkdirAll(dir, 0700); e != nil {
		t.Fatal(e)
	}
	p := filepath.Join(dir, name)
	if e := os.WriteFile(p, []byte(body), 0600); e != nil {
		t.Fatal(e)
	}
	return p
}

func finishCompleted(t *testing.T, s *Store, a Agent) {
	t.Helper()
	code := 0
	if e := s.finishAgent(a, "completed", "Tentative terminée", &code); e != nil {
		t.Fatal(e)
	}
}

func taskStatus(t *testing.T, s *Store, work string) Task {
	t.Helper()
	w, e := s.get(work)
	if e != nil {
		t.Fatal(e)
	}
	return w.Tasks[0]
}

func agentJournal(t *testing.T, s *Store, id string) string {
	t.Helper()
	logs, e := s.logs(id, 0)
	if e != nil {
		t.Fatal(e)
	}
	lines := []string{}
	for _, l := range logs {
		lines = append(lines, l.Message)
	}
	return strings.Join(lines, "\n")
}

func TestConductorRelaysProvenHandoff(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)
	writeReport(t, s, "t1.md", "Observations, commandes, résultats, prochaine action.")
	finishCompleted(t, s, a)

	task := taskStatus(t, s, w.ID)
	if task.Status != "submitted" {
		t.Fatalf("handoff prouvé non relayé : statut %q", task.Status)
	}
	if !strings.Contains(task.Next, "docs/t1.md") {
		t.Fatalf("chemin du rapport absent de la prochaine action : %q", task.Next)
	}
	if journal := agentJournal(t, s, a.ID); !strings.Contains(journal, conductorAuthor) {
		t.Fatalf("relais non journalisé : %s", journal)
	}
}

func TestConductorRelayIsNotAcceptance(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)
	writeReport(t, s, "t1.md", "handoff factuel")
	finishCompleted(t, s, a)

	task := taskStatus(t, s, w.ID)
	if task.Status == "accepted" || task.Status == "waived" {
		t.Fatalf("le conducteur ne doit jamais accepter : %q", task.Status)
	}
	if task.Gate != nil {
		t.Fatal("aucune gate ne doit être enregistrée par le conducteur")
	}
}

func TestConductorKeepsBlockedWithoutProof(t *testing.T) {
	cases := map[string]func(*testing.T, *Store){
		"rapport absent": func(*testing.T, *Store) {},
		"rapport vide": func(t *testing.T, s *Store) {
			writeReport(t, s, "t1.md", "")
		},
	}
	for name, prepare := range cases {
		t.Run(name, func(t *testing.T) {
			s := storeTest(t)
			w, a := conductorAgent(t, s)
			prepare(t, s)
			finishCompleted(t, s, a)

			task := taskStatus(t, s, w.ID)
			if task.Status != "blocked" {
				t.Fatalf("sans preuve lisible la tâche reste bloquée : %q", task.Status)
			}
			if task.Blocker == "" {
				t.Fatal("motif de blocage obligatoire")
			}
		})
	}
}

func TestConductorIgnoresUnsuccessfulAttempt(t *testing.T) {
	for _, state := range []string{"failed", "interrupted"} {
		t.Run(state, func(t *testing.T) {
			s := storeTest(t)
			w, a := conductorAgent(t, s)
			writeReport(t, s, "t1.md", "rapport présent malgré l'échec")
			code := 7
			if e := s.finishAgent(a, state, "Tentative "+state, &code); e != nil {
				t.Fatal(e)
			}
			if task := taskStatus(t, s, w.ID); task.Status != "blocked" {
				t.Fatalf("une tentative %s ne se relaie pas : %q", state, task.Status)
			}
		})
	}
}

func TestConductorRefusesReportOlderThanAttempt(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	p := writeReport(t, s, "t1.md", "rapport d'une tentative précédente")
	old := time.Now().Add(-2 * time.Hour)
	if e := os.Chtimes(p, old, old); e != nil {
		t.Fatal(e)
	}
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	finishCompleted(t, s, a)

	if task := taskStatus(t, s, w.ID); task.Status != "blocked" {
		t.Fatalf("un rapport antérieur à la tentative ne prouve rien : %q", task.Status)
	}
}

func TestConductorRefusesAmbiguousReports(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)
	writeReport(t, s, "t1.md", "premier rapport")
	writeReport(t, s, "t1-handoff-2026.md", "second rapport")
	finishCompleted(t, s, a)

	task := taskStatus(t, s, w.ID)
	if task.Status != "blocked" {
		t.Fatalf("plusieurs rapports candidats : relais interdit, statut %q", task.Status)
	}
	if journal := agentJournal(t, s, a.ID); !strings.Contains(journal, "plusieurs rapports") {
		t.Fatalf("motif d'ambiguïté non journalisé : %s", journal)
	}
}

func TestConductorReplayHasNoDoubleEffect(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)
	writeReport(t, s, "t1.md", "handoff unique")
	finishCompleted(t, s, a)
	first := taskStatus(t, s, w.ID)

	stored, e := s.agent(a.ID)
	if e != nil {
		t.Fatal(e)
	}
	finishCompleted(t, s, stored)

	second := taskStatus(t, s, w.ID)
	if second.Status != "submitted" {
		t.Fatalf("statut altéré par le rejeu : %q", second.Status)
	}
	if len(second.Attempts) != len(first.Attempts) {
		t.Fatalf("tentatives dupliquées : %d puis %d", len(first.Attempts), len(second.Attempts))
	}
	if n := strings.Count(agentJournal(t, s, a.ID), "Relais automatique"); n != 1 {
		t.Fatalf("relais journalisé %d fois", n)
	}
}

func TestConductorSkipsBrainstormTask(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	req := Request{Schema: 1, EventID: newID("e-"), Revision: w.Revision, ID: "t1"}
	raw, _ := json.Marshal(req)
	w, e := s.mutate(w.ID, "task.update", req.EventID, req.Revision, raw, func(w *Work) error {
		w.Tasks[0].Brainstorm = true
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	r.Revision = w.Revision
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	writeReport(t, s, "t1.md", "réponse de dialogue")
	finishCompleted(t, s, a)

	if task := taskStatus(t, s, w.ID); task.Status != "blocked" {
		t.Fatalf("un dialogue ne se soumet pas comme un livrable : %q", task.Status)
	}
}

func TestExecutionDirectivesNameTheRelayedReport(t *testing.T) {
	d := executionDirectives("/projet", "/projet/ws", "t1", RunLimits{})
	if !strings.Contains(d, "docs/t1.md") {
		t.Fatalf("le chemin de handoff relayé doit être explicite : %s", d)
	}
}

func TestConductorRefusalRaisesDecision(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)
	finishCompleted(t, s, a)

	if task := taskStatus(t, s, w.ID); task.Status != "blocked" {
		t.Fatalf("préalable : la tâche doit rester bloquée, statut %q", task.Status)
	}
	decisions, e := s.decisions(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	for _, d := range decisions {
		if d.TaskID == "t1" && d.AgentID == a.ID && d.ResolvedAt == "" {
			return
		}
	}
	t.Fatalf("relais refusé sans entrée à traiter : %+v", decisions)
}

func TestConductorLeavesSettledTaskAlone(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)
	writeReport(t, s, "t1.md", "handoff")
	finishCompleted(t, s, a)
	before := taskStatus(t, s, w.ID)

	// Tâche déjà soumise : un relais supplémentaire ne doit rien changer.
	relayed, reason := s.relayHandoff(a, "completed")
	if relayed != "" || reason != "" {
		t.Fatalf("relais sur tâche déjà réglée : %q / %q", relayed, reason)
	}
	after := taskStatus(t, s, w.ID)
	if after.Status != before.Status || after.Next != before.Next {
		t.Fatalf("état modifié : %+v puis %+v", before, after)
	}
}

func TestProvenReportToleratesClockSkew(t *testing.T) {
	s := storeTest(t)
	p := writeReport(t, s, "t1.md", "handoff")
	started := time.Now().UTC()

	// Fichier écrit une seconde « avant » le départ déclaré : dérive d'horloge,
	// pas une preuve périmée.
	skewed := started.Add(-1 * time.Second)
	if e := os.Chtimes(p, skewed, skewed); e != nil {
		t.Fatal(e)
	}
	if got, reason := s.provenReport("t1", started.Format(time.RFC3339Nano)); got != "docs/t1.md" {
		t.Fatalf("dérive d'une seconde refusée : %q / %q", got, reason)
	}

	// Au-delà de la marge, le rapport appartient au passé.
	old := started.Add(-30 * time.Second)
	if e := os.Chtimes(p, old, old); e != nil {
		t.Fatal(e)
	}
	if got, reason := s.provenReport("t1", started.Format(time.RFC3339Nano)); got != "" || reason == "" {
		t.Fatalf("rapport antérieur accepté : %q / %q", got, reason)
	}
}
