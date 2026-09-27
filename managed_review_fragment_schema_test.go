//go:build linux

package main

import (
	"encoding/json"
	"strconv"
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
	if len(findings["required"].([]any)) != len(packet.Artifacts) {
		t.Fatal("incomplete inventory")
	}
	entries := findings["properties"].(map[string]any)
	for i, a := range packet.Artifacts {
		item := entries[strconv.Itoa(i)].(map[string]any)["properties"].(map[string]any)
		for _, quote := range item["e"].(map[string]any)["enum"].([]any) {
			if !strings.Contains(a.Content, quote.(string)) {
				t.Fatal("invented anchor")
			}
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
