//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func fragmentContractQuestionFixture(t *testing.T, criterion string, sources ...ReviewSource) (managedReviewContext, managedReviewFragmentPlan, []string, string, managedReviewContext) {
	t.Helper()
	c := managedReviewContext{
		Sources:   sources,
		Candidate: strings.Repeat("b", 40),
		Previous:  strings.Repeat("a", 40),
		Diff:      "diff --git a/value.txt b/value.txt\n+Original implementation behaviour\n",
		Receipt:   json.RawMessage(`{"candidate_commit":"` + strings.Repeat("b", 40) + `"}`),
		Tasks: []managedReviewTaskContext{{
			Task: "bounded-task", Title: "A bounded implementation", Deliverable: "Candidate implementation",
			Criteria: []string{criterion}, Report: "Original current implementation report",
			Delivery: &ManagedDelivery{Criteria: []DeliveryCriterion{{Reason: "Producer opinion must not resolve a question"}}},
			Binding:  ManagedTaskReview{Reason: "Prior review opinion must not resolve a question"},
		}},
	}
	p, err := planManagedReviewFragments(c, 12, 2)
	if err != nil {
		t.Fatal(err)
	}
	replies := []string{}
	for _, packet := range p.Packets {
		encoded, _ := json.Marshal(packet)
		r := managedFragmentInspection{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PacketDigest: hash(encoded)}
		for i, a := range packet.Artifacts {
			excerpt := a.Content
			if len(excerpt) > 40 {
				excerpt = excerpt[:40]
			}
			finding := managedFragmentFinding{Artifact: i, Digest: a.Digest, Verdict: "inspected", Reason: "Independent original inspection", Evidence: excerpt, Needs: []string{}}
			if packet.Index == 0 && i == 0 {
				finding.Verdict = "unknown"
				finding.Needs = []string{"What exact current criterion was assigned to this task?"}
			}
			r.Findings = append(r.Findings, finding)
		}
		encoded, _ = json.Marshal(r)
		replies = append(replies, string(encoded))
	}
	prompt, visible, err := managedFragmentDecisionPrompt("", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	return c, p, replies, prompt, visible
}

func fragmentContractDecision(t *testing.T, c managedReviewContext, quote, implementationQuote string) string {
	t.Helper()
	response := map[string]any{
		"review": map[string]any{"candidate_commit": c.Candidate, "tasks": []any{
			map[string]any{"task": c.Tasks[0].Task, "reason": "Current original implementation evidence reviewed", "criteria": []any{
				map[string]any{"index": 1, "verdict": "pass", "evidence": implementationQuote},
			}},
		}},
		"resolutions": []any{map[string]any{"packet": 0, "artifact": 0, "need_index": 0, "verdict": "resolved", "reason": "The assigned original criterion is present in this exact current context", "evidence": quote}},
	}
	raw, err := json.Marshal(response)
	if err != nil {
		t.Fatal(err)
	}
	return string(raw)
}

func TestManagedFragmentResolutionAcceptsVisibleTaskContract(t *testing.T) {
	for _, criterion := range []string{
		// Original failing quotation, kept without a mission-specific exception.
		"Matrice JAN-1..JAN-7 et FEB-1..FEB-4 avec faits, hypothèses et preuves distincts.",
		"Evidence must cover a fresh candidate and its current scope.",
		"Préserver les \"droits\" <actuels> & les preuves\navec leur attribution.",
	} {
		t.Run(criterion, func(t *testing.T) {
			c, p, replies, prompt, visible := fragmentContractQuestionFixture(t, criterion)
			raw, _ := json.Marshal(c.Tasks[0].Criteria)
			quote := `"criteria":` + string(raw)
			if !strings.Contains(prompt, quote) {
				t.Fatal("contract quotation was not actually provided to reviewer")
			}
			before, _ := json.Marshal(c)
			state, records, err := parseManagedFragmentDecision(fragmentContractDecision(t, c, quote, c.Tasks[0].Report), visible, c, p, replies)
			if err != nil || state != "passed" || len(records) != 1 {
				t.Fatalf("visible original task contract was rejected: state=%q err=%v", state, err)
			}
			after, _ := json.Marshal(c)
			if string(before) != string(after) {
				t.Fatal("original context changed")
			}
		})
	}
}

func TestManagedFragmentContractResolutionDoesNotAdmitOpinionsOrOldContracts(t *testing.T) {
	c, p, replies, prompt, visible := fragmentContractQuestionFixture(t, "Only the current assigned criterion is authoritative.")
	for _, quote := range []string{
		"Producer opinion must not resolve a question",
		"Prior review opinion must not resolve a question",
		`"criteria":["An obsolete criterion from another candidate."]`,
		"Invented evidence absent from every original",
	} {
		t.Run(quote, func(t *testing.T) {
			if strings.Contains(quote, "opinion") && !strings.Contains(prompt, quote) {
				t.Fatal("test did not exercise a visible opinion")
			}
			_, _, err := parseManagedFragmentDecision(fragmentContractDecision(t, c, quote, c.Tasks[0].Report), visible, c, p, replies)
			if err == nil {
				t.Fatal("opinion, obsolete contract or invented evidence admitted")
			}
		})
	}
}

