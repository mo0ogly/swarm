//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestReviewTimeoutConfigurationPreservesBudgetAndHistory(t *testing.T) {
	s, w := managedFixture(t)
	original := *w.Planning.Reviewer
	if seconds, err := reviewTimeoutSeconds(w.Planning.Reviewer); err != nil || seconds != 90 {
		t.Fatal(seconds, err)
	}
	for _, seconds := range []int{-1, 0, 901} {
		if _, err := s.planningChange(w.ID, "review-timeout", PlanningRequest{Schema: 1, EventID: newID("invalid-"), Revision: w.Revision, ReviewTimeoutSeconds: seconds, Reason: "Explicit fixture configuration"}); err == nil {
			t.Fatalf("invalid timeout accepted: %d", seconds)
		}
	}
	r := PlanningRequest{Schema: 1, EventID: "set-review-timeout", Revision: w.Revision, ReviewTimeoutSeconds: 300, Reason: "Explicit operator adjustment for a longer review"}
	file := filepath.Join(t.TempDir(), "request.json")
	data, _ := json.Marshal(r)
	os.WriteFile(file, data, 0600)
	var out bytes.Buffer
	if err := s.planningCLI([]string{"planning", "review-timeout", w.ID}, file, &out); err != nil {
		t.Fatal(err)
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	want := original
	want.TimeoutSeconds = 300
	if !reflect.DeepEqual(*after.Planning.Reviewer, want) || !reflect.DeepEqual(w.Tasks, after.Tasks) {
		t.Fatal("configuration changed budget or task history")
	}
	replay, err := s.planningChange(w.ID, "review-timeout", r)
	if err != nil || replay.Revision != after.Revision {
		t.Fatal("configuration not idempotent", err)
	}
	r.EventID = "stale-revision"
	if _, err := s.planningChange(w.ID, "review-timeout", r); err == nil {
		t.Fatal("stale revision accepted")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	w, err = reopened.get(w.ID)
	if err != nil || w.Planning.Reviewer.TimeoutSeconds != 300 {
		t.Fatal("timeout not durable", err)
	}
	w.Tasks[0].IndependentReview = &IndependentReview{ID: "active-review", State: "running", TimeoutSeconds: 300}
	body, _ := json.Marshal(w)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID)
	r.EventID = "cannot-change-active"
	r.Revision = w.Revision
	r.ReviewTimeoutSeconds = 600
	if _, err := s.planningChange(w.ID, "review-timeout", r); err == nil {
		t.Fatal("active deadline changed")
	}
}

func TestManagedReviewTimeoutRetryReusesCandidateWithoutRefund(t *testing.T) {
	s, w := managedFixture(t)
	base := w.Planning.Repository.Candidate
	fixture := filepath.Join(s.root, "review-fixture", "claude")
	data, err := os.ReadFile(fixture)
	if err != nil {
		t.Fatal(err)
	}
	data = bytes.Replace(data, []byte("text=sys.stdin.read()"), []byte("import time\ntext=sys.stdin.read()\ntime.sleep(2)"), 1)
	if err = os.WriteFile(fixture, data, 0700); err != nil {
		t.Fatal(err)
	}
	w, err = s.planningChange(w.ID, "review-timeout", PlanningRequest{Schema: 1, EventID: "short-review", Revision: w.Revision, ReviewTimeoutSeconds: 1, Reason: "Fixture reproduces bounded review timeout"})
	if err != nil {
		t.Fatal(err)
	}
	a := managedCompleted(t, s, w, "first", "timeout candidate\n")
	if err = s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task("first")
	first := *task.IndependentReview
	if first.State != "error" || first.TimeoutSeconds != 1 || !strings.Contains(first.Reason, "délai") || w.Planning.Repository.Candidate != base || w.Planning.Reviewer.Calls != 1 {
		t.Fatalf("deadline not enforced: %+v", first)
	}
	w, err = s.planningChange(w.ID, "review-timeout", PlanningRequest{Schema: 1, EventID: "longer-review", Revision: w.Revision, ReviewTimeoutSeconds: 4, Reason: "Explicit longer deadline after reproduced timeout"})
	if err != nil {
		t.Fatal(err)
	}
	if w.Tasks[0].IndependentReview.ID != first.ID || w.Planning.Reviewer.Calls != 1 {
		t.Fatal("changing timeout erased review")
	}
	w, err = s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "explicit-timeout-retry", Revision: w.Revision, Task: "first", Reason: "Deadline corrected from one second to four; candidate unchanged"})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ = w.task("first")
	second := task.IndependentReview
	if second.State != "passed" || second.TimeoutSeconds != 4 || second.ID == first.ID || second.CandidateSHA != first.CandidateSHA || len(task.Attempts) != 1 || w.Planning.Reviewer.Calls != 2 || task.Status != "accepted" {
		t.Fatalf("retry changed candidate/history or failed: %+v", task)
	}
}
