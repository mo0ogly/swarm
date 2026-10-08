//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestFragmentMarkdownQuoteFormattingOnly(t *testing.T) {
	for _, tc := range []struct {
		name, kind, content, quote string
		ok                         bool
	}{
		{"ticks", "diff", "+`preview` est une lecture indicative ;", "preview est une lecture indicative", true},
		{"wrap", "diff", "+Aucun essai\n+avec un modèle réel", "Aucun essai avec un modèle réel", true},
		{"negation", "diff", "+ne garantit pas que le fournisseur sera disponible.", "garantit que le fournisseur sera disponible", false},
		{"invented", "diff", "+Une page périmée est refusée.", "Une page périmée est acceptée.", false},
		{"mixed", "diff", "-Aucun essai\n+avec un modèle réel", "Aucun essai avec un modèle réel", false},
		{"hunks", "diff", "+Aucun essai\n@@ -4,1 +4,1 @@\n+avec un modèle réel", "Aucun essai avec un modèle réel", false},
		{"paragraphs", "diff", "+Aucun essai\n+\n+avec un modèle réel", "Aucun essai avec un modèle réel", false},
		{"fenced", "diff", "+```go\n+return `raw` text\n+```", "return raw text", false},
		{"source", "source", "Aucun essai\navec un modèle réel", "Aucun essai avec un modèle réel", true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			a := managedReviewFragmentArtifact{Kind: tc.kind, Name: "docs/proof.md", Content: tc.content}
			if got := fragmentEvidencePresent(a, tc.quote); got != tc.ok {
				t.Fatal(got)
			}
			a.Name = "proof.go"
			if !strings.Contains(tc.content, tc.quote) && fragmentEvidencePresent(a, tc.quote) {
				t.Fatal("code normalized")
			}
		})
	}
}
