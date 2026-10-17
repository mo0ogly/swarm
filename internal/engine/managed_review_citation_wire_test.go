//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func referencedCitationFixture(t *testing.T) (managedReviewContext, managedReviewFragmentPlan, []string, managedReviewContext, managedCitationCatalog, int) {
	t.Helper()
	c, p, replies, _, visible := fragmentContractQuestionFixture(t, "Judge the actual supplied implementation evidence.")
	originals, err := managedFragmentResolutionOriginals(visible)
	if err != nil {
		t.Fatal(err)
	}
	catalog, err := buildManagedCitationCatalog(c.Candidate, p.ContextDigest, originals)
	if err != nil {
		t.Fatal(err)
	}
	reference := -1
	for _, q := range catalog.Quotes {
		if q.Text == c.Tasks[0].Report {
			reference = q.ID
		}
	}
	if reference < 0 {
		t.Fatal("original report is absent from the reference catalogue")
	}
	return c, p, replies, visible, catalog, reference
}

func referencedCitationReply(t *testing.T, c managedReviewContext, catalog managedCitationCatalog, reference int, questions bool) string {
	t.Helper()
	review := map[string]any{"candidate_commit": c.Candidate, "tasks": []any{map[string]any{"task": c.Tasks[0].Task, "reason": "Independent assessment of the supplied original implementation", "criteria": []any{map[string]any{"index": 1, "verdict": "pass", "evidence_ref": reference}}}}}
	var reply map[string]any
	if questions {
		reply = map[string]any{"review": review, "resolutions": []any{map[string]any{"packet": 0, "artifact": 0, "need_index": 0, "verdict": "resolved", "reason": "The original document answers this scope question", "evidence_ref": reference}}}
	} else {
		reply = review
	}
	reply["catalog_sha256"] = catalog.digest()
	raw, err := json.Marshal(reply)
	if err != nil {
		t.Fatal(err)
	}
	return string(raw)
}

func TestReferencedCitationUsesExistingLiteralAndScopeGuards(t *testing.T) {
	c, p, replies, visible, catalog, reference := referencedCitationFixture(t)
	for _, questions := range []bool{true, false} {
		raw := referencedCitationReply(t, c, catalog, reference, questions)
		if strings.Contains(raw, c.Tasks[0].Report) {
			t.Fatal("provider is still required to copy original text")
		}
		decoded, err := decodeManagedReferencedDecision(raw, catalog, c.Candidate, p.ContextDigest, questions)
		if err != nil {
			t.Fatal(err)
		}
		var state string
		var records []ManagedTaskReview
		if questions {
			state, records, err = parseManagedFragmentDecision(decoded, visible, c, p, replies)
		} else {
			state, records, err = parseManagedReview(decoded, visible)
		}
		if err != nil || state != "passed" || len(records) != 1 || records[0].Criteria[0].Evidence != c.Tasks[0].Report {
			t.Fatal("reference did not materialize the exact existing original", state, err)
		}
	}
	metadata := -1
	for _, q := range catalog.Quotes {
		if q.Text == string(visible.Receipt) {
			metadata = q.ID
		}
	}
	if metadata < 0 {
		t.Fatal("receipt scope regression not exercised")
	}
	decoded, err := decodeManagedReferencedDecision(referencedCitationReply(t, c, catalog, metadata, true), catalog, c.Candidate, p.ContextDigest, true)
	if err != nil {
		t.Fatal(err)
	}
	if _, _, err = parseManagedFragmentDecision(decoded, visible, c, p, replies); err == nil {
		t.Fatal("a reference turned metadata into business proof")
	}
}

func TestReferencedCitationRejectsInvalidWireBeforeMaterialization(t *testing.T) {
	c, p, _, _, catalog, reference := referencedCitationFixture(t)
	good := referencedCitationReply(t, c, catalog, reference, true)
	for _, mode := range []string{"foreign catalogue", "foreign candidate", "unknown reference", "negative reference", "text alongside reference", "null text alongside reference", "null reference", "unknown field", "duplicate field", "invalid UTF8"} {
		t.Run(mode, func(t *testing.T) {
			var reply map[string]any
			if err := json.Unmarshal([]byte(good), &reply); err != nil {
				t.Fatal(err)
			}
			criterion := reply["review"].(map[string]any)["tasks"].([]any)[0].(map[string]any)["criteria"].([]any)[0].(map[string]any)
			switch mode {
			case "foreign catalogue":
				reply["catalog_sha256"] = hash([]byte("foreign"))
			case "foreign candidate":
				reply["review"].(map[string]any)["candidate_commit"] = "foreign"
			case "unknown reference":
				criterion["evidence_ref"] = len(catalog.Quotes)
			case "negative reference":
				criterion["evidence_ref"] = -1
			case "text alongside reference":
				criterion["evidence"] = c.Tasks[0].Report
			case "null text alongside reference":
				criterion["evidence"] = nil
			case "null reference":
				criterion["evidence_ref"] = nil
			case "unknown field":
				criterion["other"] = "not part of the contract"
			}
			raw, _ := json.Marshal(reply)
			if mode == "duplicate field" {
				raw = []byte(strings.Replace(string(raw), `"evidence_ref":`, `"evidence_ref":0,"evidence_ref":`, 1))
			}
			if mode == "invalid UTF8" {
				raw = append(raw, 0xff)
			}
			if _, err := decodeManagedReferencedDecision(string(raw), catalog, c.Candidate, p.ContextDigest, true); err == nil {
				t.Fatal("malformed reference accepted")
			}
		})
	}
}

func TestReferencedCitationSchemaConstrainsChoicesAndPreservesUnknown(t *testing.T) {
	c, p, _, _, catalog, reference := referencedCitationFixture(t)
	schema := managedReferencedDecisionSchema(c, catalog, []managedFragmentQuestion{{0, 0, 0, "Original evidence required"}})
	var definition map[string]any
	if err := json.Unmarshal([]byte(schema), &definition); err != nil {
		t.Fatal(err)
	}
	ref := definition["$defs"].(map[string]any)["original-reference"].(map[string]any)
	if ref["type"] != "integer" || len(ref["enum"].([]any)) == 0 || !strings.Contains(schema, catalog.digest()) {
		t.Fatal("schema still admits arbitrary copied strings for positive evidence")
	}
	var reply map[string]any
	if err := json.Unmarshal([]byte(referencedCitationReply(t, c, catalog, reference, true)), &reply); err != nil {
		t.Fatal(err)
	}
	resolution := reply["resolutions"].([]any)[0].(map[string]any)
	resolution["verdict"], resolution["evidence"] = "unknown", "A required original still needs independent examination"
	delete(resolution, "evidence_ref")
	raw, _ := json.Marshal(reply)
	decoded, err := decodeManagedReferencedDecision(string(raw), catalog, c.Candidate, p.ContextDigest, true)
	if err != nil || !strings.Contains(decoded, `"verdict":"unknown"`) {
		t.Fatal("reference transport erased uncertainty", err)
	}
}
