//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"reflect"
	"testing"
)

func TestPlanningScopeTransportPreservesCompleteInventory(t *testing.T) {
	files := []string{}
	for i := 0; i < 272; i++ {
		files = append(files, fmt.Sprintf("internal/engine/complete-source-%03d.go", i))
	}
	files = append(files, "docs/épreuve espace .md", "rootfile")
	p := ManagedScopePlan{Candidate: "candidate", Evidence: "proof", Files: files, Criteria: []string{"task#1"}, Decomposition: &ReviewDecomposition{Suggestions: []ReviewLotSuggestion{{Component: "all", Files: files, Dependencies: "not examined"}}}}
	before, _ := json.Marshal(p)
	raw, _ := json.Marshal(compactPlanningScopeInventory(p))
	if len(raw) >= len(before)/2 {
		t.Fatal("inventory not usefully compacted", len(raw), len(before))
	}
	var got struct {
		Candidate string `json:"candidate_commit"`
		Evidence  string `json:"evidence_sha256"`
		Paths     struct {
			Prefixes []string            `json:"prefixes"`
			Rows     []struct{}          `json:"unused"`
			Values   [][]json.RawMessage `json:"rows"`
		} `json:"file_paths"`
		Decomposition struct {
			Suggestions []struct {
				Indices []int `json:"file_indices"`
			} `json:"suggested_lots"`
		} `json:"decomposition"`
	}
	if err := json.Unmarshal(raw, &got); err != nil {
		t.Fatal(err)
	}
	rebuilt := []string{}
	previous := ""
	for _, row := range got.Paths.Values {
		var i int
		var name string
		json.Unmarshal(row[0], &i)
		json.Unmarshal(row[1], &name)
		previous = previous[:i] + name
		rebuilt = append(rebuilt, previous)
	}
	if !reflect.DeepEqual(rebuilt, files) || got.Candidate != p.Candidate || got.Evidence != p.Evidence {
		t.Fatal("proof identity/order/paths lost")
	}
	for i, index := range got.Decomposition.Suggestions[0].Indices {
		if rebuilt[index] != files[i] {
			t.Fatal("suggested lot mapping lost")
		}
	}
	after, _ := json.Marshal(p)
	if string(before) != string(after) {
		t.Fatal("public preview changed")
	}
	p.Decomposition.Suggestions[0].Files = []string{"unknown"}
	if !reflect.DeepEqual(compactPlanningScopeInventory(p), p) {
		t.Fatal("unknown reference hidden")
	}
}
