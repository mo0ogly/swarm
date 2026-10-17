//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// A corrective handoff must not replace the retained refusal merely to discover
// that the new evidence cannot reach its reviewer. Exercise the public planning
// operation with isolated stores and the ordinary reviewer fixture.
func TestRecoveredResultEvidencePreflightPreservesStateAndAllowsCorrection(t *testing.T) {
	for _, mode := range []string{"submit", "revise"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, req := recoveredResultFixture(t)
			action := "submit-recovered-result"
			if mode == "revise" {
				managedReviewMode(t, s, "fail")
				var err error
				w, err = s.planningChange(w.ID, action, req)
				if err != nil {
					t.Fatal(err)
				}
				action = "revise-recovered-result"
				req.ReviewID = w.Tasks[0].IndependentReview.ID
			}
			task, _ := w.task(a.TaskID)
			d := completeDelivery(task, a)
			d.Criteria[0].Evidence = []string{"docs/evidence-one.txt", "docs/evidence-two.txt"}
			write := func(name, text string) {
				t.Helper()
				if err := os.WriteFile(filepath.Join(a.CWD, name), []byte(text), 0600); err != nil {
					t.Fatal(err)
				}
			}
			raw, _ := json.Marshal(d)
			write("docs/first.delivery.json", string(raw))
			// Each file fits on its own; their union exceeds the task capacity.
			write("docs/evidence-one.txt", strings.Repeat("a", 70*1024))
			write("docs/evidence-two.txt", strings.Repeat("b", 70*1024))
			gitTest(t, a.CWD, "add", "-A")
			req.EventID = "oversized-correction"
			req.Revision = w.Revision
			req.ResultTree = gitTest(t, a.CWD, "write-tree")
			beforeItem, _ := s.managedAttempt(a.ID)
			beforeCalls := managedReviewCalls(t, s)
			if _, err := s.planningChange(w.ID, action, req); err == nil || !strings.Contains(err.Error(), "128 Kio") {
				t.Fatal("unreviewable repair not refused at source preflight", err)
			}
			after, _ := s.get(w.ID)
			afterItem, _ := s.managedAttempt(a.ID)
			if !reflect.DeepEqual(w, after) || !reflect.DeepEqual(beforeItem, afterItem) || managedReviewCalls(t, s) != beforeCalls {
				t.Fatal("preflight changed result, review, revision, attempt or budget")
			}
			// Correct the files, not the capacity, and re-examine the exact tree.
			write("docs/evidence-one.txt", "Original execution evidence\n")
			write("docs/evidence-two.txt", "Explicit origin and limits\n")
			gitTest(t, a.CWD, "add", "-A")
			req.EventID = "bounded-correction"
			req.ResultTree = gitTest(t, a.CWD, "write-tree")
			managedReviewMode(t, s, "pass")
			accepted, err := s.planningChange(w.ID, action, req)
			if err != nil {
				t.Fatal(err)
			}
			if accepted.Tasks[0].Status != "accepted" || managedReviewCalls(t, s) != beforeCalls+1 || len(accepted.Tasks[0].Attempts) != 1 {
				t.Fatal("corrected evidence did not pass ordinary checks and review")
			}
		})
	}
}

