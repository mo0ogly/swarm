//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func setupSupervisionR3(t *testing.T, instruction string, timeout int) (*Store, Work, Agent) {
	t.Helper()
	s := storeTest(t)
	w, r := setupAgent(t, s)
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	t.Setenv("SWARM_LOOP_FIXTURE", "1")
	providers := Providers{Schema: 1, Providers: map[string]Provider{"fixture": {
		Command: exe, Args: []string{"-test.run=^TestLoopProvider$"}, Env: []string{"SWARM_LOOP_FIXTURE"},
		Limits: RunLimits{SilenceSeconds: 1, ToolSeconds: 3, MaxToolCalls: 20, MaxRepeatedCalls: 4, MaxConsecutiveErrors: 3},
	}}}
	raw, _ := json.Marshal(providers)
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	r.Instruction = instruction
	r.Timeout = timeout
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	return s, w, a
}

func TestSupervisionR3SilenceThenResponseBeforeTotalTimeout(t *testing.T) {
	s, _, a := setupSupervisionR3(t, "R3_SILENT_RESPONSE", 4)
	started := time.Now()
	if err := s.supervise(a.ID); err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	if time.Since(started) < time.Second || a.Status != "completed" || a.ExitCode == nil || *a.ExitCode != 0 {
		t.Fatalf("visible silence interrupted a valid response: %+v", a)
	}
	if (a.Progress.OutputState != "recent" && a.Progress.OutputState != "silent") || a.Progress.Vitality != "unknown" || a.Progress.LastOutput == "" {
		t.Fatalf("output and vitality were conflated: %+v", a.Progress)
	}
	health := pilotAgentHealth(a, "", now())
	if health["completion_state"] != "finished" || a.Ended == "" {
		t.Fatalf("effective end was not confirmed by process wait: %+v", health)
	}
}

func TestSupervisionR3TotalTimeoutStillBoundsSilence(t *testing.T) {
	s, _, a := setupSupervisionR3(t, "R3_SILENT_FOREVER", 1)
	if err := s.supervise(a.ID); err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	if a.Status != "interrupted" || a.StopKind != "delai" || a.Progress.OutputState != "none" || a.Progress.Vitality != "unknown" {
		t.Fatalf("total timeout or unknown vitality lost: %+v", a)
	}
}

func TestSupervisionR3UnknownVitalityHasNoInventedProgress(t *testing.T) {
	limits, _ := (RunLimits{SilenceSeconds: 1}).normalized()
	guard := newLoopGuard(limits)
	guard.outputSeen = true
	guard.lastOutput = time.Now().Add(-2 * time.Second)
	progress := guard.summary()
	a := Agent{ID: "uncertain", Attempt: "attempt", Status: "running", Host: "other-host", Progress: progress}
	health := pilotAgentHealth(a, "", now())
	if health["output_state"] != "silent" || health["provider_vitality"] != "unknown" || health["completion_state"] != "open" {
		t.Fatalf("uncertain vitality hidden: %+v", health)
	}
	for _, forbidden := range []string{"progress_percent", "token_progress", "provider_heartbeat"} {
		if _, exists := health[forbidden]; exists {
			t.Fatalf("invented progress field %q: %+v", forbidden, health)
		}
	}
	if health["heartbeat"] != "" || health["supervisor_heartbeat"] != "" {
		t.Fatalf("a supervisor heartbeat was fabricated: %+v", health)
	}
}

func TestSupervisionR3HumanStopRemainsEffective(t *testing.T) {
	s, _, a := setupSupervisionR3(t, "R3_SILENT_FOREVER", 10)
	done := make(chan error, 1)
	go func() { done <- s.supervise(a.ID) }()
	waitAgent(t, s, a.ID, func(current Agent) bool { return current.Status == "running" })
	if err := s.stopAgent(a.ID); err != nil {
		t.Fatal(err)
	}
	if err := <-done; err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	if a.Status != "interrupted" || a.StopKind != "operateur" || a.Ended == "" {
		t.Fatalf("human stop not confirmed: %+v", a)
	}
}

func TestSupervisionR3StopResultRaceHonorsPersistedStop(t *testing.T) {
	s, _, a := setupSupervisionR3(t, "R3_WAIT_RELEASE", 10)
	done := make(chan error, 1)
	go func() { done <- s.supervise(a.ID) }()
	a = waitAgent(t, s, a.ID, func(current Agent) bool { return current.Status == "running" })
	if err := s.stopAgent(a.ID); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(a.CWD, "r3-release"), []byte("release"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := <-done; err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	if a.Status != "interrupted" || a.StopKind != "operateur" || a.ExitCode == nil || a.Ended == "" {
		t.Fatalf("persisted stop did not win with a confirmed process end: %+v", a)
	}
}
