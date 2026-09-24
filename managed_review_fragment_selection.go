//go:build linux

package main

import (
	"encoding/json"
	"fmt"
)

// References identify original, whole artifacts. A request never supplies its
// own replacement content, and selected evidence is not an acceptance verdict.
type managedFragmentEvidenceRef struct {
	Packet   int    `json:"packet"`
	Artifact int    `json:"artifact"`
	Digest   string `json:"sha256"`
}
type managedFragmentEvidenceSelection struct {
	Candidate     string                          `json:"candidate_commit"`
	ContextDigest string                          `json:"context_sha256"`
	PlanDigest    string                          `json:"plan_sha256"`
	References    []managedFragmentEvidenceRef    `json:"references"`
	Artifacts     []managedReviewFragmentArtifact `json:"original_artifacts"`
}

// Build the exact original evidence requested for cross-fragment examination.
// maxBytes is the caller's remaining prompt space after fixed metadata/schema;
// it may tighten the transport limit, never enlarge it. Refuse rather than trim.
func selectManagedFragmentEvidence(c managedReviewContext, p managedReviewFragmentPlan, refs []managedFragmentEvidenceRef, maxBytes int) (managedFragmentEvidenceSelection, error) {
	empty := managedFragmentEvidenceSelection{}
	if err := validateManagedReviewFragments(c, p); err != nil {
		return empty, err
	}
	if len(refs) == 0 || maxBytes <= 0 || maxBytes > managedReviewPromptLimit {
		return empty, fmt.Errorf("sélection de preuves ou capacité invalide")
	}
	raw, _ := json.Marshal(p)
	out := managedFragmentEvidenceSelection{Candidate: p.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw), References: append([]managedFragmentEvidenceRef(nil), refs...)}
	seen := map[[2]int]bool{}
	for _, ref := range refs {
		key := [2]int{ref.Packet, ref.Artifact}
		if ref.Packet < 0 || ref.Packet >= len(p.Packets) || ref.Artifact < 0 || ref.Artifact >= len(p.Packets[ref.Packet].Artifacts) || seen[key] {
			return empty, fmt.Errorf("référence de preuve inconnue ou dupliquée")
		}
		a := p.Packets[ref.Packet].Artifacts[ref.Artifact]
		if ref.Digest != a.Digest {
			return empty, fmt.Errorf("empreinte de preuve périmée")
		}
		seen[key] = true
		out.Artifacts = append(out.Artifacts, a)
	}
	raw, err := json.Marshal(out)
	if err != nil {
		return empty, err
	}
	if len(raw) > maxBytes {
		return empty, fmt.Errorf("preuves originales demandées trop grandes : %d octets pour %d disponibles", len(raw), maxBytes)
	}
	return out, nil
}
