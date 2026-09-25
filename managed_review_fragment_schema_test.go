//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestManagedFragmentPacketSchemaBoundToInventory(t *testing.T) {
	packet, _ := inspectionReplyFixture()
	schema := managedFragmentPacketSchema(packet)
	var parsed map[string]any
	if e := json.Unmarshal([]byte(schema), &parsed); e != nil {
		t.Fatal(e)
	}
	props := parsed["properties"].(map[string]any)
	raw, _ := json.Marshal(packet)
	for k, want := range map[string]string{"candidate_commit": packet.Candidate, "context_sha256": packet.ContextDigest, "packet_sha256": hash(raw)} {
		got := props[k].(map[string]any)["enum"].([]any)
		if len(got) != 1 || got[0] != want {
			t.Fatal(k, got)
		}
	}
	findings := props["findings"].(map[string]any)
	if findings["minItems"] != float64(len(packet.Artifacts)) || findings["maxItems"] != float64(len(packet.Artifacts)) {
		t.Fatal("incomplete result still allowed")
	}
	item := findings["items"].(map[string]any)["properties"].(map[string]any)
	if item["artifact"].(map[string]any)["maximum"] != float64(len(packet.Artifacts)-1) {
		t.Fatal("unbounded index")
	}
	allowed := item["sha256"].(map[string]any)["enum"].([]any)
	for _, a := range packet.Artifacts {
		found := false
		for _, v := range allowed {
			found = found || v == a.Digest
		}
		if !found {
			t.Fatal("missing identity")
		}
	}
	if schema != managedFragmentPacketSchema(packet) {
		t.Fatal("unstable schema")
	}
}
func TestManagedFragmentPromptCountsBoundSchema(t *testing.T) {
	p, _ := inspectionReplyFixture()
	prompt, e := managedFragmentInspectionPrompt("", p)
	if e != nil {
		t.Fatal(e)
	}
	// A prefix that fits with the old generic schema must fail with the actual one.
	n := managedReviewPromptLimit - len(prompt) - len(managedFragmentInspectionSchema)
	if _, e = managedFragmentInspectionPrompt(strings.Repeat("x", n), p); e == nil {
		t.Fatal("packet schema omitted from size preflight")
	}
}
