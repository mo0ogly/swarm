//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestManagedScopeBlocksBeforePaidReviewAndExportsExactSelection(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "repaired\n")
	a.DeliveryVersion = 1
	originalTask, _ := w.task(a.TaskID)
	delivery, _ := json.Marshal(completeDelivery(originalTask, a))
	os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), delivery, 0600)
	a.Ended = now()
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 101; i++ {
		if err := os.WriteFile(filepath.Join(a.CWD, fmt.Sprintf("extra-%03d.txt", i)), []byte("kept\n"), 0600); err != nil {
			t.Fatal(err)
		}
	}
	// A whitespace-bearing filename must survive literal selection.
	binaryName := "literal[1] .bin"
	os.WriteFile(filepath.Join(a.CWD, binaryName), []byte("exact content  \n"), 0600)
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	if task.Status != "blocked" || !strings.HasPrefix(task.Blocker, managedScopeRefusal) || managedReviewCalls(t, s) != 0 {
		t.Fatal(task.Status, task.Blocker)
	}
	before, _ := s.get(w.ID)
	item, _ := s.managedAttempt(a.ID)
	p, err := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
	if err != nil {
		t.Fatal(err)
	}
	if !p.RequiresReduction || len(p.Files) < 102 {
		t.Fatal(p)
	}
	req := PlanningRequest{Task: a.TaskID, Revision: w.Revision, ExpectedCandidate: p.Candidate, ScopeFiles: []string{binaryName, "value.txt"}}
	request := filepath.Join(t.TempDir(), "request.json")
	raw, _ := json.Marshal(req)
	os.WriteFile(request, raw, 0600)
	var out bytes.Buffer
	if err = s.planningCLI([]string{"planning", "scope-patch", w.ID}, request, &out); err != nil {
		t.Fatal(err)
	}
	var patch ManagedScopePlan
	json.Unmarshal(out.Bytes(), &patch)
	target := t.TempDir()
	gitTest(t, target, "clone", filepath.Join(w.Planning.Repository.Storage, "repository.git"), ".")
	gitTest(t, target, "checkout", "--detach", p.Base)
	patchFile := filepath.Join(t.TempDir(), "selection.patch")
	os.WriteFile(patchFile, []byte(patch.Patch), 0600)
	gitTest(t, target, "apply", "--index", patchFile)
	got, _ := os.ReadFile(filepath.Join(target, binaryName))
	if !bytes.Equal(got, []byte("exact content  \n")) {
		t.Fatal("content changed", got)
	}
	if _, err = os.Stat(filepath.Join(target, "extra-000.txt")); !os.IsNotExist(err) {
		t.Fatal("deferred file included")
	}
	if len(patch.Deferred) != len(p.Files)-2 {
		t.Fatal("missing deferred paths")
	}
	for _, bad := range []PlanningRequest{{Task: a.TaskID, Revision: w.Revision - 1, ExpectedCandidate: p.Candidate, ScopeFiles: req.ScopeFiles}, {Task: a.TaskID, Revision: w.Revision, ExpectedCandidate: p.Candidate, ScopeFiles: []string{"../escape"}}, {Task: a.TaskID, Revision: w.Revision, ExpectedCandidate: p.Candidate, ScopeFiles: []string{"value.txt", "value.txt"}}} {
		if _, err = s.managedScopePreview(w.ID, bad, true); err == nil {
			t.Fatal("invalid selection accepted")
		}
	}
	mux := http.NewServeMux()
	s.registerPlanning(mux, func(w http.ResponseWriter, v any) { json.NewEncoder(w).Encode(v) }, func(w http.ResponseWriter, e error) { http.Error(w, e.Error(), 400) })
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, httptest.NewRequest("POST", "/api/v1/planning?work="+w.ID+"&action=scope-patch", bytes.NewReader(raw)))
	if rec.Code != 200 {
		t.Fatal(rec.Body.String())
	}
	var httpPatch ManagedScopePlan
	json.Unmarshal(rec.Body.Bytes(), &httpPatch)
	if !reflect.DeepEqual(patch, httpPatch) {
		t.Fatal("CLI/API differ")
	}
	if _, err = s.prepareManagedPreflightRetry(w.ID, a.TaskID); err == nil {
		t.Fatal("retry bypassed scope limit")
	}
	after, _ := s.get(w.ID)
	afterItem, _ := s.managedAttempt(a.ID)
	if !reflect.DeepEqual(before, after) || !reflect.DeepEqual(item, afterItem) || managedReviewCalls(t, s) != 0 {
		t.Fatal("scope export mutated mission")
	}
	// Reducing a complete but oversized delivery must not require a fictitious
	// independent refusal: it never reached a reviewer in the first place.
	for i := 0; i < 101; i++ {
		os.Remove(filepath.Join(a.CWD, fmt.Sprintf("extra-%03d.txt", i)))
	}
	os.Remove(filepath.Join(a.CWD, binaryName))
	gitTest(t, a.CWD, "add", "-A")
	recovered, err := s.planningChange(w.ID, "revise-recovered-result", PlanningRequest{Schema: 1, EventID: "reduce-preserved-result", Revision: after.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ConfirmRecovery: true, ResultTree: gitTest(t, a.CWD, "write-tree"), Reason: "Explicitly reduced delivery; original result preserved"})
	if err != nil {
		t.Fatal(err)
	}
	accepted, _ := recovered.task(a.TaskID)
	if accepted.Status != "accepted" || accepted.RecoveredResult.ReplacesResult != item.Result || managedReviewCalls(t, s) != 1 {
		t.Fatal("reduction did not undergo normal review", accepted.Status)
	}

}
