//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func nestedDefectFixture() (managedReviewFragmentPacket, map[string]any) {
	p := managedReviewFragmentPacket{Version: 2, FindingContractVersion: 2, DefectCitationContractVersion: 1, Candidate: strings.Repeat("a", 40), ContextDigest: strings.Repeat("b", 64)}
	for _, text := range []string{"first original source\nunsafeOperation()\n", "second original source\nsafeOperation()\n"} {
		p.Artifacts = append(p.Artifacts, managedReviewFragmentArtifact{Kind: "diff", Content: text, Digest: hash([]byte(text))})
	}
	raw, _ := json.Marshal(p)
	return p, map[string]any{"candidate_commit": p.Candidate, "context_sha256": p.ContextDigest, "packet_sha256": hash(raw), "findings": map[string]any{
		"0": map[string]any{"v": "fail", "r": "Unsafe operation", "e": 0, "d": []any{map[string]any{"quote": "unsafeOperation()", "explanation": "The operation accepts the invalid fixture without checking its authority.", "reproduction": "Run the isolated fixture with an unauthorized input.", "expected": "Reject the unauthorized input."}}},
		"1": map[string]any{"v": "inspected", "r": "Source inspected", "e": 0},
	}}
}

func marshalFragmentInspectionFixture(packet managedReviewFragmentPacket, inspection managedFragmentInspection) ([]byte, error) {
	if packet.FindingContractVersion < 2 {
		return json.Marshal(inspection)
	}
	findings := map[string]any{}
	for _, f := range inspection.Findings {
		entry := map[string]any{"v": f.Verdict, "r": f.Reason, "e": 0}
		if f.Verdict == "unknown" {
			entry["n"] = f.Needs
		}
		if f.Verdict == "fail" {
			claims := []managedNestedDefectClaim{}
			for _, d := range inspection.Defects {
				if d.Artifact == f.Artifact {
					claims = append(claims, managedNestedDefectClaim{d.Quote, d.Explanation, d.Reproduction, d.Expected})
				}
			}
			entry["d"] = claims
		}
		findings[fmt.Sprint(f.Artifact)] = entry
	}
	return json.Marshal(map[string]any{"candidate_commit": inspection.Candidate, "context_sha256": inspection.ContextDigest, "packet_sha256": inspection.PacketDigest, "findings": findings})
}

func TestFragmentNestedDefectBinding(t *testing.T) {
	p, reply := nestedDefectFixture()
	raw, _ := json.Marshal(reply)
	state, inspection, err := parseManagedFragmentInspection(string(raw), p)
	if err != nil || state != "changes_requested" || len(inspection.Defects) != 1 || inspection.Defects[0].Artifact != 0 || inspection.Defects[0].Line != 2 || inspection.Findings[0].Verdict != "fail" {
		t.Fatalf("nested independent claim not bound to its failed finding: %s %+v %v", state, inspection, err)
	}
	if inspection.Defects[0].Explanation != reply["findings"].(map[string]any)["0"].(map[string]any)["d"].([]any)[0].(map[string]any)["explanation"] {
		t.Fatal("reviewer opinion changed")
	}
}

func TestFragmentNestedDefectInvalidBoundaries(t *testing.T) {
	for _, variant := range []string{"inspected defect", "unknown defect", "fail without defect", "fail with needs", "inspected with empty needs", "unknown without needs", "global defects", "global null", "foreign artifact", "line supplied", "string anchor", "missing anchor", "altered quote", "duplicate claim", "too many claims", "array findings"} {
		t.Run(variant, func(t *testing.T) {
			p, reply := nestedDefectFixture()
			findings := reply["findings"].(map[string]any)
			first := findings["0"].(map[string]any)
			defect := first["d"].([]any)[0].(map[string]any)
			switch variant {
			case "inspected defect":
				first["v"] = "inspected"
			case "unknown defect":
				first["v"] = "unknown"
				first["n"] = []string{"Provide the original authority check"}
			case "fail without defect":
				delete(first, "d")
			case "fail with needs":
				first["n"] = []string{}
			case "inspected with empty needs":
				findings["1"].(map[string]any)["n"] = []string{}
			case "unknown without needs":
				first["v"] = "unknown"
				delete(first, "d")
				first["n"] = []string{}
			case "global defects":
				reply["defects"] = []any{}
			case "global null":
				reply["defects"] = nil
			case "foreign artifact":
				defect["artifact"] = 1
			case "line supplied":
				defect["line"] = 2
			case "string anchor":
				first["e"] = fragmentQuoteChoices(p.Artifacts[0].Content)[0]
			case "missing anchor":
				delete(first, "e")
			case "altered quote":
				defect["quote"] = "unsafeOperation(true)"
			case "duplicate claim":
				first["d"] = []any{defect, defect}
			case "too many claims":
				first["d"] = []any{defect, defect, defect, defect, defect}
			case "array findings":
				reply["findings"] = []any{}
			}
			raw, _ := json.Marshal(reply)
			if _, _, err := parseManagedFragmentInspection(string(raw), p); err == nil || err.(*CommandError).Code != reviewFragmentProtocolError {
				t.Fatalf("invalid response admitted or not routed: %v", err)
			}
		})
	}
}

