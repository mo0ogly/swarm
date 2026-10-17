//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func unicodeInspectionFixture(t *testing.T, version int, needs []string) (managedReviewFragmentPacket, string) {
	t.Helper()
	p, r := nestedDefectFixture()
	raw, _ := json.Marshal(p)
	var obj map[string]any
	if err := json.Unmarshal(raw, &obj); err != nil {
		t.Fatal(err)
	}
	obj["inspection_contract_version"] = version
	raw, _ = json.Marshal(obj)
	if err := json.Unmarshal(raw, &p); err != nil {
		t.Fatal(err)
	}
	raw, _ = json.Marshal(p)
	r["packet_sha256"] = hash(raw)
	r["findings"].(map[string]any)["0"] = map[string]any{"v": "unknown", "r": "Proof is missing", "e": 0, "n": needs}
	raw, _ = json.Marshal(r)
	return p, string(raw)
}

func TestFragmentUnicodeNeedsMatchesPublishedSchema(t *testing.T) {
	for _, tc := range []struct {
		name, need string
		valid      bool
	}{
		{"accent boundary", strings.Repeat("é", 240), true},
		{"astral boundary", strings.Repeat("🙂", 240), true},
		{"minimum", "é🙂a", true},
		{"overlong", strings.Repeat("é", 241), false},
		{"underlength", "éa", false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			p, reply := unicodeInspectionFixture(t, 1, []string{tc.need})
			state, _, err := parseManagedFragmentInspection(reply, p)
			if tc.valid && (err != nil || state != "unknown") {
				t.Fatal("schema-valid Unicode need rejected", state, err)
			}
			if !tc.valid && err == nil {
				t.Fatal("need outside schema admitted")
			}
		})
	}
	p, reply := unicodeInspectionFixture(t, 1, []string{"Same needed original", "Same needed original"})
	if _, _, err := parseManagedFragmentInspection(reply, p); err == nil {
		t.Fatal("duplicate need admitted")
	}
	p, reply = unicodeInspectionFixture(t, 1, []string{"Needed original proof"})
	invalid := strings.Replace(reply, "Proof is missing", "Proof \xff missing", 1)
	if invalid == reply {
		t.Fatal("invalid UTF-8 fixture unchanged")
	}
	if _, _, err := parseManagedFragmentInspection(invalid, p); err == nil || !strings.Contains(err.Error(), "UTF-8") {
		t.Fatal("invalid UTF-8 admitted", err)
	}
	prompt, err := managedFragmentInspectionPrompt("", p)
	if err != nil || !strings.Contains(prompt, "ne signifie pas delta Git vide") || strings.Contains(prompt, "scope.changed_sections") {
		t.Fatal("transport guidance requires an absent scope", err)
	}
}

func TestFragmentUnicodeNeedsPreservesHistoricalWire(t *testing.T) {
	p, reply := unicodeInspectionFixture(t, 0, []string{strings.Repeat("é", 240)})
	if _, _, err := parseManagedFragmentInspection(reply, p); err == nil {
		t.Fatal("historical byte boundary changed")
	}
	p, reply = unicodeInspectionFixture(t, 0, []string{"éa"})
	if state, _, err := parseManagedFragmentInspection(reply, p); err != nil || state != "unknown" {
		t.Fatal("historical minimum changed", err)
	}
	p, reply = unicodeInspectionFixture(t, 999, []string{"Needed original"})
	if _, _, err := parseManagedFragmentInspection(reply, p); err == nil {
		t.Fatal("unknown inspection contract admitted")
	}
	if _, err := managedFragmentInspectionPrompt("", p); err == nil {
		t.Fatal("unknown contract reached prompt construction")
	}
}
