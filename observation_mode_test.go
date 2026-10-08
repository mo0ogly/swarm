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

func TestObservationCountsWithoutStopping(t *testing.T) {
	l, _ := (RunLimits{ObservationMode: 1, MaxToolCalls: 1, MaxRepeatedCalls: 1, MaxConsecutiveErrors: 1, ToolSeconds: 1, SilenceSeconds: 1}).normalized()
	g := newLoopGuard(l)
	at := time.Now()
	for _, id := range []string{"a", "b", "c"} {
		g.call(id, "Bash", map[string]any{"command": "same"}, at)
		g.result(id, true)
	}
	g.call("pending", "Read", nil, at)
	if g.check(at.Add(time.Hour)) != "" || g.reason != "" || g.calls != 4 || g.completed != 3 || len(g.pending) != 1 {
		t.Fatalf("observation lost data or stopped: %+v", g)
	}
	if strings.Contains(executionDirectives("", "/tmp", "t1", l), "Budget superviseur") {
		t.Fatal("prompt still promises active ceilings")
	}
	// Ordinary execution remains bounded.
	l.ObservationMode = 0
	g = newLoopGuard(l)
	g.call("one", "Read", nil, at)
	g.result("one", false)
	if g.check(at) == "" {
		t.Fatal("default limits disabled")
	}
}

func TestObservationAuthorizesRetryWithoutResetAndCanBeDisabled(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", MaxAttempts: 1})
	r.Revision = w.Revision
	first, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.finishAgent(first, "interrupted", "Limite d'appels d'outils atteinte", nil); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	r.Revision = w.Revision
	r.Previous = first.ID
	r.EventID = newID("retry-")
	if _, _, err = s.prepareLaunch(w.ID, r, true); err == nil {
		t.Fatal("ordinary exhausted attempt allowed")
	}
	change := RunLimitsConfigChange{Schema: 1, EventID: "observe", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{ObservationMode: 1}, Reason: "operator requests measurement without caps"}
	if _, err = s.configureRunLimits(change); err != nil {
		t.Fatal(err)
	}
	if !s.assistCanStart(&w, &w.Tasks[0]) {
		t.Fatal("UI still blocks on count")
	}
	next, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if !next.Limits.observing() || next.Previous != first.ID {
		t.Fatal("retry lost mode or identity")
	}
	if !strings.Contains(next.Prompt, "MODE OBSERVATION") {
		t.Fatal("mode missing in actual prompt")
	}
	var count int
	s.db.QueryRow("SELECT count(*) FROM agents WHERE task_id='t1'").Scan(&count)
	if count != 2 {
		t.Fatalf("history reset: %d", count)
	}
	change.EventID = "bounded"
	change.Revision = 1
	change.Values = RunLimits{}
	change.Reason = "end experiment"
	if _, err = s.configureRunLimits(change); err != nil {
		t.Fatal(err)
	}
	persisted, _ := s.agent(next.ID)
	if !persisted.Limits.observing() {
		t.Fatal("changed active attempt")
	}
	if err = s.finishAgent(next, "interrupted", "operator stop", nil); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	r.Revision = w.Revision
	r.Previous = next.ID
	r.EventID = newID("retry-")
	if _, _, err = s.prepareLaunch(w.ID, r, true); err == nil {
		t.Fatal("restored attempt cap ignored")
	}
}

func TestObservationSupervisorDoesNotCutAtToolOrExecutionDeadline(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	exe, _ := os.Executable()
	t.Setenv("SWARM_LOOP_FIXTURE", "1")
	p := Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: exe, Args: []string{"-test.run=^TestLoopProvider$"}, Env: []string{"SWARM_LOOP_FIXTURE"}, Limits: RunLimits{MaxToolCalls: 1, MaxRepeatedCalls: 1, ToolSeconds: 1, SilenceSeconds: 1}}}}
	raw, _ := json.Marshal(p)
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	_, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "observe", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{ObservationMode: 1}, Reason: "test"})
	if err != nil {
		t.Fatal(err)
	}
	r.Instruction = "GUARD_LAST_RESULT_FAST"
	r.Timeout = 1
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.supervise(a.ID); err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	if a.Status != "completed" || a.Progress.ToolCalls != 1 || a.Progress.ToolResults != 1 {
		t.Fatalf("premature stop: %s %+v", a.Status, a.Progress)
	}
}

func TestObservationRequiresAdministrativeMissionChoice(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	for _, c := range []RunLimitsConfigChange{
		{Schema: 1, EventID: "bad", Scope: ScopeProject, Values: RunLimits{ObservationMode: 1}, Reason: "test"},
		{Schema: 1, EventID: "bad", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{ObservationMode: 1}},
		{Schema: 1, EventID: "bad", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{ObservationMode: 2}, Reason: "test"},
	} {
		if _, err := s.configureRunLimits(c); err == nil {
			t.Fatal("invalid administrative choice accepted")
		}
	}
	if _, err := (RunLimits{}).tightened(RunLimits{ObservationMode: 1}); err == nil {
		t.Fatal("launch request silently disables limits")
	}
}

func TestObservationStillHonorsManualStop(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	exe, _ := os.Executable()
	t.Setenv("SWARM_LOOP_FIXTURE", "1")
	p := Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: exe, Args: []string{"-test.run=^TestLoopProvider$"}, Env: []string{"SWARM_LOOP_FIXTURE"}}}}
	raw, _ := json.Marshal(p)
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	_, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "observe", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{ObservationMode: 1}, Reason: "test"})
	if err != nil {
		t.Fatal(err)
	}
	r.Instruction = "GUARD_TOOL"
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { done <- s.supervise(a.ID) }()
	deadline := time.Now().Add(10 * time.Second)
	for {
		current, _ := s.agent(a.ID)
		if current.Status == "running" {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("not started")
		}
		time.Sleep(10 * time.Millisecond)
	}
	if err = s.stopAgent(a.ID); err != nil {
		t.Fatal(err)
	}
	select {
	case err = <-done:
		if err != nil {
			t.Fatal(err)
		}
	case <-time.After(8 * time.Second):
		t.Fatal("manual stop ignored")
	}
	a, _ = s.agent(a.ID)
	if a.Status != "interrupted" {
		t.Fatalf("not interrupted: %s", a.Status)
	}
}
