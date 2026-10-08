//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"testing"
)

func TestAutomaticValidationClosesRootAfterFreshReviewDespiteOldFailure(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	current.Planning.Failure = "old reviewer timeout"
	raw, _ := json.Marshal(current)
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, current.ID); err != nil {
		t.Fatal(err)
	}
	s.conduct(a, "completed")
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	root, _ := got.Planning.scope("root")
	if got.Tasks[0].Status != "accepted" || root.State != "closed" {
		t.Fatalf("fresh accepted result did not close root: task=%s root=%s", got.Tasks[0].Status, root.State)
	}
	if got.Planning.Failure != "old reviewer timeout" {
		t.Fatal("historical failure was erased")
	}
}

func TestAutomaticValidationDoesNotCloseRootWithoutReviewOrWithStaleProof(t *testing.T) {
	t.Run("missing-review", func(t *testing.T) {
		s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
		current, _ := s.get(w.ID)
		current.Tasks[0].IndependentReview = nil
		raw, _ := json.Marshal(current)
		if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, current.ID); err != nil {
			t.Fatal(err)
		}
		accepted, _ := s.runAutomaticValidation(a, report)
		if accepted {
			t.Fatal("result accepted without independent review")
		}
		got, _ := s.get(w.ID)
		root, _ := got.Planning.scope("root")
		if root.State == "closed" {
			t.Fatal("root closed without independent review")
		}
	})

	t.Run("stale-proof", func(t *testing.T) {
		s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
		s.conduct(a, "completed")
		if err := os.WriteFile(filepathJoin(s.root, report), []byte("changed after acceptance\n"), 0600); err != nil {
			t.Fatal(err)
		}
		got, err := s.reconcilePlanningProofs(mustGetWork(t, s, w.ID))
		if err != nil {
			t.Fatal(err)
		}
		root, _ := got.Planning.scope("root")
		if root.State == "closed" {
			t.Fatal("root remained closed with stale proof")
		}
	})
}

func filepathJoin(root, path string) string { return root + string(os.PathSeparator) + path }

func mustGetWork(t *testing.T, s *Store, id string) Work {
	t.Helper()
	w, err := s.get(id)
	if err != nil {
		t.Fatal(err)
	}
	return w
}
