//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestIncrementalFinalTransportDistinguishesOmittedAndEmptyDiff(t *testing.T) {
	c := fragmentPlanFixture()
	c.Baseline = &ManagedReviewBaseline{Version: 2, Candidate: c.Previous}
	prefix := incrementalReviewPrefix("prefix", c)
	for _, required := range []string{"DIFF CANONIQUE ET TRANSPORT", "ne signifie pas un diff Git vide", "changes_since_observations", "rows", "unknown"} {
		if !strings.Contains(prefix, required) {
			t.Fatalf("missing transport distinction %q", required)
		}
	}
	p, err := planManagedReviewFragments(c, 12, 2)
	if err != nil {
		t.Fatal(err)
	}
	replies := []string{}
	diff := ""
	for i := range p.Packets {
		packet := &p.Packets[i]
		packet.FindingContractVersion = 1
		raw, _ := json.Marshal(packet)
		response := managedFragmentInspection{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PacketDigest: hash(raw)}
		for j, a := range packet.Artifacts {
			excerpt := a.Content
			if len(excerpt) > 40 {
				excerpt = excerpt[:40]
			}
			response.Findings = append(response.Findings, managedFragmentFinding{Artifact: j, Digest: a.Digest, Verdict: "inspected", Reason: "Independent current original inspected", Evidence: excerpt, Needs: []string{}})
			if a.Kind == "diff" {
				diff += a.Content
			}
		}
		raw, _ = json.Marshal(response)
		replies = append(replies, string(raw))
	}
	if diff != c.Diff {
		t.Fatal("inventory does not cover the complete canonical delta")
	}
	decision, visible, err := managedFragmentDecisionPrompt("", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	selection, err := managedFragmentRequestPromptVersion("", c, p, replies, 5)
	if err != nil {
		t.Fatal(err)
	}
	for _, prompt := range []string{decision, selection} {
		if !strings.Contains(prompt, "DIFF CANONIQUE ET TRANSPORT") || strings.Contains(prompt, c.Diff) {
			t.Fatal("transport omitted its scope or exposed an unselected complete diff")
		}
		for _, packet := range p.Packets {
			for _, a := range packet.Artifacts {
				if a.Kind == "diff" && !strings.Contains(prompt, a.Digest) {
					t.Fatal("changed-file inventory lost an original", a.Name)
				}
			}
		}
	}
	if visible.Diff != "" {
		t.Fatal("parser gained the omitted canonical diff")
	}
	for _, source := range visible.Sources {
		if source.Content == c.Diff {
			t.Fatal("hidden full diff became implementation evidence")
		}
	}
}

func TestIncrementalHistoricalPrefixRemainsStable(t *testing.T) {
	c := managedReviewContext{Baseline: &ManagedReviewBaseline{Version: 0, Candidate: "baseline"}}
	old := incrementalReviewPrefix("prefix", c)
	c.Baseline.Version = 1
	if incrementalReviewPrefix("prefix", c) != old || strings.Contains(old, "DIFF CANONIQUE ET TRANSPORT") {
		t.Fatal("historical protocol changed")
	}
	if hash([]byte(old)) != "ab9be60c2918941a66c63c6da79c7198d1e8266228c03298881d6943519ad261" {
		t.Fatal("historical prefix bytes changed")
	}
}
