//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// A corrective handoff must not replace the retained refusal merely to discover
// that the new evidence cannot reach its reviewer. Exercise the public planning
// operation with isolated stores and the ordinary reviewer fixture.
func TestRecoveredResultEvidencePreflightPreservesStateAndAllowsCorrection(t *testing.T) {
	for _, mode := range []string{"submit", "revise"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, req := recoveredResultFixture(t)
			action := "submit-recovered-result"
			if mode == "revise" {
				managedReviewMode(t, s, "fail")
				var err error
				w, err = s.planningChange(w.ID, action, req)
				if err != nil {
					t.Fatal(err)
				}
				action = "revise-recovered-result"
				req.ReviewID = w.Tasks[0].IndependentReview.ID
			}
			task, _ := w.task(a.TaskID)
			d := completeDelivery(task, a)
			d.Criteria[0].Evidence = []string{"docs/evidence-one.txt", "docs/evidence-two.txt"}
			write := func(name, text string) {
				t.Helper()
				if err := os.WriteFile(filepath.Join(a.CWD, name), []byte(text), 0600); err != nil {
					t.Fatal(err)
				}
			}
			raw, _ := json.Marshal(d)
			write("docs/first.delivery.json", string(raw))
			// Each file fits on its own; their union exceeds the task capacity.
			write("docs/evidence-one.txt", strings.Repeat("a", 70*1024))
			write("docs/evidence-two.txt", strings.Repeat("b", 70*1024))
			gitTest(t, a.CWD, "add", "-A")
			req.EventID = "oversized-correction"
			req.Revision = w.Revision
			req.ResultTree = gitTest(t, a.CWD, "write-tree")
			beforeItem, _ := s.managedAttempt(a.ID)
			beforeCalls := managedReviewCalls(t, s)
			if _, err := s.planningChange(w.ID, action, req); err == nil || !strings.Contains(err.Error(), "128 Kio") {
				t.Fatal("unreviewable repair not refused at source preflight", err)
			}
			after, _ := s.get(w.ID)
			afterItem, _ := s.managedAttempt(a.ID)
			if !reflect.DeepEqual(w, after) || !reflect.DeepEqual(beforeItem, afterItem) || managedReviewCalls(t, s) != beforeCalls {
				t.Fatal("preflight changed result, review, revision, attempt or budget")
			}
			// Correct the files, not the capacity, and re-examine the exact tree.
			write("docs/evidence-one.txt", "Original execution evidence\n")
			write("docs/evidence-two.txt", "Explicit origin and limits\n")
			gitTest(t, a.CWD, "add", "-A")
			req.EventID = "bounded-correction"
			req.ResultTree = gitTest(t, a.CWD, "write-tree")
			managedReviewMode(t, s, "pass")
			accepted, err := s.planningChange(w.ID, action, req)
			if err != nil {
				t.Fatal(err)
			}
			if accepted.Tasks[0].Status != "accepted" || managedReviewCalls(t, s) != beforeCalls+1 || len(accepted.Tasks[0].Attempts) != 1 {
				t.Fatal("corrected evidence did not pass ordinary checks and review")
			}
		})
	}
}
