//go:build linux

package main

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
	for _, d := range r.Defects {
		if !failed[d.Artifact] || seen[d.Artifact] || d.Artifact < 0 || d.Artifact >= len(p.Artifacts) {
			return fmt.Errorf("defect without unique failed artifact")
		}
		seen[d.Artifact] = true
		lines := strings.Split(fragmentDefectText(p.Artifacts[d.Artifact]), "\n")
		if d.Line < 1 || d.Line > len(lines) || utf8.RuneCountInString(strings.TrimSpace(d.Quote)) < 8 || utf8.RuneCountInString(d.Quote) > 256 || !strings.HasPrefix(strings.Join(lines[d.Line-1:], "\n"), d.Quote) {
			return fmt.Errorf("defect quote absent at original line")
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
