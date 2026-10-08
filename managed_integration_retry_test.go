//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

func integrationRetryFixture(t *testing.T, legacy ...bool) (*Store, Work, Agent, PlanningRequest) {
	t.Helper()
	s, w := managedFixture(t)
	var e error
	w, e = s.mutate(w.ID, "test.policy", "retry-control", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].ValidationPolicy = automaticPolicy("python3", "-c", "from pathlib import Path; assert Path('ready.txt').exists()")
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	if len(legacy) > 0 && legacy[0] {
		raw, _ := json.Marshal(map[string]string{"candidate_commit": w.Planning.Repository.Candidate})
		w, e = s.mutate(w.ID, "managed.integrated", "legacy-base-publication", w.Revision, raw, func(w *Work) error { return nil })
		if e != nil {
			t.Fatal(e)
		}
	}
	a := managedCompleted(t, s, w, "first", "initial\n")
	a.Ended = now()
	raw, _ := json.Marshal(a)
	if _, e = s.db.Exec("UPDATE agents SET body=? WHERE id=?", raw, a.ID); e != nil {
		t.Fatal(e)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "blocked" || managedReviewCalls(t, s) != 0 {
		t.Fatal("failed control published")
	}
	if len(legacy) > 0 && legacy[0] {
		if e = s.managedFailure(a, strings.Split(w.Tasks[0].Blocker, " ; diagnostic : ")[0]); e != nil {
			t.Fatal(e)
		}
		w, _ = s.get(w.ID)
	}
	b := managedCompleted(t, s, w, "second", "initial\n")
	if e = os.WriteFile(filepath.Join(b.CWD, "ready.txt"), []byte("available\n"), 0600); e != nil {
		t.Fatal(e)
	}
	if e = s.integrateManagedAttempt(b); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[1].Status != "accepted" {
		t.Fatal(w.Tasks[1].Blocker)
	}
	item, _ := s.managedAttempt(a.ID)
	return s, w, a, PlanningRequest{Schema: 1, EventID: "integration-retry", Revision: w.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ResultCommit: item.Result, ExpectedCandidate: w.Planning.Repository.Candidate, Reason: "La base validée fournit désormais la précondition du contrôle."}
}

func TestIntegrationRetryPublicPreservesResultAndReviews(t *testing.T) {
	s, w, a, r := integrationRetryFixture(t)
	item, _ := s.managedAttempt(a.ID)
	oldReason := w.Tasks[0].Blocker
	calls := w.Planning.Reviewer.Calls
	after, e := s.planningChange(w.ID, "retry-integration", r)
	if e != nil {
		t.Fatal(e)
	}
	if after.Tasks[0].Status != "blocked" || len(after.Tasks[0].Attempts) != 1 || after.Planning.Reviewer.Calls != calls || len(after.Tasks[0].IntegrationRetries) != 1 || after.Tasks[0].IntegrationRetries[0].Failure != oldReason {
		t.Fatal("retry accepted task or lost history")
	}
	if _, e = s.planningChange(w.ID, "retry-integration", r); e != nil {
		t.Fatal("idempotent retry", e)
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	if e = reopened.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ = reopened.get(w.ID)
	accepted := after.Tasks[0]
	if accepted.Status != "accepted" || accepted.IndependentReview == nil || accepted.IndependentReview.State != "passed" || accepted.IndependentReview.CandidateSHA != after.Planning.Repository.Candidate || len(accepted.Attempts) != 1 || len(accepted.IntegrationRetries) != 1 {
		t.Fatal("not reviewed", accepted.Blocker)
	}
	got, _ := reopened.managedAttempt(a.ID)
	if got.Result != item.Result {
		t.Fatal("producer result replaced")
	}
	if after.Planning.Reviewer.Calls <= calls || len(accepted.AutoValidation.Controls) != 1 || after.Tasks[1].AutoValidation.CandidateSHA != accepted.AutoValidation.CandidateSHA {
		t.Fatal("cumulative controls/review missing")
	}
	if _, e = reopened.planningChange(w.ID, "retry-integration", r); e != nil {
		t.Fatal("replay after publication", e)
	}
	final, _ := reopened.get(w.ID)
	if final.Revision != after.Revision || final.Planning.Reviewer.Calls != after.Planning.Reviewer.Calls {
		t.Fatal("replay consumed work")
	}
	paths, _ := filepath.Glob(filepath.Join(w.Planning.Repository.Storage, "diagnostics", "control-failure-*.json"))
	if len(paths) != 1 {
		t.Fatal("old diagnostic lost")
	}
}

func TestIntegrationRetryRejectsChangedInputs(t *testing.T) {
	for _, mode := range []string{"result", "attempt", "candidate", "budget", "active", "checkpoint", "diagnostic", "policy", "same-base", "same-pair", "legacy-unproven"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, r := integrationRetryFixture(t)
			originalItem, _ := s.managedAttempt(a.ID)
			switch mode {
			case "result":
				r.ResultCommit = strings.Repeat("a", 40)
			case "attempt":
				r.Attempt = "old-attempt"
			case "candidate":
				r.ExpectedCandidate = strings.Repeat("b", 40)
			case "active":
				a.Status = "running"
				raw, _ := json.Marshal(a)
				s.db.Exec("UPDATE agents SET status='running',body=? WHERE id=?", raw, a.ID)
			case "checkpoint":
				os.WriteFile(filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID, "candidate.json"), []byte(`{}`), 0600)
			case "diagnostic":
				paths, _ := filepath.Glob(filepath.Join(w.Planning.Repository.Storage, "diagnostics", "*.json"))
				os.WriteFile(paths[0], []byte(`{}`), 0600)
			default:
				w, _ = s.mutate(w.ID, "test.guard", "guard-"+mode, w.Revision, []byte(`{}`), func(w *Work) error {
					switch mode {
					case "budget":
						w.Planning.Reviewer.Calls = w.Planning.Reviewer.MaxCalls
					case "policy":
						w.Tasks[0].ValidationPolicy.Controls[0].Command = []string{"git", "status"}
					case "same-base":
						w.Planning.Repository.Candidate = originalItem.Base
					case "same-pair":
						w.Tasks[0].IntegrationRetries = []IntegrationRetry{{Result: r.ResultCommit, Candidate: r.ExpectedCandidate}}
					case "legacy-unproven":
						w.Tasks[0].Blocker = strings.Split(w.Tasks[0].Blocker, " ; diagnostic : ")[0]
					}
					return nil
				})
				if mode == "legacy-unproven" {
					s.db.Exec("UPDATE managed_attempts SET detail=? WHERE agent_id=?", w.Tasks[0].Blocker, a.ID)
				}
				r.Revision = w.Revision
				r.ExpectedCandidate = w.Planning.Repository.Candidate
			}
			before, _ := s.get(w.ID)
			if _, e := s.planningChange(w.ID, "retry-integration", r); e == nil {
				t.Fatal("unsafe retry accepted")
			}
			after, _ := s.get(w.ID)
			item, _ := s.managedAttempt(a.ID)
			if after.Revision != before.Revision || after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls || item.State != "conflict" {
				t.Fatal("refusal mutated state")
			}
		})
	}
}

