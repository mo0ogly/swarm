//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRunLimitsActualLaunchFreezesConfiguration(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Workspace = filepath.Join(s.root, "first")
	if err := os.Mkdir(r.Workspace, 0700); err != nil {
		t.Fatal(err)
	}
	changes := []RunLimitsConfigChange{
		{Schema: 1, EventID: "project", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 60}},
		{Schema: 1, EventID: "mission", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{MaxToolCalls: 50}},
		{Schema: 1, EventID: "role", Scope: ScopeRole, Mission: w.ID, Key: "worker", Values: RunLimits{MaxToolCalls: 40}},
		{Schema: 1, EventID: "task", Scope: ScopeTask, Mission: w.ID, Key: "t1", Values: RunLimits{MaxToolCalls: 30}},
	}
	for _, c := range changes {
		if _, err := s.configureRunLimits(c); err != nil {
			t.Fatal(err)
		}
	}
	first, created, err := s.prepareLaunch(w.ID, r, false)
	if err != nil || !created {
		t.Fatalf("launch: %v", err)
	}
	if first.Limits.MaxToolCalls != 30 || !strings.Contains(first.Prompt, "30 appels d'outils") {
		t.Fatalf("configuration not frozen in launch: %+v", first.Limits)
	}
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "role-update", Scope: ScopeRole, Mission: w.ID, Key: "worker", Revision: 1, Values: RunLimits{MaxToolCalls: 12}}); err != nil {
		t.Fatal(err)
	}
	persisted, err := s.agent(first.ID)
	if err != nil || persisted.Limits.MaxToolCalls != 30 {
		t.Fatalf("existing reservation changed: %+v %v", persisted.Limits, err)
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "next", Deliverable: "report", Criteria: []string{"proof"}})
	r.TaskID = "t2"
	r.Revision = w.Revision
	r.EventID = newID("agent-")
	r.Workspace = filepath.Join(s.root, "next")
	if err = os.Mkdir(r.Workspace, 0700); err != nil {
		t.Fatal(err)
	}
	second, created, err := s.prepareLaunch(w.ID, r, false)
	if err != nil || !created {
		t.Fatalf("next launch: %v", err)
	}
	if second.Limits.MaxToolCalls != 12 {
		t.Fatalf("next launch ignores changed config: %+v", second.Limits)
	}
	saved, err := s.agent(second.ID)
	if err != nil || saved.Limits != second.Limits {
		t.Fatalf("limits not persisted: %v", err)
	}
}

func TestRunLimitsLaunchCannotRaiseCeilings(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "large", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 900}}); err != nil {
		t.Fatal(err)
	}
	a, _, err := s.prepareLaunch(w.ID, r, true)
	if err != nil || a.Limits.MaxToolCalls != 100 {
		t.Fatalf("provider ceiling lost: %+v %v", a.Limits, err)
	}
	r.Limits = &RunLimits{MaxToolCalls: 7}
	a, _, err = s.prepareLaunch(w.ID, r, true)
	if err != nil || a.Limits.MaxToolCalls != 7 {
		t.Fatalf("explicit ceiling lost: %+v %v", a.Limits, err)
	}
}
