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

// Isolated engine fixture; no live database or real provider calls.
func TestMissionClosedHistoryAndCurrentBlockCLI(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("t1")}
	var err error
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	w = gateTest(t, s, w, fixture(t, s.root))
	organizedFixtureStore(t, s)
	w, err = s.mutate(w.ID, "test.accept", "accepted", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].Status = "accepted"
		w.Tasks[0].Attempts = []Attempt{{ID: "fixture-production", Status: "completed"}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	approveReportFixture(t, s, w.ID, "t1", "fixture-producer", "proof.txt")
	w, _ = s.get(w.ID)
	w, r = planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "close"}}
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "test.historical-failure", "old-timeout", w.Revision, []byte(`{}`), func(w *Work) error { w.Planning.Failure = "old timeout"; w.Planning.Paused = true; return nil })
	if err != nil {
		t.Fatal(err)
	}
	d, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if d.Validated != 1 || d.Understanding.Situation != "termine" || d.Guidance.Primary.Kind != "results" {
		t.Fatalf("closed mission presented as blocked: %+v", d)
	}
	for _, lang := range []string{"fr", "en"} {
		t.Setenv("SWARM_LANG", lang)
		var out bytes.Buffer
		if err = missionCLI(s, []string{"mission", "status", w.ID}, "", false, &out); err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(out.String(), uiText("Voir les résultats")) || strings.Contains(out.String(), "old timeout") {
			t.Fatalf("CLI active history: %s", out.String())
		}
		t.Logf("CLI isolated mission status language=%s actual_output=%s", lang, out.String())
	}
	var history bytes.Buffer
	if err = missionCLI(s, []string{"mission", "changes", w.ID}, "", true, &history); err != nil {
		t.Fatal(err)
	}
	current, _ := s.get(w.ID)
	if current.Planning.Failure != "old timeout" || current.Planning.Scopes[0].State != "closed" {
		t.Fatal("historical state mutated")
	}
	if dir := os.Getenv("SWARM_QW6_EVIDENCE"); dir != "" {
		if err = os.MkdirAll(dir, 0700); err != nil {
			t.Fatal(err)
		}
		raw, _ := json.MarshalIndent(map[string]any{"work": current, "mission": d}, "", "  ")
		if err = os.WriteFile(filepath.Join(dir, "closed-fixture.json"), raw, 0600); err != nil {
			t.Fatal(err)
		}
	}
	t.Log("Isolated fixture: historical failure=old timeout, scope closed, history unchanged; no real provider")
	// Proof drift must still re-open attention; an old planner error must not hide it.
	if err = os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600); err != nil {
		t.Fatal(err)
	}
	d, err = s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if d.Validated != 0 || d.Guidance.Primary.Kind == "results" && d.Guidance.Primary.Tone == "succes" || d.Understanding.Situation == "termine" {
		t.Fatalf("proof drift hidden: %+v", d)
	}
	t.Logf("After proof drift: validated=%d, situation=%s", d.Validated, d.Understanding.Situation)
	if current.Planning.Failure != "old timeout" {
		t.Fatal("history lost")
	}
}
