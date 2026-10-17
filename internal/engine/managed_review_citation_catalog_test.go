//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
	"unicode/utf8"
)

func TestCitationCatalogPreservesOriginalBytesWithoutAgentCopy(t *testing.T) {
	for _, tc := range []struct{ name, original, rejected string }{
		{"patch indentation", "+\tv, err := s.trustOperate(action, cmd, os.Getenv(\"SWARM_TRUST_TOKEN\"), time.Now().UTC())\n", "+v, err := s.trustOperate(action, cmd, os.Getenv(\"SWARM_TRUST_TOKEN\"), time.Now().UTC())"},
		{"captured JSON output", `{"stdout":"PASS i18n build check accepts current output and refuses stale output without mutation\n"}`, "PASS i18n build check accepts current output and refuses stale output without mutation\n"},
		{"multi-line patch", "+if a.mode=='race':\n+    argv=['go','test','-race']\n+else:\n+    env['GOARCH']='386'\n", "if a.mode=='race':\n argv=['go','test','-race']\nelse:\n env['GOARCH']='386'"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if strings.Contains(tc.original, tc.rejected) {
				t.Fatal("native counterexample lost")
			}
			c, err := buildManagedCitationCatalog("candidate", hash([]byte("context")), []managedReviewFragmentArtifact{{Name: "original.txt", Kind: "source", Content: tc.original}})
			if err != nil {
				t.Fatal(err)
			}
			r, err := c.resolver(c.Candidate, c.ContextDigest, c.digest())
			if err != nil {
				t.Fatal(err)
			}
			text, err := r.quote(0)
			if err != nil || text != tc.original || c.Originals[0].Digest != hash([]byte(tc.original)) {
				t.Fatal("reference changed original bytes", err)
			}
		})
	}
}

func TestCitationCatalogCoversUnicodeAndTailWithoutTruncation(t *testing.T) {
	for _, text := range []string{"", "small", strings.Repeat("界é😀\n", 150), strings.Repeat("x", managedCitationChunkBytes+1), strings.Repeat("abc\t\n", 700)} {
		c, err := buildManagedCitationCatalog("candidate", hash([]byte("context")), []managedReviewFragmentArtifact{{Name: "utf8.txt", Kind: "source", Content: text}})
		if err != nil {
			t.Fatal(err)
		}
		if _, err = c.resolver(c.Candidate, c.ContextDigest, c.digest()); err != nil {
			t.Fatal(err)
		}
		var rebuilt strings.Builder
		for _, span := range c.Originals[0].Spans {
			quote := c.Quotes[span.Reference]
			if span.Offset != rebuilt.Len() || !utf8.ValidString(quote.Text) || len(quote.Text) > managedCitationChunkBytes+7 {
				t.Fatal("source range missing, oversized or split inside UTF-8")
			}
			rebuilt.WriteString(managedCitationSpanText(quote, span))
		}
		if rebuilt.String() != text {
			t.Fatal("original truncated or normalized")
		}
	}
	if _, err := buildManagedCitationCatalog("candidate", hash([]byte("context")), []managedReviewFragmentArtifact{{Name: "bad", Kind: "source", Content: "invalid \xff"}}); err == nil {
		t.Fatal("invalid UTF-8 silently substituted")
	}
}

func TestCitationCatalogRejectsForeignAndMalformedReferences(t *testing.T) {
	c, err := buildManagedCitationCatalog("candidate", hash([]byte("context")), []managedReviewFragmentArtifact{{Name: "proof.txt", Kind: "source", Content: "Original attributed implementation proof"}})
	if err != nil {
		t.Fatal(err)
	}
	digest := c.digest()
	for _, mode := range []string{"candidate", "context", "digest", "content", "offset", "source hash", "orphan", "quote id"} {
		raw, _ := json.Marshal(c)
		var copy managedCitationCatalog
		if err = json.Unmarshal(raw, &copy); err != nil {
			t.Fatal(err)
		}
		expected := digest
		switch mode {
		case "candidate":
			copy.Candidate = "foreign"
		case "context":
			copy.ContextDigest = hash([]byte("foreign"))
		case "digest":
			expected = hash([]byte("foreign"))
		case "content":
			copy.Quotes[0].Text = "Another original has been substituted"
		case "offset":
			copy.Originals[0].Spans[0].Offset = 1
			expected = copy.digest()
		case "source hash":
			copy.Originals[0].Digest = hash([]byte("foreign"))
			expected = copy.digest()
		case "orphan":
			copy.Quotes = append(copy.Quotes, managedCitationQuote{ID: 1, Text: "No source supplies this fragment"})
			expected = copy.digest()
		case "quote id":
			copy.Quotes[0].ID = 1
			expected = copy.digest()
		}
		if _, err = copy.resolver(c.Candidate, c.ContextDigest, expected); err == nil {
			t.Fatal("foreign or malformed proof admitted", mode)
		}
	}
	r, err := c.resolver(c.Candidate, c.ContextDigest, digest)
	if err != nil {
		t.Fatal(err)
	}
	for _, id := range []int{-1, len(c.Quotes)} {
		if _, err = r.quote(id); err == nil {
			t.Fatal("unknown proof index accepted")
		}
	}
}

