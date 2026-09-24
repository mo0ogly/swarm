//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedFragmentIntegrationPublishesOnlyFinalVerdict(t *testing.T) {
	for _, mode := range []string{"pass", "decision-fail", "retry", "exit"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			a := managedCompleted(t, s, w, "first", "candidate\n")
			for _, name := range []string{"one.txt", "two.txt", "three.txt", "four.txt"} {
				if e := os.WriteFile(filepath.Join(a.CWD, name), []byte(strings.Repeat("complete original evidence line\n", 2200)), 0600); e != nil {
					t.Fatal(e)
				}
			}
			if e := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fragmentRuntimeProviderFixture), 0700); e != nil {
				t.Fatal(e)
			}
			managedReviewMode(t, s, mode)
			if mode == "retry" {
				a.Ended = now()
				raw, _ := json.Marshal(a)
				if _, e := s.db.Exec("UPDATE agents SET body=? WHERE id=?", raw, a.ID); e != nil {
					t.Fatal(e)
				}
				current, _ := s.get(w.ID)
				current.Planning.Reviewer.Failure = "fixture preflight stop"
				raw, _ = json.Marshal(current)
				if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
					t.Fatal(e)
				}
				if e := s.integrateManagedAttempt(a); e != nil {
					t.Fatal(e)
				}
				current, _ = s.get(w.ID)
				current.Planning.Reviewer.Failure = ""
				raw, _ = json.Marshal(current)
				if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
					t.Fatal(e)
				}
				if e := s.managedFailure(a, managedReviewContextTooLarge+" : tâche first"); e != nil {
					t.Fatal(e)
				}
				current, _ = s.get(w.ID)
				request := PlanningRequest{Schema: 1, EventID: "fragment-retry", Revision: current.Revision, Task: a.TaskID, Reason: "Complete fragment transport now available"}
				if _, e := s.planningChange(w.ID, "retry-review", request); e != nil {
					t.Fatal(e)
				}
				if managedReviewCalls(t, s) != 0 {
					t.Fatal("preflight spent calls")
				}
			}
			if e := s.integrateManagedAttempt(a); e != nil {
				t.Fatal(e)
			}
			after, e := s.get(w.ID)
			if e != nil {
				t.Fatal(e)
			}
			task, _ := after.task(a.TaskID)
			if task.IndependentReview == nil || task.IndependentReview.FragmentJournal == nil {
				t.Fatal("fragment path not used", task.Blocker)
			}
			if mode == "pass" || mode == "retry" {
				if task.Status != "accepted" || after.Planning.Repository.Candidate == w.Planning.Repository.Candidate || !s.acceptedFresh(&after, task, map[string]bool{}) {
					t.Fatal("candidate not accepted", task.Status, task.Blocker)
				}
			} else if task.Status == "accepted" || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
				t.Fatal("failed review published")
			}
			calls := managedReviewCalls(t, s)
			if calls < 3 && mode != "exit" {
				t.Fatal("missing inspection/final calls")
			}
			if e = s.integrateManagedAttempt(a); e != nil {
				t.Fatal(e)
			}
			if managedReviewCalls(t, s) != calls {
				t.Fatal("replay spent twice")
			}
			if mode == "exit" {
				current, _ := s.get(w.ID)
				request := PlanningRequest{Schema: 1, EventID: "interrupted-fragments", Revision: current.Revision, Task: a.TaskID, Reason: "Retry must preserve the spent inspection journal"}
				if _, e := s.planningChange(w.ID, "retry-review", request); e == nil {
					t.Fatal("discarded fragment journal")
				}
				current, _ = s.get(w.ID)
				saved, _ := current.task(a.TaskID)
				if saved.IndependentReview == nil || saved.IndependentReview.ID != task.IndependentReview.ID || managedReviewCalls(t, s) != calls {
					t.Fatal("interrupted proof lost")
				}
			}

		})
	}
}
