//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestOrdinaryRefusedReviewRemainsActionable(t *testing.T) {
	s, prep := preparedTeam(t)
	w, _ := s.get(prep.WorkID)
	task := &w.Tasks[0]
	task.Status = "blocked"
	task.Attempts = []Attempt{{ID: "completed", Status: "completed"}}
	a := Agent{ID: "producer", TaskID: task.ID, Attempt: "completed", Status: "completed"}
	name := "docs/refused.md"
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	os.WriteFile(filepath.Join(s.root, name), []byte("missing browser evidence"), 0600)
	task.IndependentReview = &IndependentReview{State: "changes_requested", Attempt: a.Attempt, Producer: a.ID, Report: name, Digest: hash([]byte("missing browser evidence")), Contract: reviewContract(task), Reason: "Browser evidence missing"}
	for _, state := range []string{"changes_requested", "running", "error"} {
		task.IndependentReview.State = state
		p := s.resultPresentation(&w, task, []Agent{a}, TaskValidation{})
		expected := "review_blocked"
		if state == "running" {
			expected = "review_in_progress"
		}
		if p.State != expected || p.ReportState != "submitted" || p.ProcessState != "completed" || p.ValidationState == "fresh" {
			t.Fatalf("review hidden: %+v", p)
		}
		u := taskUnderstanding(MissionTask{State: "intervention", Result: p}, MissionStatus{}, []Agent{a})
		if u.What != p.Reason || u.NextStep != p.NextStep {
			t.Fatalf("cause or recovery hidden: %+v", u)
		}
	}
	task.IndependentReview.State = "changes_requested"
	os.WriteFile(filepath.Join(s.root, name), []byte("corrected evidence"), 0600)
	p := s.resultPresentation(&w, task, []Agent{a}, TaskValidation{})
	if p.State != "review_blocked" || p.ReportState != "stale" {
		t.Fatalf("changed evidence shown fresh: %+v", p)
	}
	task.Attempts = append(task.Attempts, Attempt{ID: "next"})
	a.Attempt = "next"
	p = s.resultPresentation(&w, task, []Agent{a}, TaskValidation{})
	if strings.HasPrefix(p.State, "review_") {
		t.Fatal("old refusal assigned to new producer", p)
	}
}
