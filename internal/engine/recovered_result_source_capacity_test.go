//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRecoveredResultCompletedSourceCapacityRefusalCanBeCorrected(t *testing.T) {
	s := storeTest(t)
	source := filepath.Join(s.root, "project")
	os.MkdirAll(source, 0700)
	gitTest(t, source, "init")
	for _, name := range []string{"large1.txt", "large2.txt"} {
		if err := os.WriteFile(filepath.Join(source, name), []byte(strings.Repeat("original source\n", 4700)), 0600); err != nil {
			t.Fatal(err)
		}
	}
	os.WriteFile(filepath.Join(source, "value.txt"), []byte("initial\n"), 0600)
	gitTest(t, source, "add", ".")
	gitTest(t, source, "commit", "-m", "original evidence")
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 20, MaxActivations: 30, Repository: &ManagedRepositoryRequest{Path: source, CommittedOnly: true}, Checks: map[string][]ValidationControl{"req-1": automaticPolicy("git", "diff", "--exit-code").Controls}})
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("first")}
	var err error
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	if err = organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 2); err != nil {
		t.Fatal(err)
	}
	if err = s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	w = managedReviewFixture(t, s, w)
	a := managedCompleted(t, s, w, "first", "candidate\n")
	a.DeliveryVersion = 1
	a.Ended = now()
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	d := completeDelivery(task, a)
	d.Criteria[0].Evidence = []string{"value.txt"}
	data, _ := json.Marshal(d)
	os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), data, 0600)
	os.WriteFile(filepath.Join(a.CWD, "docs/first.review-context.json"), []byte(`{"version":1,"files":["large1.txt","large2.txt"]}`), 0600)
	if err = s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ = w.task(a.TaskID)
	if task.Status != "blocked" || task.IndependentReview != nil || managedReviewCalls(t, s) != 0 || !strings.Contains(task.Blocker, "sources de contexte supérieures") {
		t.Fatal("source-capacity preflight not reached", task.Blocker)
	}
	old, err := s.managedAttempt(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	scope, err := managedScope(filepath.Join(w.Planning.Repository.Storage, "repository.git"), w.Planning.Repository.Candidate, old.Result)
	if err != nil || scope.RequiresReduction {
		t.Fatal("fixture scope should already be bounded", err)
	}
	if _, err = s.readManagedPreflightEvidence(w.ID, a.TaskID); !managedReviewSourceCapacityRefusal(err) {
		t.Fatal("ordinary review retry should still refuse oversized sources", err)
	}
	os.WriteFile(filepath.Join(a.CWD, "docs/first.review-context.json"), []byte(`{"version":1,"files":["large1.txt"]}`), 0600)
	gitTest(t, a.CWD, "add", "-A")
	request := PlanningRequest{Schema: 1, EventID: "correct-source-capacity", Revision: w.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ConfirmRecovery: true, ResultTree: gitTest(t, a.CWD, "write-tree"), Reason: "Operator removes only redundant declarations; originals retained"}
	cp := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID, "candidate.json")
	original, err := os.ReadFile(cp)
	if err != nil {
		t.Fatal(err)
	}
	os.WriteFile(cp, []byte(`{}`), 0600)
	if _, err = s.planningChange(w.ID, "revise-recovered-result", request); err == nil {
		t.Fatal("modified preflight checkpoint accepted")
	}
	os.WriteFile(cp, original, 0600)
	receipt := filepath.Join(filepath.Dir(cp), "receipt.json")
	originalReceipt, err := os.ReadFile(receipt)
	if err != nil {
		t.Fatal(err)
	}
	os.WriteFile(receipt, []byte(`{}`), 0600)
	if _, err = s.planningChange(w.ID, "revise-recovered-result", request); err == nil {
		t.Fatal("modified control receipt accepted")
	}
	os.WriteFile(receipt, originalReceipt, 0600)
	unchanged, err := s.get(w.ID)
	if err != nil || unchanged.Revision != w.Revision || managedReviewCalls(t, s) != 0 {
		t.Fatal("refused correction mutated revision or consumed a review", err)
	}
	got, err := s.planningChange(w.ID, "revise-recovered-result", request)
	if err != nil {
		t.Fatal("completed source-capacity result cannot be repaired", err)
	}
	current, _ := got.task(a.TaskID)
	if current.Status != "accepted" || !s.acceptedFresh(&got, current, map[string]bool{}) || len(current.Attempts) != 1 || current.RecoveredResult.ReplacesResult != old.Result || managedReviewCalls(t, s) != 1 {
		t.Fatal("repair lost custody, history or fresh review", current.Blocker)
	}
	if _, err = s.planningChange(w.ID, "revise-recovered-result", request); err != nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("replay repeated review", err)
	}
	if gitTest(t, filepath.Join(w.Planning.Repository.Storage, "repository.git"), "rev-parse", "refs/swarm/attempts/"+a.ID) != old.Result {
		t.Fatal("original immutable result overwritten")
	}
}
