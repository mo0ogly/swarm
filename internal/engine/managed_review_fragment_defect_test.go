//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestFragmentV2ActionableRefusal(t *testing.T) {
	p, r := inspectionReplyFixture()
	p.Version = 2
	raw, _ := json.Marshal(p)
	r.PacketDigest = hash(raw)
	r.Findings[0].Verdict = "fail"
	defect := managedFragmentDefect{Artifact: 0, Line: 1, Quote: p.Artifacts[0].Content, Explanation: "The supplied fixture omits the expected guard in this branch.", Reproduction: "Call the branch with the documented invalid input.", Expected: "Reject invalid input."}
	for _, mode := range []string{"valid", "missing", "invented", "line", "short", "duplicate", "unrelated"} {
		t.Run(mode, func(t *testing.T) {
			reply := r
			reply.Defects = []managedFragmentDefect{defect}
			switch mode {
			case "missing":
				reply.Defects = nil
			case "invented":
				reply.Defects[0].Quote = "invented condition"
			case "line":
				reply.Defects[0].Line = 2
			case "short":
				reply.Defects[0].Explanation = "bug"
			case "duplicate":
				reply.Defects = append(reply.Defects, defect)
			case "unrelated":
				reply.Defects[0].Artifact = 1
			}
			data, _ := json.Marshal(reply)
			state, decoded, err := parseManagedFragmentInspection(string(data), p)
			if mode == "valid" {
				if err != nil || state != "changes_requested" || decoded.Defects[0].Explanation != defect.Explanation {
					t.Fatal(state, err)
				}
			} else if err == nil {
				t.Fatal("unproved refusal accepted")
			}
		})
	}
}

func TestFragmentV2SchemaAndPromptPreserveLegacy(t *testing.T) {
	p, _ := inspectionReplyFixture()
	old := managedFragmentPacketSchema(p)
	if strings.Contains(old, "reproduction") {
		t.Fatal("historical schema changed")
	}
	p.Version = 2
	schema := managedFragmentPacketSchema(p)
	if !strings.Contains(schema, "reproduction") || !strings.Contains(schema, "explanation") {
		t.Fatal("missing details")
	}
	prompt, err := managedFragmentInspectionPrompt("method", p)
	if err != nil || !strings.Contains(prompt, "PROTOCOLE V2") {
		t.Fatal(err)
	}
}

func TestFragmentDefectSourceLineUsesDecodedContents(t *testing.T) {
	content := "first source line\nif calls >= limit { return err }\n"
	source := ReviewSource{Path: "budget.go", Content: content, Bytes: len(content), Digest: hash([]byte(content))}
	raw, _ := json.Marshal(source)
	a := managedReviewFragmentArtifact{Kind: "source", Name: source.Path, Content: string(raw), Digest: hash(raw)}
	if fragmentDefectText(a) != content {
		t.Fatal("line would point to JSON transport")
	}
}

func TestFragmentDefectQuoteStartsWithinDeclaredLine(t *testing.T) {
	quote := "Correction externe : sorties originales attribuées jointes."
	content := "diff --git a/proof.json b/proof.json\n+  \"reason\": \"" + strings.Repeat("contexte ", 80) + quote + "\"\n+  \"state\": \"partial\"\n"
	for _, tc := range []struct {
		name, quote string
		line        int
		valid       bool
	}{
		{"inline Unicode on long JSON line", quote, 2, true},
		{"exact multiline", quote + "\"\n+  \"state\"", 2, true},
		{"wrong preceding line", quote, 1, false},
		{"wrong following line", quote, 3, false},
		{"invented quote", "Correction externe : preuve inventée.", 2, false},
		{"normalized text", "Correction externe: sorties originales attribuées jointes.", 2, false},
		{"line separator only", "\n+  \"state\": \"partial\"", 2, false},
		{"zero line", quote, 0, false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			p, r := inspectionReplyFixture()
			p.Version = 2
			p.Artifacts[0].Content = content
			p.Artifacts[0].Digest = hash([]byte(content))
			raw, _ := json.Marshal(p)
			r.PacketDigest = hash(raw)
			r.Findings[0].Digest = p.Artifacts[0].Digest
			r.Findings[0].Verdict = "fail"
			r.Defects = []managedFragmentDefect{{Artifact: 0, Line: tc.line, Quote: tc.quote, Explanation: "The supplied fixture omits the expected guard in this branch.", Reproduction: "Call the branch with the documented invalid input.", Expected: "Reject invalid input."}}
			reply, _ := json.Marshal(r)
			state, _, err := parseManagedFragmentInspection(string(reply), p)
			if tc.valid && (err != nil || state != "changes_requested") {
				t.Fatal(state, err)
			}
			if !tc.valid && err == nil {
				t.Fatal("unattributed quote accepted")
			}
		})
	}
}
