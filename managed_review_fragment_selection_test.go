//go:build linux

package main

import (
	"encoding/json"
	"reflect"
	"testing"
)

func TestManagedFragmentSelectionOriginalAndBounded(t *testing.T) {
	c, p, _ := finalBundleFixture(t)
	a := p.Packets[0].Artifacts[0]
	refs := []managedFragmentEvidenceRef{{0, 0, a.Digest}}
	got, err := selectManagedFragmentEvidence(c, p, refs, managedReviewPromptLimit)
	if err != nil {
		t.Fatal(err)
	}
	if len(got.Artifacts) != 1 || !reflect.DeepEqual(got.Artifacts[0], a) {
		t.Fatal("original evidence replaced")
	}
	raw, _ := json.Marshal(got)
	if _, err = selectManagedFragmentEvidence(c, p, refs, len(raw)); err != nil {
		t.Fatal(err)
	}
	if _, err = selectManagedFragmentEvidence(c, p, refs, len(raw)-1); err == nil {
		t.Fatal("oversized selection accepted")
	}
	refs[0].Digest = "changed"
	if got.References[0].Digest != a.Digest {
		t.Fatal("reference aliases caller")
	}
}
func TestManagedFragmentSelectionRefusesInvalidRequests(t *testing.T) {
	c, p, _ := finalBundleFixture(t)
	a := p.Packets[0].Artifacts[0]
	valid := managedFragmentEvidenceRef{0, 0, a.Digest}
	for _, refs := range [][]managedFragmentEvidenceRef{nil, {valid, valid}, {{-1, 0, a.Digest}}, {{len(p.Packets), 0, a.Digest}}, {{0, -1, a.Digest}}, {{0, len(p.Packets[0].Artifacts), a.Digest}}, {{0, 0, "changed"}}} {
		if _, err := selectManagedFragmentEvidence(c, p, refs, managedReviewPromptLimit); err == nil {
			t.Fatal("invalid evidence accepted", refs)
		}
	}
	for _, limit := range []int{0, -1, managedReviewPromptLimit + 1} {
		if _, err := selectManagedFragmentEvidence(c, p, []managedFragmentEvidenceRef{valid}, limit); err == nil {
			t.Fatal("invalid capacity accepted")
		}
	}
	p.Candidate = "other"
	if _, err := selectManagedFragmentEvidence(c, p, []managedFragmentEvidenceRef{valid}, managedReviewPromptLimit); err == nil {
		t.Fatal("stale candidate accepted")
	}
}
