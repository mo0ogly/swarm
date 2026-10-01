package main

import (
	"strings"
	"testing"
)

func TestWorkerContractUsesLatestPlanAndCurrentAttempt(t *testing.T) {
	source := "prep-example"
	id := "plan-" + hash([]byte(source))[:10] + "-T1"
	w := Work{ID: "work", Scope: "shared definitions ADM-07", Criteria: []string{"owned requirement", "private sibling requirement"}}
	w.Planning = &PlanningState{Scopes: []PlanningScope{{ID: "root"}, {ID: "child", Parent: "root"}}}
	w.Plans = []ApprovedPlan{
		{Source: source, BriefHash: "old", Spec: ActionPlan{Tasks: []PlanMission{{ID: "T1", Scope: "obsolete scope"}}}},
		{Source: source, BriefHash: "current", Spec: ActionPlan{Tasks: []PlanMission{{ID: "T1", Scope: "corrected scope", Proof: "fresh evidence"}, {ID: "T2", Scope: "private sibling scope"}}}},
	}
	task := Task{ID: id, ScopeID: "root", Requirements: []string{"req-1"}, Next: "generic status", Attempts: []Attempt{{ID: "old-attempt"}, {ID: "new-attempt"}}, PlanMaxAttempts: 3}
	got := workerExecutionContext(w, &task, "new-agent")
	for _, want := range []string{"corrected scope", "fresh evidence", "shared definitions ADM-07", "new-attempt", "new-agent", "2/3", "current", "owned requirement"} {
		if !strings.Contains(got, want) {
			t.Errorf("missing %q in %s", want, got)
		}
	}
	for _, unwanted := range []string{"obsolete scope", "private sibling scope", "private sibling requirement"} {
		if strings.Contains(got, unwanted) {
			t.Errorf("leaked %q", unwanted)
		}
	}
	task.ScopeID = "child"
	if strings.Contains(workerExecutionContext(w, &task, "agent"), "shared definitions ADM-07") {
		t.Fatal("root brief leaked into delegated worker")
	}
}

func TestWorkerContractCarriesExplicitCorrectiveGrant(t *testing.T) {
	task := Task{PlanMaxAttempts: 4, Attempts: []Attempt{{ID: "first"}, {ID: "second"}, {ID: "third"}, {ID: "fourth"}}, CorrectiveRecovery: &CorrectiveRecovery{Event: "approved-grant", Attempt: "third", Instruction: "Produce the current missing report"}}
	got := workerExecutionContext(Work{ID: "mission"}, &task, "current-agent")
	for _, want := range []string{"4/4", "approved-grant", "Produce the current missing report", "plafond historique"} {
		if !strings.Contains(got, want) {
			t.Fatalf("missing corrective context %q", want)
		}
	}
}
