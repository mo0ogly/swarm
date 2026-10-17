//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestManagedDeliveryReadyRequiresActualControlsAndIndependentReview(t *testing.T) {
	for _, mode := range []string{"pass", "control-fails", "review-fails", "known-local-failure", "no-declared-control"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			if mode == "control-fails" {
				var err error
				w, err = s.mutate(w.ID, "fixture.policy", "ready-policy", w.Revision, []byte(`{}`), func(w *Work) error {
					policy := automaticPolicy("sh", "-c", "exit 7")
					w.Tasks[0].ValidationPolicy = policy
					return nil
				})
				if err != nil {
					t.Fatal(err)
				}
			}
			if mode == "review-fails" {
				managedReviewMode(t, s, "fail")
			}
			a := managedCompleted(t, s, w, "first", "ready implementation\n")
			a.DeliveryVersion = 2
			if err := s.saveAgent(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task(a.TaskID)
			d := ManagedDelivery{Version: 2, Task: task.ID, Attempt: a.Attempt, Contract: reviewContract(task), Outcome: "ready"}
			for i := range task.Criteria {
				controls := []string{}
				if len(task.ValidationPolicy.Controls) > 0 {
					controls = append(controls, task.ValidationPolicy.Controls[0].ID)
				}
				d.Criteria = append(d.Criteria, DeliveryCriterion{Index: i + 1, Status: "not_tested", Reason: "Sources and recipe ready; engine must execute its required control in its own environment.", Controls: controls, Evidence: []string{"value.txt"}})
			}
			if mode == "known-local-failure" {
				d.Criteria[0].Status = "fail"
			}
			if mode == "no-declared-control" {
				d.Criteria[0].Controls = nil
			}
			raw, _ := json.Marshal(d)
			if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), raw, 0600); err != nil {
				t.Fatal(err)
			}
			if err := s.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			after, _ := s.get(w.ID)
			got, _ := after.task(a.TaskID)
			if (got.Status == "accepted") != (mode == "pass") {
				t.Fatal("wrong readiness outcome", mode, got.Status, got.Blocker)
			}
			expectedCalls := 0
			if mode == "pass" || mode == "review-fails" {
				expectedCalls = 1
			}
			if managedReviewCalls(t, s) != expectedCalls {
				t.Fatal("wrong reviewer calls", mode, managedReviewCalls(t, s))
			}
			if len(got.Attempts) != 1 || got.PlanMaxAttempts != 2 {
				t.Fatal("readiness changed attempts")
			}
			if mode == "pass" && !s.acceptedFresh(&after, got, map[string]bool{}) {
				t.Fatal("ready result was not actually qualified")
			}
		})
	}
}