func TestCitationCatalogDeduplicatesTextAndRetainsAttribution(t *testing.T) {
	originals := []managedReviewFragmentArtifact{{Name: "a.go", Kind: "source", Content: "Shared original text\n"}, {Name: "control.txt", Kind: "control_output", Content: "Shared original text\n"}, {Name: "empty", Kind: "context", Content: ""}}
	c, err := buildManagedCitationCatalog("candidate", hash([]byte("context")), originals)
	if err != nil {
		t.Fatal(err)
	}
	again, err := buildManagedCitationCatalog(c.Candidate, c.ContextDigest, originals)
	if err != nil || c.digest() != again.digest() || len(c.Quotes) != 2 || len(c.Originals) != 3 || len(c.admissibleReferences()) != 1 {
		t.Fatal("catalog nondeterministic, duplicate or missing originals", err)
	}
	if c.Originals[0].Name == c.Originals[1].Name || c.Originals[0].Kind == c.Originals[1].Kind || c.Originals[0].Spans[0].Reference != c.Originals[1].Spans[0].Reference {
		t.Fatal("text deduplication lost source ownership")
	}
	r, err := c.resolver(c.Candidate, c.ContextDigest, c.digest())
	if err != nil {
		t.Fatal(err)
	}
	if _, err = r.quote(1); err == nil {
		t.Fatal("empty original turned into positive evidence")
	}
}

func TestStructuredCitationContractRequiresOriginalReferences(t *testing.T) {
	c := fragmentPlanFixture()
	c.CitationContractVersion = 2
	p, err := planManagedReviewFragments(c, 12, 2)
	if err != nil {
		t.Fatal("structured original references are required before another native review", err)
	}
	if _, err = managedFragmentRequestPrompt("", c, p, nil); err == nil {
		t.Fatal("incomplete inspections must still prevent a decision")
	}
	c, p, replies, _, _ := fragmentContractQuestionFromContext(t, c)
	prompt, _, err := managedFragmentDecisionPrompt("", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(prompt, "citation_catalog") || !strings.Contains(prompt, "catalog_sha256") {
		t.Fatal("final exchange still requires copying code and escaped output")
	}
}

func TestCitationCatalogSharesCapturedJSONWithoutLosingOriginals(t *testing.T) {
	output := strings.Repeat("PASS original tabs\tquoted \"output\"\\n and Unicode é界\n", 55)
	controls, _ := json.Marshal(map[string]any{"output": output, "exit": 0})
	receipt, _ := json.Marshal(map[string]any{"controls": json.RawMessage(controls), "candidate": "candidate"})
	originals := []managedReviewFragmentArtifact{{Name: "output.txt", Kind: "control_output", Content: output}, {Name: "controls.json", Kind: "context", Content: string(controls)}, {Name: "receipt.json", Kind: "context", Content: string(receipt)}}
	c, err := buildManagedCitationCatalog("candidate", hash([]byte("context")), originals)
	if err != nil {
		t.Fatal(err)
	}
	bytes := 0
	for _, q := range c.Quotes {
		bytes += len(q.Text)
	}
	if bytes > len(output)+512 {
		t.Fatal("captured JSON repeats the same original output", bytes, len(output))
	}
	if _, err = c.resolver(c.Candidate, c.ContextDigest, c.digest()); err != nil {
		t.Fatal("deduplication lost original bytes", err)
	}
}
