//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestReviewRecoveryPreviewBudgetAndNoMutation(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	r.State = "error"
	r.Reason = "délai du planificateur dépassé"
	task, _ := w.task(a.TaskID)
	task.IndependentReview = &r
	w.Planning.Reviewer.Calls = 20
	w.Planning.Reviewer.MaxCalls = 20
	raw, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	before, _ := s.get(w.ID)
	v, e := s.reviewRecoveryPreview(w.ID, a.TaskID)
	if e != nil {
		t.Fatal(e)
	}
	if v.Required != len(p.Packets)+p.ReservedFinalCalls || v.MinimumLimit != 20+v.Required || v.Missing != v.Required || v.State != "budget_authorization_required" || v.Candidate != r.CandidateSHA {
		t.Fatalf("wrong decision %+v", v)
	}
	f := filepath.Join(t.TempDir(), "request.json")
	request, _ := json.Marshal(map[string]string{"task_id": a.TaskID})
	os.WriteFile(f, request, 0600)
	var out bytes.Buffer
	if e = s.planningCLI([]string{"planning", "recovery-preview", w.ID}, f, &out); e != nil {
		t.Fatal(e)
	}
	var got ReviewRecovery
	if e = json.Unmarshal(out.Bytes(), &got); e != nil || !reflect.DeepEqual(got, v) {
		t.Fatal("CLI mismatch", e)
	}
	after, _ := s.get(w.ID)
	if !reflect.DeepEqual(before, after) {
		t.Fatal("preview mutated work")
	}
	b, e := fragmentRecoveryBudget(p, j, 20, 20+v.Required)
	if e != nil || b.Missing != 0 {
		t.Fatal(b, e)
	}
	if e = os.WriteFile(filepath.Join(s.root, r.FragmentJournal.Journal), []byte("corrupted"), 0600); e != nil {
		t.Fatal(e)
	}
	if _, e = s.reviewRecoveryPreview(w.ID, a.TaskID); e == nil {
		t.Fatal("corrupt journal accepted")
	}
}
