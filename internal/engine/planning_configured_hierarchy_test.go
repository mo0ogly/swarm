package engine

import (
	"strconv"
	"testing"
)

func TestHierarchyBudgetsRemainSharedAboveTwentyLevels(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 100, MaxActivations: 100})
	scope := "root"
	for level := 1; level <= 22; level++ {
		var request PlanningRequest
		w, request = planningClaim(t, s, w, scope)
		next := "level-" + strconv.Itoa(level)
		request.Operations = []PlanningOperation{{Kind: "delegate", ID: next, Title: "Scoped recursive planning", Requirements: []string{"req-1"}, Next: "Continue only within the shared budget"}}
		updated, err := s.planningChange(w.ID, "decide", request)
		if err != nil {
			t.Fatal("explicit budgets were overridden by a fixed hierarchy cutoff", level, err)
		}
		w, scope = updated, next
	}
	root, _ := w.Planning.scope("root")
	if len(w.Planning.Scopes) != 23 || w.Planning.Activations != 22 || root.Activations != 22 || len(planningAncestors(w.Planning, scope)) != 23 {
		t.Fatal("deep ownership chain lost ancestor accounting or consumed budget")
	}
	if err := validatePlanningState(&w); err != nil {
		t.Fatal(err)
	}
}

func TestConfiguredHierarchyLimitsAndCycleGuard(t *testing.T) {
	s, w := planningFixture(t)
	var err error
	w, err = s.configureQuotas(w.ID, QuotaChange{Schema: 1, EventID: "one-supervision-level", Revision: w.Revision, Reason: "Operator chooses an explicit hierarchy policy", Limits: QuotaValues{Planning: w.Planning.MaxActivations, Decisions: w.Planning.MaxDecisions, Scopes: quotaInt(1), Depth: quotaInt(1)}})
	if err != nil {
		t.Fatal(err)
	}
	w, request := planningClaim(t, s, w, "root")
	if !w.Planning.Scopes[0].Delivery.DelegationUnavailable {
		t.Fatal("provider was offered delegation beyond the operator policy")
	}
	request.Operations = []PlanningOperation{{Kind: "delegate", ID: "forbidden-child", Title: "Forbidden scope", Requirements: []string{"req-1"}}}
	if _, err = s.planningChange(w.ID, "decide", request); err == nil {
		t.Fatal("explicit supervision policy ignored")
	}
	cyclic := &PlanningState{Scopes: []PlanningScope{{ID: "a", Parent: "b"}, {ID: "b", Parent: "a"}}}
	if planningAncestors(cyclic, "a") != nil {
		t.Fatal("cycle produced valid ancestor accounting")
	}
}
