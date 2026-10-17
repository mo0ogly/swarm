//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestManagedReviewLiteralControlOutputRemainsOriginalEvidence(t *testing.T) {
	output := "{\"cases\":[{\"state\":\"reconciled\",\"ledger_entries\":1}],\"label\":\"é <proof> & original\"}\n\tExact control output."
	for _, tc := range []struct {
		name, quote    string
		supplied, pass bool
	}{
		{"literal", output, true, true},
		{"flattened", strings.ReplaceAll(output, "\n\t", " "), true, false},
		{"mutated", strings.ReplaceAll(output, "ledger_entries\":1", "ledger_entries\":2"), true, false},
		{"not-supplied", output, false, false},
		{"cross-control", "tail original\nhead original", true, false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			c := managedReviewContext{Candidate: strings.Repeat("d", 40), Tasks: []managedReviewTaskContext{{Task: "control-original", Criteria: []string{"Observe the attributed actual effect"}, Report: "Report differs from captured control evidence"}}}
			if tc.supplied {
				c.Tasks[0].Controls = []ValidationControlResult{{ID: "literal.md", ReviewOutput: output, OutputBytes: len(output), OutputHash: hash([]byte(output)), Executed: true, Passed: true}, {ID: "tail", ReviewOutput: "tail original"}, {ID: "head", ReviewOutput: "head original"}}
			}
			raw, _ := json.Marshal(map[string]any{"candidate_commit": c.Candidate, "tasks": []any{map[string]any{"task": c.Tasks[0].Task, "reason": "Original captured control bytes independently examined", "criteria": []any{map[string]any{"index": 1, "verdict": "pass", "evidence": tc.quote}}}}})
			state, _, err := parseManagedReview(string(raw), c)
			if tc.pass && (err != nil || state != "passed") {
				t.Fatalf("literal supplied output rejected: %s %v", state, err)
			}
			if !tc.pass && err == nil {
				t.Fatal("altered, hidden or concatenated control evidence admitted")
			}
		})
	}
}

func TestManagedFragmentResolutionUsesLiteralControlOutputBoundaries(t *testing.T) {
	output := "{\"verified\":true,\"count\":1}\n\tOriginal control evidence."
	c := managedReviewContext{Candidate: strings.Repeat("b", 40), Previous: strings.Repeat("a", 40), Diff: "diff --git a/value.txt b/value.txt\n+Original implementation behaviour\n", Receipt: json.RawMessage(`{}`), Tasks: []managedReviewTaskContext{{Task: "control-task", Criteria: []string{"Check the actual effect"}, Report: "Original current implementation report", Controls: []ValidationControlResult{{ID: "literal", ReviewOutput: output, Passed: true, Executed: true}, {ID: "tail", ReviewOutput: "tail original"}, {ID: "head", ReviewOutput: "head original"}}}}}
	c, p, replies, prompt, visible := fragmentContractQuestionFromContext(t, c)
	if !strings.Contains(prompt, `\"verified\":true`) {
		t.Fatal("control output not actually supplied")
	}
	for _, tc := range []struct {
		quote string
		pass  bool
	}{{output, true}, {strings.ReplaceAll(output, "\n\t", " "), false}, {"tail original\nhead original", false}} {
		state, _, err := parseManagedFragmentDecision(fragmentContractDecision(t, c, tc.quote, c.Tasks[0].Report), visible, c, p, replies)
		if tc.pass && (err != nil || state != "passed") {
			t.Fatalf("literal resolution rejected: %s %v", state, err)
		}
		if !tc.pass && err == nil {
			t.Fatal("reformatted or cross-control resolution admitted")
		}
	}
}

func TestManagedFinalReviewFailuresRemainRoutable(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	planJSON, _ := json.Marshal(p)
	request := managedFragmentFinalRequest{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(planJSON), State: "ready", Reason: "Select unchanged original sources", References: []managedFragmentEvidenceRef{{0, 0, "incorrect-digest"}}}
	raw, _ := json.Marshal(request)
	qc, qp, qr, _, visible := fragmentContractQuestionFixture(t, "Current original contract")
	cases := map[string]func() error{
		"final-citation": func() error {
			_, _, _, err := reviewReplyWithEvidence(`{"reason":"Original candidate examined","criteria":[{"index":1,"verdict":"pass","evidence":"invented citation"}]}`, &Task{Criteria: []string{"Criterion"}}, nil)
			return err
		},
		"managed-json":     func() error { _, _, err := parseManagedReview(`{"unexpected":true}`, c); return err },
		"selection-digest": func() error { _, err := parseManagedFragmentFinalRequest(string(raw), "", c, p, replies); return err },
		"resolution-citation": func() error {
			_, _, err := parseManagedFragmentDecision(fragmentContractDecision(t, qc, "Invented absent original evidence", qc.Tasks[0].Report), visible, qc, qp, qr)
			return err
		},
	}
	for name, check := range cases {
		t.Run(name, func(t *testing.T) {
			err := check()
			if err == nil || commandFailure(err).Code != reviewFragmentProtocolError {
				t.Fatalf("reply failure cannot reach its protocol owner: %v (%s)", err, commandFailure(err).Code)
			}
			task := Task{Attempts: []Attempt{{ID: "attempt"}}, IndependentReview: &IndependentReview{Attempt: "attempt", State: "error", FailureCode: commandFailure(err).Code}}
			if !currentReviewProtocolFailure(&task) {
				t.Fatal("typed final failure lost recovery routing")
			}
		})
	}
	t.Run("immutable-proof-is-not-a-response-error", func(t *testing.T) {
		p.Packets[0].Artifacts[0].Content += " altered"
		_, err := parseManagedFragmentFinalRequest(string(raw), "", c, p, replies)
		if err == nil || commandFailure(err).Code == reviewFragmentProtocolError {
			t.Fatal("damaged original authorized a protocol retry", err)
		}
	})
	t.Run("business-refusal-stays-independent", func(t *testing.T) {
		state, _, _, err := reviewReplyWithEvidence(`{"reason":"Product defect independently confirmed","criteria":[{"index":1,"verdict":"fail","evidence":"Missing required implementation"}]}`, &Task{Criteria: []string{"Criterion"}}, nil)
		if err != nil || state != "changes_requested" {
			t.Fatal("product refusal became a protocol error", state, err)
		}
	})
}
