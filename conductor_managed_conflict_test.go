//go:build linux

package main

import (
	"os"
	"path/filepath"
	"testing"
)

// A durable refusal is not a queued integration. Polling it must not reopen
// Git or repeat a preflight; only an explicit recovery may rearm it.
func TestConductorManagedConflictDoesNotReenterGit(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "pending\n")
	if err := s.managedFailure(a, managedReviewContextTooLarge); err != nil {
		t.Fatal(err)
	}
	before, _ := s.get(w.ID)
	itemBefore, _ := s.managedAttempt(a.ID)
	lock := filepath.Join(w.Planning.Repository.Storage, "operation.lock")
	if err := os.Remove(lock); err != nil {
		t.Fatal(err)
	}
	// If polling touches the Git lock at all, it now fails observably. This
	// directory is confined to the disposable fixture, never a live mission.
	if err := os.Mkdir(lock, 0700); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 3; i++ {
		s.conduct(a, "completed")
	}
	var logs int
	if err := s.db.QueryRow("SELECT count(*) FROM agent_logs WHERE agent_id=?", a.ID).Scan(&logs); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	itemAfter, _ := s.managedAttempt(a.ID)
	if logs != 0 || after.Revision != before.Revision || itemAfter.State != itemBefore.State || itemAfter.Detail != itemBefore.Detail || managedReviewCalls(t, s) != 0 {
		t.Fatalf("poll reopened a retained refusal: logs=%d revision=%d->%d state=%s detail=%s", logs, before.Revision, after.Revision, itemAfter.State, itemAfter.Detail)
	}
}
