//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestBudgetReservationReplayAndExhaustion(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Second", Deliverable: "report", Criteria: []string{"review"}})
	for _, d := range []string{"a", "b"} {
		os.Mkdir(filepath.Join(s.root, d), 0700)
	}
	if e := s.setBudget(w.ID, Budget{Limit: 1, Reserve: 1, Source: "estimation de recette, pas un tarif fournisseur", PriceDate: "2026-09-12"}); e != nil {
		t.Fatal(e)
	}
	r.Revision = w.Revision
	r.Workspace = "a"
	a, created, e := s.prepare(w.ID, r)
	if e != nil || !created {
		t.Fatal(e)
	}
	if _, created, e = s.prepare(w.ID, r); e != nil || created {
		t.Fatal("duplicate reservation", e)
	}
	current, _ := s.get(w.ID)
	second := r
	second.EventID = newID("test-")
	second.TaskID = "t2"
	second.Workspace = "b"
	second.Revision = current.Revision
	if _, _, e = s.prepare(w.ID, second); e == nil || commandFailure(e).Code != "budget_exhausted" {
		t.Fatal("budget not enforced", e)
	}
	b, e := s.budget(w.ID)
	if e != nil || b.Reserved != 1 || !b.Warning || b.ActualCost != nil {
		t.Fatal(b, e)
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 1 {
		t.Fatal("orphan intent")
	}
	if e = s.finishAgent(a, "interrupted", "Annulé avant création du processus", nil); e != nil {
		t.Fatal(e)
	}
	b, _ = s.budget(w.ID)
	if b.Reserved != 0 || b.Estimated != 0 {
		t.Fatal("unstarted reservation not released", b)
	}
}
func TestUsageIsExplicitAndNotInvented(t *testing.T) {
	var d map[string]any
	json.Unmarshal([]byte(`{"type":"turn.completed","usage":{"input_tokens":42,"output_tokens":7}}`), &d)
	u := providerUsage(d)
	if u == nil || u.Input != 42 || u.Output != 7 || u.Scope == "" || u.Source == "" || u.At == "" {
		t.Fatal(u)
	}
	if providerUsage(map[string]any{"type": "result", "usage": map[string]any{"input_tokens": -1.0, "output_tokens": 7.0}}) != nil {
		t.Fatal("negative tokens")
	}
	if providerUsage(map[string]any{"type": "result"}) != nil {
		t.Fatal("invented missing usage")
	}
}

// Le budget affichait des réservations forfaitaires et laissait le coût réel à
// nil, alors que les fournisseurs le rapportent par tentative.
func TestBudgetExposesReportedCostOnlyWhenKnown(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	v, e := s.budget(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if v.ActualCost != nil {
		t.Fatalf("aucune tentative n'a rapporté : le coût réel doit rester inconnu, obtenu %v", *v.ActualCost)
	}

	cout := 2.50
	a.Usage = &Usage{ReportedCost: &cout, Source: "test"}
	if e = s.saveAgent(a); e != nil {
		t.Fatal(e)
	}
	if v, e = s.budget(w.ID); e != nil {
		t.Fatal(e)
	}
	if v.ActualCost == nil || *v.ActualCost != 2.50 {
		t.Fatalf("le coût rapporté doit remonter au budget : %+v", v.ActualCost)
	}
}
