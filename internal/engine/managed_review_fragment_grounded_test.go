//go:build linux

package engine

import (
	"encoding/json"
	"strconv"
	"testing"
)

func TestGroundedFragmentReply(t *testing.T) {
	for _, variant := range []string{"valid", "missing", "invented", "extra", "anchor", "fail", "unknown", "n", "candidate"} {
		t.Run(variant, func(t *testing.T) {
			p, r := inspectionReplyFixture()
			entries := map[string]any{}
			for i := range p.Artifacts {
				entries[strconv.Itoa(i)] = map[string]any{"v": "inspected", "r": "Examined content", "e": 0, "n": []string{}}
			}
			first := entries["0"].(map[string]any)
			switch variant {
			case "missing":
				delete(entries, "1")
			case "invented":
				first["e"] = "invented content"
			case "anchor":
				first["e"] = 99
			case "extra":
				entries["2"] = first
			case "fail", "unknown":
				first["v"] = variant
			case "n":
				first["n"] = []string{"missing dependency"}
			case "candidate":
				r.Candidate = "wrong"
			}
			raw, _ := json.Marshal(map[string]any{"candidate_commit": r.Candidate, "context_sha256": r.ContextDigest, "packet_sha256": r.PacketDigest, "findings": entries})
			state, decoded, err := parseManagedFragmentInspection(string(raw), p)
			valid := variant == "valid" || variant == "fail" || variant == "unknown"
			if (err == nil) != valid {
				t.Fatalf("%s %v", state, err)
			}
			if valid && (len(decoded.Findings) != 2 || decoded.Findings[0].Digest != p.Artifacts[0].Digest) {
				t.Fatal("identity lost")
			}
			if variant == "valid" && state != "inspected" {
				t.Fatal(state)
			}
			if variant == "fail" && state != "changes_requested" {
				t.Fatal(state)
			}
			if variant == "unknown" && state != "unknown" {
				t.Fatal(state)
			}
		})
	}
}

func TestGroundedFragmentWithoutUsableQuote(t *testing.T) {
	p, _ := inspectionReplyFixture()
	p.Artifacts[0].Content = "short"
	p.Artifacts[0].Digest = hash([]byte("short"))
	rawPacket, _ := json.Marshal(p)
	for _, verdict := range []string{"inspected", "unknown"} {
		entries := map[string]any{
			"0": map[string]any{"v": verdict, "r": "Missing excerpt", "e": "indisponible", "n": []string{}},
			"1": map[string]any{"v": "inspected", "r": "Content examined", "e": fragmentQuoteChoices(p.Artifacts[1].Content)[0], "n": []string{}},
		}
		raw, _ := json.Marshal(map[string]any{"candidate_commit": p.Candidate, "context_sha256": p.ContextDigest, "packet_sha256": hash(rawPacket), "findings": entries})
		state, _, err := parseManagedFragmentInspection(string(raw), p)
		if verdict == "inspected" && err == nil {
			t.Fatal("unanchored inspection")
		}
		if verdict == "unknown" && (err != nil || state != "unknown") {
			t.Fatal(state, err)
		}
	}
}
