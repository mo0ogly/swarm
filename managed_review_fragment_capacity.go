//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"strings"
)

// A conservative byte bound, not evidence and never passed to a provider.
// Real replies must satisfy boundedFragmentFindingText before reuse. Selected
// whole artifacts are measured separately; the returned space is not a promise
// that every future evidence request can fit.
func managedFragmentDecisionCapacity(prefix string, c managedReviewContext, p managedReviewFragmentPlan) (int, error) {
	if err := validateManagedReviewFragments(c, p); err != nil {
		return 0, err
	}
	raw, _ := json.Marshal(p)
	bundle := managedFragmentFinalEvidence{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw)}
	for i, packet := range p.Packets {
		bundle.ReplyDigests = append(bundle.ReplyDigests, strings.Repeat("f", 64))
		for j, a := range packet.Artifacts {
			bundle.Evidence = append(bundle.Evidence, managedFragmentOriginalEvidence{Packet: i, Artifact: j, Kind: a.Kind, Name: a.Name, Digest: a.Digest, Excerpt: strings.Repeat("x", managedFragmentFindingTextLimit), Opinion: strings.Repeat("x", managedFragmentFindingTextLimit)})
		}
	}
	prompt, _, err := renderManagedFragmentDecision(prefix, c, bundle, nil)
	if err != nil {
		return 0, err
	}
	remaining := managedReviewPromptLimit - len(prompt) - len(managedReviewSchema)
	// Reserve the property name/comma in addition to the selection's encoded bytes.
	remaining -= len(`,"requested_original_evidence":`)
	if remaining <= 0 {
		return 0, fmt.Errorf("aucune capacité restante pour les preuves finales")
	}
	return remaining, nil
}
