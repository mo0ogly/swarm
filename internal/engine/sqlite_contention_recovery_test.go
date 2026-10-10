//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"testing"
	"time"
)

func sqliteRecoveryEventCount(t *testing.T, s *Store, work, kind, id string) int {
	t.Helper()
	events, err := s.events(work)
	if err != nil {
		t.Fatal(err)
	}
	count := 0
	for _, event := range events {
		if event.Kind == kind && (id == "" || event.ID == id) {
			count++
		}
	}
	return count
}

func TestSQLiteRecoveryPlanning(t *testing.T) {
	s, w, request := sqliteAuditPlanningDecision(t)
	sqliteAuditRetryPolicy(t, s, 1, 5, 5)
	beforeAttempts := len(w.Tasks)
	_, tx := sqliteAuditWriter(t, s, w.ID)

	if _, err := s.planningChange(w.ID, "decide", request); err == nil || !strings.Contains(err.Error(), "SQLITE_BUSY") {
		t.Fatalf("contention stockage non qualifiée: %v", err)
	}
	if err := tx.Rollback(); err != nil {
		t.Fatal(err)
	}

	first, err := s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	replay, err := s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	if first.Revision != replay.Revision || len(replay.Tasks) != beforeAttempts+1 || replay.Planning.Inbox[0].Decision != request.EventID {
		t.Fatalf("rejeu non idempotent: first=%d replay=%d tasks=%d decision=%q", first.Revision, replay.Revision, len(replay.Tasks), replay.Planning.Inbox[0].Decision)
	}
	if replay.Planning.Reviewer != nil || sqliteRecoveryEventCount(t, s, w.ID, "planning.decide", request.EventID) != 1 {
		t.Fatalf("budget ou événement doublé: reviewer=%+v events=%d", replay.Planning.Reviewer, sqliteRecoveryEventCount(t, s, w.ID, "planning.decide", request.EventID))
	}

	stale := request
	stale.EventID = "sqlite-recovery-stale-revision"
	if _, err = s.planningChange(w.ID, "decide", stale); commandFailure(err).Code != "revision_conflict" || strings.Contains(fmt.Sprint(err), "SQLITE_BUSY") {
		t.Fatalf("conflit de révision confondu avec le stockage: %v", err)
	}
}

func sqliteRecoveryReviewBlocked(t *testing.T) (*Store, Work, string, string) {
	t.Helper()
	s, w, command := sqliteAuditReviewFixture(t, true)
	sqliteAuditRetryPolicy(t, s, 1, 5, 5)
	data, err := os.ReadFile(command)
	if err != nil {
		t.Fatal(err)
	}
	data = []byte(strings.Replace(string(data), "cat >\"$0.prompt\"", "cat >\"$0.prompt\"\nprintf x >>\"$0.calls\"", 1))
	if err = os.WriteFile(command, data, 0700); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { done <- s.independentReviewStep(w.ID) }()
	sqliteAuditWaitFile(t, command+".entered")
	_, tx := sqliteAuditWriter(t, s, w.ID)
	if err = os.WriteFile(command+".release", []byte("go"), 0600); err != nil {
		t.Fatal(err)
	}
	select {
	case err = <-done:
	case <-time.After(2 * time.Second):
		t.Fatal("persistance de revue non bornée")
	}
	if err == nil || !strings.Contains(err.Error(), "SQLITE_BUSY") {
		t.Fatalf("cause stockage non observable: %v", err)
	}
	if err = tx.Rollback(); err != nil {
		t.Fatal(err)
	}
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := got.task(w.Tasks[0].ID)
	if task.IndependentReview == nil || task.IndependentReview.State != "running" || got.Planning.Reviewer.Calls != 1 || len(task.Attempts) != 1 {
		t.Fatalf("réservation ou budgets inattendus: review=%+v calls=%d attempts=%d", task.IndependentReview, got.Planning.Reviewer.Calls, len(task.Attempts))
	}
	return s, got, command, task.IndependentReview.ID
}

