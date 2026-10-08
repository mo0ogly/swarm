//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestCorrectiveRecoveryIncompleteDeliveryRequiresExactResult(t *testing.T) {
	for _, mode := range []string{"valid", "wrong-result", "wrong-attempt", "unconfirmed", "no-reviewer"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			w = managedReviewFixture(t, s, w)
			a := managedCompleted(t, s, w, "first", "retained incomplete work\n")
			a.DeliveryVersion = 1
			if err := s.saveAgent(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task(a.TaskID)
			d := completeDelivery(task, a)
			d.Outcome = "partial"
			raw, _ := json.Marshal(d)
			if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), raw, 0600); err != nil {
				t.Fatal(err)
			}
			if err := s.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			task, _ = w.task(a.TaskID)
			task.PlanMaxAttempts = 3
			task.Attempts = append([]Attempt{{ID: "prior1", Status: "completed"}, {ID: "prior2", Status: "completed"}}, task.Attempts...)
			if mode == "no-reviewer" {
				w.Planning.Reviewer = nil
			}
			raw, _ = json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			m, err := s.managedAttempt(a.ID)
			if err != nil {
				t.Fatal(err)
			}
			r := PlanningRequest{Schema: 1, EventID: "delivery-recovery", Revision: w.Revision, Task: a.TaskID, Attempt: a.Attempt, ResultCommit: m.Result, ConfirmRecovery: true, Reason: "Operator authorizes correction after examining incomplete result", RecoveryInstruction: "Perform the missing autonomous acceptance trial and retain all evidence; independent review remains mandatory."}
			switch mode {
			case "wrong-result":
				r.ResultCommit = m.Base
			case "wrong-attempt":
				r.Attempt = "prior2"
			case "unconfirmed":
				r.ConfirmRecovery = false
			}
			after, err := s.planningChange(w.ID, "authorize-recovery", r)
			if mode != "valid" {
				if err == nil {
					t.Fatal("unsafe recovery accepted")
				}
				got, _ := s.get(w.ID)
				if got.Revision != w.Revision {
					t.Fatal("refusal mutated work")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			task, _ = after.task(a.TaskID)
			if task.PlanMaxAttempts != 4 || len(task.Attempts) != 3 || task.Status != "blocked" || task.IndependentReview != nil || task.CorrectiveRecovery.Result != m.Result || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
				t.Fatal("history, review or allowance corrupted")
			}
			replay, err := s.planningChange(w.ID, "authorize-recovery", r)
			if err != nil || replay.Revision != after.Revision {
				t.Fatal("replay not idempotent", err)
			}
			r.EventID = "second-grant"
			r.Revision = after.Revision
			if _, err = s.planningChange(w.ID, "authorize-recovery", r); err == nil {
				t.Fatal("second exceptional grant accepted")
			}
		})
	}
}
