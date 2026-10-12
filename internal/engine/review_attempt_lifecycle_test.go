//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestNewProductionArchivesPriorIndependentReview(t *testing.T) {
	s, w, request := correctiveFixture(t)
	w, err := s.planningChange(w.ID, "authorize-recovery", request)
	if err != nil {
		t.Fatal(err)
	}
	old := *w.Tasks[0].IndependentReview
	beforeCalls := w.Planning.Reviewer.Calls
	if err = s.apply(&w, "task.update", Request{ID: w.Tasks[0].ID, Status: "running", Owner: "worker"}); err != nil {
		t.Fatal(err)
	}
	task := w.Tasks[0]
	if task.IndependentReview != nil || len(task.PreviousReviews) != 1 || !reflect.DeepEqual(task.PreviousReviews[0], old) || len(task.Attempts) != 4 || w.Planning.Reviewer.Calls != beforeCalls {
		t.Fatal("new attempt retained an old current opinion or lost history")
	}
}

func TestNewProductionPreservesActiveReview(t *testing.T) {
	for _, state := range []string{"running", "queued"} {
		t.Run(state, func(t *testing.T) {
			s, w, request := correctiveFixture(t)
			w, err := s.planningChange(w.ID, "authorize-recovery", request)
			if err != nil {
				t.Fatal(err)
			}
			w.Tasks[0].IndependentReview.State = state
			before, err := json.Marshal(w)
			if err != nil {
				t.Fatal(err)
			}
			if err = s.apply(&w, "task.update", Request{ID: w.Tasks[0].ID, Status: "running", Owner: "worker"}); err == nil {
				t.Fatal("new production replaced an active review")
			}
			after, _ := json.Marshal(w)
			if !bytes.Equal(before, after) {
				t.Fatal("refused production changed review, attempts or budgets")
			}
		})
	}
}

func TestLegacyPriorReviewDoesNotBlockPublicPreflight(t *testing.T) {
	for _, state := range []string{"changes_requested", "error", "running", "queued", "invalid", "current", "unattributed", "unfinished"} {
		t.Run(state, func(t *testing.T) {
			s, w, a := unpaidReviewFixture(t)
			old := IndependentReview{ID: "older-review", Attempt: "prior-attempt", Producer: "prior-producer", State: state, Finished: now()}
			w, err := s.mutate(w.ID, "test.legacy-review", "legacy-review", w.Revision, []byte(`{}`), func(current *Work) error {
				task, e := current.task(a.TaskID)
				if e != nil {
					return e
				}
				task.Attempts = append([]Attempt{{ID: "prior-attempt", Status: "completed", Ended: now()}}, task.Attempts...)
				if state == "current" {
					old.Attempt = a.Attempt
					old.State = "changes_requested"
				}
				if state == "unattributed" {
					old.Attempt = "not-a-prior-attempt"
					old.State = "changes_requested"
				}
				if state == "unfinished" {
					old.State = "error"
					old.Finished = ""
				}
				task.IndependentReview = &old
				return nil
			})
			if err != nil {
				t.Fatal(err)
			}
			mux := http.NewServeMux()
			s.registerPlanning(mux, func(out http.ResponseWriter, v any) { json.NewEncoder(out).Encode(v) }, func(out http.ResponseWriter, e error) { http.Error(out, e.Error(), 400) })
			out := httptest.NewRecorder()
			mux.ServeHTTP(out, httptest.NewRequest("GET", "/api/v1/planning?work="+w.ID+"&task="+a.TaskID+"&action=scope-preview", nil))
			allowed := state == "changes_requested" || state == "error"
			if (out.Code == http.StatusOK) != allowed {
				t.Fatalf("legacy preflight state=%s code=%d: %s", state, out.Code, out.Body.String())
			}
			after, _ := s.get(w.ID)
			if !reflect.DeepEqual(w, after) || managedReviewCalls(t, s) != 0 {
				t.Fatal("read-only preview changed evidence or spent a call")
			}
			if !allowed {
				absolute := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID, "receipt.json")
				receipt, err := os.ReadFile(absolute)
				if err != nil {
					t.Fatal(err)
				}
				raw, err := os.ReadFile(filepath.Join(filepath.Dir(absolute), "candidate.json"))
				if err != nil {
					t.Fatal(err)
				}
				var cp managedReviewCheckpoint
				if err = json.Unmarshal(raw, &cp); err != nil {
					t.Fatal(err)
				}
				path, err := filepath.Rel(s.root, absolute)
				if err != nil {
					t.Fatal(err)
				}
				if err = s.reviewManagedCandidate(w, a, cp.Candidate, path, receipt); err == nil {
					t.Fatal("invalid prior review replaced by a paid opinion")
				}
				after, _ = s.get(w.ID)
				if !reflect.DeepEqual(after, w) || managedReviewCalls(t, s) != 0 {
					t.Fatal("direct settlement changed invalid prior review or spent a call")
				}
				return
			}
			prepared, err := s.readManagedPreflightEvidence(w.ID, a.TaskID)
			if err != nil {
				t.Fatal(err)
			}
			absolute := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID, "receipt.json")
			data, err := os.ReadFile(absolute)
			if err != nil {
				t.Fatal(err)
			}
			path, err := filepath.Rel(s.root, absolute)
			if err != nil {
				t.Fatal(err)
			}
			fresh, err := s.beginManagedFragmentReview(w, a, prepared.Context, path, data)
			if err != nil {
				t.Fatal("new fragment claim blocked by older terminal review", err)
			}
			after, _ = s.get(w.ID)
			task, _ := after.task(a.TaskID)
			if fresh.Attempt != a.Attempt || task.IndependentReview.ID != fresh.ID || len(task.PreviousReviews) != 1 || !reflect.DeepEqual(task.PreviousReviews[0], old) || managedReviewCalls(t, s) != 0 {
				t.Fatal("old review not archived atomically or a call was spent")
			}
		})
	}
}

