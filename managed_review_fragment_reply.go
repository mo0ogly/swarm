//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
)

// Inspection evidence cannot be used as ManagedTaskReview or an acceptance.
// Each artifact requires its own explicit result tied to the exact packet.
type managedFragmentFinding struct {
	Artifact int      `json:"artifact"`
	Digest   string   `json:"sha256"`
	Verdict  string   `json:"verdict"`
	Reason   string   `json:"reason"`
	Evidence string   `json:"evidence"`
	Needs    []string `json:"needs"`
}
type managedFragmentInspection struct {
	Candidate     string                   `json:"candidate_commit"`
	ContextDigest string                   `json:"context_sha256"`
	PacketDigest  string                   `json:"packet_sha256"`
	Findings      []managedFragmentFinding `json:"findings"`
}

const managedFragmentReplyLimit = 16 * 1024

// Bound the serialized fields, including JSON escapes, before reserving calls.
// This keeps a worst-case final input calculable without truncating responses.
const managedFragmentFindingTextLimit = 96

func boundedFragmentFindingText(text string) bool {
	raw, err := json.Marshal(text)
	return err == nil && len(raw) <= managedFragmentFindingTextLimit+2
}

func parseManagedFragmentInspection(reply string, packet managedReviewFragmentPacket) (string, managedFragmentInspection, error) {
	var result managedFragmentInspection
	fail := func(reason string) (string, managedFragmentInspection, error) {
		return "", managedFragmentInspection{}, fmt.Errorf("inspection de fragment : %s", reason)
	}
	if len(reply) > managedFragmentReplyLimit {
		return fail("réponse trop grande")
	}
	if err := decodeManagedFragmentInspection([]byte(reply), packet, &result); err != nil {
		return fail("réponse structurée invalide")
	}
	raw, err := json.Marshal(packet)
	if err != nil || len(packet.Artifacts) == 0 || result.Candidate != packet.Candidate || result.ContextDigest != packet.ContextDigest || result.PacketDigest != hash(raw) || len(result.Findings) != len(packet.Artifacts) {
		return fail("identité ou couverture différente du paquet")
	}
	seen := map[int]bool{}
	state := "inspected"
	for _, finding := range result.Findings {
		if finding.Artifact < 0 || finding.Artifact >= len(packet.Artifacts) || seen[finding.Artifact] {
			return fail("pièce inconnue ou dupliquée")
		}
		seen[finding.Artifact] = true
		artifact := packet.Artifacts[finding.Artifact]
		if artifact.Digest != hash([]byte(artifact.Content)) || finding.Digest != artifact.Digest || len(strings.TrimSpace(finding.Reason)) < 8 || !boundedFragmentFindingText(finding.Reason) || !boundedFragmentFindingText(finding.Evidence) || len(finding.Needs) > 16 {
			return fail("preuve ou justification invalide")
		}
		needs := map[string]bool{}
		for _, need := range finding.Needs {
			if len(strings.TrimSpace(need)) < 3 || len(need) > 240 || needs[need] {
				return fail("demande de preuve invalide")
			}
			needs[need] = true
		}
		switch finding.Verdict {
		case "inspected":
			if len(strings.TrimSpace(finding.Evidence)) < 8 || !fragmentEvidencePresent(artifact, finding.Evidence) || len(finding.Needs) != 0 {
				return fail("extrait original absent ou preuve encore demandée")
			}
		case "fail":
			state = "changes_requested"
		case "unknown":
			if state != "changes_requested" {
				state = "unknown"
			}
		default:
			return fail("verdict inconnu ; inspected ne signifie jamais accepted")
		}
	}
	return state, result, nil
}

// Reserve room for a full inspected response and formatting. This bounds the
// normal response, not latency or arbitrarily many requests for missing evidence.
// Existing journals remain readable; only new calls/plans use this preflight.
func managedFragmentReplyFits(p managedReviewFragmentPacket) bool {
	r := managedFragmentInspection{Candidate: p.Candidate, ContextDigest: p.ContextDigest, PacketDigest: strings.Repeat("a", 64)}
	for i, a := range p.Artifacts {
		r.Findings = append(r.Findings, managedFragmentFinding{Artifact: i, Digest: a.Digest, Verdict: "inspected", Reason: strings.Repeat("r", managedFragmentFindingTextLimit), Evidence: strings.Repeat("e", managedFragmentFindingTextLimit), Needs: []string{}})
	}
	raw, err := json.Marshal(r)
	return err == nil && len(raw)+2048 <= managedFragmentReplyLimit
}

// Historical array journals remain readable. New responses bind every finding
// to an explicit inventory key and the immutable whole-packet digest.
func decodeManagedFragmentInspection(raw []byte, packet managedReviewFragmentPacket, result *managedFragmentInspection) error {
	var envelope struct {
		Candidate string          `json:"candidate_commit"`
		Context   string          `json:"context_sha256"`
		Packet    string          `json:"packet_sha256"`
		Findings  json.RawMessage `json:"findings"`
	}
	if err := strict(raw, &envelope); err != nil {
		return err
	}
	if strings.HasPrefix(strings.TrimSpace(string(envelope.Findings)), "[") {
		return strict(raw, result)
	}
	var entries map[string]json.RawMessage
	if err := strict(envelope.Findings, &entries); err != nil {
		return err
	}
	if len(entries) != len(packet.Artifacts) {
		return fmt.Errorf("incomplete inventory")
	}
	*result = managedFragmentInspection{Candidate: envelope.Candidate, ContextDigest: envelope.Context, PacketDigest: envelope.Packet}
	for i, a := range packet.Artifacts {
		raw, ok := entries[strconv.Itoa(i)]
		if !ok {
			return fmt.Errorf("missing artifact")
		}
		var entry struct {
			Verdict  string          `json:"v"`
			Reason   string          `json:"r"`
			Evidence json.RawMessage `json:"e"`
			Needs    []string        `json:"n"`
		}
		if err := strict(raw, &entry); err != nil {
			return err
		}
		allowed := fragmentQuoteChoices(a.Content)
		if len(allowed) == 0 {
			allowed = []string{"indisponible"}
			if entry.Verdict == "inspected" {
				return fmt.Errorf("no quote available")
			}
		}
		var excerpt string
		if strings.HasPrefix(strings.TrimSpace(string(entry.Evidence)), "\"") {
			if err := json.Unmarshal(entry.Evidence, &excerpt); err != nil {
				return err
			}
		} else {
			var index int
			if err := json.Unmarshal(entry.Evidence, &index); err != nil {
				return err
			}
			if string(entry.Evidence) == "null" || len(entry.Evidence) == 0 || index < 0 || index >= len(allowed) {
				return fmt.Errorf("unknown anchor")
			}
			excerpt = allowed[index]
		}
		found := false
		for _, q := range allowed {
			found = found || q == excerpt
		}
		if !found {
			return fmt.Errorf("unoffered quote")
		}
		result.Findings = append(result.Findings, managedFragmentFinding{Artifact: i, Digest: a.Digest, Verdict: entry.Verdict, Reason: entry.Reason, Evidence: excerpt, Needs: entry.Needs})
	}
	return nil
}
