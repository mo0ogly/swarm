//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestManagedFragmentCumulativeReportProvenance(t *testing.T) {
	s, w, a := managedBatchRuntimeFixture(t)
	w.Planning.Reviewer.Failure = "fixture stop before review"
	raw, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	w.Planning.Reviewer.Failure = ""
	raw, _ = json.Marshal(w)
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	dir := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID)
	raw, e = os.ReadFile(filepath.Join(dir, "candidate.json"))
	if e != nil {
		t.Fatal(e)
	}
	var cp managedReviewCheckpoint
	if e = json.Unmarshal(raw, &cp); e != nil {
		t.Fatal(e)
	}
	receipt, e := os.ReadFile(filepath.Join(dir, "receipt.json"))
	if e != nil {
		t.Fatal(e)
	}
	path, e := filepath.Rel(s.root, filepath.Join(dir, "receipt.json"))
	if e != nil {
		t.Fatal(e)
	}
	c, e := s.managedReviewContext(w, a, cp.Candidate, receipt)
	if e != nil {
		t.Fatal(e)
	}
	if len(c.Tasks) < 2 {
		t.Fatal("fixture not cumulative")
	}
	r, e := s.beginManagedFragmentReview(w, a, c, filepath.ToSlash(path), receipt)
	if e != nil {
		t.Fatal(e)
	}
	for _, tc := range c.Tasks {
		copy := r
		copy.Attempt = tc.Binding.Attempt
		copy.Producer = tc.Binding.Producer
		copy.Contract = tc.Binding.Contract
		copy.GitReport = tc.Binding.Report
		copy.Digest = tc.Binding.ReportDigest
		copy.Report = managedFragmentReportPath(r, tc.Task)
		if _, _, e = s.readFragmentJournalAnchor(copy); e != nil {
			t.Fatal(tc.Task, e)
		}
		if e = s.managedReviewFilesIntact(copy); e != nil {
			t.Fatal(tc.Task, e)
		}
		copy.Attempt = "forged"
		if _, _, e = s.readFragmentJournalAnchor(copy); e == nil {
			t.Fatal("forged copy accepted")
		}
	}
	anchor := *r.FragmentJournal
	anchor.InspectionAttempt = "forged"
	r.FragmentJournal = &anchor
	if _, _, e = s.readFragmentJournalAnchor(r); e == nil {
		t.Fatal("forged original attempt accepted")
	}
}
