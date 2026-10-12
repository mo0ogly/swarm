//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func nativeInspectedNeedsFixture(t *testing.T) (managedReviewFragmentPacket, []byte) {
	t.Helper()
	var packet managedReviewFragmentPacket
	raw, err := os.ReadFile("testdata/fragment-inspected-needs-native-packet.json")
	if err != nil {
		t.Fatal(err)
	}
	if err = json.Unmarshal(raw, &packet); err != nil {
		t.Fatal(err)
	}
	reply, err := os.ReadFile("testdata/fragment-inspected-needs-native-reply.json")
	if err != nil {
		t.Fatal(err)
	}
	if hash(reply) != "4e59aff7aa67c15e56c29148d1de159f17e39f78fa8f16da2cc327599ba198d3" {
		t.Fatal("original native reply changed")
	}
	return packet, reply
}

func rebindFindingFixture(t *testing.T, packet managedReviewFragmentPacket, original []byte, repair bool) map[string]any {
	t.Helper()
	var reply map[string]any
	if err := json.Unmarshal(original, &reply); err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(packet)
	reply["packet_sha256"] = hash(raw)
	if repair {
		for _, value := range reply["findings"].(map[string]any) {
			finding := value.(map[string]any)
			if finding["v"] == "inspected" && len(finding["n"].([]any)) > 0 {
				finding["v"] = "unknown"
			}
		}
	}
	return reply
}

func TestFragmentFindingContractNativeInspectedNeeds(t *testing.T) {
	packet, original := nativeInspectedNeedsFixture(t)
	var decoded managedFragmentInspection
	if err := decodeManagedFragmentInspection(original, packet, &decoded); err != nil {
		t.Fatal(err)
	}
	contradictions := 0
	for _, finding := range decoded.Findings {
		if finding.Verdict == "inspected" {
			if !fragmentEvidencePresent(packet.Artifacts[finding.Artifact], finding.Evidence) {
				t.Fatal("native anchor was not present")
			}
			if len(finding.Needs) == 0 {
				t.Fatal("counterexample no longer requests proofs")
			}
			contradictions++
		}
	}
	if contradictions != 5 {
		t.Fatalf("got %d contradictory inspected findings", contradictions)
	}
	if _, _, err := parseManagedFragmentInspection(string(original), packet); err == nil || !strings.Contains(err.Error(), "inspected incompatible") {
		t.Fatalf("native contradiction admitted or obscured: %v", err)
	}

	current := packet
	current.FindingContractVersion = managedFragmentFindingContractVersion
	uncertain := rebindFindingFixture(t, current, original, true)
	encoded, _ := json.Marshal(uncertain)
	state, _, err := parseManagedFragmentInspection(string(encoded), current)
	if err != nil || state != "unknown" {
		t.Fatalf("uncertainty must remain unknown: %s %v", state, err)
	}
	current.FindingContractVersion++
	if _, _, err = parseManagedFragmentInspection(string(encoded), current); err == nil {
		t.Fatal("unregistered finding contract accepted")
	}
}

// Export the actual engine-produced schemas and original counterexample for an
// independent JSON Schema validator during qualification. This fixture export
// never mutates native proofs or creates an independent business verdict.
func TestFragmentFindingContractSchemaOracle(t *testing.T) {
	packet, original := nativeInspectedNeedsFixture(t)
	legacySchema := managedFragmentPacketSchema(packet)
	current := packet
	current.FindingContractVersion = managedFragmentFindingContractVersion
	currentSchema := managedFragmentPacketSchema(current)
	if legacySchema == currentSchema {
		t.Fatal("finding contract has no effect on schema")
	}
	var schema map[string]any
	if err := json.Unmarshal([]byte(currentSchema), &schema); err != nil {
		t.Fatal(err)
	}
	defs := schema["$defs"].(map[string]any)
	for _, name := range []string{"finding-complete-1", "finding-pending-1"} {
		object := defs[name].(map[string]any)
		props := object["properties"].(map[string]any)
		_, needs := props["n"]
		if needs != strings.Contains(name, "pending") || object["additionalProperties"] != false {
			t.Fatalf("invalid state boundary %s", name)
		}
	}
	bad := rebindFindingFixture(t, current, original, false)
	uncertain := rebindFindingFixture(t, current, original, true)
	mixed := rebindFindingFixture(t, current, original, true)
	first := mixed["findings"].(map[string]any)["0"].(map[string]any)
	first["v"] = "inspected"
	delete(first, "n")
	encoded, _ := json.Marshal(mixed)
	if state, _, err := parseManagedFragmentInspection(string(encoded), current); err != nil || state != "unknown" {
		t.Fatalf("valid mixed local outcomes rejected: %s %v", state, err)
	}
	output := os.Getenv("SWARM_FINDING_SCHEMA_ORACLE_DIR")
	if output != "" {
		if err := os.MkdirAll(output, 0700); err != nil {
			t.Fatal(err)
		}
		cases := map[string]any{"legacy-schema.json": json.RawMessage(legacySchema), "current-schema.json": json.RawMessage(currentSchema), "original-native-reply.json": json.RawMessage(original), "current-contradiction.json": bad, "current-unknown.json": uncertain, "current-mixed.json": mixed}
		for name, value := range cases {
			raw, err := json.MarshalIndent(value, "", "  ")
			if err != nil {
				t.Fatal(err)
			}
			if err = os.WriteFile(filepath.Join(output, name), raw, 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
}
