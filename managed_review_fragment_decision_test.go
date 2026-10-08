//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestManagedFragmentDecisionOnlyVisibleOriginals(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	before, _ := json.Marshal(c)
	prompt, visible, err := managedFragmentDecisionPrompt("review guidance", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	after, _ := json.Marshal(c)
	if string(before) != string(after) {
		t.Fatal("canonical context mutated")
	}
	if visible.Diff != "" || len(visible.Tasks) != len(c.Tasks) || !strings.Contains(prompt, "DECISION FINALE INDEPENDANTE") {
		t.Fatal("final contract absent")
	}
	if len(prompt)+len(managedReviewSchema) > managedReviewPromptLimit {
		t.Fatal("oversized prompt")
	}
	for _, source := range visible.Sources {
		if len(source.Content) > 40 {
			t.Fatal("unseen complete source allowed")
		}
		if strings.Contains(source.Content, "Independent artifact inspection") {
			t.Fatal("opinion used as evidence")
		}
	}
	a := p.Packets[0].Artifacts[0]
	_, selected, err := managedFragmentDecisionPrompt("review guidance", c, p, replies, []managedFragmentEvidenceRef{{0, 0, a.Digest}})
	if err != nil {
		t.Fatal(err)
	}
	if selected.Sources[len(selected.Sources)-1].Content != a.Content {
		t.Fatal("requested original not delivered")
	}
}
func TestManagedFragmentDecisionRefusesIncompleteOrOversized(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	if _, _, err := managedFragmentDecisionPrompt("", c, p, replies[:len(replies)-1], nil); err == nil {
		t.Fatal("incomplete inspections allowed")
	}
	if prompt, _, err := managedFragmentDecisionPrompt(strings.Repeat("x", managedReviewPromptLimit), c, p, replies, nil); err == nil || prompt != "" {
		t.Fatal("oversized decision emitted")
	}
}

func TestManagedFragmentDecisionRejectsCitationNotPresented(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	_, visible, err := managedFragmentDecisionPrompt("", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	quote := strings.Repeat("+évidence\n", 8)
	if !strings.Contains(c.Diff, quote) {
		t.Fatal("fixture lacks hidden evidence")
	}
	tasks := []map[string]any{}
	for _, task := range c.Tasks {
		criteria := []ReviewCriterion{}
		for i := range task.Criteria {
			criteria = append(criteria, ReviewCriterion{Index: i + 1, Verdict: "pass", Evidence: quote})
		}
		tasks = append(tasks, map[string]any{"task": task.Task, "reason": "Criterion supported by cited original evidence", "criteria": criteria})
	}
	raw, _ := json.Marshal(map[string]any{"candidate_commit": c.Candidate, "tasks": tasks})
	if state, _, err := parseManagedReview(string(raw), c); err != nil || state != "passed" {
		t.Fatal("fixture full-context verdict invalid", state, err)
	}
	if state, _, err := parseManagedReview(string(raw), visible); err == nil && state == "passed" {
		t.Fatal("unseen citation validated final verdict")
	}
	a := p.Packets[0].Artifacts[0]
	_, selected, err := managedFragmentDecisionPrompt("", c, p, replies, []managedFragmentEvidenceRef{{0, 0, a.Digest}})
	if err != nil {
		t.Fatal(err)
	}
	if state, _, err := parseManagedReview(string(raw), selected); err != nil || state != "passed" {
		t.Fatal("requested original citation was not available", state, err)
	}
}
