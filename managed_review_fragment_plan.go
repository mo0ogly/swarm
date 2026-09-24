//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"reflect"
	"strings"
	"unicode/utf8"
)

// A transport plan is not an independent review. Keep it separate from
// managedReviewBatch: existing acceptance code must never treat an inspection
// packet as a complete verdict on a task.
type managedReviewFragmentArtifact struct {
	Kind    string `json:"kind"`
	Name    string `json:"name"`
	Digest  string `json:"sha256"`
	Content string `json:"content"`
}
type managedReviewFragmentPacket struct {
	Version       int                             `json:"version"`
	Candidate     string                          `json:"candidate_commit"`
	ContextDigest string                          `json:"context_sha256"`
	Index         int                             `json:"index"`
	Artifacts     []managedReviewFragmentArtifact `json:"artifacts"`
}
type managedReviewFragmentPlan struct {
	Version            int                           `json:"version"`
	ContextDigest      string                        `json:"context_sha256"`
	Candidate          string                        `json:"candidate_commit"`
	Packets            []managedReviewFragmentPacket `json:"packets"`
	ReservedFinalCalls int                           `json:"reserved_final_calls"`
	AvailableCalls     int                           `json:"available_calls"`
	Executable         bool                          `json:"executable"`
}

const managedFragmentPromptReserve = 24 * 1024

func managedFragmentArtifacts(c managedReviewContext) ([]managedReviewFragmentArtifact, error) {
	if c.Candidate == "" || len(c.Tasks) == 0 || !strings.HasPrefix(c.Diff, "diff --git ") || !utf8.ValidString(c.Diff) || managedDiffHasBinary(c.Diff) {
		return nil, fmt.Errorf("contexte de fragments incomplet ou diff non textuel")
	}
	artifacts := []managedReviewFragmentArtifact{}
	appendArtifact := func(kind, name, content string) {
		artifacts = append(artifacts, managedReviewFragmentArtifact{kind, name, hash([]byte(content)), content})
	}
	// Split at file headers, retaining the original separator and all bytes.
	remainder := c.Diff
	seen := map[string]bool{}
	for remainder != "" {
		end := strings.Index(remainder[1:], "\ndiff --git ")
		section := remainder
		if end >= 0 {
			end += 2
			section = remainder[:end]
			remainder = remainder[end:]
		} else {
			remainder = ""
		}
		name := strings.SplitN(section, "\n", 2)[0]
		if seen[name] {
			return nil, fmt.Errorf("section de diff dupliquée")
		}
		seen[name] = true
		appendArtifact("diff", name, section)
	}
	// Everything except diff and supplemental contents is retained as canonical
	// metadata, including reports, deliveries, receipts and accepted baselines.
	metadata := c
	metadata.Diff = ""
	metadata.Sources = nil
	raw, err := json.Marshal(metadata)
	if err != nil {
		return nil, err
	}
	appendArtifact("context", "contracts-reports-controls-baseline", string(raw))
	seen = map[string]bool{}
	for _, source := range c.Sources {
		if source.Path == "" || seen[source.Path] || !utf8.ValidString(source.Content) || source.Bytes != len(source.Content) || source.Digest != hash([]byte(source.Content)) {
			return nil, fmt.Errorf("source annexe incohérente")
		}
		seen[source.Path] = true
		raw, err = json.Marshal(source)
		if err != nil {
			return nil, err
		}
		appendArtifact("source", source.Path, string(raw))
	}
	return artifacts, nil
}

// All canonical evidence is included, not only the diff. No disk/Store write,
// provider call, budget reservation or publication is performed by this plan.
func planManagedReviewFragments(c managedReviewContext, available, finalCalls int) (managedReviewFragmentPlan, error) {
	empty := managedReviewFragmentPlan{}
	if finalCalls < 1 || available <= finalCalls {
		return empty, fmt.Errorf("budget de revue finale absent")
	}
	artifacts, err := managedFragmentArtifacts(c)
	if err != nil {
		return empty, err
	}
	canonical, _ := json.Marshal(c)
	plan := managedReviewFragmentPlan{Version: 1, ContextDigest: hash(canonical), Candidate: c.Candidate, AvailableCalls: available, ReservedFinalCalls: finalCalls}
	packet := managedReviewFragmentPacket{Version: 1, Candidate: c.Candidate, ContextDigest: plan.ContextDigest, Index: 0}
	fits := func(p managedReviewFragmentPacket) bool {
		raw, e := json.Marshal(p)
		return e == nil && len(raw)+managedFragmentPromptReserve <= managedReviewPromptLimit && managedFragmentReplyFits(p)
	}
	for _, artifact := range artifacts {
		next := packet
		next.Artifacts = append(append([]managedReviewFragmentArtifact(nil), packet.Artifacts...), artifact)
		if !fits(next) {
			if len(packet.Artifacts) == 0 {
				return empty, fmt.Errorf("pièce indivisible trop grande : %s", artifact.Name)
			}
			plan.Packets = append(plan.Packets, packet)
			packet = managedReviewFragmentPacket{Version: 1, Candidate: c.Candidate, ContextDigest: plan.ContextDigest, Index: len(plan.Packets), Artifacts: []managedReviewFragmentArtifact{artifact}}
			if !fits(packet) {
				return empty, fmt.Errorf("pièce indivisible trop grande : %s", artifact.Name)
			}
		} else {
			packet = next
		}
	}
	if len(packet.Artifacts) > 0 {
		plan.Packets = append(plan.Packets, packet)
	}
	if len(plan.Packets)+finalCalls > available {
		return empty, fmt.Errorf("budget insuffisant : %d inspections et %d appels finaux pour %d disponibles", len(plan.Packets), finalCalls, available)
	}
	if err = validateManagedReviewFragments(c, plan); err != nil {
		return empty, err
	}
	return plan, nil
}

func validateManagedReviewFragments(c managedReviewContext, p managedReviewFragmentPlan) error {
	canonical, err := json.Marshal(c)
	if err != nil {
		return err
	}
	if p.Version != 1 || p.Executable || p.Candidate != c.Candidate || p.ContextDigest != hash(canonical) || p.ReservedFinalCalls < 1 || len(p.Packets) == 0 || len(p.Packets)+p.ReservedFinalCalls > p.AvailableCalls {
		return fmt.Errorf("plan de fragments périmé ou incohérent")
	}
	expected, err := managedFragmentArtifacts(c)
	if err != nil {
		return err
	}
	actual := []managedReviewFragmentArtifact{}
	for i, packet := range p.Packets {
		raw, err := json.Marshal(packet)
		if err != nil || packet.Version != 1 || packet.Index != i || packet.Candidate != p.Candidate || packet.ContextDigest != p.ContextDigest || len(packet.Artifacts) == 0 || len(raw)+managedFragmentPromptReserve > managedReviewPromptLimit {
			return fmt.Errorf("paquet de revue modifié ou trop grand")
		}
		actual = append(actual, packet.Artifacts...)
	}
	if !reflect.DeepEqual(actual, expected) {
		return fmt.Errorf("couverture des preuves incomplète ou modifiée")
	}
	return nil
}
