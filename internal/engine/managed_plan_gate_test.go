//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// Fixtures only: reproduce the imported plan's persisted phases, which the
// older managed fixtures (without PlanChecks) did not exercise.
func managedPlanGateFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s, w := managedFixture(t)
	task := &w.Tasks[0]
	task.PlanChecks = map[string]string{"plan-entry": "entry", "plan-validation": "validation", "plan-delivery": "delivery", "plan-criterion-1": "validation"}
	policy := *task.ValidationPolicy
	policy.Controls = nil
	for _, id := range []string{"plan-entry", "plan-validation", "plan-delivery", "plan-criterion-1"} {
		control := automaticPolicy("git", "diff", "--exit-code").Controls[0]
		control.ID = id
		policy.Controls = append(policy.Controls, control)
	}
	task.ValidationPolicy = &policy
	w.Tasks[1].Depends = []string{task.ID}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	return s, w
}

func TestManagedImportedPlanPhasesProduceFreshAcceptance(t *testing.T) {
	s, w := managedPlanGateFixture(t)
	a := managedCompleted(t, s, w, "first", "phase-aware\n")
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	task := &after.Tasks[0]
	if task.Status != "accepted" || !s.acceptedFresh(&after, task, map[string]bool{}) {
		t.Fatalf("integrated plan still blocks its dependent: status=%s blocker=%s", task.Status, task.Blocker)
	}
	if err := requiredPlanChecks(task, task.Gate.Document); err != nil {
		t.Fatal(err)
	}
	if len(task.Attempts) != 1 || managedReviewCalls(t, s) != 1 {
		t.Fatal("unexpected retry or paid review")
	}
}

func TestRequalifyExplicitManagedPlanPhaseMismatch(t *testing.T) {
	for _, mode := range []string{"confirmed", "unconfirmed", "wrong-review", "fresh"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedPlanGateFixture(t)
			a := managedCompleted(t, s, w, "first", "reviewed\n")
			if err := s.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			task := &w.Tasks[0]
			oldReview := *task.IndependentReview
			oldReceipt, err := os.ReadFile(filepath.Join(s.root, task.AutoValidation.Receipt))
			if err != nil {
				t.Fatal(err)
			}
			if mode != "fresh" {
				var doc map[string]any
				json.Unmarshal(task.Gate.Document, &doc)
				for _, c := range doc["checks"].([]any) {
					c.(map[string]any)["gate"] = "delivery"
				}
				task.Gate.Document, _ = json.Marshal(doc)
				task.Gate.Evaluation, err = evaluate(task.Gate.Document, s.root, "delivery")
				if err != nil {
					t.Fatal(err)
				}
			}
			body, _ := json.Marshal(w)
			s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID)
			a.Ended = now()
			body, _ = json.Marshal(a)
			s.db.Exec("UPDATE agents SET body=? WHERE id=?", body, a.ID)
			if err := s.independentReviewGuard(&w, task); err != nil {
				t.Fatal("fixture review unexpectedly stale", err)
			}
			item, _ := s.managedAttempt(a.ID)
			r := PlanningRequest{Schema: 1, EventID: "repair-plan-phases", Revision: w.Revision, Task: task.ID, Agent: a.ID, Attempt: a.Attempt, ResultCommit: item.Result, ExpectedCandidate: w.Planning.Repository.Candidate, ReviewID: oldReview.ID, ConfirmRecovery: mode != "unconfirmed", Reason: "Requalify retained candidate after generated plan gate phase mismatch"}
			if mode == "wrong-review" {
				r.ReviewID = "other"
			}
			after, err := s.planningChange(w.ID, "requalify", r)
			if mode != "confirmed" {
				if err == nil {
					t.Fatal("unsafe or unnecessary requalification accepted")
				}
				current, _ := s.get(w.ID)
				if current.Revision != w.Revision || len(current.Tasks[0].Requalifications) != 0 {
					t.Fatal("rejection mutated history")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			prior := after.Tasks[0].Requalifications[0]
			if after.Tasks[0].Status != "blocked" || prior.Gate == nil || prior.Review.ID != oldReview.ID || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
				t.Fatal("history lost or reservation grants acceptance")
			}
			if err = s.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			after, _ = s.get(w.ID)
			fresh := &after.Tasks[0]
			if !s.acceptedFresh(&after, fresh, map[string]bool{}) || fresh.IndependentReview.ID == oldReview.ID || len(fresh.Attempts) != 1 || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls+1 {
				t.Fatal("fresh controls and review did not qualify retained attempt", fresh.Blocker)
			}
			item, _ = s.managedAttempt(a.ID)
			if item.Result != r.ResultCommit {
				t.Fatal("retained result changed")
			}
			saved, _ := os.ReadFile(filepath.Join(s.root, prior.Validation.Receipt))
			if string(saved) != string(oldReceipt) {
				t.Fatal("historical receipt overwritten")
			}
			if _, err = s.planningChange(w.ID, "requalify", r); err != nil {
				t.Fatal("idempotent replay", err)
			}
		})
	}
}
