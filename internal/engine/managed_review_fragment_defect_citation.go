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
			return nil, &CommandError{Code: reviewFragmentProtocolError, Message: fmt.Sprintf("citation de la pièce %d : %s", *d.Artifact, err)}
		}
		defects = append(defects, managedFragmentDefect{Artifact: *d.Artifact, Line: line, Quote: d.Quote, Explanation: d.Explanation, Reproduction: d.Reproduction, Expected: d.Expected})
	}
	return defects, nil
}
