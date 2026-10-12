//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func binaryPreflightInspectionFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "original engine result")
	a.Ended = now()
	raw, _ := json.Marshal(a)
	if _, err := s.db.Exec("UPDATE agents SET body=? WHERE id=?", raw, a.ID); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 101; i++ {
		if err := os.WriteFile(filepath.Join(a.CWD, fmt.Sprintf("source-%03d.txt", i)), []byte("complete original source\n"), 0600); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.WriteFile(filepath.Join(a.CWD, "picture.png"), []byte{137, 80, 78, 71, 13, 10, 26, 10, 0, 1, 2}, 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	if !strings.HasPrefix(w.Tasks[0].Blocker, managedScopeRefusal) {
		t.Fatal("size refusal not preserved", w.Tasks[0].Blocker)
	}
	return s, w, a
}
func TestBinaryPreflightScopeInspectionPreservesRefusalAndSpending(t *testing.T) {
	s, w, a := binaryPreflightInspectionFixture(t)
	before, _ := s.managedAttempt(a.ID)
	preview, err := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
	if err != nil {
		t.Fatalf("scope inventory blocked by unreadable binary: %v", err)
	}
	if len(preview.Files) < 103 || preview.Evidence == "" || len(preview.Criteria) == 0 {
		t.Fatal("incomplete attributed scope inventory", preview)
	}
	found := false
	for _, name := range preview.Files {
		found = found || name == "picture.png"
	}
	if !found {
		t.Fatal("binary silently dropped from inventory")
	}
	if _, err = s.prepareManagedPreflightRetry(w.ID, a.TaskID); err == nil {
		t.Fatal("text-only paid review of binary admitted")
	}
	after, _ := s.get(w.ID)
	item, _ := s.managedAttempt(a.ID)
	if after.Revision != w.Revision || item.Result != before.Result || item.State != before.State || managedReviewCalls(t, s) != 0 {
		t.Fatal("scope inspection changed result, refusal or spending")
	}
}

func TestBinaryPreflightScopeInspectionRequiresOriginalPassedReceipts(t *testing.T) {
	for _, mode := range []string{"failed-control", "wrong-candidate", "wrong-attempt"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := binaryPreflightInspectionFixture(t)
			path := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID, "receipt.json")
			raw, err := os.ReadFile(path)
			if err != nil {
				t.Fatal(err)
			}
			var r map[string]json.RawMessage
			if err = json.Unmarshal(raw, &r); err != nil {
				t.Fatal(err)
			}
			switch mode {
			case "wrong-candidate":
				r["candidate_commit"] = json.RawMessage(`"replaced"`)
			case "wrong-attempt":
				r["attempt_id"] = json.RawMessage(`"replaced"`)
			case "failed-control":
				var controls map[string][]ValidationControlResult
				if err = json.Unmarshal(r["controls"], &controls); err != nil {
					t.Fatal(err)
				}
				for id, rows := range controls {
					rows[0].Passed = false
					controls[id] = rows
					break
				}
				r["controls"], _ = json.Marshal(controls)
			}
			raw, _ = json.Marshal(r)
			if err = os.WriteFile(path, raw, 0600); err != nil {
				t.Fatal(err)
			}
			if _, err = s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false); err == nil {
				t.Fatal("altered receipt admitted", mode)
			}
			after, _ := s.get(w.ID)
			if after.Revision != w.Revision || managedReviewCalls(t, s) != 0 {
				t.Fatal("rejected inspection changed state")
			}
		})
	}
}
func TestBinaryPreflightCorrectionInspectionKeepsInterruptedResultAndTextRefusal(t *testing.T) {
	s, w, a := binaryPreflightInspectionFixture(t)
	item, _ := s.managedAttempt(a.ID)
	a.Status = "interrupted"
	raw, _ := json.Marshal(a)
	if _, err := s.db.Exec("UPDATE agents SET status=?,body=? WHERE id=?", a.Status, raw, a.ID); err != nil {
		t.Fatal(err)
	}
	w.Tasks[0].RecoveredResult = &RecoveredResult{Agent: a.ID, Attempt: a.Attempt, Result: item.Result, ProcessStatus: a.Status}
	raw, _ = json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	prepared, err := s.readManagedPreflightCorrectionEvidence(w.ID, a.TaskID)
	if err != nil || !prepared.Context.InspectionOnly {
		t.Fatal("recovery inspection unavailable", err)
	}
	if _, err = s.prepareManagedPreflightRetry(w.ID, a.TaskID); err == nil {
		t.Fatal("inspection authorized binary text review")
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision || len(after.Tasks[0].Attempts) != 1 || managedReviewCalls(t, s) != 0 {
		t.Fatal("inspection spent or replaced producer")
	}
}
