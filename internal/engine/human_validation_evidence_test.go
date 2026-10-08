//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestHumanControlsProduceEvidenceNeverAcceptance(t *testing.T) {
	policy := automaticPolicy("go", "version")
	policy.Mode = "human"
	s, w, a, report := automaticValidationFixture(t, policy, true)
	s.conduct(a, "completed")
	got, _ := s.get(w.ID)
	task, _ := got.task("t1")
	downstream, _ := got.task("t2")
	if task.Status != "submitted" || task.Gate != nil || task.AutoValidation == nil || task.AutoValidation.State != "pending_human" {
		t.Fatalf("human review bypassed: %+v", task)
	}
	if s.dependenciesReady(&got, downstream) {
		t.Fatal("downstream released before human decision")
	}
	evidence, _, err := s.independentValidationEvidence(task)
	if err != nil || evidence == "" {
		t.Fatalf("evidence not delivered: %s %v", evidence, err)
	}
	if changed, err := s.resumeAutomaticValidations(w.ID); changed || err != nil {
		t.Fatalf("unchanged human receipt reprocessed: %v %v", changed, err)
	}
	projection := s.taskEvidence(&got, task, false)
	if projection.Freshness != "fresh" || projection.Controls.Items[0].Freshness != "fresh" || projection.Acceptance.State == "accepted" {
		t.Fatalf("human receipt freshness confused with acceptance: %+v", projection)
	}
	at := task.AutoValidation.At
	if accepted, _ := s.runAutomaticValidation(a, report); accepted {
		t.Fatal("poll accepted human review")
	}
	got, _ = s.get(w.ID)
	task, _ = got.task("t1")
	if task.AutoValidation.At != at {
		t.Fatal("poll reran unchanged checks")
	}
	if accepted, _ := s.acceptReviewedValidation(got, task, a); accepted {
		t.Fatal("review auto-accepted human task")
	}
	os.WriteFile(filepath.Join(s.root, report), []byte("changed"), 0600)
	if _, _, err = s.independentValidationEvidence(task); err == nil {
		t.Fatal("changed evidence accepted")
	}
	projection = s.taskEvidence(&got, task, false)
	if projection.Freshness != "stale" || projection.Controls.Items[0].Freshness != "stale" {
		t.Fatal("changed human evidence shown fresh")
	}
}

func TestHumanControlCoverageAndBounds(t *testing.T) {
	p := automaticPolicy("go", "version")
	p.Mode = "human"
	if _, err := normalizeValidationPolicy(*p); err != nil {
		t.Fatal(err)
	}
	task := &Task{Criteria: []string{"test", "qualitative"}}
	if err := validationPolicyCoversTask(*p, task); err != nil {
		t.Fatal(err)
	}
	p.Controls[0].Criteria = []int{3}
	if err := validationPolicyCoversTask(*p, task); err == nil {
		t.Fatal("unknown criterion")
	}
	p.Controls[0].Inputs = []string{"../outside"}
	if _, err := normalizeValidationPolicy(*p); err == nil {
		t.Fatal("outside input")
	}
}

func TestHumanControlFailureIsNotEvidence(t *testing.T) {
	p := automaticPolicy("go", "tool", "missing-control")
	p.Mode = "human"
	s, w, a, _ := automaticValidationFixture(t, p, false)
	s.conduct(a, "completed")
	got, _ := s.get(w.ID)
	task, _ := got.task("t1")
	if task.Status != "blocked" {
		t.Fatal(task.Status)
	}
	if _, _, err := s.independentValidationEvidence(task); err == nil {
		t.Fatal("failed checks used for review")
	}
}

func TestHumanControlInputChangeInvalidatesEvidence(t *testing.T) {
	p := automaticPolicy("go", "version")
	p.Mode = "human"
	p.Controls[0].Inputs = []string{"source.txt"}
	s, w, a, _ := automaticValidationFixture(t, p, false)
	path := filepath.Join(s.root, "source.txt")
	if err := os.WriteFile(path, []byte("original"), 0600); err != nil {
		t.Fatal(err)
	}
	s.conduct(a, "completed")
	got, _ := s.get(w.ID)
	task, _ := got.task("t1")
	if _, _, err := s.independentValidationEvidence(task); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte("modified"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, _, err := s.independentValidationEvidence(task); err == nil {
		t.Fatal("changed source accepted")
	}
}
func TestHumanPolicyChangeArchivesReviewAndGuardsRunning(t *testing.T) {
	for _, state := range []string{"running", "unknown"} {
		t.Run(state, func(t *testing.T) {
			s, w := validationConfigFixture(t)
			w.Tasks[0].Status = "submitted"
			w.Tasks[0].IndependentReview = &IndependentReview{ID: "old-review", State: state}
			raw, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			change := automaticChange(w, []string{"go", "version"})
			change.Policy.Mode = "human"
			preview, err := s.previewValidationPolicy(w.ID, change)
			if err != nil {
				t.Fatal(err)
			}
			change.EventID = "human-policy"
			change.PreviewToken = preview.Token
			got, err := s.applyValidationPolicy(w.ID, change)
			if state == "running" {
				if err == nil {
					t.Fatal("active review changed")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			task := got.Tasks[0]
			if task.Status != "submitted" || task.IndependentReview != nil || len(task.PreviousReviews) != 1 {
				t.Fatalf("review not archived: %+v", task)
			}
		})
	}
}
