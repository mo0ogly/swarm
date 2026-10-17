//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestRecoveredResultCorrectionAfterPublicPolicyChange(t *testing.T) {
	s, w, a, r := recoveredResultFixture(t)
	managedReviewMode(t, s, "fail")
	w, err := s.planningChange(w.ID, "submit-recovered-result", r)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	old := *task.IndependentReview
	original, err := os.ReadFile(filepath.Join(s.root, old.Receipt))
	if err != nil {
		t.Fatal(err)
	}
	policy := *task.ValidationPolicy
	policy.Controls = append([]ValidationControl(nil), policy.Controls...)
	policy.Controls[0].Timeout++
	change := ValidationPolicyChange{Schema: 1, Revision: w.Revision, TaskID: a.TaskID, Intent: "replace", Policy: &policy}
	preview, err := s.previewValidationPolicy(w.ID, change)
	if err != nil {
		t.Fatal(err)
	}
	change.PreviewToken, change.EventID = preview.Token, "strengthen-after-refusal"
	w, err = s.applyValidationPolicy(w.ID, change)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = w.task(a.TaskID)
	if task.IndependentReview != nil || task.PreviousReviews[len(task.PreviousReviews)-1].ID != old.ID {
		t.Fatal("policy failed to preserve archived review")
	}
	if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("Corrected implementation evidence after explicit controller policy change."), 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	next := r
	next.EventID, next.Revision, next.ReviewID = "correct-after-policy", w.Revision, old.ID
	next.ResultTree = gitTest(t, a.CWD, "write-tree")
	bad := next
	bad.ReviewID = "other-review"
	if _, err := s.planningChange(w.ID, "revise-recovered-result", bad); err == nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("unbound archive accepted or charged")
	}
	for _, mode := range []string{"contract", "producer", "attempt", "base", "unfinished", "unrelated-last", "missing-repair"} {
		raw, _ := json.Marshal(w)
		var changed Work
		if err := json.Unmarshal(raw, &changed); err != nil {
			t.Fatal(err)
		}
		ct, _ := changed.task(a.TaskID)
		prev := &ct.PreviousReviews[len(ct.PreviousReviews)-1]
		switch mode {
		case "contract":
			ct.Criteria = append(ct.Criteria, "A newly assigned criterion")
		case "producer":
			prev.Producer = "other-producer"
		case "attempt":
			prev.Attempt = "other-attempt"
		case "base":
			prev.PreviousCandidate = "other-base"
		case "unfinished":
			prev.Finished = ""
		case "unrelated-last":
			ct.PreviousReviews = append(ct.PreviousReviews, IndependentReview{ID: "unrelated"})
		case "missing-repair":
			ct.RecoveredResult = nil
		}
		if recoveryRevisionReview(changed, ct, next) != nil {
			t.Fatal("unrelated archive selected", mode)
		}
	}

	if err := os.WriteFile(filepath.Join(s.root, old.Receipt), []byte("altered archived receipt"), 0600); err != nil {
		t.Fatal(err)
	}
	rejected := next
	rejected.EventID = "reject-altered-archive"
	if _, err := s.planningChange(w.ID, "revise-recovered-result", rejected); err == nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("altered archive accepted or charged")
	}
	if err := os.WriteFile(filepath.Join(s.root, old.Receipt), original, 0600); err != nil {
		t.Fatal(err)
	}
	managedReviewMode(t, s, "pass")
	after, err := s.planningChange(w.ID, "revise-recovered-result", next)
	if err != nil {
		t.Fatal("a corrected policy must not strand a refused result", err)
	}
	current, _ := after.task(a.TaskID)
	if current.Status != "accepted" || current.RecoveredResult.PriorReview != old.ID || current.AutoValidation.PolicyDigest != validationPolicyDigest(policy) || current.IndependentReview.CandidateSHA == old.CandidateSHA || len(current.Attempts) != 1 || managedReviewCalls(t, s) != 2 {
		t.Fatal("fresh current controls, review or history missing", current.Status, current.Blocker)
	}
	kept, err := os.ReadFile(filepath.Join(s.root, old.Receipt))
	if err != nil || string(kept) != string(original) {
		t.Fatal("original refusal receipt changed")
	}
	if _, err := s.planningChange(w.ID, "revise-recovered-result", next); err != nil || managedReviewCalls(t, s) != 2 {
		t.Fatal("replay repeated review", err)
	}
}