func TestLegacyPriorReviewPublicRetryKeepsCurrentCandidate(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	old := IndependentReview{ID: "older-review", Attempt: "prior-attempt", Producer: "prior-producer", State: "changes_requested", Finished: now()}
	w, err := s.mutate(w.ID, "test.legacy-review", "legacy-review", w.Revision, []byte(`{}`), func(current *Work) error {
		task, e := current.task(a.TaskID)
		if e != nil {
			return e
		}
		task.Attempts = append([]Attempt{{ID: old.Attempt, Status: "completed", Ended: now()}}, task.Attempts...)
		task.IndependentReview = &old
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	before, err := s.managedAttempt(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	mux := http.NewServeMux()
	s.registerPlanning(mux, func(out http.ResponseWriter, v any) { json.NewEncoder(out).Encode(v) }, func(out http.ResponseWriter, e error) { http.Error(out, e.Error(), 400) })
	request := PlanningRequest{Schema: 1, EventID: "legacy-public-retry", Revision: w.Revision, Task: a.TaskID, Reason: "Archive attributable historical opinion and retry the unchanged current candidate."}
	body, _ := json.Marshal(request)
	for i := 0; i < 2; i++ {
		out := httptest.NewRecorder()
		mux.ServeHTTP(out, httptest.NewRequest("POST", "/api/v1/planning?work="+w.ID+"&action=retry-review", bytes.NewReader(body)))
		if out.Code != http.StatusOK {
			t.Fatal(out.Code, out.Body.String())
		}
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	item, _ := s.managedAttempt(a.ID)
	if task.IndependentReview != nil || len(task.PreviousReviews) != 1 || !reflect.DeepEqual(task.PreviousReviews[0], old) || len(task.Attempts) != 2 || item.State != "integrating" || item.Result != before.Result || managedReviewCalls(t, s) != 0 {
		t.Fatal("retry replaced the candidate, lost history or spent a call")
	}
	if err = s.reconcileKnownMissionResult(a, "fixture-conductor"); err != nil {
		t.Fatal(err)
	}
	after, _ = s.get(w.ID)
	task, _ = after.task(a.TaskID)
	if task.Status != "accepted" || task.IndependentReview == nil || task.IndependentReview.Attempt != a.Attempt || len(task.PreviousReviews) != 1 || !reflect.DeepEqual(task.PreviousReviews[0], old) || len(task.Attempts) != 2 || managedReviewCalls(t, s) != 1 {
		t.Fatal("retry did not produce a fresh current review", task.Status, task.Blocker)
	}
	if err = s.reconcileKnownMissionResult(a, "fixture-conductor"); err != nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("duplicate review after replay", err)
	}
}

func TestLegacyPriorReviewCoordinationArchivesOnlyAfterValidDecision(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	old := IndependentReview{ID: "older-review", Attempt: "prior-attempt", Producer: "prior-producer", State: "changes_requested", Finished: now()}
	w, err := s.mutate(w.ID, "test.legacy-review", "legacy-review", w.Revision, []byte(`{}`), func(current *Work) error {
		task, e := current.task(a.TaskID)
		if e != nil {
			return e
		}
		task.Attempts = append([]Attempt{{ID: old.Attempt, Status: "completed", Ended: now()}}, task.Attempts...)
		task.IndependentReview = &old
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	w, r := planningClaim(t, s, w, task.ScopeID)
	preview, err := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
	if err != nil {
		t.Fatal(err)
	}
	proposal := ReviewCoordinationProposal{Candidate: preview.Candidate, Evidence: preview.Evidence, FinalReview: "Review all interactions and criteria of the complete candidate.", Lots: []ReviewCoordinationLot{{ID: "whole", Kind: "component", Objective: "Review all changed files", Files: preview.Files, Criteria: preview.Criteria}}}
	invalid := proposal
	invalid.Candidate = "different-candidate"
	raw, _ := json.Marshal(invalid)
	bad := r
	bad.EventID = "invalid-historical-coordination"
	bad.Operations = []PlanningOperation{{Kind: "review-plan", ID: a.TaskID, Deliverable: string(raw)}}
	if _, err = s.planningChange(w.ID, "decide", bad); err == nil {
		t.Fatal("wrong candidate accepted")
	}
	after, _ := s.get(w.ID)
	beforeTask, _ := w.task(a.TaskID)
	afterTask, _ := after.task(a.TaskID)
	if !reflect.DeepEqual(afterTask, beforeTask) || managedReviewCalls(t, s) != 0 {
		t.Fatal("invalid plan changed task evidence or spent a review call")
	}
	// The public planner rejection itself remains durable; use its current revision.
	r.Revision = after.Revision
	raw, _ = json.Marshal(proposal)
	r.Operations = []PlanningOperation{{Kind: "review-plan", ID: a.TaskID, Deliverable: string(raw)}}
	after, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = after.task(a.TaskID)
	if task.IndependentReview != nil || len(task.PreviousReviews) != 1 || !reflect.DeepEqual(task.PreviousReviews[0], old) || task.ReviewCoordination == nil || task.ReviewCoordination.State != "validated_not_executed" || len(task.Attempts) != 2 || managedReviewCalls(t, s) != 0 {
		t.Fatal("coordination did not atomically retain history without execution")
	}
	if _, err = s.planningChange(w.ID, "decide", r); err != nil {
		t.Fatal(err)
	}
	replay, _ := s.get(w.ID)
	if !reflect.DeepEqual(replay, after) {
		t.Fatal("decision replay duplicated archival")
	}
}

func TestPreviousRecoveryReviewUsesExactArchivedAttribution(t *testing.T) {
	original := IndependentReview{ID: "original", Producer: "producer", Attempt: "attempt", State: "changes_requested"}
	unrelated := IndependentReview{ID: "other", Producer: "different", Attempt: "attempt", State: "passed"}
	task := Task{RecoveredResult: &RecoveredResult{Agent: "producer", Attempt: "attempt"}, PreviousReviews: []IndependentReview{original, unrelated}}
	if got := previousRecoveryReview(&task); got == nil || !reflect.DeepEqual(*got, original) {
		t.Fatal("unrelated newer opinion selected")
	}
	latest := original
	latest.ID = "latest"
	task.PreviousReviews = append(task.PreviousReviews, latest)
	if got := previousRecoveryReview(&task); got == nil || got.ID != latest.ID {
		t.Fatal("latest exact historical opinion missing")
	}
	task.RecoveredResult.Attempt = "different-attempt"
	if previousRecoveryReview(&task) != nil {
		t.Fatal("other attempt misattributed")
	}
	task.RecoveredResult.Attempt = ""
	task.RecoveredResult.Agent = ""
	task.PreviousReviews = append(task.PreviousReviews, IndependentReview{ID: "blank"})
	if previousRecoveryReview(&task) != nil {
		t.Fatal("blank attribution accepted")
	}
	task.IndependentReview = &unrelated
	if previousRecoveryReview(&task) != task.IndependentReview {
		t.Fatal("current evidence substituted with history")
	}
}
