//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func changedReviewMethodFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w, a := planningProtocolReviewFixture(t)
	w, err := s.mutate(w.ID, "test.changed-method", "changed-method", w.Revision, []byte(`{}`), func(current *Work) error {
		task, _ := current.task(a.TaskID)
		task.IndependentReview.FragmentJournal.WorkflowDigest = strings.Repeat("a", 64)
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return s, w, a
}

func TestReviewMethodRestartRequiresExplicitCorrectionAndKeepsHistory(t *testing.T) {
	s, w, a := changedReviewMethodFixture(t)
	task, _ := w.task(a.TaskID)
	old := *task.IndependentReview
	w.Planning.Reviewer.TimeoutSeconds = 77
	rawWork, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", rawWork, w.ID); err != nil {
		t.Fatal(err)
	}
	preview, err := s.reviewRecoveryPreview(w.ID, a.TaskID)
	if err != nil || preview.State != "method_restart_ready" || preview.Required < 1 || preview.Reusable != 0 || preview.Timeout != 77 {
		t.Fatal(preview, err)
	}
	request := PlanningRequest{Schema: 1, EventID: "method-restart", Revision: w.Revision, Task: a.TaskID, Reason: "The review method has been corrected; rerun every inspection of the unchanged candidate."}
	if _, err := s.planningChange(w.ID, "retry-review", request); err == nil {
		t.Fatal("method change adopted without explicit confirmation")
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision {
		t.Fatal("unconfirmed operation mutated state")
	}
	request.ConfirmReviewErrorRepair = true
	after, err = s.planningChange(w.ID, "retry-review", request)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = after.task(a.TaskID)
	item, _ := s.managedAttempt(a.ID)
	if task.IndependentReview != nil || len(task.PreviousReviews) == 0 || task.PreviousReviews[len(task.PreviousReviews)-1].ID != old.ID || task.PreviousReviews[len(task.PreviousReviews)-1].FragmentJournal.WorkflowDigest != old.FragmentJournal.WorkflowDigest || task.PlanningRetry || len(task.Attempts) != 1 || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || item.State != "integrating" {
		t.Fatal("method restart reused an opinion, spent a call or lost history")
	}
	// Read-only preview and explicit restart cannot change the old journal.
	raw, err := os.ReadFile(filepath.Join(s.root, old.FragmentJournal.Journal))
	if err != nil || hash(raw) != old.FragmentJournal.JournalDigest {
		t.Fatal("old journal changed", err)
	}
}

func TestPlanningReviewRetryCanRestartChangedMethod(t *testing.T) {
	s, w, a := changedReviewMethodFixture(t)
	w, request := planningClaim(t, s, w, "root")
	request.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "The engine review method is corrected; inspect the complete candidate again."}}
	after, err := s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := after.task(a.TaskID)
	item, _ := s.managedAttempt(a.ID)
	if task.IndependentReview != nil || len(task.PreviousReviews) == 0 || task.PlanningRetry || item.State != "integrating" || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
		t.Fatal("planner restart altered production or reused an opinion")
	}
}

func TestChangedReviewMethodCannotReuseRecoveredPriorOpinion(t *testing.T) {
	s, w, a := planningProtocolReviewFixture(t)
	task, _ := w.task(a.TaskID)
	old := *task.IndependentReview
	// Change actual project guidance through its public fixture configuration.
	// The stored review and journal keep their genuine original method digest.
	profileFixture(t, s)
	changed, _, err := s.fragmentReviewMethodChanged(&old)
	if err != nil || !changed {
		t.Fatal("method drift not established", err)
	}
	task.RecoveredResult = &RecoveredResult{PriorReview: old.ID}
	contextRaw, err := os.ReadFile(filepath.Join(s.root, old.Context))
	if err != nil {
		t.Fatal(err)
	}
	var context managedReviewContext
	if err := json.Unmarshal(contextRaw, &context); err != nil {
		t.Fatal(err)
	}
	plan, err := s.planLegacyCandidateFragments(w, a, context, w.Planning.Reviewer.MaxCalls-w.Planning.Reviewer.Calls)
	if err != nil || len(plan.Reused) != 0 {
		t.Fatal("old method blocked a full review or supplied reusable opinions", err)
	}
	if _, _, err := s.readFragmentJournalAnchor(old); err == nil {
		t.Fatal("ordinary reuse guard accepted the changed method")
	}
}

func TestReviewMethodRestartRejectsTamperedOriginalsAndSemanticRefusal(t *testing.T) {
	for _, mode := range []string{"journal", "receipt", "semantic refusal", "budget"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := changedReviewMethodFixture(t)
			task, _ := w.task(a.TaskID)
			switch mode {
			case "journal":
				if err := os.WriteFile(filepath.Join(s.root, task.IndependentReview.FragmentJournal.Journal), []byte(`{}`), 0600); err != nil {
					t.Fatal(err)
				}
			case "receipt":
				if err := os.WriteFile(filepath.Join(s.root, task.IndependentReview.Receipt), []byte(`{}`), 0600); err != nil {
					t.Fatal(err)
				}
			case "semantic refusal":
				task.IndependentReview.State = "changes_requested"
			case "budget":
				w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls
			}
			raw, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			if _, err := s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "rejected-method-restart", Revision: w.Revision, Task: a.TaskID, Reason: "The method has changed; preserve all original evidence.", ConfirmReviewErrorRepair: true}); err == nil {
				t.Fatal("invalid restart accepted")
			}
			after, _ := s.get(w.ID)
			if after.Revision != w.Revision || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
				t.Fatal("rejected restart changed state")
			}
		})
	}
}
