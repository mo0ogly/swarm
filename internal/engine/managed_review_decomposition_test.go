//go:build linux

package engine

import (
	"fmt"
	"reflect"
	"testing"
)

func TestReviewDecompositionSignalsAndExactCoverage(t *testing.T) {
	cases := []struct {
		name     string
		files    []string
		bytes    int
		required bool
	}{
		{"small", []string{"planning.go", "planning_test.go", "docs/change.md"}, 1024, false},
		{"large-single-file", []string{"planning.go"}, managedReviewPromptLimit + 1, true},
		{"at-limit", []string{"planning.go"}, managedReviewPromptLimit, false},
	}
	large := []string{}
	for i := 0; i < 101; i++ {
		large = append(large, fmt.Sprintf("managed_%03d.go", i))
	}
	cases = append(cases, struct {
		name     string
		files    []string
		bytes    int
		required bool
	}{"file-count", large, 100, true})
	mixed := []string{}
	for i := 0; i < 5; i++ {
		for _, prefix := range []string{"web/", "docs/", "tests/", "planning_"} {
			mixed = append(mixed, fmt.Sprintf("%s%d.go", prefix, i))
		}
	}
	cases = append(cases, struct {
		name     string
		files    []string
		bytes    int
		required bool
	}{"mixed", mixed, 100, true})
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			d := detectReviewDecomposition("candidate", tc.files, tc.bytes)
			if d.Required != tc.required || d.Candidate != "candidate" || d.State != "proposal_only" || len(d.Options) != 5 || d.FinalReview == "" {
				t.Fatal(d)
			}
			seen := map[string]int{}
			for _, lot := range d.Suggestions {
				if lot.Dependencies == "" {
					t.Fatal("invented independence")
				}
				for _, f := range lot.Files {
					seen[f]++
				}
			}
			for _, f := range tc.files {
				if seen[f] != 1 {
					t.Fatal("coverage", f, seen[f])
				}
			}
			if len(seen) != len(tc.files) {
				t.Fatal("foreign paths")
			}
			reverse := append([]string{}, tc.files...)
			for i, j := 0, len(reverse)-1; i < j; i, j = i+1, j-1 {
				reverse[i], reverse[j] = reverse[j], reverse[i]
			}
			if !reflect.DeepEqual(d, detectReviewDecomposition("candidate", reverse, tc.bytes)) {
				t.Fatal("unstable proposal")
			}
		})
	}
}
