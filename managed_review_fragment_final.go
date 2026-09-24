//go:build linux

package main

import (
	"encoding/json"
	"fmt"
)

// Only original quoted bytes become final-review evidence. Inspection rationale
// remains a separate opinion. This bundle is not a task verdict or permission to
// publish, and the original full packets must remain durably available.
type managedFragmentFinalEvidence struct {
	Candidate     string                            `json:"candidate_commit"`
	ContextDigest string                            `json:"context_sha256"`
	PlanDigest    string                            `json:"plan_sha256"`
	ReplyDigests  []string                          `json:"reply_sha256"`
	Evidence      []managedFragmentOriginalEvidence `json:"original_evidence"`
}
type managedFragmentOriginalEvidence struct {
	Packet   int    `json:"packet"`
	Artifact int    `json:"artifact"`
	Kind     string `json:"kind"`
	Name     string `json:"name"`
	Digest   string `json:"sha256"`
	Excerpt  string `json:"original_excerpt"`
	Opinion  string `json:"inspection_opinion"`
}

func managedFragmentFinalBundle(c managedReviewContext, p managedReviewFragmentPlan, replies []string) (managedFragmentFinalEvidence, error) {
	empty := managedFragmentFinalEvidence{}
	if e := validateManagedReviewFragments(c, p); e != nil {
		return empty, e
	}
	if len(replies) != len(p.Packets) {
		return empty, fmt.Errorf("inspections de fragments incomplètes")
	}
	raw, _ := json.Marshal(p)
	out := managedFragmentFinalEvidence{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw)}
	for i, packet := range p.Packets {
		state, inspection, e := parseManagedFragmentInspection(replies[i], packet)
		if e != nil {
			return empty, e
		}
		if state != "inspected" {
			return empty, fmt.Errorf("fragment %d : défaut ou preuve manquante, décision finale interdite", i)
		}
		byIndex := map[int]managedFragmentFinding{}
		for _, finding := range inspection.Findings {
			byIndex[finding.Artifact] = finding
		}
		out.ReplyDigests = append(out.ReplyDigests, hash([]byte(replies[i])))
		for j, a := range packet.Artifacts {
			finding := byIndex[j]
			out.Evidence = append(out.Evidence, managedFragmentOriginalEvidence{Packet: i, Artifact: j, Kind: a.Kind, Name: a.Name, Digest: a.Digest, Excerpt: finding.Evidence, Opinion: finding.Reason})
		}
	}
	return out, nil
}
