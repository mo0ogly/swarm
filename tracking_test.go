//go:build linux

package main

import (
	"fmt"
	"path/filepath"
	"strings"
	"testing"
)

func TestDecisionDedupResolveAndArchive(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	one, e := s.decisions(w.ID)
	if e != nil || len(one) != 1 {
		t.Fatal(one, e)
	}
	two, e := s.decisions(w.ID)
	if e != nil || len(two) != 1 || two[0].ID != one[0].ID {
		t.Fatal("duplicate", e)
	}
	if e = s.resolveDecision(w.ID, one[0].ID, "reviewer", "Rapport lu, preuves à contrôler"); e != nil {
		t.Fatal(e)
	}
	current, _ := s.get(w.ID)
	if current.Tasks[0].Status != "submitted" {
		t.Fatal("acknowledgement accepted task")
	}
	if e = s.markVisit(w.ID, "operator", w.Revision); e != nil {
		t.Fatal(e)
	}
	zip := filepath.Join(t.TempDir(), "archive.zip")
	if e = s.export(w.ID, zip); e != nil {
		t.Fatal(e)
	}
	dest := storeTest(t)
	if _, e = dest.importBundle(zip); e != nil {
		t.Fatal(e)
	}
	got, e := dest.decisions(w.ID)
	if e != nil || len(got) != 1 || got[0].Author != "reviewer" || got[0].ResolvedAt == "" {
		t.Fatal(got, e)
	}
	v, e := dest.visit(w.ID, "operator")
	if e != nil || v.Revision != w.Revision {
		t.Fatal(v, e)
	}
}
func TestLogCursorLiteralSearchAndBounds(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	for i := 0; i < 31; i++ {
		if e = s.log(a.ID, "test", fmt.Sprintf("trace %% %02d", i)); e != nil {
			t.Fatal(e)
		}
	}
	p, e := s.queryLogs(w.ID, a.ID, "trace %", "", 0, 7)
	if e != nil || len(p.Entries) != 7 || !p.More {
		t.Fatal(p, e)
	}
	ids := map[int64]bool{}
	for {
		for _, l := range p.Entries {
			if ids[l.Seq] {
				t.Fatal("duplicate cursor")
			}
			ids[l.Seq] = true
		}
		if !p.More {
			break
		}
		p, e = s.queryLogs(w.ID, a.ID, "trace %", "", p.Next, 7)
		if e != nil {
			t.Fatal(e)
		}
	}
	if len(ids) != 31 {
		t.Fatal("lost records", len(ids))
	}
	p, e = s.queryLogs(w.ID, a.ID, "not found", "", 0, 7)
	if e != nil || len(p.Entries) != 0 {
		t.Fatal(p, e)
	}
}
func TestHierarchyRoleAndOODARestart(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Role = "planner"
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	tree := s.hierarchyText(w.ID)
	for _, x := range []string{w.Title, "t1", a.ID, "planner", "processus"} {
		if !strings.Contains(tree, x) {
			t.Fatal("missing", x)
		}
	}
	w, _ = s.get(w.ID)
	w, e = s.executeRequest(w.ID, "ooda", Request{Schema: 1, EventID: newID("test-"), Revision: w.Revision, Observation: "Échec constaté", Orientation: "Corriger la cause", Decision: "Revoir le test", Result: "Diagnostic enregistré", Next: "Rejouer le test corrigé", Owner: "reviewer"})
	if e != nil {
		t.Fatal(e)
	}
	if w.Summary != "Diagnostic enregistré" || !strings.Contains(s.resumeSinceText(w.ID, Visit{Operator: "test"}), "Revoir le test") {
		t.Fatal("OODA missing")
	}
}
