package main

import (
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"testing"
)

func TestPlanningIndependentReviewBudget(t *testing.T) {
	for _, limit := range []int{0, 1, 4, 100} {
		t.Run(strconv.Itoa(limit), func(t *testing.T) {
			s := storeTest(t)
			script := filepath.Join(t.TempDir(), "claude")
			if err := os.WriteFile(script, []byte("#!/bin/sh\nexit 1\n"), 0700); err != nil {
				t.Fatal(err)
			}
			raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"review-fixture": {Command: script}}})
			if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
				t.Fatal(err)
			}
			w := createTest(t, s)
			request, _ := json.Marshal(PlanningRequest{Schema: 1, EventID: newID("budget"), Revision: w.Revision, Provider: "review-fixture", MaxTasks: 10, MaxDecisions: 20, MaxActivations: 12, MaxReviewCalls: limit})
			input := filepath.Join(t.TempDir(), "enable.json")
			if err := os.WriteFile(input, request, 0600); err != nil {
				t.Fatal(err)
			}
			if err := s.planningCLI([]string{"planning", "enable", w.ID}, input, io.Discard); err != nil {
				t.Fatal(err)
			}
			// Exact CLI replay must neither reset counters nor consume another revision.
			before, _ := s.get(w.ID)
			if err := s.planningCLI([]string{"planning", "enable", w.ID}, input, io.Discard); err != nil {
				t.Fatal(err)
			}
			after, _ := s.get(w.ID)
			if before.Revision != after.Revision {
				t.Fatal("replay changed revision")
			}
			want := limit
			if want == 0 {
				want = 12
			}
			other, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer other.db.Close()
			saved, err := other.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			if saved.Planning.MaxActivations != 12 || saved.Planning.Reviewer.MaxCalls != want || saved.Planning.Reviewer.Calls != 0 {
				t.Fatalf("wrong persisted budgets: %+v", saved.Planning)
			}
		})
	}
}

func TestPlanningReviewBudgetRejectsUnusedOrInvalidLimit(t *testing.T) {
	for _, tc := range []struct {
		action, provider string
		limit            int
	}{
		{"enable", "fixture", -1}, {"enable", "fixture", 101}, {"enable", "", 4}, {"configure-reviewer", "fixture", 4}, {"claim", "fixture", 4},
	} {
		s := storeTest(t)
		w := createTest(t, s)
		_, err := s.planningChange(w.ID, tc.action, PlanningRequest{Schema: 1, EventID: newID("budget"), Revision: w.Revision, Provider: tc.provider, MaxReviewCalls: tc.limit})
		if err == nil {
			t.Fatalf("accepted invalid budget: %+v", tc)
		}
		after, err := s.get(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		if after.Revision != w.Revision || after.Planning != nil {
			t.Fatalf("invalid request changed work: %+v", after)
		}
	}
}
