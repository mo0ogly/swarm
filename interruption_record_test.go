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

func TestExecutionLimitDominatesEarlierMissingFile(t *testing.T) {
	a := failedRecoveryAgent("stopped", "first", "configuration", "no such file or directory", time.Now())
	a.Status, a.StopKind, a.Activity = "interrupted", "garde", "Limite d'appels d'outils atteinte ; fin du processus confirmée"
	a.Diagnostic.LimitReached = true
	a.Diagnostic.LimitReason = "Limite d'appels d'outils atteinte"
	finalizeRecoveryState(&a)
	r := assessRecovery(a, Task{PlanMaxAttempts: 4}, time.Now().Add(time.Hour))
	if r.Category != recoveryExecutionLimit || r.Disposition != recoveryDispositionIntervention || !r.NextEligibleAt.IsZero() {
		t.Fatalf("budget exhaustion misclassified or automatically retryable: %+v", r)
	}
}

func TestStoppedManagedAttemptHasEngineHandoffWithoutWorkerReport(t *testing.T) {
	s, w := managedFixture(t)
	a, _, err := s.prepare(w.ID, Launch{Schema: 1, EventID: "stopped-record", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "bounded work", Timeout: 60})
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(a.CWD, "value.txt"), []byte("unfinished changes"), 0600); err != nil {
		t.Fatal(err)
	}
	a.Progress = AgentProgress{ToolCalls: 100, ToolResults: 99, PendingTools: 1, Action: "Reading a file"}
	a.StopKind = "garde"
	if err = s.finishAgent(a, "interrupted", "Limite d'appels d'outils atteinte ; fin du processus confirmée", nil); err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	w, _ = s.get(w.ID)
	var events []PlanningEvent
	for _, e := range w.Planning.Inbox {
		if e.Attempt == a.Attempt && e.Kind == "attempt_ended" {
			events = append(events, e)
		}
	}
	if len(events) != 1 || len(events[0].Artifacts) != 1 {
		t.Fatal("owner did not receive interruption evidence", events)
	}
	f, err := localFile(s.root, events[0].Artifacts[0].Path)
	if err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(f)
	if err != nil {
		t.Fatal(err)
	}
	var record InterruptionRecord
	if err = json.Unmarshal(raw, &record); err != nil {
		t.Fatal(err)
	}
	if hash(raw) != events[0].Artifacts[0].SHA256 || record.Attempt != a.Attempt || record.Progress.ToolCalls != 100 || record.Workspace != a.CWD || record.Status != "interrupted" {
		t.Fatal("unattributed or incomplete record", record)
	}
	if err = s.settleAgentTask(a); err != nil {
		t.Fatal(err)
	}
	again, _ := os.ReadFile(f)
	if string(again) != string(raw) {
		t.Fatal("replay changed evidence")
	}
	w, _ = s.get(w.ID)
	count := 0
	for _, e := range w.Planning.Inbox {
		if e.Kind == "attempt_ended" && e.Attempt == a.Attempt {
			count++
		}
	}
	if count != 1 || w.Tasks[0].Status != "blocked" || len(w.Tasks[0].Attempts) != 1 || managedReviewCalls(t, s) != 0 {
		t.Fatal("replay, acceptance or budget changed", w.Tasks[0])
	}
	if _, err = os.Stat(filepath.Join(a.CWD, "docs/first.md")); !os.IsNotExist(err) {
		t.Fatal("engine fabricated a worker report")
	}
	value, _ := os.ReadFile(filepath.Join(a.CWD, "value.txt"))
	if string(value) != "unfinished changes" {
		t.Fatal("worker copy altered")
	}
	if err = os.WriteFile(f, []byte("tampered"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err = s.interruptionRecord(a); err == nil {
		t.Fatal("altered evidence overwritten")
	}
}

func TestExecutionDirectivesHaveOneCodeRootAndEarlyHandoff(t *testing.T) {
	l, _ := (RunLimits{}).normalized()
	p := executionDirectives("/host/source", "/isolated/worker", "first", l)
	if strings.Contains(p, "/host/source") || !strings.Contains(p, "/isolated/worker") || !strings.Contains(p, "Créer dès le début docs/first.md") {
		t.Fatal(p)
	}
}
