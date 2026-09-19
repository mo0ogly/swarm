package main

import (
	"encoding/json"
	"fmt"
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
