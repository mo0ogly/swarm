//go:build linux

package engine

import "strings"

// Prose citations may omit Markdown inline-code delimiters and physical line
// wrapping. Code artifacts still require an exact substring. Diff runs never
// cross a hunk, file header, blank paragraph or addition/deletion boundary.
func fragmentEvidencePresent(a managedReviewFragmentArtifact, evidence string) bool {
	if strings.Contains(a.Content, evidence) {
		return true
	}
	if !strings.HasSuffix(a.Name, ".md") || (a.Kind != "diff" && a.Kind != "source") {
		return false
	}
	normalize := func(s string) string { return strings.Join(strings.Fields(strings.ReplaceAll(s, "`", "")), " ") }
	quote := normalize(evidence)
	if len(quote) < 8 {
		return false
	}
	var run []string
	prefix := byte(0)
	fenced := false
	matches := func() bool { return strings.Contains(normalize(strings.Join(run, " ")), quote) }
	for _, line := range strings.Split(a.Content, "\n") {
		next := byte(0)
		if a.Kind == "diff" {
			if len(line) == 0 || strings.HasPrefix(line, "+++") || strings.HasPrefix(line, "---") || (line[0] != '+' && line[0] != '-' && line[0] != ' ') {
				if matches() {
					return true
				}
				run = nil
				prefix = 0
				continue
			}
			next = line[0]
			line = line[1:]
		}
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "```") || strings.HasPrefix(trimmed, "~~~") {
			if matches() {
				return true
			}
			run = nil
			fenced = !fenced
			prefix = next
			continue
		}
		if fenced {
			run = nil
			prefix = next
			continue
		}
		if strings.TrimSpace(line) == "" || next != prefix {
			if matches() {
				return true
			}
			run = nil
		}
		prefix = next
		if strings.TrimSpace(line) != "" {
			run = append(run, line)
		}
	}
	return matches()
}
