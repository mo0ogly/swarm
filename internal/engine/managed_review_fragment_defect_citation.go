//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strings"
)

const managedFragmentDefectCitationContractVersion = 1

func supportedFragmentDefectCitationContract(version int) bool {
	return version == 0 || version == managedFragmentDefectCitationContractVersion
}

// Locate unchanged, uniquely occurring bytes; never repair a quote or infer an
// opinion. The existing physical-line verifier remains authoritative.
func fragmentDefectUniqueQuoteLine(text, quote string) (int, error) {
	if strings.TrimSpace(quote) == "" {
		return 0, fmt.Errorf("empty quote")
	}
	offset := strings.Index(text, quote)
	if offset < 0 {
		return 0, fmt.Errorf("quote absent")
	}
	if strings.Contains(text[offset+1:], quote) {
		return 0, fmt.Errorf("ambiguous quote")
	}
	line := strings.Count(text[:offset], "\n") + 1
	if !fragmentDefectQuoteAtLine(text, line, quote) {
		return 0, fmt.Errorf("quote starts at a line separator")
	}
	return line, nil
}

func managedFragmentDefectQuoteOnlySchema() map[string]any {
	schema := managedFragmentDefectSchema()
	item := schema["items"].(map[string]any)
	props := item["properties"].(map[string]any)
	delete(props, "line")
	props["quote"].(map[string]any)["minLength"] = 1
	item["required"] = []string{"artifact", "quote", "explanation", "reproduction", "expected"}
	return schema
}

type managedNestedDefectClaim struct {
	Quote        string `json:"quote"`
	Explanation  string `json:"explanation"`
	Reproduction string `json:"reproduction"`
	Expected     string `json:"expected"`
}

func fragmentDefectCitationFailure(packet managedReviewFragmentPacket, artifact int, quote string, cause error) error {
	matches := []int{}
	originals := []map[string]any{}
	count := 0
	if strings.TrimSpace(quote) != "" {
		for i, a := range packet.Artifacts {
			if i == artifact || !strings.Contains(fragmentDefectText(a), quote) {
				continue
			}
			count++
			if len(matches) < managedFragmentDefectLimit {
				matches = append(matches, i)
				name := []rune(fragmentDiagnosticExcerpt(a.Name))
				for !boundedFragmentFindingText(string(name)) {
					name = name[:len(name)-1]
				}
				originals = append(originals, map[string]any{"artifact": i, "name_excerpt": string(name), "sha256": a.Digest})
			}
		}
	}
	facts := map[string]any{"packet": packet.Index, "artifact": artifact, "error": cause.Error(), "quote_field_empty": strings.TrimSpace(quote) == "", "quote_sha256": hash([]byte(quote)), "rejected_quote": fragmentDiagnosticExcerpt(quote), "literal_matches_elsewhere": matches, "matching_artifact_count": count, "other_originals": originals}
	raw, _ := json.Marshal(facts)
	return &CommandError{Code: reviewFragmentProtocolError, Message: fmt.Sprintf("citation de la pièce %d : texte cité invalide pour cette pièce ; ne pas confondre texte introuvable et champ JSON absent. Diagnostic non validant : %s", artifact, raw)}
}

func decodeManagedFragmentNestedDefects(raw json.RawMessage, artifact int, packet managedReviewFragmentPacket) ([]managedFragmentDefect, error) {
	var supplied []managedNestedDefectClaim
	if err := strict(raw, &supplied); err != nil {
		return nil, err
	}
	if len(supplied) == 0 || len(supplied) > managedFragmentDefectLimit {
		return nil, fmt.Errorf("constat fail sans détails bornés")
	}
	defects := make([]managedFragmentDefect, 0, len(supplied))
	for _, d := range supplied {
		line, err := fragmentDefectUniqueQuoteLine(fragmentDefectText(packet.Artifacts[artifact]), d.Quote)
		if err != nil {
			return nil, fragmentDefectCitationFailure(packet, artifact, d.Quote, err)
		}
		defects = append(defects, managedFragmentDefect{Artifact: artifact, Line: line, Quote: d.Quote, Explanation: d.Explanation, Reproduction: d.Reproduction, Expected: d.Expected})
	}
	return defects, nil
}

func decodeManagedFragmentDefectCitations(raw json.RawMessage, packet managedReviewFragmentPacket) ([]managedFragmentDefect, error) {
	if !supportedFragmentDefectCitationContract(packet.DefectCitationContractVersion) {
		return nil, fmt.Errorf("unsupported citation contract")
	}
	if len(raw) == 0 {
		return nil, nil
	}
	if packet.DefectCitationContractVersion == 0 {
		var defects []managedFragmentDefect
		err := strict(raw, &defects)
		return defects, err
	}
	var supplied []struct {
		Artifact     *int   `json:"artifact"`
		Quote        string `json:"quote"`
		Explanation  string `json:"explanation"`
		Reproduction string `json:"reproduction"`
		Expected     string `json:"expected"`
	}
	if err := strict(raw, &supplied); err != nil {
		return nil, err
	}
	defects := make([]managedFragmentDefect, 0, len(supplied))
	for _, d := range supplied {
		if d.Artifact == nil || *d.Artifact < 0 || *d.Artifact >= len(packet.Artifacts) {
			return nil, fmt.Errorf("unknown defect artifact")
		}
		line, err := fragmentDefectUniqueQuoteLine(fragmentDefectText(packet.Artifacts[*d.Artifact]), d.Quote)
		if err != nil {
			return nil, fragmentDefectCitationFailure(packet, *d.Artifact, d.Quote, err)
		}
		defects = append(defects, managedFragmentDefect{Artifact: *d.Artifact, Line: line, Quote: d.Quote, Explanation: d.Explanation, Reproduction: d.Reproduction, Expected: d.Expected})
	}
	return defects, nil
}
