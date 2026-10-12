//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"strconv"
	"testing"
)

func TestReviewRetryStorageMigrationPreservesQueuedInstruction(t *testing.T) {
	s, w, a := planningProtocolReviewFixture(t)
	w, request := planningClaim(t, s, w, "root")
	request.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "Keep the authorized supervisor correction across a storage upgrade."}}
	after, err := s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	var before []byte
	if err = s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&before); err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("PRAGMA user_version=27"); err != nil {
		t.Fatal(err)
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reader, err := openStoreWithMigration(s.root, false, false)
	if err == nil {
		reader.db.Close()
		t.Fatal("v27 writer-compatible storage still allows losing the newly saved review instruction")
	}
	if commandFailure(err).Code != "storage_upgrade_required" {
		t.Fatal(err)
	}
	upgraded, err := openStoreWithMigration(s.root, false, true)
	if err != nil {
		t.Fatal(err)
	}
	defer upgraded.db.Close()
	var body []byte
	if err = upgraded.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&body); err != nil {
		t.Fatal(err)
	}
	if string(body) != string(before) {
		t.Fatal("migration rewrote original mission evidence")
	}
	current, err := upgraded.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := current.task(a.TaskID)
	if task.IndependentReview.FragmentJournal.RetryDirective == nil || task.IndependentReview.FragmentJournal.RetryDirective.Decision != request.EventID || current.Revision != after.Revision || current.Planning.Reviewer.Calls != after.Planning.Reviewer.Calls {
		t.Fatal("queued instruction, revision or consumption lost")
	}
	backups, err := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v28-*.db"))
	if err != nil || len(backups) != 1 {
		t.Fatal("original migration backup missing", err)
	}
	stat, err := os.Stat(backups[0])
	if err != nil || stat.Mode().Perm() != 0600 {
		t.Fatal("migration evidence is not private", err)
	}
}

func TestReviewRetryConnectedWriterRejectsChangedStorageBeforeMutation(t *testing.T) {
	s, w, _ := planningProtocolReviewFixture(t)
	var before []byte
	if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&before); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("PRAGMA user_version=" + strconv.Itoa(schemaVersion+1)); err != nil {
		t.Fatal(err)
	}
	_, err := s.mutate(w.ID, "fixture.version", "changed-storage-must-not-write", w.Revision, []byte(`{}`), func(current *Work) error {
		current.Title = "Must not be written by the outdated connected process"
		return nil
	})
	if err == nil {
		t.Fatal("outdated open connection rewrote newer mission storage")
	}
	if commandFailure(err).Code != "storage_version_conflict" {
		t.Fatal(err)
	}
	var after []byte
	if err = s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&after); err != nil {
		t.Fatal(err)
	}
	var count int
	if err = s.db.QueryRow("SELECT count(*) FROM events WHERE id=?", "changed-storage-must-not-write").Scan(&count); err != nil {
		t.Fatal(err)
	}
	if string(before) != string(after) || count != 0 {
		t.Fatal("storage rejection mutated mission or event history")
	}
}
