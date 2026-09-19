//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

func TestPilotageDependenciesAndHealth(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w.Tasks = append(w.Tasks, Task{ID: "t2", Title: "Suite", Status: "todo", Depends: []string{"t1"}})
	p := s.pilotage(&w, nil, s.validationState(&w))
	edges := p["edges"].([]map[string]any)
	if len(edges) != 1 || edges[0]["satisfied_now"] != false {
		t.Fatal(edges)
	}
	w.Tasks[0].Status = "waived"
	w.Tasks[0].Override = &ManualOverride{Reason: "Décision explicite"}
	p = s.pilotage(&w, nil, s.validationState(&w))
	if p["edges"].([]map[string]any)[0]["satisfied_now"] != true {
		t.Fatal(p)
	}
	a := Agent{ID: "session", Attempt: "attempt", TaskID: "t1", Status: "running", Host: "other-host"}
	h := pilotAgentHealth(a, "", now())
	if h["process_state"] != "unknown/other-host" || h["activity_state"] != "unknown" {
		t.Fatal(h)
	}
	if h["agent_id"] == h["task_attempt_id"] {
		t.Fatal("identités confondues")
	}
}

func TestPilotageLaunchPreviewNoSideEffects(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	before := w.Revision
	r.EventID = newID("preview-")
	if _, _, e := s.prepareLaunch(w.ID, r, true); e != nil {
		t.Fatal(e)
	}
	current, e := s.get(w.ID)
	if e != nil || current.Revision != before {
		t.Fatal(current, e)
	}
	as, _ := s.agents(w.ID)
	if len(as) != 0 {
		t.Fatal(as)
	}
	var count int
	if e = s.db.QueryRow("SELECT count(*) FROM reservations WHERE work_id=?", w.ID).Scan(&count); e != nil || count != 0 {
		t.Fatal(count, e)
	}
	if e = s.setBudget(w.ID, Budget{Limit: 1, Reserve: 2, Source: "fixture", PriceDate: "2026-09-15"}); e != nil {
		t.Fatal(e)
	}
	if _, _, e = s.prepareLaunch(w.ID, r, true); e == nil {
		t.Fatal("aperçu doit refuser le budget comme le lancement")
	}
}

func TestPilotageReadinessAllowsDisjointWorkspaces(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	dir := filepath.Join(s.root, "one")
	if e := os.Mkdir(dir, 0700); e != nil {
		t.Fatal(e)
	}
	r.Workspace = dir
	if _, _, e := s.prepare(w.ID, r); e != nil {
		t.Fatal(e)
	}
	current, _ := s.get(w.ID)
	task := Task{ID: "next", Status: "todo"}
	if !s.assistCanStart(&current, &task) {
		t.Fatal("un autre agent n'interdit pas de préparer un départ disjoint")
	}
}

func TestPilotageEdgesInvalidateChangedEvidence(t *testing.T) {
	s := storeTest(t)
	raw := fixture(t, s.root)
	ev, err := evaluate(raw, s.root, "delivery")
	if err != nil {
		t.Fatal(err)
	}
	w := Work{Tasks: []Task{
		{ID: "t1", Status: "accepted", Gate: &GateRecord{Document: raw, Evaluation: ev}},
		{ID: "shared", Status: "waived", Override: &ManualOverride{Reason: "Explicit waiver"}, Depends: []string{"t1"}},
		{ID: "next", Status: "todo", Depends: []string{"shared"}},
	}}
	edges := s.readScope().pilotage(&w, nil, s.validationState(&w))["edges"].([]map[string]any)
	for _, edge := range edges {
		if edge["satisfied_now"] != true {
			t.Fatal(edges)
		}
	}
	if err := os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600); err != nil {
		t.Fatal(err)
	}
	edges = s.readScope().pilotage(&w, nil, s.validationState(&w))["edges"].([]map[string]any)
	for _, edge := range edges {
		if edge["satisfied_now"] != false {
			t.Fatal("stale transitive dependency shown satisfied", edges)
		}
	}
}

func TestPilotageKeepsOldActiveAttemptsBeyondHistory(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	for i := 0; i < 205; i++ {
		a := Agent{ID: fmt.Sprintf("session-%03d", i), WorkID: w.ID, TaskID: fmt.Sprintf("t%d", i), Status: "completed"}
		if i == 0 {
			a.Status = "running"
			a.Host = "other-host"
		}
		raw, _ := json.Marshal(a)
		if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, a.ID, a.Status, raw, []byte("{}")); err != nil {
			t.Fatal(err)
		}
	}
	agents, err := s.pilotAgents(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	// Each archived session belongs to a different task, so each is also that
	// task's latest retry candidate, in addition to the old active session.
	if len(agents) != 205 || agents[len(agents)-1].ID != "session-000" {
		t.Fatal("old active attempt lost", len(agents))
	}
	summary := s.pilotage(&w, agents, s.validationState(&w))["summary"].(map[string]any)
	if summary["total"] != 205 || summary["unknown"] != 1 || summary["occupied"] != 1 {
		t.Fatal(summary)
	}
}

func TestPilotageFinishedActivityDoesNotExpire(t *testing.T) {
	for _, state := range []string{"completed", "failed", "interrupted"} {
		a := Agent{ID: "ended", Status: state}
		a.Progress.LastResult = "2026-09-01T10:00:00Z"
		h := pilotAgentHealth(a, "", "2026-09-15T10:00:00Z")
		if h["activity_state"] != "recorded" || h["activity_label"] != "Résultat public reçu — tentative terminée" {
			t.Fatal(state, h)
		}
		a.Progress.LastResult = ""
		h = pilotAgentHealth(a, "", "2026-09-15T10:00:00Z")
		if h["activity_state"] != "unknown" || h["activity_label"] != "Tentative terminée — aucun résultat public enregistré" {
			t.Fatal(state, h)
		}
		a.Progress.Degraded = "journal incomplet"
		h = pilotAgentHealth(a, "", "2026-09-15T10:00:00Z")
		if h["activity_label"] != "Flux d'activité incomplet : journal incomplet" {
			t.Fatal(state, h)
		}
	}
	a := Agent{ID: "unconfirmed", Status: "running", Host: "other-host"}
	a.Progress.LastResult = "2026-09-01T10:00:00Z"
	h := pilotAgentHealth(a, "", "2026-09-15T10:00:00Z")
	if h["activity_state"] != "old" {
		t.Fatal("Le suivi du silence doit rester actif tant que la fin est inconnue", h)
	}
}
