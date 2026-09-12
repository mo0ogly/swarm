//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestValidationDriftBlocksAckAndDependencies(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	w = gateTest(t, s, w, fixture(t, s.root))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
	if v := s.validationState(&w); v.State != "VALIDÉ" || v.Validated != 1 {
		t.Fatal(v)
	}
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "suite", Deliverable: "preuve", Criteria: []string{"ok"}, Depends: []string{"t1"}})
	if err := os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("modified"), 0600); err != nil {
		t.Fatal(err)
	}
	v := s.validationState(&w)
	if v.Validated != 0 || v.Historical != 1 || v.Stale != 1 || v.State != "BLOQUÉ" || !strings.Contains(strings.Join(v.Tasks["t1"].Blockers, " "), "proof.txt") {
		t.Fatal(v)
	}
	if err := s.apply(&w, "task.update", Request{ID: "t2", Status: "running"}); err == nil {
		t.Fatal("stale dependency launched")
	}
	ds, err := s.decisions(w.ID)
	if err != nil || len(ds) != 1 {
		t.Fatal(ds, err)
	}
	if err = s.resolveDecision(w.ID, ds[0].ID, "operator", "masquer cette alerte"); err == nil {
		t.Fatal("acknowledged active gate blocker")
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "todo"})
	if w.Tasks[0].Revalidation == nil {
		t.Fatal("missing revalidation guard")
	}
	raw := fixture(t, s.root)
	if err = s.applyGateDocument(&w, "t1", "delivery", "Gate de recette", raw); err == nil {
		t.Fatal("new gate bypasses report")
	}
	// Merely restoring a file does not close the task after explicit reopening.
	ds, err = s.decisions(w.ID)
	if err != nil || ds[0].ResolvedAt != "" {
		t.Fatal(ds, err)
	}
}

func TestHandoffClosesAfterFreshAcceptance(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	ds, err := s.decisions(w.ID)
	if err != nil || len(ds) != 1 || ds[0].ResolvedAt != "" {
		t.Fatal(ds, err)
	}
	w = gateTest(t, s, w, fixture(t, s.root))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
	ds, err = s.decisions(w.ID)
	if err != nil || ds[0].ResolvedAt == "" {
		t.Fatal("accepted handoff still pending", ds, err)
	}
	if err = os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600); err != nil {
		t.Fatal(err)
	}
	ds, err = s.decisions(w.ID)
	if err != nil || len(ds) != 2 || ds[1].ResolvedAt != "" {
		t.Fatal("drift hidden", ds, err)
	}
}
