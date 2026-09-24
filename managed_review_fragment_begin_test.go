//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func fragmentBeginFixture(t *testing.T) (*Store, Work, Agent, managedReviewContext, string, []byte) {
	t.Helper()
	s, w, a := unpaidReviewFixture(t)
	p, e := s.readManagedPreflightEvidence(w.ID, a.TaskID)
	if e != nil {
		t.Fatal(e)
	}
	path := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID, "receipt.json")
	raw, e := os.ReadFile(path)
	if e != nil {
		t.Fatal(e)
	}
	rel, e := filepath.Rel(s.root, path)
	if e != nil {
		t.Fatal(e)
	}
	return s, w, a, p.Context, filepath.ToSlash(rel), raw
}
func TestManagedFragmentBeginNoSpendAndReopen(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	r, e := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if e != nil {
		t.Fatal(e)
	}
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if current.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
		t.Fatal("initialization charged call")
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	p, j, e := reopened.readFragmentJournalAnchor(r)
	if e != nil || len(j.Entries) != 0 || p.Candidate != c.Candidate {
		t.Fatal(p, j, e)
	}
	before, e := os.ReadFile(filepath.Join(s.root, r.Context))
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.beginManagedFragmentReview(current, a, c, path, receipt); e == nil {
		t.Fatal("duplicate initialization")
	}
	after, _ := os.ReadFile(filepath.Join(s.root, r.Context))
	if string(before) != string(after) {
		t.Fatal("previous evidence overwritten")
	}
}
func TestManagedFragmentBeginRefusalsPreserveWork(t *testing.T) {
	for _, mode := range []string{"pause", "budget", "receipt", "path", "attempt", "model", "context"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, c, path, receipt := fragmentBeginFixture(t)
			switch mode {
			case "pause":
				if e := s.pause(w.ID, true); e != nil {
					t.Fatal(e)
				}
			case "budget":
				w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls
				raw, _ := json.Marshal(w)
				if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
					t.Fatal(e)
				}
			case "context":
				c.Diff += "\n+invented change\n"
			case "receipt":
				receipt = []byte("{}")
			case "path":
				path = "../receipt.json"
			case "attempt":
				a.Attempt = "different"
			case "model":
				actual := w
				raw, _ := json.Marshal(w)
				json.Unmarshal(raw, &actual)
				actual.Planning.Reviewer.ModelRoute = &ModelRoute{Level: "standard", Model: "changed"}
				raw, _ = json.Marshal(actual)
				if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
					t.Fatal(e)
				}
			}
			if _, e := s.beginManagedFragmentReview(w, a, c, path, receipt); e == nil {
				t.Fatal("invalid initialization accepted")
			}
			after, e := s.get(w.ID)
			if e != nil {
				t.Fatal(e)
			}
			task, _ := after.task(a.TaskID)
			if task.IndependentReview != nil || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
				t.Fatal("failed initialization mutated work")
			}
		})
	}
}
