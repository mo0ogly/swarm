//go:build linux

package main

import "testing"

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
