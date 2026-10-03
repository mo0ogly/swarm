//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestReviewOutputRequiresExplicitAuthorization(t *testing.T) {
	c := ValidationControl{ID: "observation", Command: []string{"python3", "-c", "print('observed=42')"}, Timeout: 10}
	off := runValidationControl(t.TempDir(), c)
	if !off.Passed || off.ReviewOutput != "" || off.OutputBytes != 0 {
		t.Fatalf("default leaked output: %+v", off)
	}
	c.ReviewOutput = true
	on := runValidationControl(t.TempDir(), c)
	if !on.Passed || on.ReviewOutput != "observed=42\n" || on.OutputBytes != 12 || on.ReviewOutputTruncated || on.OutputHash != hash([]byte(on.ReviewOutput)) {
		t.Fatalf("actual observation missing: %+v", on)
	}
	p := ValidationPolicy{Mode: "human", Controls: []ValidationControl{c}}
	shared := validationPolicyDigest(p)
	p.Controls[0].ReviewOutput = false
	if validationPolicyDigest(p) == shared {
		t.Fatal("authorization absent from policy digest")
	}
}

func TestReviewOutputExplicitlyMarksTruncatedObservation(t *testing.T) {
	c := ValidationControl{ID: "large", Command: []string{"python3", "-c", "print('x'*100000)"}, Timeout: 10, ReviewOutput: true}
	r := runValidationControl(t.TempDir(), c)
	if !r.Passed || len(r.ReviewOutput) != maxValidationReviewOutput || r.OutputBytes != 100001 || !r.ReviewOutputTruncated {
		t.Fatalf("unbounded or false complete output: %+v", r)
	}
}

func TestReviewOutputFlowsThroughPersistedEngineReceipt(t *testing.T) {
	p := automaticPolicy("python3", "-c", "print('modal_closed=true focus_returned=true')")
	p.Mode = "human"
	p.Controls[0].ReviewOutput = true
	s, w, a, _ := automaticValidationFixture(t, p, false)
	s.conduct(a, "completed")
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := current.task(a.TaskID)
	evidence, _, err := s.independentValidationEvidence(task)
	if err != nil || !strings.Contains(evidence, "modal_closed=true focus_returned=true") {
		t.Fatalf("captured output unavailable to reviewer: %s %v", evidence, err)
	}
	task.ValidationPolicy.Controls[0].ReviewOutput = false
	if _, _, err = s.independentValidationEvidence(task); err == nil {
		t.Fatal("receipt reused after authorization changed")
	}
	task.ValidationPolicy.Controls[0].ReviewOutput = true
	task.AutoValidation.Controls[0].ReviewOutputTruncated = true
	if _, _, err = s.independentValidationEvidence(task); err == nil {
		t.Fatal("incoherent capture accepted")
	}
}
