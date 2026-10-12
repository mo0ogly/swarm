//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func runSupervisionFixture(t *testing.T, child bool) (*Store, Work, Agent) {
	t.Helper()
	return runSupervisionFixtureInstruction(t, child, "GUARD_TOOL")
}

func runSupervisionFixtureInstruction(t *testing.T, child bool, instruction string) (*Store, Work, Agent) {
	t.Helper()
	s := storeTest(t)
	w := planningDo(t, s, createTest(t, s), "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15, Checks: map[string][]ValidationControl{"req-1": automaticPolicy("go", "version").Controls}})
	scope := "root"
	if child {
		var r PlanningRequest
		w, r = planningClaim(t, s, w, "root")
		r.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Responsable enfant", Requirements: []string{"req-1"}}}
		var err error
		w, err = s.planningChange(w.ID, "decide", r)
		if err != nil {
			t.Fatal(err)
		}
		scope = "child"
	}
	w, r := planningClaim(t, s, w, scope)
	r.Operations = []PlanningOperation{planningTask("first")}
	var err error
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	t.Setenv("SWARM_LOOP_FIXTURE", "1")
	raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: exe, Args: []string{"-test.run=^TestLoopProvider$"}, Env: []string{"SWARM_LOOP_FIXTURE"}, Limits: RunLimits{ToolSeconds: 1}}}})
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	w = managedReviewFixture(t, s, w)
	a, _, err := s.prepare(w.ID, Launch{Schema: 1, EventID: newID("run-"), Revision: w.Revision, TaskID: "first", Provider: "fixture", Role: "worker", Timeout: 1, Instruction: instruction})
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	return s, w, a
}

