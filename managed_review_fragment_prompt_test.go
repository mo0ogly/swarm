//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestManagedFragmentPromptBoundAndIdentity(t *testing.T) {
	p, _ := inspectionReplyFixture()
	raw, _ := json.Marshal(p)
	prompt, e := managedFragmentInspectionPrompt("Reviewer workflow", p)
	if e != nil || !strings.Contains(prompt, string(raw)) || !strings.Contains(prompt, "packet_sha256="+hash(raw)) {
		t.Fatal("packet not exact", e)
	}
	var schema map[string]any
	if e = json.Unmarshal([]byte(managedFragmentInspectionSchema), &schema); e != nil {
		t.Fatal(e)
	}
	if _, e = managedFragmentInspectionPrompt(strings.Repeat("x", managedReviewPromptLimit), p); e == nil {
		t.Fatal("oversized instructions")
	}
	p.Artifacts[0].Content += "changed"
	if _, e = managedFragmentInspectionPrompt("", p); e == nil {
		t.Fatal("tampered artifact")
	}
}
