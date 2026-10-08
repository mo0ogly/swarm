//go:build linux

package main

import (
	"encoding/json"
	"os"
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

func TestReviewOutputLiteralQuotationSourcesAreBoundedAndExact(t *testing.T) {
	t.Run("complete authorized JSON observation", func(t *testing.T) {
		p := automaticPolicy("python3", "-c", `print('{"observed":true,"count":2}')`)
		p.Mode = "human"
		p.Controls[0].ReviewOutput = true
		s, w, a, _ := automaticValidationFixture(t, p, false)
		s.conduct(a, "completed")
		current, err := s.get(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		task, _ := current.task(a.TaskID)
		controls, _, observations, err := s.independentValidationReviewEvidence(task)
		if err != nil || len(observations) != 1 || observations[0] != "{\"observed\":true,\"count\":2}\n" {
			t.Fatalf("literal observation unavailable: %#v %v", observations, err)
		}
		if !strings.Contains(controls, a.Attempt) || !strings.Contains(controls, task.AutoValidation.PolicyDigest) {
			t.Fatal("observation lost its attempt or policy-digest binding")
		}
		sources := "report text\n\n" + controls + "\n\n" + strings.Join(observations, "\n\n")
		reply, _ := json.Marshal(map[string]any{"reason": "La sortie moteur JSON contient l'observation attendue", "criteria": []ReviewCriterion{{Index: 1, Verdict: "pass", Evidence: `"observed":true`}}})
		state, _, _, err := reviewReply(string(reply), &Task{Criteria: []string{"observation JSON"}}, sources)
		if err != nil || state != "passed" {
			t.Fatal("decoded engine observation was not quotable", state, err)
		}
		altered, _ := json.Marshal(map[string]any{"reason": "Une altération ne doit pas être acceptée comme preuve", "criteria": []ReviewCriterion{{Index: 1, Verdict: "pass", Evidence: `"observed":false`}}})
		if _, _, _, err = reviewReply(string(altered), &Task{Criteria: []string{"observation JSON"}}, sources); err == nil {
			t.Fatal("altered observation accepted")
		}
	})

	t.Run("unshared observation", func(t *testing.T) {
		p := automaticPolicy("python3", "-c", `print('{"private":true}')`)
		p.Mode = "human"
		s, w, a, _ := automaticValidationFixture(t, p, false)
		s.conduct(a, "completed")
		current, _ := s.get(w.ID)
		task, _ := current.task(a.TaskID)
		controls, _, observations, err := s.independentValidationReviewEvidence(task)
		if err != nil || len(observations) != 0 || strings.Contains(controls, `"private":true`) {
			t.Fatalf("unshared output leaked: %#v %v", observations, err)
		}
	})

	t.Run("truncated observation remains partial", func(t *testing.T) {
		p := automaticPolicy("python3", "-c", "print('x'*100000)")
		p.Mode = "human"
		p.Controls[0].ReviewOutput = true
		s, w, a, _ := automaticValidationFixture(t, p, false)
		s.conduct(a, "completed")
		current, _ := s.get(w.ID)
		task, _ := current.task(a.TaskID)
		controls, _, observations, err := s.independentValidationReviewEvidence(task)
		if err != nil || len(observations) != 0 || !strings.Contains(controls, `"review_output_truncated":true`) {
			t.Fatalf("truncated observation became a full quotation source: %#v %v", observations, err)
		}
		sources := "exact report remains readable and coherent\n\n" + controls
		reply, _ := json.Marshal(map[string]any{"reason": "Le rapport exact reste une source indépendante de la capture partielle", "criteria": []ReviewCriterion{{Index: 1, Verdict: "pass", Evidence: "exact report remains readable"}}})
		if state, _, _, err := reviewReply(string(reply), &Task{Criteria: []string{"rapport exact"}}, sources); err != nil || state != "passed" {
			t.Fatal("truncated capture prevented use of the exact report", state, err)
		}
	})
}

func TestIndependentReviewPreflightUsesLiteralObservationBeforeReservation(t *testing.T) {
	p := automaticPolicy("python3", "-c", `print('{"observed":true}')`)
	p.Mode = "human"
	p.Controls[0].ReviewOutput = true
	s, w, a, _ := automaticValidationFixture(t, p, false)
	s.conduct(a, "completed")
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := current.task(a.TaskID)
	task.IndependentReview = nil
	body, _ := json.Marshal(current)
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID); err != nil {
		t.Fatal(err)
	}
	providers, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	provider := providers.Providers[current.Planning.Reviewer.Provider]
	response := `{"reason":"La sortie moteur JSON contient la valeur attendue","criteria":[{"index":1,"verdict":"pass","evidence":"\"observed\":true"}]}`
	envelope, _ := json.Marshal(map[string]any{"type": "result", "result": response})
	if err = os.WriteFile(provider.Command, []byte("#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' '"+string(envelope)+"'\n"), 0700); err != nil {
		t.Fatal(err)
	}
	if err = s.independentReviewStep(w.ID); err != nil {
		t.Fatal(err)
	}
	got, _ := s.get(w.ID)
	gotTask, _ := got.task(a.TaskID)
	if gotTask.IndependentReview == nil || gotTask.IndependentReview.State != "passed" || got.Planning.Reviewer.Calls != 1 {
		t.Fatalf("literal observation did not pass through the paid review boundary: %+v", gotTask.IndependentReview)
	}
	prompt, err := os.ReadFile(provider.Command + ".prompt")
	if err != nil || !strings.Contains(string(prompt), `\"review_output\":\"{\\\"observed\\\":true}\\n\"`) {
		t.Fatalf("reviewer did not receive the actual captured observation: %v", err)
	}
}
