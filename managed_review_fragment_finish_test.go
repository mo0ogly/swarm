//go:build linux

package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestManagedFragmentFinishRequiresDurableDecision(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	before, _ := s.get(w.ID)
	if s.finishManagedFragmentReview(w.ID, a, r.ID, nil) == nil {
		t.Fatal("incomplete review finalized")
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.IndependentReview.State == "passed" || task.Status == "accepted" || after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls {
		t.Fatal("premature completion or charge")
	}
}
func TestManagedFragmentSaveRejectsStaleJournalAnchor(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	anchor := *r.FragmentJournal
	anchor.JournalDigest = "stale"
	r.FragmentJournal = &anchor
	r.State = "error"
	if s.saveManagedReview(w.ID, a, r) == nil {
		t.Fatal("stale anchor overwrote current journal")
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.IndependentReview.FragmentJournal.JournalDigest == "stale" || task.IndependentReview.State != "running" {
		t.Fatal("journal mutated")
	}
}

func TestManagedFragmentRefusalRequiresOriginalProof(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fragmentRuntimeProviderFixture), 0700); err != nil {
		t.Fatal(err)
	}
	managedReviewMode(t, s, "fail")
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	_, _, runErr := s.runManagedFragmentReview(w, a, r)
	if runErr == nil {
		t.Fatal("missing refusal")
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	r = *task.IndependentReview
	reason, err := s.managedFragmentRefusal(r)
	if err != nil || reason == "" {
		t.Fatal(reason, err)
	}
	// The legacy error label is not authority; original proof remains necessary.
	r.State = "error"
	reason, err = s.managedFragmentRefusal(r)
	if err != nil || reason == "" {
		t.Fatal(reason, err)
	}
	before := managedReviewCalls(t, s)
	journal := filepath.Join(s.root, r.FragmentJournal.Journal)
	if err = os.WriteFile(journal, []byte("{}"), 0600); err != nil {
		t.Fatal(err)
	}
	if reason, err = s.managedFragmentRefusal(r); err == nil || reason != "" {
		t.Fatal("altered journal authorized correction")
	}
	if managedReviewCalls(t, s) != before {
		t.Fatal("proof check spent a call")
	}
}
