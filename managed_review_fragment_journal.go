//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
)

// Journal integrity alone does not authorize a provider call. A future runtime
// must anchor the journal digest and each reservation in the Store transaction.
type managedFragmentJournal struct {
	Version        int                           `json:"version"`
	PlanDigest     string                        `json:"plan_sha256"`
	Attempt        string                        `json:"attempt"`
	ProviderDigest string                        `json:"provider_sha256"`
	Entries        []managedFragmentJournalEntry `json:"entries"`
}
type managedFragmentJournalEntry struct {
	Packet       int    `json:"packet"`
	PacketDigest string `json:"packet_sha256"`
	CallID       string `json:"call_id"`
	State        string `json:"state"`
	Reply        string `json:"reply,omitempty"`
	ReplyDigest  string `json:"reply_sha256,omitempty"`
}

// Returns reusable completed inspections, including unresolved questions. Reserved/interrupted calls remain
// spent but non-reusable; neither this result nor a complete journal is a verdict.
func validateManagedFragmentJournal(j managedFragmentJournal, p managedReviewFragmentPlan, attempt, provider string) (map[int]string, error) {
	raw, e := json.Marshal(p)
	if e != nil {
		return nil, e
	}
	if j.Version != 1 || attempt == "" || provider == "" || j.Attempt != attempt || j.ProviderDigest != provider || j.PlanDigest != hash(raw) {
		return nil, fmt.Errorf("journal de fragments périmé")
	}
	reusable := map[int]string{}
	seenCalls := map[string]bool{}
	last := map[int]string{}
	for _, entry := range j.Entries {
		if entry.Packet < 0 || entry.Packet >= len(p.Packets) || entry.CallID == "" || seenCalls[entry.CallID] {
			return nil, fmt.Errorf("réservation dupliquée ou paquet inconnu")
		}
		seenCalls[entry.CallID] = true
		packet := p.Packets[entry.Packet]
		packetRaw, _ := json.Marshal(packet)
		if entry.PacketDigest != hash(packetRaw) {
			return nil, fmt.Errorf("paquet du journal altéré")
		}
		if prior := last[entry.Packet]; prior != "" && prior != "interrupted" {
			return nil, fmt.Errorf("réservation précédente non interrompue")
		}
		last[entry.Packet] = entry.State
		switch entry.State {
		case "reserved", "interrupted":
			if entry.Reply != "" || entry.ReplyDigest != "" {
				return nil, fmt.Errorf("réponse sans verdict durable")
			}
		case "inspected", "unknown", "changes_requested":
			if entry.ReplyDigest == "" || hash([]byte(entry.Reply)) != entry.ReplyDigest {
				return nil, fmt.Errorf("réponse du journal modifiée")
			}
			state, _, err := parseManagedFragmentInspection(entry.Reply, packet)
			if err != nil || state != entry.State {
				return nil, fmt.Errorf("verdict du journal différent de la réponse originale")
			}
			if state == "inspected" || state == "unknown" {
				reusable[entry.Packet] = entry.Reply
			}
		default:
			return nil, fmt.Errorf("état du journal invalide")
		}
	}
	return reusable, nil
}

// The caller must obtain expectedDigest from durable trusted state, never from
// this file. Reads have no mutation, reservation or acceptance side effects.
func readManagedFragmentJournal(path, expectedDigest string, p managedReviewFragmentPlan, attempt, provider string) (managedFragmentJournal, map[int]string, error) {
	var j managedFragmentJournal
	if expectedDigest == "" {
		return j, nil, fmt.Errorf("ancrage durable absent")
	}
	raw, e := os.ReadFile(path)
	if e != nil {
		return j, nil, e
	}
	if hash(raw) != expectedDigest {
		return j, nil, fmt.Errorf("journal modifié ou non ancré")
	}
	if e = strict(raw, &j); e != nil {
		return j, nil, e
	}
	reusable, e := validateManagedFragmentJournal(j, p, attempt, provider)
	return j, reusable, e
}