func TestRecoveredResultExplicitCorrectionAfterUnknown(t *testing.T) {
	for _, mode := range []string{"valid", "confirmation", "unchanged", "stale", "active", "tampered"} {
		t.Run(mode, func(t *testing.T) {
			s, first, a := selectionCapacityLegacyFixture(t, "root")
			w := first
			item, err := s.managedAttempt(a.ID)
			if err != nil {
				t.Fatal(err)
			}
			req := PlanningRequest{Schema: 1, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ConfirmRecovery: true}
			req.ResultTree = gitTest(t, a.CWD, "rev-parse", item.Result+"^{tree}")
			beforeCalls := first.Planning.Reviewer.Calls
			task, _ := first.task(a.TaskID)
			prior := *task.IndependentReview
			if prior.State != "unknown" || prior.Finished == "" || task.Status != "blocked" {
				t.Fatal("missing valid terminal uncertainty", prior.State, prior.Finished, task.Status, prior.Reason)
			}
			original, err := os.ReadFile(filepath.Join(s.root, prior.Context))
			if err != nil {
				t.Fatal(err)
			}
			next := req
			next.EventID = "evidence-after-unknown"
			next.Revision = first.Revision
			next.ReviewID = prior.ID
			next.Reason = "Complete missing declared original evidence; preserve unknown, attempts and spending; fresh controls and independent review required."
			if mode != "unchanged" {
				// Keep this fixture's fresh delivery within ordinary single-call transport.
				// Original large candidate evidence remains immutable in its retained result.
				if err = os.WriteFile(filepath.Join(a.CWD, "value.txt"), []byte("bounded corrected value\n"), 0600); err != nil {
					t.Fatal(err)
				}
				if err = os.WriteFile(filepath.Join(a.CWD, "other.txt"), []byte("bounded corrected auxiliary\n"), 0600); err != nil {
					t.Fatal(err)
				}
				if err = os.WriteFile(filepath.Join(a.CWD, "docs/missing-original.txt"), []byte("Complete attributed original that was absent from the first review."), 0600); err != nil {
					t.Fatal(err)
				}
				if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.review-context.json"), []byte(`{"version":1,"files":["docs/missing-original.txt"]}`), 0600); err != nil {
					t.Fatal(err)
				}
				manifest := ManagedDelivery{Version: 1, Task: a.TaskID, Attempt: a.Attempt, Contract: reviewContract(task), Outcome: "complete"}
				for i := range task.Criteria {
					ids := []string{}
					for _, control := range task.ValidationPolicy.Controls {
						for _, index := range control.Criteria {
							if index == i+1 {
								ids = append(ids, control.ID)
							}
						}
					}
					manifest.Criteria = append(manifest.Criteria, DeliveryCriterion{Index: i + 1, Status: "pass", Reason: "Corrected bounded fixture source; engine checks and independent review remain required.", Controls: ids, Evidence: []string{"value.txt", "docs/missing-original.txt"}})
				}
				deliveryBytes, _ := json.Marshal(manifest)
				if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), deliveryBytes, 0600); err != nil {
					t.Fatal(err)
				}
				gitTest(t, a.CWD, "add", "-A")
				next.ResultTree = gitTest(t, a.CWD, "write-tree")
			}
			// Exercise the JSON producer contract before the field exists in Go.
			raw, _ := json.Marshal(next)
			var wire map[string]any
			json.Unmarshal(raw, &wire)
			wire["confirm_review_unknown_correction"] = mode != "confirmation"
			raw, _ = json.Marshal(wire)
			if err = json.Unmarshal(raw, &next); err != nil {
				t.Fatal(err)
			}
			switch mode {
			case "stale":
				next.Revision--
			case "active":
				first, err = s.mutate(first.ID, "fixture.active-review", "fixture-active-unknown", first.Revision, []byte(`{}`), func(current *Work) error {
					task, _ := current.task(a.TaskID)
					task.IndependentReview.State = "running"
					return nil
				})
				if err != nil {
					t.Fatal(err)
				}
				next.Revision = first.Revision
			case "tampered":
				if err = os.WriteFile(filepath.Join(s.root, prior.Context), append(append([]byte{}, original...), ' '), 0600); err != nil {
					t.Fatal(err)
				}
			}
			before, _ := s.get(w.ID)
			beforeRaw, _ := json.Marshal(before)
			itemBefore, _ := s.managedAttempt(a.ID)
			managedReviewMode(t, s, "pass")
			after, err := s.planningChange(w.ID, "revise-recovered-result", next)
			if mode != "valid" {
				if err == nil {
					t.Fatal("invalid unknown correction admitted", mode)
				}
				current, _ := s.get(w.ID)
				currentRaw, _ := json.Marshal(current)
				itemAfter, _ := s.managedAttempt(a.ID)
				if string(beforeRaw) != string(currentRaw) || !reflect.DeepEqual(itemBefore, itemAfter) || current.Planning.Reviewer.Calls != beforeCalls {
					t.Fatal("refusal changed state or spending", mode)
				}
				return
			}
			if err != nil {
				t.Fatal("valid explicit evidence correction blocked", err)
			}
			got, _ := after.task(a.TaskID)
			if got.Status != "accepted" || got.RecoveredResult.PriorReview != prior.ID || got.IndependentReview.CandidateSHA == prior.CandidateSHA || len(got.Attempts) != len(task.Attempts) || after.Planning.Reviewer.Calls != beforeCalls+1 {
				t.Fatal("correction bypassed fresh acceptance or changed attempts")
			}
			kept, err := os.ReadFile(filepath.Join(s.root, prior.Context))
			if err != nil || string(kept) != string(original) {
				t.Fatal("original uncertainty evidence changed", err)
			}
			observed, err := os.ReadFile(filepath.Join(s.root, "review-fixture/observed.json"))
			if err != nil {
				t.Fatal(err)
			}
			var received struct {
				Context managedReviewContext `json:"context"`
			}
			json.Unmarshal(observed, &received)
			found := false
			for _, source := range received.Context.Sources {
				if source.Path == "docs/missing-original.txt" && source.Content == "Complete attributed original that was absent from the first review." {
					found = true
				}
			}
			if !found {
				t.Fatal("fresh reviewer did not receive declared original")
			}
			if _, err = s.planningChange(w.ID, "revise-recovered-result", next); err != nil || managedReviewCalls(t, s) != 1 {
				t.Fatal("replay duplicated review", err)
			}
		})
	}
}
