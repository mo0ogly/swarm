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

func TestIndependentReviewVerdictsRequireEveryCriterionAndRealQuotation(t *testing.T) {
	task := Task{Criteria: []string{"preuve"}}
	for _, raw := range []string{`{"reason":"Un avis détaillé","criteria":[]}`, `{"reason":"Un avis détaillé","criteria":[{"index":1,"verdict":"pass","evidence":"preuve inventée"}]}`, `{"reason":"Un avis détaillé","criteria":[{"index":2,"verdict":"pass","evidence":"preuve observée"}]}`} {
		if _, _, _, e := reviewReply(raw, &task, "preuve observée"); e == nil {
			t.Fatal("avis invalide accepté", raw)
		}
	}
	state, _, _, e := reviewReply(`{"reason":"Le contrôle externe manque","criteria":[{"index":1,"verdict":"unknown","evidence":"Le journal de test manque"}]}`, &task, "preuve observée")
	if e != nil || state != "changes_requested" {
		t.Fatal(state, e)
	}
}
func TestIndependentReviewProcessPersistsAndBlocksStaleEvidence(t *testing.T) {
	s, p := preparedTeam(t)
	p, e := s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
	if e != nil {
		t.Fatal(e)
	}
	w, _ := s.get(p.WorkID)
	task := &w.Tasks[0]
	task.Status = "submitted"
	task.Attempts = []Attempt{{ID: "production-attempt", Status: "completed"}}
	task.Criteria = []string{"Le rapport contient la phrase : preuve observée"}
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	report := "docs/" + task.ID + ".md"
	os.WriteFile(filepath.Join(s.root, report), []byte("preuve observée dans ce rapport"), 0600)
	a := Agent{ID: "producer-review-test", WorkID: w.ID, TaskID: task.ID, Attempt: "production-attempt", Status: "completed", CWD: s.root, Started: time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano), Role: "worker"}
	// Reuse the fixture's real process adapter, with a deterministic model reply.
	ps, _ := s.providers()
	provider := ps.Providers[w.Planning.Reviewer.Provider]
	response := `{"reason":"La preuve textuelle attendue est présente","criteria":[{"index":1,"verdict":"pass","evidence":"preuve observée"}]}`
	env, _ := json.Marshal(map[string]any{"type": "result", "result": response})
	if e := os.WriteFile(provider.Command, []byte("#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' '"+string(env)+"'\n"), 0700); e != nil {
		t.Fatal(e)
	}
	if real := os.Getenv("SWARM_TEST_REVIEW_REAL"); real != "" {
		provider.Command = real
		provider.Args = []string{"--model", "sonnet"}
		ps.Providers[w.Planning.Reviewer.Provider] = provider
		configBytes, _ := json.Marshal(ps)
		if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), configBytes, 0600); e != nil {
			t.Fatal(e)
		}
		config, e := s.reviewerConfig(w.Planning.Reviewer.Provider, "standard", 2)
		if e != nil {
			t.Fatal(e)
		}
		w.Planning.Reviewer = config
	}
	raw, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	// Agent is deliberately independent from the reviewer process identity.
	body, _ := json.Marshal(a)
	if _, e := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,?,?,?)", a.ID, a.WorkID, a.TaskID, a.CWD, "completed", "run", body, []byte(`{}`)); e != nil {
		t.Fatal(e)
	}
	if e := s.independentReviewStep(w.ID); e != nil {
		t.Fatal(e)
	}
	got, _ := s.get(w.ID)
	gt, _ := got.task(task.ID)
	if gt.IndependentReview == nil || gt.IndependentReview.State != "passed" || gt.Status != "submitted" || gt.IndependentReview.Reviewer == a.ID {
		t.Fatalf("review not independent or accepted implicitly: %+v", gt.IndependentReview)
	}
	if os.Getenv("SWARM_TEST_REVIEW_REAL") == "" {
		observed, err := os.ReadFile(provider.Command + ".prompt")
		if err != nil {
			t.Fatal(err)
		}
		assertWorkflowDelivery(t, string(observed), "reviewer", gt.IndependentReview.Workflow)
	}
	if e := s.independentReviewGuard(&got, gt); e != nil {
		t.Fatal(e)
	}
	if e := s.independentReviewStep(w.ID); e != nil {
		t.Fatal(e)
	}
	again, _ := s.get(w.ID)
	if again.Planning.Reviewer.Calls != 1 {
		t.Fatal("duplicate paid call")
	}
	os.WriteFile(filepath.Join(s.root, report), []byte("contenu modifié"), 0600)
	if e := s.independentReviewGuard(&got, gt); e == nil {
		t.Fatal("changed evidence accepted")
	}
	got.Planning.Reviewer = nil
	if organization(got).Ready {
		t.Fatal("missing reviewer accepted")
	}
	if !strings.Contains(gt.IndependentReview.Reason, "preuve") {
		t.Fatal("missing readable reason")
	}
}

func TestIndependentReviewRetryIsExplicitBoundedAndCannotReplaceApproval(t *testing.T) {
	s, p := preparedTeam(t)
	w, _ := s.get(p.WorkID)
	task := &w.Tasks[0]
	task.Status = "submitted"
	task.IndependentReview = &IndependentReview{ID: "old-review", State: "error", Reason: "incident de test"}
	w.Planning.Reviewer.Calls = 1
	raw, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	request := PlanningRequest{Schema: 1, EventID: "explicit-review-retry", Revision: w.Revision, Task: task.ID, Reason: "Accès au fournisseur rétabli et vérifié"}
	next, e := s.planningChange(w.ID, "retry-review", request)
	if e != nil {
		t.Fatal(e)
	}
	if next.Tasks[0].IndependentReview != nil || next.Planning.Reviewer.Calls != 1 {
		t.Fatal("retry refunded budget or retained claim")
	}
	replay, e := s.planningChange(w.ID, "retry-review", request)
	if e != nil || replay.Revision != next.Revision {
		t.Fatal("retry not idempotent", e)
	}
	next.Tasks[0].IndependentReview = &IndependentReview{ID: "approved", State: "passed"}
	raw, _ = json.Marshal(next)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	request.EventID = "cannot-reroll-approval"
	request.Revision = next.Revision
	if _, e = s.planningChange(w.ID, "retry-review", request); e == nil {
		t.Fatal("approved review silently replaced")
	}
	next.Tasks[0].IndependentReview.State = "error"
	next.Planning.Reviewer.Calls = next.Planning.Reviewer.MaxCalls
	raw, _ = json.Marshal(next)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	request.EventID = "cannot-raise-budget"
	if _, e = s.planningChange(w.ID, "retry-review", request); e == nil {
		t.Fatal("budget bypass")
	}
}
