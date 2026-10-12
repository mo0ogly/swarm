//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strings"
	"unicode/utf8"
)

// A defect is an independently supplied claim, not an engine-certified bug.
// Version 2 requires a reproducible location and explanation in the original
// reply; the engine checks attribution, never invents the explanation.
type managedFragmentDefect struct {
	Artifact     int    `json:"artifact"`
	Line         int    `json:"line"`
	Quote        string `json:"quote"`
	Explanation  string `json:"explanation"`
	Reproduction string `json:"reproduction"`
	Expected     string `json:"expected"`
}

const managedFragmentDefectLimit = 4
const managedFragmentDefectReserve = 8 * 1024

func validateManagedFragmentDefects(p managedReviewFragmentPacket, r managedFragmentInspection) error {
	if p.Version < 2 {
		if len(r.Defects) > 0 {
			return fmt.Errorf("defects require protocol v2")
		}
		return nil
	}
	if len(r.Defects) > managedFragmentDefectLimit {
		return fmt.Errorf("too many defects")
	}
	failed := map[int]bool{}
	for _, f := range r.Findings {
		if f.Verdict == "fail" {
			failed[f.Artifact] = true
		}
	}
	seen := map[int]bool{}
	claims := map[string]bool{}
	for _, d := range r.Defects {
		if !failed[d.Artifact] || (p.DefectCitationContractVersion == 0 && seen[d.Artifact]) || d.Artifact < 0 || d.Artifact >= len(p.Artifacts) {
			return fmt.Errorf("defect without unique failed artifact")
		}
		if p.DefectCitationContractVersion == managedFragmentDefectCitationContractVersion {
			key, err := json.Marshal(d)
			if err != nil || claims[string(key)] {
				return fmt.Errorf("duplicate defect claim")
			}
			claims[string(key)] = true
		}
		seen[d.Artifact] = true
		minimumQuote := 8
		if p.DefectCitationContractVersion == managedFragmentDefectCitationContractVersion {
			minimumQuote = 1
		}
		if utf8.RuneCountInString(strings.TrimSpace(d.Quote)) < minimumQuote || utf8.RuneCountInString(d.Quote) > 256 {
			return fmt.Errorf("defect quote outside required length (artifact %d, minimum %d)", d.Artifact, minimumQuote)
		}
		if !fragmentDefectQuoteAtLine(fragmentDefectText(p.Artifacts[d.Artifact]), d.Line, d.Quote) {
			return fmt.Errorf("defect quote absent at original line (artifact %d, line %d)", d.Artifact, d.Line)
		}

		if p.DefectCitationContractVersion == managedFragmentDefectCitationContractVersion {
			line, err := fragmentDefectUniqueQuoteLine(fragmentDefectText(p.Artifacts[d.Artifact]), d.Quote)
			if err != nil || line != d.Line {
				return fmt.Errorf("defect citation no longer unique at its original line")
			}
		}
		for _, v := range []struct {
			s        string
			min, max int
		}{{d.Explanation, 32, 512}, {d.Reproduction, 16, 384}, {d.Expected, 8, 256}} {
			n := utf8.RuneCountInString(strings.TrimSpace(v.s))
			if n < v.min || n > v.max {
				return fmt.Errorf("defect explanation or reproduction incomplete")
			}
		}
	}
	if len(seen) != len(failed) {
		return fmt.Errorf("failed artifact without actionable defect")
	}
	raw, e := json.Marshal(r.Defects)
	if e != nil || len(raw) > managedFragmentDefectReserve {
		return fmt.Errorf("defect details exceed reserved capacity")
	}
	return nil
}

// Line is a physical starting line, not an implicit column-zero constraint.
// Match original bytes, including indentation, patch signs and line separators.
// A multiline quote must begin before the end of the declared line.
func fragmentDefectQuoteAtLine(text string, line int, quote string) bool {
	lines := strings.Split(text, "\n")
	if line < 1 || line > len(lines) || quote == "" {
		return false
	}
	offset := strings.Index(strings.Join(lines[line-1:], "\n"), quote)
	return offset >= 0 && offset < len(lines[line-1])
}

func managedFragmentDefectSchema() map[string]any {
	str := func(min, max int) any { return map[string]any{"type": "string", "minLength": min, "maxLength": max} }
	return map[string]any{"type": "array", "maxItems": managedFragmentDefectLimit, "items": map[string]any{"type": "object", "additionalProperties": false, "required": []string{"artifact", "line", "quote", "explanation", "reproduction", "expected"}, "properties": map[string]any{"artifact": map[string]any{"type": "integer", "minimum": 0}, "line": map[string]any{"type": "integer", "minimum": 1}, "quote": str(8, 256), "explanation": str(32, 512), "reproduction": str(16, 384), "expected": str(8, 256)}}}
}

// Supplemental sources are JSON-wrapped for transport; line numbers refer to
// their original source contents. Diff locations refer to the supplied patch.
func fragmentDefectText(a managedReviewFragmentArtifact) string {
	if a.Kind == "source" {
		var v ReviewSource
		if json.Unmarshal([]byte(a.Content), &v) == nil && v.Path != "" && v.Bytes == len(v.Content) && v.Digest == hash([]byte(v.Content)) {
			return v.Content
		}
	}
	return a.Content
}
