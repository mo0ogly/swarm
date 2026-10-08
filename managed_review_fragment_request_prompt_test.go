//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestManagedFragmentAllCallsPreflight(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	if err := preflightManagedFragmentCalls("guidance", c, p); err != nil {
		t.Fatal(err)
	}
	prompt, err := managedFragmentRequestPrompt("guidance", c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	if len(prompt)+len(managedFragmentRequestSchema) > managedReviewPromptLimit || !strings.Contains(prompt, "selection_bytes_limit") || !strings.Contains(prompt, "AUCUN VERDICT") {
		t.Fatal("unbounded or misleading selector")
	}
	if err = preflightManagedFragmentCalls(strings.Repeat("x", managedReviewPromptLimit), c, p); err == nil {
		t.Fatal("oversized prefix accepted")
	}
	p.ReservedFinalCalls = 1
	if err = preflightManagedFragmentCalls("", c, p); err == nil {
		t.Fatal("missing final call accepted")
	}
}
func TestManagedFragmentSelectionCostsCoverActualEncoding(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	b, err := managedFragmentFinalBundle(c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	fixed, _ := json.Marshal(managedFragmentEvidenceSelection{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: b.PlanDigest, References: []managedFragmentEvidenceRef{}, Artifacts: []managedReviewFragmentArtifact{}})
	refs := []managedFragmentEvidenceRef{}
	bound := len(fixed)
	for j, a := range p.Packets[0].Artifacts {
		if j == 2 {
			break
		}
		ref := managedFragmentEvidenceRef{0, j, a.Digest}
		refs = append(refs, ref)
		ar, _ := json.Marshal(a)
		rr, _ := json.Marshal(ref)
		bound += len(ar) + len(rr) + 2
	}
	selected, err := selectManagedFragmentEvidence(c, p, refs, managedReviewPromptLimit)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(selected)
	if len(raw) > bound {
		t.Fatal("cost estimate too small", len(raw), bound)
	}
}