// Actual owned subprocess plus the public CLI decision boundary. A provider
// double is intentional; this does not claim an autonomous real-model decision.
func TestRunSupervisionOwnerDecisionStopsActiveProcess(t *testing.T) {
	for _, child := range []bool{false, true} {
		t.Run(map[bool]string{false: "root", true: "child"}[child], func(t *testing.T) {
			s, w, a := runSupervisionFixture(t, child)
			done := make(chan error, 1)
			go func() { done <- s.supervise(a.ID) }()
			defer func() {
				_ = s.stopAgent(a.ID)
				select {
				case err := <-done:
					if err != nil {
						t.Error(err)
					}
				case <-time.After(7 * time.Second):
					t.Error("worker not stopped")
				}
			}()
			a = waitAgent(t, s, a.ID, func(current Agent) bool {
				return current.Progress.TimeThresholdExceeded && current.Progress.ToolTimeThresholdExceeded || !activeAgent(current)
			})
			if a.Status != "running" || a.Ended != "" || a.Progress.ToolCalls != 1 || a.Progress.PendingTools != 1 {
				t.Fatalf("working process interrupted or observations lost: %+v", a)
			}
			w, _ = s.get(w.ID)
			scope := w.Tasks[0].ScopeID
			count := 0
			for _, e := range w.Planning.Inbox {
				if e.Kind == "run_observation" {
					count++
					if e.Scope != scope || e.Attempt != a.Attempt || e.RunObservation.Agent != a.ID || e.RunObservation.Progress.ToolCalls != 1 {
						t.Fatalf("wrong owner or evidence: %+v", e)
					}
				}
			}
			if count != 2 {
				t.Fatalf("elapsed and tool observations not routed once: %d", count)
			}
			revision := w.Revision
			if _, err := s.observeRun(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			if w.Revision != revision {
				t.Fatal("duplicate threshold produced new event")
			}
			context, err := planningContext(w, scope)
			if err != nil || !bytes.Contains(context, []byte(`"run_observation"`)) || !bytes.Contains(context, []byte(a.Attempt)) {
				t.Fatal("observation missing from planner context", err)
			}
			w, r := planningClaim(t, s, w, scope)
			r.Operations = []PlanningOperation{{Kind: "stop-run", ID: a.TaskID, Next: "Arrêter pour examiner l'outil actif, puis corriger avec ses preuves"}}
			raw, _ := json.Marshal(r)
			path := filepath.Join(t.TempDir(), "decision.json")
			if err = os.WriteFile(path, raw, 0600); err != nil {
				t.Fatal(err)
			}
			var out bytes.Buffer
			if err = s.planningCLI([]string{"planning", "decide", w.ID}, path, &out); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			if w.Tasks[0].Status != "running" {
				t.Fatal("decision fabricated completion")
			}
			other, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer other.db.Close()
			reason, err := other.observeRun(a)
			if err != nil || !strings.Contains(reason, "Arrêt décidé") {
				t.Fatal("persisted decision not executable", reason, err)
			}
			replay, err := other.planningChange(w.ID, "decide", r)
			if err != nil || replay.Revision != w.Revision {
				t.Fatal("decision replay", err)
			}
			a = waitAgent(t, s, a.ID, func(current Agent) bool { return !activeAgent(current) })
			if a.Status != "interrupted" || a.StopKind != "planning" || a.ExitCode == nil || a.Ended == "" {
				t.Fatalf("owner stop not confirmed: %+v", a)
			}
		})
	}
}

func TestRunSupervisionContinueAndInvalidDecisions(t *testing.T) {
	s, w, a := runSupervisionFixture(t, true)
	// Isolated process-state double for transactional rejection/rollback cases.
	a.Status = "running"
	a.Progress = AgentProgress{ElapsedSeconds: 2, TimeThresholdExceeded: true}
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if _, err := s.observeRun(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	w, r := planningClaim(t, s, w, "child")
	r.Operations = []PlanningOperation{{Kind: "continue-run", ID: a.TaskID, Next: "Observation de durée seule : continuer dans les droits et budgets actuels"}}
	bad := r
	bad.Scope = "root"
	if _, err := s.planningChange(w.ID, "decide", bad); err == nil {
		t.Fatal("foreign owner accepted")
	}
	bad = r
	bad.Inputs = []string{"unreceived"}
	if _, err := s.planningChange(w.ID, "decide", bad); err == nil {
		t.Fatal("unreceived observation accepted")
	}
	before := w.Revision
	a.Status = "completed"
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if _, err := s.planningChange(w.ID, "decide", r); err == nil {
		t.Fatal("completed process decision accepted")
	}
	got, _ := s.get(w.ID)
	if got.Revision != before {
		t.Fatal("failed decision partially applied")
	}
	a.Status = "running"
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	w, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	if reason, err := s.observeRun(a); err != nil || reason != "" {
		t.Fatal("continue interpreted as a stop", reason, err)
	}
	if w.Tasks[0].Status != "running" || len(w.Tasks[0].Attempts) != 1 {
		t.Fatal("continue changed task/attempt")
	}
	// Stale-attempt observations must never stop a replacement attempt.
	current := a
	current.Attempt = "replacement-attempt"
	if reason, err := s.observeRun(current); err != nil || reason != "" {
		t.Fatal("stale decision controlled replacement", reason, err)
	}
}

func TestRunSupervisionPersistedStopWinsNaturalExit(t *testing.T) {
	s, w, a := runSupervisionFixtureInstruction(t, false, "R3_WAIT_RELEASE")
	done := make(chan error, 1)
	go func() { done <- s.supervise(a.ID) }()
	defer func() {
		_ = s.stopAgent(a.ID)
		select {
		case err := <-done:
			if err != nil {
				t.Error(err)
			}
		case <-time.After(7 * time.Second):
			t.Error("worker not stopped")
		}
	}()
	a = waitAgent(t, s, a.ID, func(current Agent) bool { return current.Progress.TimeThresholdExceeded || !activeAgent(current) })
	if a.Status != "running" {
		t.Fatal("worker ended before decision", a.Status)
	}
	w, _ = s.get(w.ID)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "stop-run", ID: a.TaskID, Next: "Arrêt de conduite explicitement décidé avant remise"}}
	if _, err := s.planningChange(w.ID, "decide", r); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(a.CWD, "r3-release"), []byte("release"), 0600); err != nil {
		t.Fatal(err)
	}
	a = waitAgent(t, s, a.ID, func(current Agent) bool { return !activeAgent(current) })
	if a.StopKind != "planning" || a.Status != "interrupted" || a.ExitCode == nil || a.Ended == "" {
		t.Fatal("persisted owner stop lost on process exit", a.Status, a.StopKind)
	}
}
