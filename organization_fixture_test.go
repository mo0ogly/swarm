package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

// Legacy scheduling tests exercise leases, budgets and concurrency rather than
// planning. Seed their now-required organization explicitly, without an AI call.
// This helper is test-only; production never manufactures these responsibilities.
func organizedFixtureStore(t *testing.T, s *Store) *Store {
	t.Helper()
	rows, e := s.db.Query("SELECT body FROM works")
	if e != nil {
		t.Fatal(e)
	}
	var works []Work
	for rows.Next() {
		var raw []byte
		var w Work
		if e = rows.Scan(&raw); e != nil {
			t.Fatal(e)
		}
		if e = json.Unmarshal(raw, &w); e != nil {
			t.Fatal(e)
		}
		works = append(works, w)
	}
	rows.Close()
	for _, w := range works {
		if w.Planning == nil {
			w.Planning = &PlanningState{Version: 1, Provider: "organization-fixture", Paused: true, MaxTasks: 100, MaxDecisions: 100, MaxActivations: 100, Scopes: []PlanningScope{{ID: "root", State: "waiting", Revision: 1}}, Checks: map[string][]ValidationControl{}}
		}
		p := w.Planning
		if p.Reviewer == nil {
			// Declared process fixture for launch-readiness tests. It intentionally
			// does not produce a favorable opinion; review tests supply their own.
			dir := filepath.Join(s.root, ".swarm", "review-launch-fixture")
			if e := os.MkdirAll(dir, 0700); e != nil {
				t.Fatal(e)
			}
			command := filepath.Join(dir, "claude")
			if e := os.WriteFile(command, []byte("#!/bin/sh\nexit 1\n"), 0700); e != nil {
				t.Fatal(e)
			}
			ps, e := s.providers()
			if e != nil {
				ps = Providers{Schema: 1, Providers: map[string]Provider{}}
			}
			ps.Providers["organization-review-fixture"] = Provider{Command: command}
			raw, _ := json.Marshal(ps)
			if e = os.WriteFile(filepath.Join(s.root, ".swarm", "providers.json"), raw, 0600); e != nil {
				t.Fatal(e)
			}
			p.Reviewer, e = s.reviewerConfig("organization-review-fixture", "auto", 100)
			if e != nil {
				t.Fatal(e)
			}
			p.ReviewerRequired = true
		}
		if p.Provider == "" {
			p.Provider = "organization-fixture"
		}
		if p.Checks == nil {
			p.Checks = map[string][]ValidationControl{}
		}
		for i := range w.Criteria {
			key := fmt.Sprintf("req-%d", i+1)
			if len(p.Checks[key]) == 0 {
				p.Checks[key] = automaticPolicy("go", "version").Controls
			}
		}
		if p.Provider == "organization-fixture" {
			p.Scopes[0].Requirements = nil
			for i := range w.Criteria {
				p.Scopes[0].Requirements = append(p.Scopes[0].Requirements, fmt.Sprintf("req-%d", i+1))
			}
		}
		for i := range w.Tasks {
			task := &w.Tasks[i]
			if task.ScopeID == "" {
				task.ScopeID = "root"
			}
			if len(task.Requirements) == 0 {
				task.Requirements = []string{"req-1"}
			}
			if task.ValidationPolicy == nil {
				task.ValidationPolicy = &ValidationPolicy{Mode: "human"}
			}

			task.ValidationPolicy.Authorized = now()
			task.ValidationPolicy.Actor = "explicit test fixture"
		}
		raw, e := json.Marshal(w)
		if e != nil {
			t.Fatal(e)
		}
		if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
			t.Fatal(e)
		}
	}
	return s
}

// approveReportFixture establishes an explicit precondition for legacy tests
// isolating controls or handoff interactions. It does not demonstrate AI review.
// The managed-review and independent-review suites exercise actual subprocesses.
func approveReportFixture(t *testing.T, s *Store, work, taskID, producer, report string) {
	t.Helper()
	w, e := s.get(work)
	if e != nil {
		t.Fatal(e)
	}
	if w.Planning == nil {
		return
	}
	task, e := w.task(taskID)
	if e != nil || len(task.Attempts) == 0 {
		t.Fatal("review fixture requires a production attempt", e)
	}
	b, e := os.ReadFile(filepath.Join(s.root, report))
	if e != nil {
		t.Fatal(e)
	}
	task.IndependentReview = &IndependentReview{ID: newID("fixture-review-"), Attempt: task.Attempts[len(task.Attempts)-1].ID,
		Producer: producer, Reviewer: "reviewer://organization-review-fixture", Report: report, Digest: hash(b), Contract: reviewContract(task), State: "passed",
		Reason: "Précondition explicite de fixture ; aucune revue IA observée", Started: now(), Finished: now()}
	raw, _ := json.Marshal(w)
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
}
