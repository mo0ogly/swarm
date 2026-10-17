//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func scopeContextFixture(t *testing.T, version int) managedReviewContext {
	t.Helper()
	c := fragmentPlanFixture()
	c.Baseline = &ManagedReviewBaseline{Version: 2, Candidate: c.Previous}
	c.Tasks[0].Criteria = []string{"Observe exact scoped effect and reject invalid proof"}
	raw, _ := json.Marshal(c)
	var obj map[string]any
	if err := json.Unmarshal(raw, &obj); err != nil {
		t.Fatal(err)
	}
	obj["inspection_scope_version"] = version
	raw, _ = json.Marshal(obj)
	if err := json.Unmarshal(raw, &c); err != nil {
		t.Fatal(err)
	}
	return c
}

func TestManagedFragmentEveryInspectionReceivesCurrentCriteria(t *testing.T) {
	c := scopeContextFixture(t, 1)
	p, err := planManagedReviewFragments(c, 16, 2)
	if err != nil {
		t.Fatal(err)
	}
	if len(p.Packets) < 2 {
		t.Fatal("not a fragmented review")
	}
	withoutMetadata := 0
	for _, packet := range p.Packets {
		raw, _ := json.Marshal(packet)
		var obj map[string]json.RawMessage
		if err := json.Unmarshal(raw, &obj); err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(string(obj["scope"]), c.Tasks[0].Criteria[0]) {
			t.Fatalf("packet %d receives no current criterion", packet.Index)
		}
		prompt, err := managedFragmentInspectionPrompt("", packet)
		if err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(prompt, "PÉRIMÈTRE CANONIQUE") || !strings.Contains(prompt, "unknown") {
			t.Fatal("scope guidance absent")
		}
		metadata := false
		for _, a := range packet.Artifacts {
			metadata = metadata || a.Kind == "context"
		}
		if !metadata {
			withoutMetadata++
		}
	}
	if withoutMetadata == 0 {
		t.Fatal("fixture never exercises missing metadata")
	}
	raw, _ := json.Marshal(p)
	var tampered map[string]any
	if err := json.Unmarshal(raw, &tampered); err != nil {
		t.Fatal(err)
	}
	packets := tampered["packets"].([]any)
	packets[0].(map[string]any)["scope"] = map[string]any{"version": 1, "tasks": []any{}}
	raw, _ = json.Marshal(tampered)
	var bad managedReviewFragmentPlan
	if err := json.Unmarshal(raw, &bad); err != nil {
		t.Fatal(err)
	}
	if validateManagedReviewFragments(c, bad) == nil {
		t.Fatal("forged scope accepted")
	}
}

func TestManagedFragmentRejectsUnknownScopeVersion(t *testing.T) {
	c := scopeContextFixture(t, 999)
	if _, err := planManagedReviewFragments(c, 16, 2); err == nil {
		t.Fatal("unknown scope contract accepted")
	}
}

func TestManagedFragmentScopeAcrossTokenPackingAndFinalSelection(t *testing.T) {
	c := scopeContextFixture(t, 1)
	legacy, err := planManagedReviewFragments(c, 100, 2)
	if err != nil {
		t.Fatal(err)
	}
	artifacts, err := managedFragmentArtifacts(c)
	if err != nil {
		t.Fatal(err)
	}
	for _, mode := range []string{"repack", "fresh"} {
		t.Run(mode, func(t *testing.T) {
			var p managedReviewFragmentPlan
			var err error
			if mode == "repack" {
				p, err = planTokenManagedFragments(c, legacy, reviewTokenBudgetFixture(), "", 12)
			} else {
				p, err = planFreshTokenFragments(c, [][]managedReviewFragmentArtifact{artifacts}, nil, nil, nil, reviewTokenBudgetFixture(), "", 12)
			}
			if err != nil {
				t.Fatal(err)
			}
			for _, packet := range p.Packets {
				raw, _ := json.Marshal(packet)
				if !strings.Contains(string(raw), c.Tasks[0].Criteria[0]) {
					t.Fatal("capacity packing dropped criteria")
				}
			}
			replies := fragmentHistoricalReplies(t, p)
			selection, err := managedFragmentRequestPrompt("", c, p, replies)
			if err != nil {
				t.Fatal(err)
			}
			decision, _, err := managedFragmentDecisionPrompt("", c, p, replies, nil)
			if err != nil {
				t.Fatal(err)
			}
			for _, prompt := range []string{selection, decision} {
				if !strings.Contains(prompt, "PERTINENCE DES QUESTIONS") || !strings.Contains(prompt, "conserver unknown") {
					t.Fatal("final scope adjudication absent")
				}
			}
		})
	}
	c.InspectionScopeVersion = 0
	old, err := planManagedReviewFragments(c, 100, 2)
	if err != nil {
		t.Fatal(err)
	}
	for _, packet := range old.Packets {
		raw, _ := json.Marshal(packet)
		if strings.Contains(string(raw), `"scope":`) {
			t.Fatal("historical packet changed")
		}
	}
}
