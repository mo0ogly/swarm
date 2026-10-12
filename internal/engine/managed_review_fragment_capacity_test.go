//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestManagedFragmentDecisionCapacityBoundsActual(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	room, err := managedFragmentDecisionCapacity("guidance", c, p)
	if err != nil || room <= 0 {
		t.Fatal(room, err)
	}
	prompt, _, err := managedFragmentDecisionPrompt("guidance", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(prompt)+len(managedReviewSchema)+room > managedReviewPromptLimit {
		t.Fatal("capacity underestimates maximum input")
	}
	if _, err = managedFragmentDecisionCapacity(strings.Repeat("x", managedReviewPromptLimit), c, p); err == nil {
		t.Fatal("oversized fixed input accepted")
	}
}
func TestManagedFragmentFindingSerializedBudget(t *testing.T) {
	for _, s := range []string{strings.Repeat("x", 96), strings.Repeat("\n", 48), strings.Repeat("é", 48)} {
		if !boundedFragmentFindingText(s) {
			t.Fatal("valid boundary", s)
		}
	}
	for _, s := range []string{strings.Repeat("x", 97), strings.Repeat("\n", 49), strings.Repeat("é", 49), strings.Repeat("<", 17)} {
		if boundedFragmentFindingText(s) {
			t.Fatal("escaped or UTF8 budget bypass", s)
		}
	}
}

func TestFragmentSchemaTextFitsSerializedBudget(t *testing.T) {
	var schema map[string]any
	if err := json.Unmarshal([]byte(managedFragmentInspectionSchema), &schema); err != nil {
		t.Fatal(err)
	}
	props := schema["properties"].(map[string]any)["findings"].(map[string]any)["items"].(map[string]any)["properties"].(map[string]any)
	for _, field := range []string{"reason", "evidence"} {
		n := int(props[field].(map[string]any)["maxLength"].(float64))
		for _, r := range []rune{'a', 'é', '"', '\\', '\n', '\x00', '<', '>', '&', '😀'} {
			if !boundedFragmentFindingText(strings.Repeat(string(r), n)) {
				t.Fatalf("schema allows overflowing %s %U", field, r)
			}
		}
	}
}

func TestFragmentEvidenceSchemaHasMinimum(t *testing.T) {
	var schema map[string]any
	if err := json.Unmarshal([]byte(managedFragmentInspectionSchema), &schema); err != nil {
		t.Fatal(err)
	}
	props := schema["properties"].(map[string]any)["findings"].(map[string]any)["items"].(map[string]any)["properties"].(map[string]any)
	evidence := props["evidence"].(map[string]any)
	if evidence["minLength"] != float64(8) || evidence["maxLength"] != float64(16) || evidence["pattern"] == nil {
		t.Fatal(evidence)
	}
}

// The size model must cover bounded UTF-8 claims, not only cheap ASCII strings.
// These values are synthetic for capacity measurement, never review evidence.
func TestManagedFragmentDecisionCapacityCoversSerializedLocalDefects(t *testing.T) {
	c, p, _ := finalBundleFixture(t)
	room, err := managedFragmentDecisionCapacity("guidance", c, p)
	if err != nil {
		t.Fatal(err)
	}
	b := maximalManagedFragmentBundle(c, p)
	for i := range b.Defects {
		b.Defects[i].Defect.Explanation = strings.Repeat("é", 512)
	}
	for packet := range p.Packets {
		defects := []managedFragmentDefect{}
		for _, claim := range b.Defects {
			if claim.Packet == packet {
				defects = append(defects, claim.Defect)
			}
		}
		raw, e := json.Marshal(defects)
		if e != nil || len(raw) > managedFragmentDefectReserve {
			t.Fatal("fixture exceeds the serialized defect contract", len(raw), e)
		}
	}
	prompt, _, err := renderManagedFragmentDecision("guidance", c, b, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(prompt)+len(fragmentDecisionSchema(b))+room > managedReviewPromptLimit {
		t.Fatal("UTF-8 local defects exceed advertised selection room")
	}
}
