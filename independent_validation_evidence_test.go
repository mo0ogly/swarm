//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestControlsPrecedeReviewAndAreNotRepeated(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("python3", "-c", "from pathlib import Path; p=Path('count'); p.write_text(p.read_text()+'x' if p.exists() else 'x')"), true)
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	review := task.IndependentReview
	task.IndependentReview = nil
	raw, _ := json.Marshal(current)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	s.conduct(a, "completed")
	current, _ = s.get(w.ID)
	task, _ = current.task(a.TaskID)
	if task.Status != "submitted" || task.AutoValidation == nil || task.AutoValidation.State != "pending_review" {
		t.Fatalf("must await review: %+v", task)
	}
	evidence, _, err := s.independentValidationEvidence(task)
	if err != nil || evidence == "" {
		t.Fatal(evidence, err)
	}
	if accepted, _ := s.runAutomaticValidation(a, report); accepted {
		t.Fatal("accepted without reviewer")
	}
	task.IndependentReview = review
	raw, _ = json.Marshal(current)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	if accepted, reason := s.runAutomaticValidation(a, report); !accepted {
		t.Fatal(reason)
	}
	count, err := os.ReadFile(filepath.Join(s.root, "count"))
	if err != nil || string(count) != "x" {
		t.Fatalf("controls repeated: %q %v", count, err)
	}
	current, _ = s.get(w.ID)
	task, _ = current.task(a.TaskID)
	if task.Status != "accepted" {
		t.Fatal(task.Status)
	}
}

func TestReviewEvidenceRejectsImportedStaleAndWrongAttempt(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	s.conduct(a, "completed")
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	if _, _, err := s.independentValidationEvidence(task); err != nil {
		t.Fatal(err)
	}
	original := task.AutoValidation
	task.AutoValidation = nil
	if _, _, err := s.independentValidationEvidence(task); err == nil {
		t.Fatal("gate alone promoted to receipt")
	}
	task.AutoValidation = original
	saved := original.Attempt
	original.Attempt = "another-attempt"
	if _, _, err := s.independentValidationEvidence(task); err == nil {
		t.Fatal("wrong attempt accepted")
	}
	original.Attempt = saved
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("changed"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, _, err := s.independentValidationEvidence(task); err == nil {
		t.Fatal("stale evidence accepted")
	}
}

func TestChangedPendingReportRechecksOnceWithoutNewWorker(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("python3", "-c", "from pathlib import Path; p=Path('count'); p.write_text(p.read_text()+'x' if p.exists() else 'x')"), false)
	currentBefore, _ := s.get(w.ID)
	before, _ := currentBefore.task(a.TaskID)
	before.IndependentReview = nil
	body, _ := json.Marshal(currentBefore)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID); err != nil {
		t.Fatal(err)
	}
	s.conduct(a, "completed")
	if accepted, reason := s.runAutomaticValidation(a, report); accepted {
		t.Fatal(reason)
	}
	checkCount := func(want string) {
		t.Helper()
		b, e := os.ReadFile(filepath.Join(s.root, "count"))
		if e != nil || string(b) != want {
			t.Fatalf("count=%q want=%q err=%v", b, want, e)
		}
	}
	checkCount("x")
	initial, _ := s.get(w.ID)
	initialTask, _ := initial.task(a.TaskID)
	oldReceipt := initialTask.AutoValidation.Receipt
	oldBytes, err := os.ReadFile(filepath.Join(s.root, oldReceipt))
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("Updated report with exact browser assertions"), 0600); err != nil {
		t.Fatal(err)
	}
	if accepted, reason := s.runAutomaticValidation(a, report); accepted {
		t.Fatal("accepted without fresh review", reason)
	}
	checkCount("xx")
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	if _, _, err := s.independentValidationEvidence(task); err != nil {
		t.Fatal("new receipt is not current", err)
	}
	if task.AutoValidation.Attempt != a.Attempt {
		t.Fatal("changed producer attempt")
	}
	if task.AutoValidation.Receipt == oldReceipt {
		t.Fatal("corrected report overwrote historical receipt")
	}
	if got, err := os.ReadFile(filepath.Join(s.root, oldReceipt)); err != nil || string(got) != string(oldBytes) {
		t.Fatal("historical receipt changed", err)
	}
	s.runAutomaticValidation(a, report)
	checkCount("xx")
}

// Polling must retain the attempt-local report selected by the handoff, even
// when the project root also contains an identically named report.
func TestResumeValidationUsesAttemptWorkspace(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("python3", "-c", "from pathlib import Path; p=Path('count'); p.write_text(p.read_text()+'x' if p.exists() else 'x')"), false)
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	task.IndependentReview = nil
	raw, _ := json.Marshal(current)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	a.CWD = filepath.Join(s.root, "worker")
	local := filepath.Join(a.CWD, report)
	if err := os.MkdirAll(filepath.Dir(local), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(local, []byte("résultat mesuré\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	s.conduct(a, "completed")
	current, _ = s.get(w.ID)
	task, _ = current.task(a.TaskID)
	if task.AutoValidation == nil {
		t.Fatal("control receipt absent")
	}
	receipt := task.AutoValidation.Receipt
	if task.AutoValidation.Artifacts[filepath.ToSlash(filepath.Join("worker", report))] == "" {
		t.Fatal("handoff did not select local report")
	}
	if _, err := s.resumeAutomaticValidations(w.ID); err != nil {
		t.Fatal(err)
	}
	current, _ = s.get(w.ID)
	task, _ = current.task(a.TaskID)
	if task.AutoValidation.Receipt != receipt {
		t.Fatal("polling replaced the current receipt with a root-copy receipt")
	}
	if count, err := os.ReadFile(filepath.Join(s.root, "count")); err != nil || string(count) != "x" {
		t.Fatalf("repeated controls: %q %v", count, err)
	}
}
