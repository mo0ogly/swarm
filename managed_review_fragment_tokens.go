//go:build linux

package main

import "fmt"

func validateFragmentInputBudget(version int, budget *managedReviewInputBudget) error {
	if version < 4 {
		if budget != nil {
			return fmt.Errorf("ancien protocole avec capacité substituée")
		}
		return nil
	}
	if version != 4 || budget == nil {
		return fmt.Errorf("capacité de protocole absente")
	}
	_, err := budget.inputTokens()
	return err
}

func fragmentInputByteLimit(b *managedReviewInputBudget) int {
	if b != nil {
		return b.MaxBytes
	}
	return managedReviewPromptLimit
}

func fragmentInputFits(b *managedReviewInputBudget, prompt, schema string) error {
	if b == nil {
		if len(prompt)+len(schema) > managedReviewPromptLimit {
			return fmt.Errorf("limite de message historique dépassée")
		}
		return nil
	}
	_, err := b.measure(prompt, schema)
	return err
}

func fragmentInputRoom(b *managedReviewInputBudget, prompt, schema string) (int, error) {
	if b == nil {
		return managedReviewPromptLimit - len(prompt) - len(schema), nil
	}
	m, err := b.measure(prompt, schema)
	if err != nil {
		return 0, err
	}
	// Original evidence is selected in bytes: one token per added UTF-8 byte is
	// conservative for BPE. Reserve additional room for JSON/message boundaries.
	// The actual complete final prompt is always counted again before spending.
	return min(b.MaxBytes-m.UTF8Bytes, m.TokenRoom) - 1024, nil
}

// Repack a previously validated, complete inventory. Historical observations
// retain their original packets/replies and identities. Only the fresh packets
// receive the new capacity contract. This function never authorizes a provider.
func planTokenManagedFragments(c managedReviewContext, previous managedReviewFragmentPlan, budget managedReviewInputBudget, prefix string, available int) (managedReviewFragmentPlan, error) {
	var empty managedReviewFragmentPlan
	if err := validateManagedReviewFragments(c, previous); err != nil {
		return empty, err
	}
	if _, err := budget.inputTokens(); err != nil {
		return empty, err
	}
	if previous.Version == 4 || available <= 2 {
		return empty, fmt.Errorf("plan source ou budget final invalide")
	}
	p := managedReviewFragmentPlan{Version: 4, InputBudget: &budget, Candidate: previous.Candidate, ContextDigest: previous.ContextDigest, AvailableCalls: available, ReservedFinalCalls: 2, ChangeDiff: previous.ChangeDiff}
	packet := func() managedReviewFragmentPacket {
		return managedReviewFragmentPacket{Version: 4, InputBudget: &budget, Candidate: p.Candidate, ContextDigest: p.ContextDigest, Index: len(p.Packets)}
	}
	current := packet()
	for i, old := range previous.Packets {
		if fragmentReuseAt(previous, i) != nil {
			continue
		}
		for _, a := range old.Artifacts {
			next := current
			next.Artifacts = append(append([]managedReviewFragmentArtifact(nil), current.Artifacts...), a)
			if _, err := managedFragmentInspectionPrompt(prefix, next); err != nil {
				if len(current.Artifacts) == 0 {
					return empty, fmt.Errorf("pièce indivisible : %w", err)
				}
				p.Packets = append(p.Packets, current)
				current = packet()
				current.Artifacts = []managedReviewFragmentArtifact{a}
				if _, err = managedFragmentInspectionPrompt(prefix, current); err != nil {
					return empty, err
				}
			} else {
				current = next
			}
		}
	}
	if len(current.Artifacts) > 0 {
		p.Packets = append(p.Packets, current)
	}
	for _, ref := range previous.Reused {
		ref.Packet = len(p.Packets)
		p.Reused = append(p.Reused, ref)
		p.Packets = append(p.Packets, managedReviewFragmentPacket{Version: 4, InputBudget: &budget, Candidate: p.Candidate, ContextDigest: p.ContextDigest, Index: ref.Packet, Artifacts: ref.Original.Artifacts})
	}
	if err := validateManagedReviewFragments(c, p); err != nil {
		return empty, err
	}
	if err := preflightManagedFragmentCalls(prefix, c, p); err != nil {
		return empty, err
	}
	return p, nil
}
