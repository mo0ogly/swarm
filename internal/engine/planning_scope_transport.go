//go:build linux

package engine

import (
	"encoding/json"
	"unicode/utf8"
)

// Lossless transport only. Public previews and durable decisions retain full
// paths. Each row combines its previous path prefix and suffix, preserving
// original UTF-8 bytes and order.
func compactPlanningScopeInventory(p ManagedScopePlan) any {
	raw, _ := json.Marshal(p)
	var out map[string]any
	json.Unmarshal(raw, &out)
	rows := [][]any{}
	files := map[string]int{}
	previous := ""
	for i, name := range p.Files {
		if _, duplicate := files[name]; duplicate {
			return p
		}
		files[name] = i
		common := 0
		for common < len(name) && common < len(previous) && name[common] == previous[common] {
			common++
		}
		for common > 0 && !utf8.ValidString(name[:common]) {
			common--
		}
		rows = append(rows, []any{common, name[common:]})
		previous = name
	}
	delete(out, "changed_files")
	out["file_paths"] = map[string]any{"columns": []string{"previous_prefix_utf8_bytes", "remaining_path"}, "rows": rows, "reconstruct": "Previous reconstructed path prefix of previous_prefix_utf8_bytes bytes + remaining_path. First previous path is empty. Exact original bytes/order; file_indices below refer to these zero-based rows."}
	if p.Decomposition != nil {
		d := out["decomposition"].(map[string]any)
		suggestions := []map[string]any{}
		for _, lot := range p.Decomposition.Suggestions {
			indices := []int{}
			for _, name := range lot.Files {
				i, ok := files[name]
				if !ok {
					return p
				}
				indices = append(indices, i)
			}
			suggestions = append(suggestions, map[string]any{"component": lot.Component, "file_indices": indices, "dependencies": lot.Dependencies})
		}
		d["suggested_lots"] = suggestions
	}
	compact, _ := json.Marshal(out)
	if len(compact) >= len(raw) {
		return p
	}
	return out
}
