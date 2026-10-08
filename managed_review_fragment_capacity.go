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
	bundle := maximalManagedFragmentBundle(c, p)
	prompt, _, err := renderManagedFragmentDecision(prefix, c, bundle, nil)
	if err != nil {
		return 0, err
	}
	remaining, err := fragmentInputRoom(p.InputBudget, prompt, fragmentDecisionSchema(bundle))
	if err != nil {
		return 0, err
	}
	if p.Version == 4 {
		// Future excerpts/opinions may tokenize worse than synthetic x strings.
		// Reserve one token per allowed byte in addition to their measured cost.
		for i, packet := range p.Packets {
			if fragmentReuseAt(p, i) == nil {
				remaining -= len(packet.Artifacts) * 2 * managedFragmentFindingTextLimit
			}
		}
	}
	// Reserve the property name/comma in addition to the selection's encoded bytes.
	remaining -= len(`,"requested_original_evidence":`)
	if remaining <= 0 {
		return 0, fmt.Errorf("aucune capacité restante pour les preuves finales")
	}
	return remaining, nil
}

// Synthetic size model only; never use as inspection evidence.
func maximalManagedFragmentBundle(c managedReviewContext, p managedReviewFragmentPlan) managedFragmentFinalEvidence {
	raw, _ := json.Marshal(p)
	bundle := managedFragmentFinalEvidence{InputBudget: p.InputBudget, Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw)}
	bundle.ChangeDiff = p.ChangeDiff
	for _, ref := range p.Reused {
		_, inspection, _ := parseManagedFragmentInspection(ref.Reply, ref.Original)
		addHistoricalImpactQuestion(&bundle, ref, inspection)
		for _, finding := range inspection.Findings {
			if finding.Verdict == "unknown" {
				for i, need := range finding.Needs {
					bundle.Questions = append(bundle.Questions, managedFragmentQuestion{ref.Packet, finding.Artifact, i, need})
				}
			}
		}
	}
	for i, packet := range p.Packets {
		if ref := fragmentReuseAt(p, i); ref != nil {
			_, inspection, _ := parseManagedFragmentInspection(ref.Reply, ref.Original)
			bundle.ReplyDigests = append(bundle.ReplyDigests, hash([]byte(ref.Reply)))
			findings := map[int]managedFragmentFinding{}
			for _, f := range inspection.Findings {
				findings[f.Artifact] = f
			}
			for j, a := range packet.Artifacts {
				f := findings[j]
				bundle.Evidence = append(bundle.Evidence, managedFragmentOriginalEvidence{Packet: i, Artifact: j, Kind: a.Kind, Name: a.Name, Digest: a.Digest, Excerpt: f.Evidence, Opinion: f.Reason})
			}
			continue
		}
		bundle.ReplyDigests = append(bundle.ReplyDigests, strings.Repeat("f", 64))
		for j, a := range packet.Artifacts {
			bundle.Evidence = append(bundle.Evidence, managedFragmentOriginalEvidence{Packet: i, Artifact: j, Kind: a.Kind, Name: a.Name, Digest: a.Digest, Excerpt: strings.Repeat("x", managedFragmentFindingTextLimit), Opinion: strings.Repeat("x", managedFragmentFindingTextLimit)})
		}
	}
	return bundle
}
