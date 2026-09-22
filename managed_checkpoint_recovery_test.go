//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// managedSimulateCrashAfterTestsBeforeReview reproduces a process crash right
// after validation controls ran and their candidate was durably checkpointed
// (proofs/<agent>/candidate.json, see preparedManagedCandidate) but before the
// independent review was even reserved. E4: resuming from here must not rerun
// controls a second time.
func managedSimulateCrashAfterTestsBeforeReview(t *testing.T, s *Store, w Work, a Agent) {
	t.Helper()
	managedSimulateCrashAfterReportCommit(t, s, w, a)
	item, e := s.managedAttempt(a.ID)
	if e != nil {
		t.Fatal(e)
	}
	task, e := w.task(a.TaskID)
	if e != nil {
		t.Fatal(e)
	}
	repo, e := s.managedRepository(w)
	if e != nil {
		t.Fatal(e)
	}
	if _, _, e = s.preparedManagedCandidate(w, task, a, item, repo); e != nil {
		t.Fatal(e)
	}
}

// TestManagedIntegrationResumesAfterTestsWithoutRerunningControls covers "arrêt
// après tests" : once controls already ran and passed, a restart must reuse
// that checkpoint and only reserve the still-pending review, never pay for the
// controls again.
func TestManagedIntegrationResumesAfterTestsWithoutRerunningControls(t *testing.T) {
	s, w := managedFixture(t)
	counter := filepath.Join(s.root, "control-calls")
	w, e := s.mutate(w.ID, "test.policy", "policy", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].ValidationPolicy = automaticPolicy("bash", "-c", "echo run >> "+counter)
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	a := managedCompleted(t, s, w, "first", "first\n")
	w, _ = s.get(w.ID)
	managedSimulateCrashAfterTestsBeforeReview(t, s, w, a)
	data, e := os.ReadFile(counter)
	if e != nil || strings.Count(string(data), "run\n") != 1 {
		t.Fatalf("controls did not run exactly once before the simulated crash : %q %v", string(data), e)
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	if e = reopened.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	data, e = os.ReadFile(counter)
	if e != nil || strings.Count(string(data), "run\n") != 1 {
		t.Fatalf("controls re-run after restart instead of reusing the checkpointed candidate : %q %v", string(data), e)
	}
	w2, _ := reopened.get(w.ID)
	task, _ := w2.task("first")
	if task.Status != "accepted" || managedReviewCalls(t, reopened) != 1 {
		t.Fatalf("resumed integration did not complete with exactly one review call : %+v calls=%d", task, managedReviewCalls(t, reopened))
	}
}
