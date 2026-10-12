//go:build linux

package engine

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
	if p.Version == 4 && p.InputBudget.FinalReserveContractVersion == 1 {
		return managedFragmentMeasuredDecisionCapacity(prefix, c, p)
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
	for i, packet := range p.Packets {
		if fragmentReuseAt(p, i) != nil {
			continue
		}
		// Details are bounded by their serialized original array, including UTF-8
		// and JSON escapes. ASCII field lengths alone underestimate that bound.
		defects := []managedFragmentDefect{}
		for _, claim := range bundle.Defects {
			if claim.Packet == i {
				defects = append(defects, claim.Defect)
			}
		}
		raw, marshalErr := json.Marshal(defects)
		if marshalErr != nil {
			return 0, marshalErr
		}
		remaining -= max(0, managedFragmentDefectReserve-len(raw))
		if p.Version == 4 {
			// Future opinions/defects may tokenize worse than synthetic x strings.
			// Reserve one token per allowed byte beyond their measured cost.
			remaining -= len(packet.Artifacts)*2*managedFragmentFindingTextLimit + managedFragmentDefectReserve
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
		for j := 0; j < len(packet.Artifacts) && j < managedFragmentDefectLimit; j++ {
			bundle.Defects = append(bundle.Defects, managedFragmentPartialDefect{Packet: i, Defect: managedFragmentDefect{Artifact: j, Line: 999999, Quote: strings.Repeat("x", 256), Explanation: strings.Repeat("x", 512), Reproduction: strings.Repeat("x", 384), Expected: strings.Repeat("x", 256)}})
			bundle.Questions = append(bundle.Questions, managedFragmentQuestion{i, j, 0, managedFragmentLocalDefectQuestion})
		}
		bundle.ReplyDigests = append(bundle.ReplyDigests, strings.Repeat("f", 64))
		for j, a := range packet.Artifacts {
			bundle.Evidence = append(bundle.Evidence, managedFragmentOriginalEvidence{Packet: i, Artifact: j, Kind: a.Kind, Name: a.Name, Digest: a.Digest, Excerpt: strings.Repeat("x", managedFragmentFindingTextLimit), Opinion: strings.Repeat("x", managedFragmentFindingTextLimit)})
		}
	}
	return bundle
}

// Fresh capacity contracts count fixed syntax once and reserve variable JSON
// bytes once. Historical zero contracts retain their exact former calculation.
func managedFragmentMeasuredDecisionCapacity(prefix string, c managedReviewContext, p managedReviewFragmentPlan) (int, error) {
	bundle := maximalManagedFragmentBundle(c, p)
	reserve := 0
	for i, packet := range p.Packets {
		if fragmentReuseAt(p, i) != nil {
			continue
		}
		for j := range bundle.Evidence {
			if bundle.Evidence[j].Packet == i {
				bundle.Evidence[j].Excerpt = ""
				bundle.Evidence[j].Opinion = ""
				reserve += managedFragmentFindingTextLimit
				excerptBytes := len("indisponible")
				for _, q := range fragmentQuoteChoices(packet.Artifacts[bundle.Evidence[j].Artifact].Content) {
					raw, _ := json.Marshal(q)
					excerptBytes = max(excerptBytes, len(raw)-2)
				}
				reserve += excerptBytes
			}
		}
		for j := range bundle.Defects {
			if bundle.Defects[j].Packet == i {
				d := &bundle.Defects[j].Defect
				d.Quote = ""
				d.Explanation = ""
				d.Reproduction = ""
				d.Expected = ""
			}
		}
		// The validated whole original defect array (including all string escapes)
		// is <=8192 bytes. Its final packet wrappers already remain in fixed syntax.
		reserve += managedFragmentDefectReserve
	}
	prompt, _, err := renderManagedFragmentDecision(prefix, c, bundle, nil)
	if err != nil {
		return 0, err
	}
	m, err := p.InputBudget.measure(prompt, fragmentDecisionSchema(bundle))
	if err != nil {
		return 0, err
	}
	// Each variable UTF-8 byte produces at most one BPE token. Reserve the fixed
	// request property and message boundary separately, independent of the bytes.
	overhead := len(`,"requested_original_evidence":`) + 1024
	room := min(p.InputBudget.MaxBytes-m.UTF8Bytes, m.TokenRoom) - reserve - overhead
	if room <= 0 {
		return 0, fmt.Errorf("aucune capacité restante pour les preuves finales (socle=%d tokens, marge=%d tokens/%d octets, variables=%d octets)", m.PromptTokens+m.SchemaTokens, m.TokenRoom, p.InputBudget.MaxBytes-m.UTF8Bytes, reserve)
	}
	return room, nil
}
