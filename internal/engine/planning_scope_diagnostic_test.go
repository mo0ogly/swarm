//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestPublicPlanningContextSizeDiagnostic(t *testing.T) {
	path := os.Getenv("SWARM_PUBLIC_WORK_DIAGNOSTIC")
	if path == "" {
		t.Skip("public input absent")
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var w Work
	if err = json.Unmarshal(raw, &w); err != nil {
		t.Fatal(err)
	}
	s := storeTest(t)
	w.Planning.Repository.Storage = filepath.Join(s.root, "isolated-nonexistent-managed-storage")
	raw, err = os.ReadFile(os.Getenv("SWARM_PUBLIC_SCOPE_DIAGNOSTIC"))
	if err != nil {
		t.Fatal(err)
	}
	var p ManagedScopePlan
	if err = json.Unmarshal(raw, &p); err != nil {
		t.Fatal(err)
	}
	context, _, err := s.planningDeliveryContext(w, "root", 200000, map[string]any{"plan-ca937edf4e-T3": p})
	if err != nil {
		t.Fatal(err)
	}
	var fields map[string]json.RawMessage
	json.Unmarshal(context, &fields)
	for k, v := range fields {
		if len(v) > 500 {
			t.Logf("field=%s bytes=%d", k, len(v))
		}
	}
	_, method, err := s.projectAgentWorkflow("planner")
	if err != nil {
		t.Fatal(err)
	}
	t.Logf("context=%d budget=%d method=%d", len(context), planningPromptContextLimit(method), len(method))
	budget := reviewTokenBudgetFixture()
	bounded, _, e := s.planningCapacityContext(w, "root", method, &budget, map[string]any{"plan-ca937edf4e-T3": p})
	if e != nil {
		t.Fatal("current complete public inventory still cannot reach supervisor", e)
	}
	t.Logf("bounded admitted=%d", len(bounded))
}
