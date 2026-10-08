//go:build linux

package main

import (
	"encoding/json"
	"os"
	"regexp"
	"strings"
	"testing"
)

func TestReportSummaryProviderSchemaBoundsLines(t *testing.T) {
	for _, provider := range []string{"claude", "codex"} {
		t.Run(provider, func(t *testing.T) {
			turn := AssistTurn{TemplateID: "report_summary.v1", Context: PageContext{Hash: strings.Repeat("a", 64), Facts: []PageFact{{ID: "report-fact"}}}}
			p, err := assistantStructuredProvider(Provider{Command: "/bin/" + provider, Args: []string{"-"}}, turn, t.TempDir())
			if err != nil {
				t.Fatal(err)
			}
			raw := []byte(p.Args[len(p.Args)-1])
			if provider == "codex" {
				raw, err = os.ReadFile(p.Args[len(p.Args)-2])
				if err != nil {
					t.Fatal(err)
				}
			}
			var schema struct {
				Properties map[string]struct {
					Pattern string `json:"pattern"`
				} `json:"properties"`
			}
			if err := json.Unmarshal(raw, &schema); err != nil {
				t.Fatal(err)
			}
			pattern := schema.Properties["interpretation"].Pattern
			if pattern == "" {
				t.Fatal("provider was not given the report summary format")
			}
			rule := regexp.MustCompile(pattern)
			for _, tc := range []struct {
				text  string
				valid bool
			}{
				{"Résultat annoncé.\nLa validation reste nécessaire.", true},
				{strings.Repeat("é", 320) + "\nSuite.", true},
				{strings.Repeat("é", 321) + "\nSuite.", false},
				{"Une seule ligne.", false}, {"Une ligne.\n", false},
				{"Une.\nDeux.\nTrois.", false}, {`Une.\nDeux.`, false},
			} {
				if rule.MatchString(tc.text) != tc.valid {
					t.Fatalf("schema accepts incorrect format: %q", tc.text)
				}
			}
		})
	}
}
