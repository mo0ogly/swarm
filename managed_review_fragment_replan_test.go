//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestManagedFragmentReplanPreservesHistory(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	j = reserveFragment(p, j)
	j.Entries[0].State = "interrupted"
	raw, _ := json.Marshal(j)
	if err := atomicWrite(filepath.Join(s.root, r.FragmentJournal.Journal), raw); err != nil {
		t.Fatal(err)
	}
	r.FragmentJournal.JournalDigest = hash(raw)
	r.State = "error"
	old := *r.FragmentJournal
	task, _ := w.task(a.TaskID)
	task.IndependentReview = &r
	calls := w.Planning.Reviewer.Calls
	if err := s.queueManagedFragmentReplan(w, task, r, p, j); err != nil {
		t.Fatal(err)
	}
	next := task.IndependentReview
	if next.ID != r.ID || next.Attempt != r.Attempt || next.State != "queued" || !reflect.DeepEqual(next.FragmentJournal.ReplannedFrom, &old) || w.Planning.Reviewer.Calls != calls {
		t.Fatal("identity, history or budget changed")
	}
	_, saved, err := s.readFragmentJournalAnchor(*next)
	if err != nil || len(saved.Entries) != 0 {
		t.Fatal(saved, err)
	}
	retained, _ := os.ReadFile(filepath.Join(s.root, old.Journal))
	if string(retained) != string(raw) {
		t.Fatal("old proof overwritten")
	}
	if err = os.WriteFile(filepath.Join(s.root, old.Journal), []byte("corrupt"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, _, err = s.readFragmentJournalAnchor(*next); err == nil {
		t.Fatal("corrupt history accepted")
	}
}
func TestManagedFragmentReplanRejectsUnsafeRestart(t *testing.T) {
	for _, kind := range []string{"budget", "active", "inspected", "unknown", "final", "running"} {
		t.Run(kind, func(t *testing.T) {
			s, w, a, r, p, j := fragmentStoreFixture(t)
			r.State = "error"
			switch kind {
			case "budget":
				w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls
			case "active":
				j = reserveFragment(p, j)
			case "inspected", "unknown":
				j = reserveFragment(p, j)
				j.Entries[0].State = kind
			case "final":
				r.FragmentJournal.FinalJournalDigest = "existing"
			case "running":
				r.State = "running"
			}
			task, _ := w.task(a.TaskID)
			task.IndependentReview = &r
			before, _ := json.Marshal(w)
			if err := s.queueManagedFragmentReplan(w, task, r, p, j); err == nil {
				t.Fatal("unsafe restart accepted")
			}
			after, _ := json.Marshal(w)
			if string(before) != string(after) {
				t.Fatal("refusal changed state")
			}
		})
	}
}