func TestManagedFragmentTaskContractCannotProveImplementationPass(t *testing.T) {
	c, p, replies, _, visible := fragmentContractQuestionFixture(t, "The contract describes a requirement, not its implementation.")
	raw, _ := json.Marshal(c.Tasks[0].Criteria)
	quote := `"criteria":` + string(raw)
	_, _, err := parseManagedFragmentDecision(fragmentContractDecision(t, c, quote, quote), visible, c, p, replies)
	if err == nil || strings.Contains(err.Error(), "résolution sans preuve") {
		t.Fatalf("resolution should accept the contract, but business pass must refuse it: %v", err)
	}
}

func TestManagedFragmentSelectedSourceKeepsOriginalTextBytes(t *testing.T) {
	content := "Original candidate source\nTwo lines with \"quotes\", <tags> and & Unicode é.\nPreserve the\twhitespace exactly.\n"
	source := ReviewSource{Path: "candidate-proof.txt", Blob: strings.Repeat("c", 40), Digest: hash([]byte(content)), Bytes: len(content), Content: content}
	c, p, replies, _, _ := fragmentContractQuestionFixture(t, "Verify the selected original source.", source)
	var refs []managedFragmentEvidenceRef
	for i, packet := range p.Packets {
		for j, a := range packet.Artifacts {
			if a.Kind == "source" {
				refs = append(refs, managedFragmentEvidenceRef{Packet: i, Artifact: j, Digest: a.Digest})
			}
		}
	}
	if len(refs) != 1 {
		t.Fatal("source fixture did not reach the actual selection transport")
	}
	raw, _ := json.Marshal(c.Tasks[0].Criteria)
	contractQuote := `"criteria":` + string(raw)
	quote := "Two lines with \"quotes\", <tags> and & Unicode é.\nPreserve the\twhitespace exactly."
	for _, selected := range []bool{false, true} {
		t.Run(map[bool]string{false: "unselected", true: "selected"}[selected], func(t *testing.T) {
			chosen := []managedFragmentEvidenceRef(nil)
			if selected {
				chosen = refs
			}
			prompt, visible, err := managedFragmentDecisionPrompt("", c, p, replies, chosen)
			if err != nil {
				t.Fatal(err)
			}
			if selected && !strings.Contains(prompt, "requested_original_evidence") {
				t.Fatal("selected original was not actually sent")
			}
			state, _, err := parseManagedFragmentDecision(fragmentContractDecision(t, c, contractQuote, quote), visible, c, p, replies)
			if selected {
				if err != nil || state != "passed" {
					t.Fatalf("literal selected source text rejected: %q %v", state, err)
				}
				flattened := strings.ReplaceAll(quote, "\n", " ")
				flattened = strings.ReplaceAll(flattened, "\t", " ")
				if _, _, err := parseManagedFragmentDecision(fragmentContractDecision(t, c, contractQuote, flattened), visible, c, p, replies); err == nil {
					t.Fatal("selected source paraphrase/flattened whitespace admitted")
				}
			} else if err == nil {
				t.Fatal("unselected source body admitted from omitted canonical context")
			}
		})
	}
}

func TestManagedReviewEvidencePreservesArtifactKindsAndBoundaries(t *testing.T) {
	for _, tc := range []struct {
		name, path, body, quote string
		pass                    bool
	}{
		{"literal-code", "value.go", "if ready {\n\treturn nil\n}", "if ready {\n\treturn nil", true},
		{"flattened-code", "value.go", "if ready {\n\treturn nil\n}", "if ready { return nil", false},
		{"literal-json", "receipt.json", "{\"mode\":\"on\",\n\"passed\":true}", "\"mode\":\"on\",\n\"passed\":true", true},
		{"flattened-json", "receipt.json", "{\"mode\":\"on\",\n\"passed\":true}", "\"mode\":\"on\", \"passed\":true", false},
		{"existing-prose-formatting", "note.md", "Original wrapped prose\nhas evidence.", "Original wrapped prose has evidence.", true},
		{"fenced-code-remains-literal", "note.md", "```go\nif ready {\n\treturn nil\n}\n```", "if ready { return nil", false},
		{"no-cross-source-concatenation", "one.txt", "tail evidence", "tail evidence\nhead evidence", false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			c := managedReviewContext{Candidate: strings.Repeat("d", 40), Tasks: []managedReviewTaskContext{{Task: "typed-task", Criteria: []string{"Check the actual original"}, Report: "Independent report text differs from the candidate proof"}}, Sources: []ReviewSource{{Path: tc.path, Content: tc.body}, {Path: "two.txt", Content: "head evidence"}}}
			raw, _ := json.Marshal(map[string]any{"candidate_commit": c.Candidate, "tasks": []any{map[string]any{"task": c.Tasks[0].Task, "reason": "Original typed evidence examined", "criteria": []any{map[string]any{"index": 1, "verdict": "pass", "evidence": tc.quote}}}}})
			state, _, err := parseManagedReview(string(raw), c)
			if tc.pass {
				if err != nil || state != "passed" {
					t.Fatalf("valid typed evidence rejected: %q %v", state, err)
				}
			} else if err == nil {
				t.Fatal("reformatted code/data or concatenated originals admitted")
			}
		})
	}
}