func TestFragmentNestedDefectSchemaOracle(t *testing.T) {
	p, reply := nestedDefectFixture()
	schema := managedFragmentPacketSchema(p)
	var value map[string]any
	if err := json.Unmarshal([]byte(schema), &value); err != nil {
		t.Fatal(err)
	}
	if _, exists := value["properties"].(map[string]any)["defects"]; exists {
		t.Fatal("global defects still representable")
	}
	unknown := map[string]any{"v": "unknown", "r": "Missing evidence", "e": 0, "n": []string{"Provide the authority original"}}
	reply["findings"].(map[string]any)["1"] = unknown
	raw, _ := json.Marshal(reply)
	if state, _, err := parseManagedFragmentInspection(string(raw), p); err != nil || state != "changes_requested" {
		t.Fatalf("mixed opinions lost: %s %v", state, err)
	}
	if output := os.Getenv("SWARM_NESTED_SCHEMA_ORACLE_DIR"); output != "" {
		if err := os.MkdirAll(output, 0700); err != nil {
			t.Fatal(err)
		}
		for name, data := range map[string][]byte{"schema.json": []byte(schema), "reply.json": raw} {
			if err := os.WriteFile(filepath.Join(output, name), data, 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
}

func TestFragmentNestedDefectLegacyFormatsStayReadable(t *testing.T) {
	for _, version := range []int{0, 1} {
		p, reply := nestedDefectFixture()
		p.FindingContractVersion = version
		raw, _ := json.Marshal(p)
		reply["packet_sha256"] = hash(raw)
		first := reply["findings"].(map[string]any)["0"].(map[string]any)
		claim := first["d"].([]any)[0].(map[string]any)
		claim["artifact"] = 0
		reply["defects"] = []any{claim}
		delete(first, "d")
		first["n"] = []string{}
		if version == 0 {
			reply["findings"].(map[string]any)["1"].(map[string]any)["n"] = []string{}
		}
		raw, _ = json.Marshal(reply)
		if state, _, err := parseManagedFragmentInspection(string(raw), p); err != nil || state != "changes_requested" {
			t.Fatalf("historical %d rejected: %s %v", version, state, err)
		}
		first["v"] = "inspected"
		raw, _ = json.Marshal(reply)
		if _, _, err := parseManagedFragmentInspection(string(raw), p); err == nil {
			t.Fatal("historical contradiction silently repaired")
		}
		prompt, err := managedFragmentInspectionPrompt("", p)
		if err != nil || strings.Contains(prompt, "CONTRAT DE CONSTAT2") {
			t.Fatalf("historical prompt changed: %v", err)
		}
	}
}

func TestFragmentNestedDefectRejectedCitationRemainsDiagnostic(t *testing.T) {
	p, reply := nestedDefectFixture()
	reply["findings"].(map[string]any)["0"].(map[string]any)["d"].([]any)[0].(map[string]any)["quote"] = "unsafeOperation(true)"
	raw, _ := json.Marshal(reply)
	packet, _ := json.Marshal(p)
	feedback := fragmentRejectedCitationFeedback(raw, p, hash(packet))
	if !strings.Contains(feedback, "quote absent") || !strings.Contains(feedback, "unsafeOperation(true)") {
		t.Fatal("original rejected quote not returned to owner", feedback)
	}
	if _, _, err := parseManagedFragmentInspection(string(raw), p); err == nil {
		t.Fatal("diagnostic changed original opinion")
	}
	p.ContextDigest = strings.Repeat("c", 64)
	if fragmentRejectedCitationFeedback(raw, p, hash(packet)) != "" {
		t.Fatal("foreign diagnostic admitted")
	}
}
