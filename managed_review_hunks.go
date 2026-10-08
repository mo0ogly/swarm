//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"regexp"
	"strconv"
	"strings"
)

type reviewHunkReference struct {
	Path   string `json:"path"`
	Number int    `json:"number"`
}

type reviewSourcePart struct {
	Bytes  int                  `json:"bytes"`
	Digest string               `json:"sha256"`
	Hunk   *reviewHunkReference `json:"from_diff_hunk,omitempty"`
	text   string
}

var reviewHunkHeader = regexp.MustCompile(`^@@ -([0-9]+)(?:,([0-9]+))? \+([0-9]+)(?:,([0-9]+))? @@(?: .*)?$`)

// Reuse only entire, ordinary, newline-terminated hunks whose candidate-side
// bytes match the complete source at the declared new-file line exactly.
// Unsupported/ambiguous patches retain their literal source. The global diff
// itself is never shortened, and canonical context/digests are never modified.
func reviewSourceParts(diff string, source ReviewSource) []reviewSourcePart {
	if source.Bytes != len(source.Content) || source.Digest != hash([]byte(source.Content)) || !strings.HasSuffix(source.Content, "\n") {
		return nil
	}
	section := ""
	for _, candidate := range strings.Split("\n"+diff, "\ndiff --git ") {
		if strings.HasPrefix(candidate, "a/"+source.Path+" b/"+source.Path+"\n") {
			if section != "" {
				return nil
			}
			section = candidate
		}
	}
	if section == "" || strings.Contains(section, "\n\\ No newline at end of file") {
		return nil
	}
	lines := strings.Split(strings.TrimSuffix(section, "\n"), "\n")
	indexOK := false
	for _, line := range lines {
		if strings.HasPrefix(line, "@@ ") {
			break
		}
		if strings.HasPrefix(line, "index ") {
			fields := strings.Fields(line)
			if len(fields) < 2 {
				return nil
			}
			pair := strings.Split(fields[1], "..")
			indexOK = len(pair) == 2 && pair[1] == source.Blob
		}
	}
	if !indexOK {
		return nil
	}
	offsets := []int{0}
	for i, b := range []byte(source.Content) {
		if b == '\n' {
			offsets = append(offsets, i+1)
		}
	}
	part := func(s string, ref *reviewHunkReference) reviewSourcePart {
		return reviewSourcePart{Bytes: len(s), Digest: hash([]byte(s)), Hunk: ref, text: s}
	}
	var parts []reviewSourcePart
	cursor, number, previousEnd := 0, 0, 0
	for i := 0; i < len(lines); i++ {
		if !strings.HasPrefix(lines[i], "@@") {
			continue
		}
		m := reviewHunkHeader.FindStringSubmatch(lines[i])
		if m == nil {
			return nil
		}
		number++
		count := func(v string) int {
			if v == "" {
				return 1
			}
			n, e := strconv.Atoi(v)
			if e != nil {
				return -1
			}
			return n
		}
		oldCount, newCount := count(m[2]), count(m[4])
		start, e := strconv.Atoi(m[3])
		if e != nil || oldCount < 0 || newCount < 0 {
			return nil
		}
		oldSeen, newSeen := 0, 0
		var target strings.Builder
		j := i + 1
		for ; j < len(lines) && !strings.HasPrefix(lines[j], "@@"); j++ {
			line := lines[j]
			if line == "" {
				return nil
			}
			switch line[0] {
			case ' ':
				oldSeen++
				newSeen++
				target.WriteString(line[1:] + "\n")
			case '+':
				newSeen++
				target.WriteString(line[1:] + "\n")
			case '-':
				oldSeen++
			default:
				return nil
			}
		}
		i = j - 1
		if oldSeen != oldCount || newSeen != newCount {
			return nil
		}
		if newCount == 0 {
			continue
		}
		if start < 1 || start-1 >= len(offsets) || newCount > len(offsets)-start {
			return nil
		}
		begin, end := offsets[start-1], offsets[start-1+newCount]
		if begin < previousEnd || source.Content[begin:end] != target.String() {
			return nil
		}
		previousEnd = end
		// Small matches cost more framing than the duplicate bytes themselves.
		if end-begin < 512 {
			continue
		}
		if begin > cursor {
			parts = append(parts, part(source.Content[cursor:begin], nil))
		}
		parts = append(parts, part(target.String(), &reviewHunkReference{source.Path, number}))
		cursor = end
	}
	if len(parts) == 0 {
		return nil
	}
	if cursor < len(source.Content) {
		parts = append(parts, part(source.Content[cursor:], nil))
	}
	return parts
}

func writeReviewSourceParts(out *strings.Builder, field, diff string, source ReviewSource) bool {
	parts := reviewSourceParts(diff, source)
	if len(parts) == 0 {
		return false
	}
	var encoded strings.Builder
	header, _ := json.Marshal(struct {
		Field  string `json:"field"`
		Bytes  int    `json:"bytes"`
		Digest string `json:"sha256"`
		Parts  int    `json:"parts"`
	}{field, source.Bytes, source.Digest, len(parts)})
	fmt.Fprintln(&encoded, string(header))
	for _, p := range parts {
		h, _ := json.Marshal(p)
		fmt.Fprintln(&encoded, string(h))
		if p.Hunk == nil {
			fmt.Fprintln(&encoded, p.text)
		}
	}
	if encoded.Len() >= len(source.Content) {
		return false
	}
	out.WriteString(encoded.String())
	return true
}