func sqliteRecoveryAssertReview(t *testing.T, s *Store, work Work, command, reviewID string) {
	t.Helper()
	if err := s.independentReviewStep(work.ID); err != nil {
		t.Fatal(err)
	}
	got, err := s.get(work.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := got.task(work.Tasks[0].ID)
	calls, err := os.ReadFile(command + ".calls")
	if err != nil {
		t.Fatal(err)
	}
	if task.IndependentReview == nil || task.IndependentReview.ID != reviewID || task.IndependentReview.State != "passed" {
		t.Fatalf("verdict causal non repris: %+v", task.IndependentReview)
	}
	if got.Planning.Reviewer.Calls != 1 || len(calls) != 1 || len(task.Attempts) != 1 {
		t.Fatalf("double effet: review_calls=%d provider_calls=%d attempts=%d", got.Planning.Reviewer.Calls, len(calls), len(task.Attempts))
	}
	if sqliteRecoveryEventCount(t, s, work.ID, "review.claim", reviewID) != 1 || sqliteRecoveryEventCount(t, s, work.ID, "review.result", reviewID+"-result") != 1 {
		t.Fatalf("événements non uniques pour %s", reviewID)
	}
	if err = s.independentReviewStep(work.ID); err != nil {
		t.Fatal(err)
	}
	again, _ := s.get(work.ID)
	againTask, _ := again.task(work.Tasks[0].ID)
	if again.Revision != got.Revision || againTask.IndependentReview.ID != reviewID || again.Planning.Reviewer.Calls != 1 {
		t.Fatal("rejeu du verdict a produit un nouvel effet")
	}
}

func TestSQLiteRecoveryReview(t *testing.T) {
	s, w, command, reviewID := sqliteRecoveryReviewBlocked(t)
	sqliteRecoveryAssertReview(t, s, w, command, reviewID)
}

func TestSQLiteRecoveryPersistent(t *testing.T) {
	t.Run("reopen_reuses_attributed_result", func(t *testing.T) {
		s, w, command, reviewID := sqliteRecoveryReviewBlocked(t)
		root := s.root
		if err := s.db.Close(); err != nil {
			t.Fatal(err)
		}
		reopened, err := openStore(root, false)
		if err != nil {
			t.Fatal(err)
		}
		t.Cleanup(func() { reopened.db.Close() })
		// Keep the same bounded policy after reopening and prove the persisted
		// review payload, not process memory, drives recovery.
		sqliteAuditRetryPolicy(t, reopened, 1, 5, 5)
		sqliteRecoveryAssertReview(t, reopened, w, command, reviewID)

		got, err := reopened.get(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		task, _ := got.task(w.Tasks[0].ID)
		raw, _ := json.Marshal(task.IndependentReview)
		if !strings.Contains(string(raw), reviewID) || task.IndependentReview.Finished == "" {
			t.Fatal("verdict durable incomplet après réouverture")
		}
	})

	t.Run("mismatched_journal_is_rejected", func(t *testing.T) {
		s, w, command, reviewID := sqliteRecoveryReviewBlocked(t)
		path, err := s.pendingIndependentReviewResultPath(reviewID)
		if err != nil {
			t.Fatal(err)
		}
		raw, err := os.ReadFile(path)
		if err != nil {
			t.Fatal(err)
		}
		var pending pendingIndependentReviewResult
		if err = json.Unmarshal(raw, &pending); err != nil {
			t.Fatal(err)
		}
		pending.Work = "different-work"
		raw, _ = json.Marshal(pending)
		if err = os.WriteFile(path, raw, 0600); err != nil {
			t.Fatal(err)
		}
		if err = s.independentReviewStep(w.ID); err == nil || !strings.Contains(err.Error(), "non attribuable") {
			t.Fatalf("journal incompatible accepté: %v", err)
		}
		got, _ := s.get(w.ID)
		task, _ := got.task(w.Tasks[0].ID)
		calls, _ := os.ReadFile(command + ".calls")
		if task.IndependentReview.State != "running" || got.Planning.Reviewer.Calls != 1 || len(calls) != 1 {
			t.Fatalf("rejet a modifié l'état ou rappelé le fournisseur: state=%s calls=%d provider=%d", task.IndependentReview.State, got.Planning.Reviewer.Calls, len(calls))
		}
	})
}
