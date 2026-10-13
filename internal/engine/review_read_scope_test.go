//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"testing"
)

func TestIndependentReviewReadScopeReusesOnlyItsOwnSnapshot(t *testing.T) {
	s := storeTest(t)
	if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(s.root, "docs/report.md")
	raw := []byte("reviewed original")
	if err := os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	task := Task{ID: "t1", Title: "original", Attempts: []Attempt{{ID: "attempt1"}}, IndependentReview: &IndependentReview{State: "passed", Attempt: "attempt1", Report: "docs/report.md", Digest: hash(raw)}}
	task.IndependentReview.Contract = reviewContract(&task)
	w := Work{ID: "work", Planning: &PlanningState{Reviewer: &ReviewerConfig{}}, Tasks: []Task{task}}
	// A read is one immutable projection, like request-local gate digests. It
	// neither updates storage nor supplies a cached approval to a later operation.
	scope := s.readScope()
	if err := scope.independentReviewGuard(&w, &task); err != nil {
		t.Fatal(err)
	}
	if err := os.Remove(path); err != nil {
		t.Fatal(err)
	}
	if err := scope.independentReviewGuard(&w, &task); err != nil {
		t.Fatalf("same projection repeats physical guard: %v", err)
	}
	if err := s.readScope().independentReviewGuard(&w, &task); err == nil {
		t.Fatal("next projection reused missing evidence")
	}
	if err := s.independentReviewGuard(&w, &task); err == nil {
		t.Fatal("live acceptance path reused read approval")
	}
	// An error also belongs only to its projection; restoring a file must make
	// the next read succeed without converting the already observed refusal.
	failed := s.readScope()
	if err := failed.independentReviewGuard(&w, &task); err == nil {
		t.Fatal("missing report approved")
	}
	if err := os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	if err := failed.independentReviewGuard(&w, &task); err == nil {
		t.Fatal("refusal lost in same projection")
	}
	if err := s.readScope().independentReviewGuard(&w, &task); err != nil {
		t.Fatal(err)
	}
	for _, variant := range []string{"contract", "attempt", "candidate", "state", "historical"} {
		t.Run(variant, func(t *testing.T) {
			clone := task
			review := *task.IndependentReview
			clone.IndependentReview = &review
			work := w
			switch variant {
			case "contract":
				clone.Title = "changed"
			case "attempt":
				clone.Attempts = []Attempt{{ID: "attempt2"}}
			case "candidate":
				planning := *w.Planning
				planning.Repository = &ManagedRepository{Candidate: "different"}
				work.Planning = &planning
			case "state":
				clone.IndependentReview.State = "changes_requested"
			case "historical":
				clone.IndependentReview.Workflow = &AgentWorkflow{Project: &ProjectContext{SHA256: "old-project"}}
				if err := scope.independentReviewGuardVersion(&work, &clone, true); err != nil {
					t.Fatal(err)
				}
			}
			if err := scope.independentReviewGuardVersion(&work, &clone, false); err == nil {
				t.Fatal("changed guard reused approval")
			}
		})
	}
}
