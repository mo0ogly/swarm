//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRecoveryEvidenceChangedUnknownAndUnchanged(t *testing.T) {
	s, w := validationConfigFixture(t)
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	for _, name := range []string{"same.md", "changed.md"} {
		if e := os.WriteFile(filepath.Join(s.root, "docs", name), []byte("before"), 0600); e != nil {
			t.Fatal(e)
		}
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Blocker: "rapport incomplet", Next: "corriger uniquement docs/changed.md"})
	task, _ := w.task("t1")
	task.IndependentReview = &IndependentReview{ID: "refusal", State: "changes_requested", Report: "docs/same.md", Digest: hash([]byte("before")), Contract: reviewContract(task), ReportArtifacts: map[string]string{"docs/changed.md": hash([]byte("before")), "docs/missing.md": hash([]byte("before"))}, Criteria: []ReviewCriterion{{Index: 1, Verdict: "pass"}, {Index: 2, Verdict: "fail"}}}
	raw, _ := json.Marshal(w)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	os.WriteFile(filepath.Join(s.root, "docs", "changed.md"), []byte("after"), 0600)
	p, e := s.recoveryPreview(w.ID, "t1", "")
	if e != nil {
		t.Fatal(e)
	}
	states := map[string]string{}
	for _, r := range p.Evidence {
		states[r.Report] = r.State
	}
	if states["docs/same.md"] != "unchanged" || states["docs/changed.md"] != "changed" || states["docs/missing.md"] != "unknown" {
		t.Fatal(states)
	}
	if len(p.Remaining) != 2 || p.SinceRefusal == nil || p.SinceRefusal.From != w.Revision {
		t.Fatal(p)
	}
	var out bytes.Buffer
	printRecoveryPreview(&out, p)
	if !strings.Contains(out.String(), "Critères restant") || !strings.Contains(out.String(), "ancienne preuve périmée") {
		t.Fatal(out.String())
	}
	// A repaired input remains distinct from the historical review verdict.
	os.WriteFile(filepath.Join(s.root, "docs", "changed.md"), []byte("before"), 0600)
	delete(task.IndependentReview.ReportArtifacts, "docs/missing.md")
	raw, _ = json.Marshal(w)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	p, e = s.recoveryPreview(w.ID, "t1", "")
	if e != nil || len(p.Remaining) != 1 || p.Remaining[0] != task.Criteria[1] {
		t.Fatal(p, e)
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision || after.Tasks[0].Status != "blocked" {
		t.Fatal("read-only preview mutated state")
	}
}

func TestRecoveryPreviewRefusalDoesNotUseVisitOrOtherTasks(t *testing.T) {
	s, w := validationConfigFixture(t)
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Blocker: "rapport incomplet"})
	refusal := w.Revision
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Other", Deliverable: "docs/other.md", Criteria: []string{"other"}})
	w = applyTest(t, s, w, "task.update", Request{ID: "t2", Status: "blocked", Blocker: "rapport incomplet"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Next: "correction bornée"})
	if e := s.markVisit(w.ID, operatorIdentity(), w.Revision); e != nil {
		t.Fatal(e)
	}
	p, e := s.recoveryPreview(w.ID, "t1", "")
	if e != nil || p.SinceRefusal == nil || p.SinceRefusal.From != refusal || len(p.SinceRefusal.Items) == 0 {
		t.Fatal(p, e)
	}
	for _, item := range p.SinceRefusal.Items {
		if item.Task != "t1" {
			t.Fatal("other task mixed into recovery", item)
		}
	}
}

func TestRecoveryEvidenceBoundedAndInvalidInputUnknown(t *testing.T) {
	s := storeTest(t)
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	f, e := os.Create(filepath.Join(s.root, "docs", "large.md"))
	if e != nil {
		t.Fatal(e)
	}
	f.Truncate((8 << 20) + 1)
	f.Close()
	task := &Task{IndependentReview: &IndependentReview{ReportArtifacts: map[string]string{"../outside": "digest", "docs/large.md": "digest", "docs/no-digest.md": ""}}}
	for _, item := range s.recoveryEvidence(task) {
		if item.State != "unknown" {
			t.Fatal(item)
		}
	}
	for i := 0; i < 70; i++ {
		task.IndependentReview.ReportArtifacts[newID("docs/proof-")] = "digest"
	}
	evidence := s.recoveryEvidence(task)
	if len(evidence) != 65 || evidence[64].Report != "" || evidence[64].State != "unknown" {
		t.Fatal("bound absent", len(evidence))
	}
}
