//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRecoveredResultInterruptedUnpaidPreflightCanBeCorrected(t *testing.T) {
	s, w, a, r := recoveredResultFixture(t)
	if err := os.WriteFile(filepath.Join(a.CWD, "indivisible.txt"), []byte(strings.Repeat("original oversized context\n", 10000)), 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	r.ResultTree = gitTest(t, a.CWD, "write-tree")
	w, err := s.planningChange(w.ID, "submit-recovered-result", r)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	if task.Status != "blocked" || task.IndependentReview != nil || managedReviewCalls(t, s) != 0 || !strings.Contains(task.Blocker, managedReviewContextTooLarge) {
		t.Fatal("fixture did not reach unpaid context preflight", task.Status, task.Blocker)
	}
	old := task.RecoveredResult.Result
	bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
	if err := os.WriteFile(filepath.Join(a.CWD, "indivisible.txt"), []byte("Explicitly corrected bounded context\n"), 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	r.EventID = "correct-unpaid-preflight"
	r.Revision = w.Revision
	r.ResultTree = gitTest(t, a.CWD, "write-tree")
	for _, name := range []string{"candidate.json", "receipt.json"} {
		path := filepath.Join(w.Planning.Repository.Storage, "proofs", managedProofKey(task, a), name)
		original, e := os.ReadFile(path)
		if e != nil {
			t.Fatal(e)
		}
		if e = os.WriteFile(path, []byte(`{}`), 0600); e != nil {
			t.Fatal(e)
		}
		if _, e = s.planningChange(w.ID, "revise-recovered-result", r); e == nil {
			t.Fatal("altered unpaid checkpoint accepted", name)
		}
		unchanged, _ := s.get(w.ID)
		if unchanged.Revision != w.Revision || managedReviewCalls(t, s) != 0 {
			t.Fatal("rejected checkpoint changed state or calls")
		}
		if e = os.WriteFile(path, original, 0600); e != nil {
			t.Fatal(e)
		}
	}
	got, err := s.planningChange(w.ID, "revise-recovered-result", r)
	if err != nil {
		t.Fatal("confirmed stopped interrupted result must support correction of its retained unpaid preflight", err)
	}
	current, _ := got.task(a.TaskID)
	if current.Status != "accepted" || !s.acceptedFresh(&got, current, map[string]bool{}) || current.RecoveredResult.ReplacesResult != old || current.RecoveredResult.ProofKey == "" || len(current.Attempts) != 1 || current.Attempts[0].Status != "interrupted" || managedReviewCalls(t, s) != 1 {
		t.Fatal("correction changed history or omitted fresh verification", current.Status, current.Blocker)
	}
	if gitTest(t, bare, "rev-parse", "refs/swarm/attempts/"+a.ID) != old {
		t.Fatal("original result ref overwritten")
	}
	if _, err = s.planningChange(w.ID, "revise-recovered-result", r); err != nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("replay repeated paid review", err)
	}
}
