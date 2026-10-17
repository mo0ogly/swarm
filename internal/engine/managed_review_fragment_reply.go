//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"unicode/utf8"
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
	Defects       []managedFragmentDefect  `json:"defects,omitempty"`
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
		return "", managedFragmentInspection{}, &CommandError{Code: reviewFragmentProtocolError, Message: "inspection de fragment : " + reason}
	}
	if !supportedFragmentInspectionContract(packet.InspectionContractVersion) {
		return fail("contrat d’inspection non pris en charge")
	}
	if packet.InspectionContractVersion == managedFragmentInspectionContractVersion && !utf8.ValidString(reply) {
		return fail("réponse UTF-8 invalide")
	}
	if !supportedFragmentDefectCitationContract(packet.DefectCitationContractVersion) {
		return fail("contrat de citation non pris en charge")
	}
	if !supportedFragmentFindingContract(packet.FindingContractVersion) {
		return fail("contrat de constat non pris en charge")
	}
	limit := managedFragmentReplyLimit
	if packet.Version >= 2 {
		limit += managedFragmentDefectReserve
	}
	if len(reply) > limit {
		return fail("réponse trop grande")
	}
	if err := decodeManagedFragmentInspection([]byte(reply), packet, &result); err != nil {
		if citation, ok := err.(*CommandError); ok && citation.Code == reviewFragmentProtocolError {
			return fail(citation.Message)
		}
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
		for index, need := range finding.Needs {
			minimum, maximum := fragmentNeedLength(need, packet.InspectionContractVersion)
			if minimum < 3 || maximum > 240 || needs[need] {
				if packet.InspectionContractVersion == managedFragmentInspectionContractVersion {
					return fail(fmt.Sprintf("demande de preuve invalide : artifact=%d need_index=%d codepoints=%d utf8_bytes=%d ; 3 à 240 caractères Unicode distincts requis", finding.Artifact, index, utf8.RuneCountInString(need), len(need)))
				}
				return fail("demande de preuve invalide")
			}
			needs[need] = true
		}
		switch finding.Verdict {
		case "inspected":
			if len(finding.Needs) != 0 {
				return fail(fmt.Sprintf("pièce %d : inspected incompatible avec %d demande(s) de preuve ; retourner unknown", finding.Artifact, len(finding.Needs)))
			}
			if len(strings.TrimSpace(finding.Evidence)) < 8 || !fragmentEvidencePresent(artifact, finding.Evidence) {
				return fail(fmt.Sprintf("pièce %d : extrait original absent", finding.Artifact))
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
	if err := validateManagedFragmentDefects(packet, result); err != nil {
		return fail(err.Error())
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
		Defects   json.RawMessage `json:"defects,omitempty"`
	}
	if err := strict(raw, &envelope); err != nil {
		return err
	}
	nested := packet.FindingContractVersion == managedFragmentFindingContractVersion
	if nested && (packet.Version < 2 || packet.DefectCitationContractVersion != managedFragmentDefectCitationContractVersion || len(envelope.Defects) != 0) {
		return fmt.Errorf("constat2 exige citations liées au constat, sans defects global")
	}
	defects, err := decodeManagedFragmentDefectCitations(envelope.Defects, packet)
	if err != nil {
		return err
	}
	if strings.HasPrefix(strings.TrimSpace(string(envelope.Findings)), "[") {
		if nested || (packet.InputBudget != nil && packet.InputBudget.FinalReserveContractVersion == 1) {
			return fmt.Errorf("nouveau contrat de réserve : constats indexés et ancres proposées requis")
		}
		var findings []managedFragmentFinding
		if err := strict(envelope.Findings, &findings); err != nil {
			return err
		}
		*result = managedFragmentInspection{Candidate: envelope.Candidate, ContextDigest: envelope.Context, PacketDigest: envelope.Packet, Findings: findings, Defects: defects}
		return nil
	}
	var entries map[string]json.RawMessage
	if err := strict(envelope.Findings, &entries); err != nil {
		return err
	}
	if len(entries) != len(packet.Artifacts) {
		return fmt.Errorf("incomplete inventory")
	}
	*result = managedFragmentInspection{Candidate: envelope.Candidate, ContextDigest: envelope.Context, PacketDigest: envelope.Packet, Defects: defects}
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
			Defects  json.RawMessage `json:"d"`
		}
		if err := strict(raw, &entry); err != nil {
			return err
		}
		if nested {
			var fields map[string]json.RawMessage
			if err := strict(raw, &fields); err != nil {
				return err
			}
			_, needs := fields["n"]
			_, details := fields["d"]
			if (entry.Verdict == "inspected" && (needs || details)) || (entry.Verdict == "unknown" && (!needs || len(entry.Needs) == 0 || details)) || (entry.Verdict == "fail" && (needs || !details)) {
				return fmt.Errorf("champs incompatibles avec le verdict du constat %d", i)
			}
			if details {
				claims, err := decodeManagedFragmentNestedDefects(entry.Defects, i, packet)
				if err != nil {
					return err
				}
				result.Defects = append(result.Defects, claims...)
			}
		} else if len(entry.Defects) != 0 {
			return fmt.Errorf("défauts liés au constat exigent contrat2")
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
			if nested {
				return fmt.Errorf("constat2 exige numéro d’ancre")
			}
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
