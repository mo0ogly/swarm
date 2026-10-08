//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestCursorContractPlannerIsToolFreeAndRootOwnsNeed(t *testing.T) {
	executable := filepath.Join(t.TempDir(), "codex")
	if err := os.WriteFile(executable, []byte("#!/bin/sh\nexit 0\n"), 0700); err != nil {
		t.Fatal(err)
	}
	provider, err := assistantProvider(Provider{Command: executable, Args: []string{"exec", "--dangerously-bypass-approvals-and-sandbox"}})
	if err != nil {
		t.Fatal(err)
	}
	args := strings.Join(provider.Args, " ")
	for _, required := range []string{"--sandbox read-only", "--ignore-user-config", "--disable shell_tool", "--disable multi_agent"} {
		if !strings.Contains(args, required) {
			t.Fatalf("tool-free planner guard missing %q in %q", required, args)
		}
	}
	if strings.Contains(args, "dangerously-bypass") {
		t.Fatalf("worker permissions leaked into planner: %q", args)
	}

	_, w := planningFixture(t)
	root, err := w.Planning.scope("root")
	if err != nil || root.Parent != "" || root.Objective != w.Objective || len(root.Requirements) != len(w.Criteria) {
		t.Fatalf("root does not own the complete initial need: root=%+v work=%+v err=%v", root, w, err)
	}
}

func TestCursorContractRecursiveDelegationHandoffAndAdaptation(t *testing.T) {
	s, w := planningFixture(t)
	w, rootDecision := planningClaim(t, s, w, "root")
	rootDecision.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Sous-périmètre", Requirements: []string{"req-1"}, Next: "Prendre en charge l’exigence"}}
	w, err := s.planningChange(w.ID, "decide", rootDecision)
	if err != nil {
		t.Fatal(err)
	}

	w, childDecision := planningClaim(t, s, w, "child")
	childDecision.Operations = []PlanningOperation{{Kind: "delegate", ID: "leaf", Title: "Périmètre feuille", Requirements: []string{"req-1"}, Next: "Déléguer un travail ciblé"}}
	w, err = s.planningChange(w.ID, "decide", childDecision)
	if err != nil {
		t.Fatal(err)
	}

	w, leafDecision := planningClaim(t, s, w, "leaf")
	// The bounded recursion is deliberate. A fourth ownership level is refused
	// instead of silently widening the architecture or its budgets. The failed
	// decision is atomic, so the same owner can adapt its still-live decision.
	tooDeep := leafDecision
	tooDeep.Operations = []PlanningOperation{{Kind: "delegate", ID: "too-deep", Title: "Niveau interdit", Requirements: []string{"req-1"}}}
	if _, err = s.planningChange(w.ID, "decide", tooDeep); err == nil || !strings.Contains(err.Error(), "profondeur") {
		t.Fatalf("unbounded recursive delegation accepted: %v", err)
	}
	leafDecision.Operations = []PlanningOperation{planningTask("worker")}
	w, err = s.planningChange(w.ID, "decide", leafDecision)
	if err != nil {
		t.Fatal(err)
	}
	worker, _ := w.task("worker")
	if worker.ScopeID != "leaf" || worker.PlanRole != "worker" {
		t.Fatalf("delegated executable is not an isolated worker: %+v", worker)
	}

	w, err = s.mutate(w.ID, "test.cursor-attempt", "cursor-attempt", w.Revision, []byte(`{}`), func(current *Work) error {
		task, _ := current.task("worker")
		task.Status = "blocked"
		task.Attempts = []Attempt{{ID: "attempt-1", Status: "failed"}}
		planningAttemptEnded(current, Agent{ID: "worker-agent", TaskID: task.ID, Attempt: "attempt-1", Activity: "test initial en échec"}, "failed")
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	agent := Agent{ID: "worker-agent", WorkID: w.ID, TaskID: "worker", Attempt: "attempt-1", Role: "worker", Status: "failed", CWD: s.root}
	body, _ := json.Marshal(agent)
	if _, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", agent.ID, w.ID, agent.TaskID, agent.CWD, agent.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	proof := []byte("échec observé et limites factuelles")
	if err = os.WriteFile(filepath.Join(s.root, "cursor-proof.txt"), proof, 0600); err != nil {
		t.Fatal(err)
	}
	handoff := PlanningRequest{Schema: 1, EventID: "cursor-handoff", Revision: w.Revision, Agent: agent.ID, Task: agent.TaskID, Attempt: agent.Attempt, Reason: "Le contrôle initial échoue ; corriger la cause avant reprise.", Artifacts: []ExchangeArtifact{{Path: "cursor-proof.txt", SHA256: hash(proof)}}}
	w, err = s.planningChange(w.ID, "handoff", handoff)
	if err != nil {
		t.Fatal(err)
	}
	last := w.Planning.Inbox[len(w.Planning.Inbox)-1]
	if last.Scope != "leaf" || last.Kind != "handoff" || last.Attempt != "attempt-1" || len(last.Artifacts) != 1 {
		t.Fatalf("handoff did not return to the exact owner: %+v", last)
	}

	w, retry := planningClaim(t, s, w, "leaf")
	retry.Operations = []PlanningOperation{{Kind: "retry", ID: "worker", Next: "Corriger la cause observée puis rejouer le contrôle ciblé."}}
	w, err = s.planningChange(w.ID, "decide", retry)
	if err != nil {
		t.Fatal(err)
	}
	worker, _ = w.task("worker")
	if !worker.PlanningRetry || worker.Next != retry.Operations[0].Next {
		t.Fatalf("owner was not reactivated with an adapted bounded plan: %+v", worker)
	}
}

func TestCursorContractHierarchicalWorkersCannotCrossTalkOrBecomePlanners(t *testing.T) {
	s, w := planningFixture(t)
	w, decision := planningClaim(t, s, w, "root")
	decision.Operations = []PlanningOperation{planningTask("one")}
	w, err := s.planningChange(w.ID, "decide", decision)
	if err != nil {
		t.Fatal(err)
	}

	_, _, err = s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "cursor-crosstalk", Kind: "help_request", AgentID: "worker-one", TaskID: "one", AttemptID: "attempt-one", RecipientTask: "one", RecipientRole: "worker", Need: "contourner le responsable", Timeout: 60})
	if err == nil || !strings.Contains(err.Error(), "responsable via planning handoff") {
		t.Fatalf("hierarchical worker cross-talk was not rejected: %v", err)
	}

	corrupt := w
	corrupt.Tasks = append([]Task(nil), w.Tasks...)
	corrupt.Tasks[0].PlanRole = "planner"
	if err = validatePlanningState(&corrupt); err == nil || !strings.Contains(err.Error(), "non exécutante") {
		t.Fatalf("executable planner task accepted in hierarchy: %v", err)
	}

	launchStore := storeTest(t)
	launchWork, launch := setupAgent(t, launchStore)
	organizedFixtureStore(t, launchStore)
	launchWork, _ = launchStore.get(launchWork.ID)
	launch.Revision = launchWork.Revision
	launch.Parent = "another-worker"
	if _, _, err = launchStore.prepare(launchWork.ID, launch); err == nil || !strings.Contains(err.Error(), "communication directe entre exécutants interdite") {
		t.Fatalf("worker parent side channel accepted: %v", err)
	}
}
