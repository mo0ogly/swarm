//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"path/filepath"
	"reflect"
	"strconv"
	"strings"
)

// Resolve only Git's file metadata, never hunk contents. Unrecognized metadata
// remains in the global evidence group; a missing planned file fails closed.
func coordinationDiffPath(a managedReviewFragmentArtifact) string {
	if a.Kind != "diff" {
		return ""
	}
	var deleted string
	for _, line := range strings.Split(a.Content, "\n") {
		if strings.HasPrefix(line, "@@") {
			break
		}
		for _, prefix := range []string{"+++ ", "--- ", "rename to "} {
			if !strings.HasPrefix(line, prefix) {
				continue
			}
			p := strings.TrimSuffix(strings.TrimPrefix(line, prefix), "\t")
			if strings.HasPrefix(p, "\"") {
				var err error
				p, err = strconv.Unquote(p)
				if err != nil {
					return ""
				}
			}
			if prefix == "rename to " {
				return p
			}
			if p == "/dev/null" {
				continue
			}
			if prefix == "+++ " && strings.HasPrefix(p, "b/") {
				return p[2:]
			}
			if prefix == "--- " && strings.HasPrefix(p, "a/") {
				deleted = p[2:]
			}
		}
	}
	return deleted
}

func gitCoordinationQuote(s string) string {
	var b bytes.Buffer
	b.WriteByte('"')
	for _, v := range []byte(s) {
		switch v {
		case '\\', '"':
			b.WriteByte('\\')
			b.WriteByte(v)
		case '\t':
			b.WriteString(`\t`)
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		default:
			if v < 32 || v >= 127 {
				fmt.Fprintf(&b, `\%03o`, v)
			} else {
				b.WriteByte(v)
			}
		}
	}
	b.WriteByte('"')
	return b.String()
}
func coordinationHeaderMatches(header, path string) bool {
	for _, a := range []string{"a/" + path, gitCoordinationQuote("a/" + path)} {
		for _, b := range []string{"b/" + path, gitCoordinationQuote("b/" + path)} {
			if header == "diff --git "+a+" "+b {
				return true
			}
		}
	}
	return false
}

func coordinatedArtifactGroups(c managedReviewContext, proposal ReviewCoordinationProposal, files []string) ([][]managedReviewFragmentArtifact, []string, error) {
	order, err := validateReviewCoordination(proposal, c.Candidate, coordinationEvidence(c), files, coordinationCriteria(c))
	if err != nil {
		return nil, nil, err
	}
	artifacts, err := managedFragmentArtifacts(c)
	if err != nil {
		return nil, nil, err
	}
	known := map[string]bool{}
	for _, f := range files {
		known[f] = true
	}
	byFile := map[string][]managedReviewFragmentArtifact{}
	global := []managedReviewFragmentArtifact{}
	for _, a := range artifacts {
		p := a.Name
		if a.Kind == "diff" {
			p = coordinationDiffPath(a)
			if p == "" {
				for _, f := range files {
					if coordinationHeaderMatches(a.Name, f) {
						p = f
						break
					}
				}
			}
		}
		if a.Kind == "diff" {
			for _, line := range strings.Split(a.Content, "\n") {
				if strings.HasPrefix(line, "@@") {
					break
				}
				if strings.HasPrefix(line, "rename from ") {
					old := strings.TrimPrefix(line, "rename from ")
					if strings.HasPrefix(old, "\"") {
						decoded, e := strconv.Unquote(old)
						if e != nil {
							return nil, nil, e
						}
						old = decoded
					}
					if known[old] && old != p {
						byFile[old] = append(byFile[old], a)
					}
				}
			}
		}
		if known[p] && (a.Kind == "diff" || a.Kind == "source") {
			byFile[p] = append(byFile[p], a)
		} else {
			global = append(global, a)
		}
	}
	groups := [][]managedReviewFragmentArtifact{}
	for _, id := range order {
		var lot ReviewCoordinationLot
		for _, l := range proposal.Lots {
			if l.ID == id {
				lot = l
				break
			}
		}
		raw, _ := json.Marshal(lot)
		group := []managedReviewFragmentArtifact{{Kind: "review-lot", Name: id, Digest: hash(raw), Content: string(raw)}}
		for _, f := range lot.Files {
			if len(byFile[f]) == 0 {
				return nil, nil, fmt.Errorf("fichier de lot sans preuve reconnue : %q", f)
			}
			for _, artifact := range byFile[f] {
				duplicate := false
				for _, prior := range group {
					if prior.Kind == artifact.Kind && prior.Name == artifact.Name && prior.Digest == artifact.Digest {
						duplicate = true
						break
					}
				}
				if !duplicate {
					group = append(group, artifact)
				}
			}
		}
		groups = append(groups, group)
	}
	// Contracts, reports, controls, historical changes and all supplemental
	// evidence are inspected too. Nothing disappears because it has no lot owner.
	if len(global) > 0 {
		groups = append(groups, global)
		order = append(order, "__global_evidence")
	}
	return groups, order, nil
}

