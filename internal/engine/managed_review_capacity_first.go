//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"reflect"
)

// Only the historical transport ceiling permits a fresh capacity calculation.
// Integrity/provenance failures must not silently discard historical obligations.
type managedFragmentLegacyCapacityError struct{ Name string }

func (e *managedFragmentLegacyCapacityError) Error() string {
	return fmt.Sprintf("pièce indivisible trop grande : %s", e.Name)
}

// Build fresh protocol4 packets directly from canonical evidence. A failed
// legacy packing attempt is not a prerequisite for measured provider capacity.
// No source is split, truncated, replaced or omitted, and no call is reserved.
func planFreshTokenFragments(c managedReviewContext, groups [][]managedReviewFragmentArtifact, order []string, proposal *ReviewCoordinationProposal, files []string, budget managedReviewInputBudget, prefix string, available int) (managedReviewFragmentPlan, error) {
	empty := managedReviewFragmentPlan{}
	if len(groups) == 0 || (proposal != nil && len(order) != len(groups)) {
		return empty, fmt.Errorf("groupes de preuves incomplets")
	}
	if available <= 2 {
		return empty, fmt.Errorf("budget de revue finale absent")
	}
	if _, err := budget.inputTokens(); err != nil {
		return empty, err
	}
	budget.FinalReserveContractVersion = 1
	canonical, err := json.Marshal(c)
	if err != nil {
		return empty, err
	}
	p := managedReviewFragmentPlan{Version: 4, InputBudget: &budget, Candidate: c.Candidate, ContextDigest: hash(canonical), AvailableCalls: available, ReservedFinalCalls: 2, Coordination: proposal, CoordinationFiles: files}
	// Reject incomplete/tampered coverage before expensive prompt counting.
	for i, group := range groups {
		packet := managedReviewFragmentPacket{Version: 4, InputBudget: &budget, Candidate: c.Candidate, ContextDigest: p.ContextDigest, Index: i, DefectCitationContractVersion: managedFragmentDefectCitationContractVersion, FindingContractVersion: managedFragmentFindingContractVersion, Artifacts: group}
		if proposal != nil {
			packet.Lot = order[i]
			packet.LotContract, packet.LotCriteria = coordinatedLotContract(c, *proposal, order[i])
		}
		p.Packets = append(p.Packets, packet)
	}
	inventory := p
	// Coverage is checked separately from packet capacity: this provisional
	// layout may be too large, but can never authorize execution.
	inventory.AvailableCalls = max(available, len(groups)+2)
	if proposal != nil {
		if err = validateCoordinatedFragmentCoverage(c, inventory); err != nil {
			return empty, err
		}
	} else {
		expected, e := managedFragmentArtifacts(c)
		if e != nil {
			return empty, e
		}
		var actual []managedReviewFragmentArtifact
		for _, group := range groups {
			actual = append(actual, group...)
		}
		if !reflect.DeepEqual(actual, expected) {
			return empty, fmt.Errorf("couverture des preuves incomplète ou modifiée")
		}
	}
	p.Packets = nil
	for i, group := range groups {
		newPacket := func() managedReviewFragmentPacket {
			packet := managedReviewFragmentPacket{Version: 4, InputBudget: &budget, Candidate: c.Candidate, ContextDigest: p.ContextDigest, Index: len(p.Packets), DefectCitationContractVersion: managedFragmentDefectCitationContractVersion, FindingContractVersion: managedFragmentFindingContractVersion}
			if proposal != nil {
				packet.Lot = order[i]
				packet.LotContract, packet.LotCriteria = coordinatedLotContract(c, *proposal, order[i])
			}
			return packet
		}
		for begin := 0; begin < len(group); {
			base := newPacket()
			fit := func(end int) bool {
				candidate := base
				candidate.Artifacts = group[begin:end]
				_, e := managedFragmentInspectionPrompt(prefix, candidate)
				return e == nil
			}
			// UTF-8 prompt bytes are monotone even where BPE token merges are
			// not. Search may conservatively choose a smaller fitting prefix;
			// no selected packet is admitted without the exact complete check.
			best := begin
			low, high := begin+1, len(group)
			for low <= high {
				mid := low + (high-low)/2
				if fit(mid) {
					best = mid
					low = mid + 1
				} else {
					high = mid - 1
				}
			}
			if best == begin {
				base.Artifacts = group[begin : begin+1]
				_, e := managedFragmentInspectionPrompt(prefix, base)
				return empty, fmt.Errorf("pièce %s : %w", group[begin].Name, e)
			}
			base.Artifacts = append([]managedReviewFragmentArtifact(nil), group[begin:best]...)
			p.Packets = append(p.Packets, base)
			begin = best
		}
	}
	if len(p.Packets)+2 > available {
		return empty, fmt.Errorf("budget insuffisant : %d inspections et 2 appels finaux pour %d disponibles", len(p.Packets), available)
	}
	if err = validateManagedReviewFragments(c, p); err != nil {
		return empty, err
	}
	if err = preflightManagedFragmentCalls(prefix, c, p); err != nil {
		return empty, fmt.Errorf("précontrôle complet (%d pièces, %d paquets): %w", func() int {
			n := 0
			for _, g := range groups {
				n += len(g)
			}
			return n
		}(), len(p.Packets), err)
	}
	return p, nil
}

func (s *Store) planFreshCapacityFragments(w Work, a Agent, c managedReviewContext, available int) (managedReviewFragmentPlan, error) {
	empty := managedReviewFragmentPlan{}
	budget, err := s.reviewInputBudget(w.Planning.Reviewer)
	if err != nil {
		return empty, err
	}
	_, method, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		return empty, err
	}
	task, err := w.task(a.TaskID)
	if err != nil {
		return empty, err
	}
	if task.ReviewCoordination == nil {
		pieces, err := managedFragmentArtifacts(c)
		if err != nil {
			return empty, err
		}
		return planFreshTokenFragments(c, [][]managedReviewFragmentArtifact{pieces}, nil, nil, nil, *budget, managedReviewPrefix(method), available)
	}
	record := task.ReviewCoordination
	raw, _ := json.Marshal(record.Proposal)
	if hash(raw) != record.Digest {
		return empty, fmt.Errorf("plan de revue altéré")
	}
	scope, err := managedScope(filepath.Join(w.Planning.Repository.Storage, "repository.git"), c.Previous, c.Candidate, effectiveReviewFileLimit(w.Planning.Reviewer))
	if err != nil {
		return empty, err
	}
	if err := validateReviewLotFilePolicy(record.Proposal, scope.Limit); err != nil {
		return empty, err
	}
	groups, order, err := coordinatedArtifactGroups(c, record.Proposal, scope.Files)
	if err != nil {
		return empty, err
	}
	if !reflect.DeepEqual(order, record.Order) {
		return empty, fmt.Errorf("ordre de revue altéré")
	}
	return planFreshTokenFragments(c, groups, order, &record.Proposal, scope.Files, *budget, managedReviewPrefix(method), available)
}
