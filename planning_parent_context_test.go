package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestPlanningParentSeesDelegationAndUnverifiedDescendant(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w.Criteria = []string{"Expected behavior"}
	w.Planning = &PlanningState{MaxTasks: 10, Scopes: []PlanningScope{
		{ID: "root", State: "ready"},
		{ID: "child", Parent: "root", State: "closed"},
		{ID: "grandchild", Parent: "child", Requirements: []string{"req-1"}, State: "closed"},
		{ID: "other", State: "ready"},
	}}
	w.Tasks = []Task{
		{ID: "worker", ScopeID: "grandchild", Status: "accepted", Requirements: []string{"req-1"}, IndependentReview: &IndependentReview{ID: "review-id", Reviewer: "reviewer://test", State: "passed", CandidateSHA: "candidate"}},
		{ID: "unrelated", ScopeID: "other", Status: "todo"},
	}
	raw, _, err := s.planningDeliveryContext(w, "root", 64000)
	if err != nil {
		t.Fatal(err)
	}
	var ctx map[string]json.RawMessage
	if err = json.Unmarshal(raw, &ctx); err != nil {
		t.Fatal(err)
	}
	if _, ok := ctx["remaining_tasks"]; ok {
		t.Fatal("ambiguous remaining work count retained")
	}
	var capacity int
	json.Unmarshal(ctx["task_capacity_remaining"], &capacity)
	if capacity != 8 {
		t.Fatal(capacity)
	}
	var req map[string]string
	json.Unmarshal(ctx["delegated_requirements"], &req)
	if req["req-1"] != "Expected behavior" {
		t.Fatal("delegated requirement lost")
	}
	var results []struct {
		Task   string            `json:"task"`
		Fresh  bool              `json:"accepted_fresh"`
		Review map[string]string `json:"review"`
	}
	if err = json.Unmarshal(ctx["descendant_validation"], &results); err != nil {
		t.Fatal(err)
	}
	if len(results) != 1 || results[0].Task != "worker" || results[0].Fresh {
		t.Fatal("unrelated task leaked or status-only acceptance trusted", results)
	}
	if results[0].Review["id"] != "review-id" || results[0].Review["candidate_commit"] != "candidate" {
		t.Fatal("review provenance missing")
	}
}

func TestPlanningParentValidationFollowsProofFreshness(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("t1")}
	var err error
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	w = gateTest(t, s, w, fixture(t, s.root))
	organizedFixtureStore(t, s)
	w, err = s.mutate(w.ID, "test.accept", "accept-parent-proof", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].Status = "accepted"
		w.Tasks[0].Attempts = []Attempt{{ID: "fixture-production", Status: "completed"}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	approveReportFixture(t, s, w.ID, "t1", "fixture-producer", "proof.txt")
	w, _ = s.get(w.ID)
	// Add a parent to this local snapshot only; no live Store is used.
	w.Planning.Scopes[0].Parent = "coordinator"
	w.Planning.Scopes = append(w.Planning.Scopes, PlanningScope{ID: "coordinator", State: "ready"})
	rows := s.planningDescendantValidation(w, "coordinator")
	if len(rows) != 1 || rows[0]["accepted_fresh"] != true {
		t.Fatal("fresh accepted descendant missing", rows)
	}
	if err = os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed evidence"), 0600); err != nil {
		t.Fatal(err)
	}
	rows = s.planningDescendantValidation(w, "coordinator")
	if len(rows) != 1 || rows[0]["accepted_fresh"] != false {
		t.Fatal("changed evidence still reported fresh", rows)
	}
}
