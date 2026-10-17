//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedFragmentSelectorIdentifiesCompleteOriginalAlreadySupplied(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	prompt, err := managedFragmentRequestPromptVersion("", c, p, replies, 4)
	if err != nil {
		t.Fatal(err)
	}
	_, raw, ok := strings.Cut(prompt, "SWARM_FRAGMENT_EVIDENCE_REQUEST\n")
	if !ok {
		t.Fatal("selector payload absent")
	}
	var received struct {
		Context  managedReviewContext         `json:"task_contracts_reports_controls"`
		Supplied []managedFragmentEvidenceRef `json:"already_supplied_originals"`
		Costs    [][]int                      `json:"cost_rows"`
	}
	if err := json.Unmarshal([]byte(raw), &received); err != nil {
		t.Fatal(err)
	}
	if len(received.Supplied) == 0 {
		t.Fatal("complete original metadata is invisible to the selector")
	}
	metadata, _ := json.Marshal(received.Context)
	for _, ref := range received.Supplied {
		a := p.Packets[ref.Packet].Artifacts[ref.Artifact]
		if a.Kind != "context" || a.Digest != ref.Digest || a.Content != string(metadata) {
			t.Fatal("advertised original differs from received metadata")
		}
		for _, cost := range received.Costs {
			if cost[0] == ref.Packet && cost[1] == ref.Artifact {
				t.Fatal("already supplied original consumes a second selection slot")
			}
		}
	}
}

func TestManagedFragmentSelectorRejectsChangedOriginalMetadata(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	for i := range p.Packets {
		for j := range p.Packets[i].Artifacts {
			if p.Packets[i].Artifacts[j].Kind == "context" {
				p.Packets[i].Artifacts[j].Content += " altered"
				if _, err := managedFragmentRequestPromptVersion("", c, p, replies, 4); err == nil {
					t.Fatal("altered metadata became an already supplied original")
				}
				return
			}
		}
	}
	t.Fatal("context original fixture absent")
}

func TestManagedFragmentSelectionSchemaBindsOriginalReference(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	var schema map[string]any
	if err := json.Unmarshal([]byte(managedFragmentSelectionSchema(p, 5)), &schema); err != nil {
		t.Fatal(err)
	}
	props := schema["properties"].(map[string]any)
	choices, ok := props["references"].(map[string]any)["items"].(map[string]any)["anyOf"].([]any)
	if !ok {
		t.Fatal("provider schema still permits a mistyped evidence digest")
	}
	seen := map[[2]int]bool{}
	for _, choice := range choices {
		fields := choice.(map[string]any)["properties"].(map[string]any)
		i := int(fields["packet"].(map[string]any)["enum"].([]any)[0].(float64))
		j := int(fields["artifact"].(map[string]any)["enum"].([]any)[0].(float64))
		digest := fields["sha256"].(map[string]any)["enum"].([]any)
		if len(digest) != 1 || digest[0] != p.Packets[i].Artifacts[j].Digest || seen[[2]int{i, j}] {
			t.Fatal("reference is not bound to a unique original")
		}
		seen[[2]int{i, j}] = true
	}
	for i, packet := range p.Packets {
		for j := range packet.Artifacts {
			if !seen[[2]int{i, j}] {
				t.Fatal("selectable original omitted")
			}
		}
	}
	for _, version := range []int{1, 2, 3, 4} {
		if managedFragmentSelectionSchema(p, version) != managedFragmentRequestSchema {
			t.Fatal("historical schema changed", version)
		}
	}
	bad := managedFragmentEvidenceRef{0, 0, "incorrect-original-digest"}
	if _, err := selectManagedFragmentEvidence(c, p, []managedFragmentEvidenceRef{bad}, managedReviewPromptLimit); err == nil {
		t.Fatal("parser admitted mistyped digest")
	}
	if _, err := managedFragmentRequestPromptVersion("", c, p, replies, 5); err != nil {
		t.Fatal(err)
	}
	if output := os.Getenv("SWARM_SELECTION_SCHEMA_ORACLE_DIR"); output != "" {
		if err := os.MkdirAll(output, 0700); err != nil {
			t.Fatal(err)
		}
		planJSON, _ := json.Marshal(p)
		ref := managedFragmentEvidenceRef{0, 0, p.Packets[0].Artifacts[0].Digest}
		request := managedFragmentFinalRequest{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(planJSON), State: "ready", Reason: "Select exact original evidence", References: []managedFragmentEvidenceRef{ref}}
		values := map[string]any{"current-schema.json": json.RawMessage(managedFragmentSelectionSchema(p, 5)), "legacy-schema.json": json.RawMessage(managedFragmentRequestSchema), "original-reply.json": request}
		for name, value := range values {
			raw, err := json.MarshalIndent(value, "", "  ")
			if err != nil {
				t.Fatal(err)
			}
			if err = os.WriteFile(filepath.Join(output, name), raw, 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
}
