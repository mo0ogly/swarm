//go:build linux

package main

import (
	"encoding/json"
	"fmt"
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
	if err := strict([]byte(reply), &result); err != nil {
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
			if len(strings.TrimSpace(finding.Evidence)) < 8 || !strings.Contains(artifact.Content, finding.Evidence) || len(finding.Needs) != 0 {
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
