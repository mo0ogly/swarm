//go:build linux

package engine

import "encoding/json"

type managedCitationSegment struct {
	Text string
	JSON bool
}

func managedCitationJSONString(text string) string {
	raw, _ := json.Marshal(text)
	return string(raw[1 : len(raw)-1])
}

func managedCitationSegments(text string) []managedCitationSegment {
	if len(text) <= managedCitationChunkBytes || !json.Valid([]byte(text)) {
		return []managedCitationSegment{{Text: text}}
	}
	segments := []managedCitationSegment{}
	last := 0
	for i := 0; i < len(text); i++ {
		if text[i] != '"' {
			continue
		}
		start := i
		i++
		for i < len(text) && text[i] != '"' {
			if text[i] == '\\' {
				i++
			}
			i++
		}
		var decoded string
		raw := text[start : i+1]
		if json.Unmarshal([]byte(raw), &decoded) != nil || "\""+managedCitationJSONString(decoded)+"\"" != raw {
			continue
		}
		segments = append(segments, managedCitationSegment{Text: text[last : start+1]}, managedCitationSegment{Text: decoded, JSON: true})
		last = i
	}
	return append(segments, managedCitationSegment{Text: text[last:]})
}

func managedCitationSpanText(quote managedCitationQuote, span managedCitationSpan) string {
	if span.JSON {
		return managedCitationJSONString(quote.Text)
	}
	return quote.Text
}

func (c managedCitationCatalog) literalReferences() []bool {
	literal := make([]bool, len(c.Quotes))
	for _, original := range c.Originals {
		for _, span := range original.Spans {
			if span.Reference >= 0 && span.Reference < len(c.Quotes) {
				quote := c.Quotes[span.Reference]
				if !span.JSON || managedCitationSpanText(quote, span) == quote.Text {
					literal[span.Reference] = true
				}
			}
		}
	}
	return literal
}
