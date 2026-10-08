//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedReviewRetainsRejectedReplyWithoutAcceptingIt(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	fixture := strings.Replace(managedReviewerFixture, "task['report'][:80]", "'Invented quotation not in the candidate evidence'", 1)
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fixture), 0700); err != nil {
		t.Fatal(err)
	}
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	r := task.IndependentReview
	if task.Status == "accepted" || r.State != "error" || !strings.Contains(r.Reason, "critère 1") || !strings.Contains(r.Reason, "first") {
		t.Fatal(task.Status, r)
	}
	raw, err := os.ReadFile(filepath.Join(s.root, r.ReplyPath))
	if err != nil || hash(raw) != r.ReplyDigest || !strings.Contains(string(raw), "Invented quotation") {
		t.Fatal("rejected response missing", err)
	}
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	again, err := os.ReadFile(filepath.Join(s.root, r.ReplyPath))
	if err != nil || string(again) != string(raw) || managedReviewCalls(t, s) != 1 {
		t.Fatal("replayed or changed rejected review", err)
	}
}

func TestManagedReviewSchemaBoundsRationaleBeforeProviderCall(t *testing.T) {
	var schema map[string]any
	if err := json.Unmarshal([]byte(managedReviewSchema), &schema); err != nil {
		t.Fatal(err)
	}
	properties := schema["properties"].(map[string]any)
	taskProperties := properties["tasks"].(map[string]any)["items"].(map[string]any)["properties"].(map[string]any)
	reason := taskProperties["reason"].(map[string]any)
	if reason["maxLength"] != float64(1000) || reason["minLength"] != float64(8) {
		t.Fatal(reason)
	}
	evidence := taskProperties["criteria"].(map[string]any)["items"].(map[string]any)["properties"].(map[string]any)["evidence"].(map[string]any)
	if evidence["maxLength"] != float64(1000) {
		t.Fatal(evidence)
	}
	task := Task{Criteria: []string{"coverage"}}
	raw, _ := json.Marshal(map[string]any{"reason": strings.Repeat("x", 4001), "criteria": []ReviewCriterion{{Index: 1, Verdict: "pass", Evidence: "exact valid evidence"}}})
	if _, _, _, err := reviewReply(string(raw), &task, "exact valid evidence"); err == nil {
		t.Fatal("oversized response accepted")
	}
}
