//go:build linux

package engine

import (
	"strings"
	"testing"
	"time"
)

func TestAdminDurationThresholdFrozenIntoLaunch(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	_, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "duration900", Scope: ScopeTask, Mission: w.ID, Key: r.TaskID, Values: RunLimits{ToolSeconds: 900, MaxToolCalls: 40}, Reason: "Operator authorizes a long verification command"})
	if err != nil {
		t.Fatal(err)
	}
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if a.Limits.ToolSeconds != 900 || a.Limits.MaxToolCalls != 40 || !strings.Contains(a.Prompt, "au plus 900 secondes") {
		t.Fatalf("admin duration not frozen into actual launch: %+v", a.Limits)
	}
	_, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "duration1200", Scope: ScopeTask, Mission: w.ID, Key: r.TaskID, Revision: 1, Values: RunLimits{ToolSeconds: 1200, MaxToolCalls: 40}, Reason: "Next attempt only"})
	if err != nil {
		t.Fatal(err)
	}
	stored, err := s.agent(a.ID)
	if err != nil || stored.Limits.ToolSeconds != 900 || stored.Prompt != a.Prompt {
		t.Fatal("live context rewritten by admin change", err)
	}
}

func TestAdminDurationThresholdRetryOverridesHistoricalDurationOnly(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Limits = &RunLimits{MaxToolCalls: 30}
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Heartbeat = time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano)
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if err = s.reconcile(a.ID); err != nil {
		t.Fatal(err)
	}
	_, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "retry-duration900", Scope: ScopeTask, Mission: w.ID, Key: r.TaskID, Values: RunLimits{ToolSeconds: 900, MaxToolCalls: 80}, Reason: "Explicitly authorize longer next verification, retain call budget"})
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	r.EventID = "retry-new-duration"
	r.Revision = w.Revision
	r.Previous = a.ID
	r.Limits = nil
	next, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if next.Limits.ToolSeconds != 900 || next.Limits.MaxToolCalls != 30 || a.Limits.ToolSeconds != 300 {
		t.Fatalf("retry duration or historical call ceiling incorrect: old=%+v next=%+v", a.Limits, next.Limits)
	}
}

func TestAdminDurationThresholdRetainsExplicitSmallerLaunch(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	_, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "explicit-duration900", Scope: ScopeTask, Mission: w.ID, Key: r.TaskID, Values: RunLimits{ToolSeconds: 900}, Reason: "Operator threshold"})
	if err != nil {
		t.Fatal(err)
	}
	r.Limits = &RunLimits{ToolSeconds: 120}
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if a.Limits.ToolSeconds != 120 {
		t.Fatalf("explicit smaller launch ignored: %+v", a.Limits)
	}
}
