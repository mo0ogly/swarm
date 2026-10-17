//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strings"
	"unicode/utf8"
)

const managedCitationChunkBytes = 768

type managedCitationSpan struct {
	JSON      bool `json:"json_string,omitempty"`
	Reference int  `json:"reference"`
	Offset    int  `json:"byte_offset"`
	Bytes     int  `json:"bytes"`
}

type managedCitationOriginal struct {
	Name   string                `json:"name"`
	Kind   string                `json:"kind"`
	Digest string                `json:"sha256"`
	Bytes  int                   `json:"bytes"`
	Spans  []managedCitationSpan `json:"spans"`
}

type managedCitationQuote struct {
	ID   int    `json:"id"`
	Text string `json:"text"`
}

type managedCitationCatalog struct {
	Version       int                       `json:"version"`
	Candidate     string                    `json:"candidate_commit"`
	ContextDigest string                    `json:"context_sha256"`
	Originals     []managedCitationOriginal `json:"originals"`
	Quotes        []managedCitationQuote    `json:"quotes"`
}

type managedCitationResolver struct {
	quotes  []string
	literal []bool
}

func splitManagedCitationText(text string) []string {
	if text == "" {
		return []string{""}
	}
	parts := []string{}
	for len(text) > managedCitationChunkBytes {
		end := managedCitationChunkBytes
		for end > 0 && !utf8.RuneStart(text[end]) {
			end--
		}
		if len(text)-end < 8 {
			break
		}
		parts = append(parts, text[:end])
		text = text[end:]
	}
	return append(parts, text)
}

func buildManagedCitationCatalog(candidate, contextDigest string, originals []managedReviewFragmentArtifact) (managedCitationCatalog, error) {
	empty := managedCitationCatalog{}
	if candidate == "" || !citationHashValid(contextDigest) {
		return empty, fmt.Errorf("identité du catalogue de preuves invalide")
	}
	c := managedCitationCatalog{Version: 1, Candidate: candidate, ContextDigest: contextDigest, Originals: []managedCitationOriginal{}, Quotes: []managedCitationQuote{}}
	indexes := map[string]int{}
	for _, original := range originals {
		if original.Name == "" || original.Kind == "" || !utf8.ValidString(original.Name) || !utf8.ValidString(original.Kind) || !utf8.ValidString(original.Content) {
			return empty, fmt.Errorf("original de catalogue invalide")
		}
		entry := managedCitationOriginal{Name: original.Name, Kind: original.Kind, Digest: hash([]byte(original.Content)), Bytes: len(original.Content), Spans: []managedCitationSpan{}}
		offset := 0
		for _, segment := range managedCitationSegments(original.Content) {
			for _, text := range splitManagedCitationText(segment.Text) {
				id, present := indexes[text]
				if !present {
					id = len(c.Quotes)
					indexes[text] = id
					c.Quotes = append(c.Quotes, managedCitationQuote{ID: id, Text: text})
				}
				span := managedCitationSpan{Reference: id, Offset: offset, JSON: segment.JSON}
				span.Bytes = len(managedCitationSpanText(c.Quotes[id], span))
				entry.Spans = append(entry.Spans, span)
				offset += span.Bytes
			}
		}
		c.Originals = append(c.Originals, entry)
	}
	return c, nil
}

func (c managedCitationCatalog) digest() string {
	raw, _ := json.Marshal(c)
	return hash(raw)
}

func (c managedCitationCatalog) resolver(candidate, contextDigest, expectedDigest string) (*managedCitationResolver, error) {
	if c.Version != 1 || candidate == "" || c.Candidate != candidate || c.ContextDigest != contextDigest || !citationHashValid(contextDigest) || !citationHashValid(expectedDigest) || c.digest() != expectedDigest {
		return nil, fmt.Errorf("catalogue de preuves périmé ou non attribué")
	}
	quotes := make([]string, len(c.Quotes))
	for i, q := range c.Quotes {
		if q.ID != i || !utf8.ValidString(q.Text) || len(q.Text) > managedCitationChunkBytes+7 {
			return nil, fmt.Errorf("référence de catalogue invalide")
		}
		quotes[i] = q.Text
	}
	used := make([]bool, len(quotes))
	for _, original := range c.Originals {
		if original.Name == "" || original.Kind == "" || !utf8.ValidString(original.Name) || !utf8.ValidString(original.Kind) || !citationHashValid(original.Digest) || original.Bytes < 0 || len(original.Spans) == 0 {
			return nil, fmt.Errorf("original de catalogue non attribué")
		}
		var text strings.Builder
		for _, span := range original.Spans {
			if span.Reference < 0 || span.Reference >= len(quotes) || span.Offset != text.Len() {
				return nil, fmt.Errorf("plage originale manquante ou modifiée")
			}
			originalText := managedCitationSpanText(c.Quotes[span.Reference], span)
			if len(originalText) != span.Bytes {
				return nil, fmt.Errorf("plage originale encodée modifiée")
			}
			used[span.Reference] = true
			text.WriteString(originalText)
		}
		if text.Len() != original.Bytes || hash([]byte(text.String())) != original.Digest {
			return nil, fmt.Errorf("original de catalogue modifié")
		}
	}
	for _, present := range used {
		if !present {
			return nil, fmt.Errorf("référence de catalogue sans original")
		}
	}
	return &managedCitationResolver{quotes: quotes, literal: c.literalReferences()}, nil
}

func (r *managedCitationResolver) quote(reference int) (string, error) {
	if r == nil || reference < 0 || reference >= len(r.quotes) {
		return "", fmt.Errorf("référence de preuve inconnue")
	}
	text := r.quotes[reference]
	if !r.literal[reference] {
		text = managedCitationJSONString(text)
	}
	if len(strings.TrimSpace(text)) < 8 || len(text) > 1000 {
		return "", fmt.Errorf("référence sans extrait original admissible")
	}
	return text, nil
}

func (c managedCitationCatalog) admissibleReferences() []int {
	refs := []int{}
	literal := c.literalReferences()
	for i, quote := range c.Quotes {
		text := quote.Text
		if !literal[i] {
			text = managedCitationJSONString(text)
		}
		if len(strings.TrimSpace(text)) >= 8 && len(text) <= 1000 {
			refs = append(refs, quote.ID)
		}
	}
	return refs
}