func coordinatedLotContract(c managedReviewContext, p ReviewCoordinationProposal, id string) (*ReviewCoordinationLot, map[string]string) {
	for _, lot := range p.Lots {
		if lot.ID == id {
			criteria := map[string]string{}
			for _, t := range c.Tasks {
				for i, text := range t.Criteria {
					key := fmt.Sprintf("%s#%d", t.Task, i+1)
					for _, wanted := range lot.Criteria {
						if key == wanted {
							criteria[key] = text
						}
					}
				}
			}
			return &lot, criteria
		}
	}
	return nil, nil
}

func planCoordinatedFragments(c managedReviewContext, proposal ReviewCoordinationProposal, files []string, available int) (managedReviewFragmentPlan, error) {
	groups, order, err := coordinatedArtifactGroups(c, proposal, files)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	p := managedReviewFragmentPlan{Version: 2, Candidate: c.Candidate, ContextDigest: coordinationEvidence(c), AvailableCalls: available, ReservedFinalCalls: 2, Coordination: &proposal, CoordinationFiles: files}
	for i, group := range groups {
		contract, criteria := coordinatedLotContract(c, proposal, order[i])
		part, e := packManagedFragmentArtifactsWithLot(c, group, 100000, 2, order[i], contract, criteria)
		if e != nil {
			return managedReviewFragmentPlan{}, fmt.Errorf("lot %s : %w", order[i], e)
		}
		for _, packet := range part.Packets {
			packet.Index = len(p.Packets)
			packet.Lot = order[i]
			packet.LotContract, packet.LotCriteria = coordinatedLotContract(c, proposal, order[i])
			p.Packets = append(p.Packets, packet)
		}
	}
	if len(p.Packets)+2 > available {
		return managedReviewFragmentPlan{}, fmt.Errorf("revues coordonnées : %d inspections et 2 appels finaux requis, %d appels disponibles", len(p.Packets), available)
	}
	return p, validateManagedReviewFragments(c, p)
}

func (s *Store) coordinatedFragments(w Work, a Agent, c managedReviewContext, available int) (managedReviewFragmentPlan, error) {
	task, e := w.task(a.TaskID)
	if e != nil {
		return managedReviewFragmentPlan{}, e
	}
	r := task.ReviewCoordination
	if r == nil {
		return managedReviewFragmentPlan{}, fmt.Errorf("plan de revue absent")
	}
	raw, _ := json.Marshal(r.Proposal)
	if hash(raw) != r.Digest {
		return managedReviewFragmentPlan{}, fmt.Errorf("plan de revue altéré")
	}
	scope, e := managedScope(filepath.Join(w.Planning.Repository.Storage, "repository.git"), c.Previous, c.Candidate)
	if e != nil {
		return managedReviewFragmentPlan{}, e
	}
	order, e := validateReviewCoordination(r.Proposal, c.Candidate, coordinationEvidence(c), scope.Files, coordinationCriteria(c))
	if e != nil {
		return managedReviewFragmentPlan{}, e
	}
	if !reflect.DeepEqual(order, r.Order) {
		return managedReviewFragmentPlan{}, fmt.Errorf("ordre de revue altéré")
	}
	return planCoordinatedFragments(c, r.Proposal, scope.Files, available)
}

func validateCoordinatedFragmentCoverage(c managedReviewContext, p managedReviewFragmentPlan) error {
	if (p.Version != 2 && p.Version != 4) || p.ReservedFinalCalls != 2 || len(p.Reused) > 0 || p.ChangeDiff != "" {
		return fmt.Errorf("protocole de revue coordonnée incompatible")
	}
	groups, order, e := coordinatedArtifactGroups(c, *p.Coordination, p.CoordinationFiles)
	if e != nil {
		return e
	}
	index := 0
	for i, group := range groups {
		actual := []managedReviewFragmentArtifact{}
		for index < len(p.Packets) && p.Packets[index].Lot == order[i] {
			lot, criteria := coordinatedLotContract(c, *p.Coordination, order[i])
			if !reflect.DeepEqual(lot, p.Packets[index].LotContract) || !reflect.DeepEqual(criteria, p.Packets[index].LotCriteria) {
				return fmt.Errorf("contrat de lot altéré")
			}
			actual = append(actual, p.Packets[index].Artifacts...)
			index++
		}
		if !reflect.DeepEqual(actual, group) {
			return fmt.Errorf("couverture ou ordre du lot %s modifié", order[i])
		}
	}
	if index != len(p.Packets) {
		return fmt.Errorf("paquets hors plan de revue")
	}
	return nil
}

func sameReviewCoordination(a, b Work, task string) bool {
	left, e := a.task(task)
	if e != nil {
		return false
	}
	right, e := b.task(task)
	if e != nil {
		return false
	}
	if left.ReviewCoordination == nil || right.ReviewCoordination == nil {
		return left.ReviewCoordination == nil && right.ReviewCoordination == nil
	}
	x, _ := json.Marshal(left.ReviewCoordination.Proposal)
	y, _ := json.Marshal(right.ReviewCoordination.Proposal)
	return string(x) == string(y) && hash(x) == left.ReviewCoordination.Digest && hash(y) == right.ReviewCoordination.Digest
}
