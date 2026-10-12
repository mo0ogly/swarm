//go:build linux

package engine

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

// Once immutable partial replies have arrived, a complete original that fits
// the actual final message must not be rejected using hypothetical future
// findings/defects. The speculative reserve remains a pre-call admission check.
func TestManagedFragmentSelectionUsesReceivedEvidenceCapacity(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	b, err := managedFragmentFinalBundle(c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	conservative, err := managedFragmentDecisionCapacity("", c, p)
	if err != nil {
		t.Fatal(err)
	}
	base, _, err := renderManagedFragmentDecision("", c, b, nil)
	if err != nil {
		t.Fatal(err)
	}
	actual, err := fragmentInputRoom(p.InputBudget, base, fragmentDecisionSchema(b))
	if err != nil {
		t.Fatal(err)
	}
	actual -= len(`,"requested_original_evidence":`)
	ref := managedFragmentEvidenceRef{}
	largest := 0
	for i, packet := range p.Packets {
		for j, artifact := range packet.Artifacts {
			if len(artifact.Content) > largest {
				ref = managedFragmentEvidenceRef{i, j, artifact.Digest}
				largest = len(artifact.Content)
			}
		}
	}
	selected, err := selectManagedFragmentEvidence(c, p, []managedFragmentEvidenceRef{ref}, fragmentInputByteLimit(p.InputBudget))
	if err != nil {
		t.Fatal(err)
	}
	original, _ := json.Marshal(selected)
	gap := actual - conservative
	if gap <= 0 || conservative <= len(original) {
		t.Fatal("fixture needs speculative reserve and tunable prefix", conservative, actual, len(original))
	}
	target := max(1, len(original)-gap/2)
	prefix := strings.Repeat("x", conservative-target)
	room, err := managedFragmentDecisionCapacity(prefix, c, p)
	if err != nil || room >= len(original) {
		t.Fatal("fixture must discriminate speculative rejection", room, len(original), err)
	}
	final, visible, err := managedFragmentDecisionPrompt(prefix, c, p, replies, []managedFragmentEvidenceRef{ref})
	if err != nil || len(final)+len(fragmentDecisionSchema(b)) > fragmentInputByteLimit(p.InputBudget) || len(visible.Sources) == 0 {
		t.Fatal("whole original must actually fit before selection", err)
	}
	prompt, err := managedFragmentRequestPrompt(prefix, c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	var received struct {
		Limit int `json:"selection_bytes_limit"`
	}
	parts := strings.SplitN(prompt, "SWARM_FRAGMENT_EVIDENCE_REQUEST\n", 2)
	if len(parts) != 2 || json.Unmarshal([]byte(parts[1]), &received) != nil {
		t.Fatal("actual selector payload unavailable")
	}
	if received.Limit < len(original) {
		t.Fatal("selector falsely declares available complete proof too large", received.Limit, len(original), room)
	}
	if _, err := selectManagedFragmentEvidence(c, p, []managedFragmentEvidenceRef{ref}, received.Limit); err != nil {
		t.Fatal("advertised selection cannot carry original", err)
	}
	if _, _, err := managedFragmentDecisionPrompt(strings.Repeat("x", managedReviewPromptLimit), c, p, replies, []managedFragmentEvidenceRef{ref}); err == nil {
		t.Fatal("real final transport limit relaxed")
	}
}
