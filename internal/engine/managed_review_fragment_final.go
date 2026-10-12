//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strings"
)

// Only original quoted bytes become final-review evidence. Inspection rationale
// remains a separate opinion. This bundle is not a task verdict or permission to
// publish, and the original full packets must remain durably available.
// A local defect remains an attributed opinion, never a global verdict.
const managedFragmentLocalDefectQuestion = "Évaluer ce constat local défavorable sur les originaux du candidat complet : confirmer le défaut, le réfuter avec preuve originale, ou demander une pièce manquante. Ne pas assimiler une absence du paquet à une absence du candidat."

type managedFragmentPartialDefect struct {
	Packet int                   `json:"packet"`
	Defect managedFragmentDefect `json:"defect"`
}

type managedFragmentFinalEvidence struct {
	Defects       []managedFragmentPartialDefect      `json:"partial_defects,omitempty"`
	Coordination  *ReviewCoordinationProposal         `json:"coordination,omitempty"`
	InputBudget   *managedReviewInputBudget           `json:"input_budget,omitempty"`
	Candidate     string                              `json:"candidate_commit"`
	ContextDigest string                              `json:"context_sha256"`
	PlanDigest    string                              `json:"plan_sha256"`
	ReplyDigests  []string                            `json:"reply_sha256"`
	Evidence      []managedFragmentOriginalEvidence   `json:"original_evidence"`
	Questions     []managedFragmentQuestion           `json:"unresolved_questions,omitempty"`
	Historical    []managedHistoricalObservationLabel `json:"historical_observations,omitempty"`
	ChangeDiff    string                              `json:"changes_since_observations,omitempty"`
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
	out := managedFragmentFinalEvidence{Coordination: p.Coordination, InputBudget: p.InputBudget, Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw)}
	for i, packet := range p.Packets {
		state, inspection, e := parsePlannedFragment(replies[i], p, i)
		if e != nil {
			return empty, e
		}
		if state != "inspected" && state != "unknown" && state != "changes_requested" {
			return empty, fmt.Errorf("fragment %d : défaut ou preuve manquante, décision finale interdite", i)
		}
		byIndex := map[int]managedFragmentFinding{}
		for _, finding := range inspection.Findings {
			byIndex[finding.Artifact] = finding
		}
		for _, defect := range inspection.Defects {
			out.Defects = append(out.Defects, managedFragmentPartialDefect{Packet: i, Defect: defect})
		}
		out.ReplyDigests = append(out.ReplyDigests, hash([]byte(replies[i])))
		if ref := fragmentReuseAt(p, i); ref != nil {
			addHistoricalImpactQuestion(&out, *ref, inspection)
		}
		for j, a := range packet.Artifacts {
			finding := byIndex[j]
			if finding.Verdict == "fail" {
				out.Questions = append(out.Questions, managedFragmentQuestion{i, j, 0, managedFragmentLocalDefectQuestion})
			}
			if finding.Verdict == "unknown" {
				if len(finding.Needs) == 0 {
					return empty, fmt.Errorf("question sans demande de preuve explicite")
				}
				if finding.Evidence != "" && !strings.Contains(a.Content, finding.Evidence) {
					return empty, fmt.Errorf("extrait de question non original")
				}
				for n, need := range finding.Needs {
					out.Questions = append(out.Questions, managedFragmentQuestion{i, j, n, need})
				}
			}
			out.Evidence = append(out.Evidence, managedFragmentOriginalEvidence{Packet: i, Artifact: j, Kind: a.Kind, Name: a.Name, Digest: a.Digest, Excerpt: finding.Evidence, Opinion: finding.Reason})
		}
	}
	out.ChangeDiff = p.ChangeDiff
	return out, nil
}