func TestIntegrationRetryBaseDriftDoesNotExecute(t *testing.T) {
	s, w, a, r := integrationRetryFixture(t)
	w, e := s.planningChange(w.ID, "retry-integration", r)
	if e != nil {
		t.Fatal(e)
	}
	calls := managedReviewCalls(t, s)
	w, e = s.mutate(w.ID, "test.drift", "drift", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].Criteria = append(w.Tasks[0].Criteria, "additional criterion")
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	if !strings.Contains(after.Tasks[0].Blocker, "contrat modifié") || managedReviewCalls(t, s) != calls {
		t.Fatal("drift was executed")
	}
	r.EventID = "second-retry"
	r.Revision = after.Revision
	if _, e = s.planningChange(w.ID, "retry-integration", r); e == nil {
		t.Fatal("same pair rearmed")
	}
}

func TestIntegrationRetryLegacyKeepsMissingEvidenceExplicit(t *testing.T) {
	s, w, a, r := integrationRetryFixture(t, true)
	next, e := s.planningChange(w.ID, "retry-integration", r)
	if e != nil {
		t.Fatal(e)
	}
	retry := next.Tasks[0].IntegrationRetries[0]
	if !retry.Legacy || !strings.HasPrefix(retry.Evidence, "event:") || retry.EvidenceDigest == "" || retry.Previous == retry.Candidate {
		t.Fatal("historical uncertainty hidden", retry)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	if after.Tasks[0].Status != "accepted" {
		t.Fatal(after.Tasks[0].Blocker)
	}
}

func TestIntegrationRetryConcurrentReservations(t *testing.T) {
	s, w, _, r := integrationRetryFixture(t)
	requests := []PlanningRequest{r, r}
	requests[1].EventID = "concurrent-other"
	var wg sync.WaitGroup
	results := make(chan error, 2)
	for _, request := range requests {
		wg.Add(1)
		go func(request PlanningRequest) {
			defer wg.Done()
			_, err := s.planningChange(w.ID, "retry-integration", request)
			results <- err
		}(request)
	}
	wg.Wait()
	close(results)
	success := 0
	for err := range results {
		if err == nil {
			success++
		}
	}
	after, _ := s.get(w.ID)
	if success != 1 || len(after.Tasks[0].IntegrationRetries) != 1 || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
		t.Fatal("duplicate reservation", success)
	}
}
