//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestStateContractUsesInvariantCodesAndSeparateLabels(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if got := s.validationState(&w); got.State != "open" || got.Label != "OUVERT" {
		t.Fatalf("open verdict: %+v", got)
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	w = gateTest(t, s, w, fixture(t, s.root))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
	fr := s.validationState(&w)
	if fr.State != "validated" || fr.Label != "VALIDÉ" {
		t.Fatalf("validated verdict: %+v", fr)
	}
	t.Setenv("SWARM_LANG", "en")
	en := s.validationState(&w)
	if en.State != fr.State || en.Label != "VALIDATED" {
		t.Fatalf("language changed business code: fr=%+v en=%+v", fr, en)
	}
	viewEN, err := s.view(w)
	if err != nil || !strings.Contains(viewEN, "VALIDATED") {
		t.Fatalf("English CLI label missing: %v\n%s", err, viewEN)
	}

	// The JSON CLI exposes the same derived verdict as the web snapshot.
	t.Setenv("SWARM_LANG", "fr")
	var stdout, stderr bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "work", "show", w.ID}, &stdout, &stderr); code != 0 {
		t.Fatalf("work show failed (%d): %s", code, stderr.String())
	}
	var cli struct {
		Validation WorkValidation `json:"validation"`
	}
	if err := json.Unmarshal(stdout.Bytes(), &cli); err != nil {
		t.Fatal(err)
	}
	web, err := s.cockpitSnapshot(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	webValidation, ok := web["validation"].(WorkValidation)
	if !ok || cli.Validation.State != webValidation.State || cli.Validation.Label != webValidation.Label {
		t.Fatalf("CLI/web verdict mismatch: cli=%+v web=%+v", cli.Validation, web["validation"])
	}

	// A changed artifact is stale even though the historic task says accepted.
	if err := os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600); err != nil {
		t.Fatal(err)
	}
	stale := s.validationState(&w)
	if stale.State != "blocked" || stale.Stale != 1 || stale.Tasks["t1"].State != "stale" {
		t.Fatalf("stale proof hidden: %+v", stale)
	}
}

func TestStateContractPartialVerdict(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "abandonnée", Deliverable: "aucun", Criteria: []string{"décision"}})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	w = gateTest(t, s, w, fixture(t, s.root))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t2", Status: "abandoned"})
	if got := s.validationState(&w); got.State != "partial" || got.Validated != 1 {
		t.Fatalf("partial verdict: %+v", got)
	}
}
