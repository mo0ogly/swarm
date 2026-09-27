//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestFragmentReviewerReplacementPublicRecovery(t *testing.T) {
	for _, mode := range []string{"ready", "budget", "tampered", "not-selected", "active", "negative"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, r, p, _ := fragmentStoreFixture(t)
			ps, err := s.providers()
			if err != nil {
				t.Fatal(err)
			}
			provider := ps.Providers[w.Planning.Reviewer.Provider]
			ps.Providers["replacement"] = provider
			raw, _ := json.Marshal(ps)
			if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
				t.Fatal(err)
			}
			if err = s.pause(w.ID, true); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task(a.TaskID)
			r.State = "error"
			task.Status = "blocked"
			task.IndependentReview = &r
			w.Planning.Reviewer.Calls = 5
			w.Planning.Reviewer.MaxCalls = 5 + len(p.Packets) + p.ReservedFinalCalls
			// A public role selection changes the model, rather than editing a digest.
			raw, _ = json.Marshal(w)
			if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			_, route, err := resolveModel(provider, "exigeant", "planning")
			if err != nil {
				t.Fatal(err)
			}
			w, err = s.configureRoleModel(w.ID, RoleModelRequest{Schema: 1, EventID: "replace-reviewer", Revision: w.Revision, Reviewer: true, Provider: "replacement", Level: "exigeant", PolicyHash: route.PolicyHash})
			if err != nil {
				t.Fatal(err)
			}
			task, _ = w.task(a.TaskID)
			switch mode {
			case "budget":
				w.Planning.Reviewer.MaxCalls--
			case "tampered":
				os.WriteFile(filepath.Join(s.root, r.Report), []byte("changed"), 0600)
			case "not-selected":
				w.Planning.Reviewer.ModelSelection = nil
			case "active":
				task.IndependentReview.State = "running"
			case "negative":
				task.IndependentReview.State = "changes_requested"
			}
			raw, _ = json.Marshal(w)
			if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			original, err := os.ReadFile(filepath.Join(s.root, r.FragmentJournal.Journal))
			if err != nil {
				t.Fatal(err)
			}
			preview, pe := s.reviewRecoveryPreview(w.ID, a.TaskID)
			if mode == "ready" || mode == "budget" {
				if pe != nil || preview.Reusable != 0 || preview.Required != len(p.Packets)+p.ReservedFinalCalls {
					t.Fatalf("preview: %+v %v", preview, pe)
				}
			} else if pe == nil {
				t.Fatal("invalid evidence allowed")
			}
			_, err = s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "replace-retry", Revision: w.Revision, Task: a.TaskID, Reason: "Operator explicitly selected a replacement reviewer after transport failure"})
			if (err == nil) != (mode == "ready") {
				t.Fatalf("retry: %v", err)
			}
			after, _ := s.get(w.ID)
			at, _ := after.task(a.TaskID)
			if after.Planning.Reviewer.Calls != 5 || len(at.Attempts) != len(task.Attempts) {
				t.Fatal("calls or production attempts changed")
			}
			if mode == "ready" && (at.IndependentReview != nil || len(at.PreviousReviews) != 1 || at.PreviousReviews[0].ID != r.ID) {
				t.Fatal("old reviewer evidence reused")
			}
			if mode != "ready" && at.IndependentReview == nil {
				t.Fatal("refusal erased review")
			}
			retained, _ := os.ReadFile(filepath.Join(s.root, r.FragmentJournal.Journal))
			if string(retained) != string(original) {
				t.Fatal("old journal overwritten")
			}
		})
	}
}

func TestRoleModelRejectsQueuedFragmentReview(t *testing.T) {
	s, w, a, r, _, _ := fragmentStoreFixture(t)
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	r.State = "queued"
	task.IndependentReview = &r
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	_, route, err := resolveModel(ps.Providers[w.Planning.Reviewer.Provider], "standard", "planning")
	if err != nil {
		t.Fatal(err)
	}
	_, err = s.configureRoleModel(w.ID, RoleModelRequest{Schema: 1, EventID: "queued-change", Revision: w.Revision, Reviewer: true, Provider: w.Planning.Reviewer.Provider, Level: "standard", PolicyHash: route.PolicyHash})
	if err == nil {
		t.Fatal("queued review model changed")
	}
}

func TestReviewerReplacementHistoricalBaselineIsNotFreshAcceptance(t *testing.T) {
	s, w, a := managedBatchRuntimeFixture(t)
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	original := w.Planning.Reviewer.Provider
	cfg := w.Planning.Reviewer
	cfg.Provider = "replacement"
	cfg.ProviderDigest = "replacement-digest"
	cfg.ModelSelection = &RoleModel{Provider: cfg.Provider, ProviderDigest: cfg.ProviderDigest}
	task := &w.Tasks[0]
	if err := s.independentReviewGuard(&w, task); err == nil {
		t.Fatal("old verdict accepted as current after reviewer replacement")
	}
	if err := s.historicalReviewBaselineGuard(w, task); err != nil {
		t.Fatal("historical evidence lost", err)
	}
	if w.Planning.Reviewer.Provider == original {
		t.Fatal("historical check changed current reviewer")
	}
	if err := os.WriteFile(filepath.Join(s.root, task.IndependentReview.Context), []byte("tampered"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.historicalReviewBaselineGuard(w, task); err == nil {
		t.Fatal("tampered baseline accepted")
	}
}
