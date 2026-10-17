//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestReferencedCitationFinalContractMaterializesNativeFailureForms(t *testing.T) {
	for _, original := range []string{"+\tv, err := s.trustOperate(action, cmd, os.Getenv(\"SWARM_TRUST_TOKEN\"), time.Now().UTC())\n", `{"stdout":"PASS i18n build check accepts current output and refuses stale output without mutation\n"}`, "+\tif pos[0] >= len(items) {\n+\t\treturn err\n+\t}\n"} {
		t.Run(hash([]byte(original))[:12], func(t *testing.T) {
			c := fragmentPlanFixture()
			c.CitationContractVersion = 2
			c.Tasks[0].Report = original
			c, p, replies, _, _ := fragmentContractQuestionFromContext(t, c)
			prompt, visible, err := managedFragmentDecisionPrompt("", c, p, replies, nil)
			if err != nil {
				t.Fatal(err)
			}
			catalog, err := managedCitationCatalogForContext(visible, p.ContextDigest)
			if err != nil {
				t.Fatal(err)
			}
			reference := -1
			for _, quote := range catalog.Quotes {
				if quote.Text == original {
					reference = quote.ID
				}
			}
			if reference < 0 || !strings.Contains(prompt, catalog.digest()) {
				t.Fatal("original or catalogue attribution absent")
			}
			reply := referencedCitationReply(t, c, catalog, reference, true)
			state, records, err := parseManagedFragmentDecision(reply, visible, c, p, replies)
			if err != nil || state != "passed" || len(records) != 1 || records[0].Criteria[0].Evidence != original {
				t.Fatal(state, records, err)
			}
			if _, _, err = parseManagedFragmentDecision(fragmentContractDecision(t, c, original, original), visible, c, p, replies); err == nil {
				t.Fatal("legacy free-text reply accepted under the reference contract")
			}
			visible.Tasks[0].Report += "changed"
			if _, _, err = parseManagedFragmentDecision(reply, visible, c, p, replies); err == nil {
				t.Fatal("stale reference accepted")
			}
		})
	}
}

func TestReferencedCitationSingleAndBatchContracts(t *testing.T) {
	c := fragmentPlanFixture()
	c.CitationContractVersion = 2
	prompt, err := boundedManagedReviewPrompt("review", c)
	if err != nil || !strings.Contains(prompt, "citation_catalog") {
		t.Fatal(err)
	}
	catalog, err := managedCitationCatalogForContext(c, managedCitationContextDigest(c))
	if err != nil {
		t.Fatal(err)
	}
	reference := -1
	for _, quote := range catalog.Quotes {
		if quote.Text == c.Tasks[0].Report {
			reference = quote.ID
		}
	}
	if reference < 0 {
		t.Fatal("report missing")
	}
	reply := referencedCitationReply(t, c, catalog, reference, false)
	state, records, err := parseManagedReview(reply, c)
	if err != nil || state != "passed" || len(records) != 1 {
		t.Fatal(state, err)
	}
	second := c.Tasks[0]
	second.Task = "second-task"
	second.Binding.Task = second.Task
	second.Report = "Second independent original report"
	c.Tasks = append(c.Tasks, second)
	catalog, err = managedCitationCatalogForContext(c, managedCitationContextDigest(c))
	if err != nil {
		t.Fatal(err)
	}
	reply = referencedCitationReply(t, c, catalog, reference, false)
	b := managedReviewBatch{Tasks: []string{c.Tasks[0].Task}, Context: c}
	state, records, err = parseManagedBatchReply(reply, b)
	if err != nil || state != "passed" || len(records) != 1 {
		t.Fatal("catalogue was rebound to the task subset", state, err)
	}
	b.Tasks = []string{second.Task}
	if _, _, err = parseManagedBatchReply(reply, b); err == nil {
		t.Fatal("foreign task accepted")
	}
}

func TestReferencedCitationBatchSchemaMatchesAssignment(t *testing.T) {
	c := fragmentPlanFixture()
	c.CitationContractVersion = 2
	second := c.Tasks[0]
	second.Task = "another-task"
	c.Tasks = append(c.Tasks, second)
	b := managedReviewBatch{Context: c, Tasks: []string{c.Tasks[0].Task}}
	raw, err := managedReviewSchemaForBatch(b)
	if err != nil {
		t.Fatal(err)
	}
	var schema map[string]any
	if err = json.Unmarshal([]byte(raw), &schema); err != nil {
		t.Fatal(err)
	}
	tasks := schema["properties"].(map[string]any)["tasks"].(map[string]any)
	ids := tasks["items"].(map[string]any)["properties"].(map[string]any)["task"].(map[string]any)["enum"].([]any)
	if tasks["minItems"] != float64(1) || tasks["maxItems"] != float64(1) || len(ids) != 1 || ids[0] != b.Tasks[0] {
		t.Fatal("provider schema contradicts the assigned batch", tasks["minItems"], tasks["maxItems"], ids)
	}
}

func TestReferencedCitationWireBoundPrecedesMaterialization(t *testing.T) {
	c := fragmentPlanFixture()
	c.CitationContractVersion = 2
	c.Tasks[0].Report = strings.Repeat("original proof with tabs\tand backslash\\n ", 18)
	c.Tasks[0].Criteria = make([]string, 150)
	for i := range c.Tasks[0].Criteria {
		c.Tasks[0].Criteria[i] = "Independently evaluate the current original"
	}
	catalog, err := managedCitationCatalogForContext(c, managedCitationContextDigest(c))
	if err != nil {
		t.Fatal(err)
	}
	reference := -1
	for _, quote := range catalog.Quotes {
		if strings.HasPrefix(quote.Text, "original proof") {
			reference = quote.ID
			break
		}
	}
	var reply map[string]any
	if err = json.Unmarshal([]byte(referencedCitationReply(t, c, catalog, reference, false)), &reply); err != nil {
		t.Fatal(err)
	}
	task := reply["tasks"].([]any)[0].(map[string]any)
	criteria := []any{}
	for i := range c.Tasks[0].Criteria {
		criteria = append(criteria, map[string]any{"index": i + 1, "verdict": "pass", "evidence_ref": reference})
	}
	task["criteria"] = criteria
	raw, _ := json.Marshal(reply)
	decoded, err := decodeManagedReferencedDecision(string(raw), catalog, c.Candidate, catalog.ContextDigest, false)
	if err != nil || len(raw) >= managedFragmentReplyLimit || len(decoded) <= managedFragmentReplyLimit {
		t.Fatal("expansion counterexample missing", len(raw), len(decoded), err)
	}
	state, records, err := parseManagedReview(string(raw), c)
	if err != nil || state != "passed" || len(records[0].Criteria) != 150 {
		t.Fatal(state, err)
	}
	if _, _, err = parseManagedReview(string(raw)+strings.Repeat(" ", structuredProviderReplyLimit), c); err == nil {
		t.Fatal("oversized provider reply accepted")
	}
}
