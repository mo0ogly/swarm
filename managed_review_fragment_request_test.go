//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestManagedFragmentFinalRequestBindings(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	raw, _ := json.Marshal(p)
	valid := managedFragmentFinalRequest{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw), State: "ready", Reason: "Original pieces selected for cross-file analysis", References: []managedFragmentEvidenceRef{{0, 0, p.Packets[0].Artifacts[0].Digest}}}
	for _, mode := range []string{"valid", "candidate", "context", "plan", "duplicate", "digest", "state", "large", "unknown"} {
		t.Run(mode, func(t *testing.T) {
			r := valid
			r.References = append([]managedFragmentEvidenceRef(nil), valid.References...)
			prefix := ""
			switch mode {
			case "candidate":
				r.Candidate = "other"
			case "context":
				r.ContextDigest = "other"
			case "plan":
				r.PlanDigest = "other"
			case "duplicate":
				r.References = append(r.References, r.References[0])
			case "digest":
				r.References[0].Digest = "other"
			case "state":
				r.State = "passed"
			case "large":
				prefix = strings.Repeat("x", managedReviewPromptLimit)
			case "unknown":
				r.State = "unknown"
			}
			raw, _ := json.Marshal(r)
			got, err := parseManagedFragmentFinalRequest(string(raw), prefix, c, p, replies)
			if mode == "valid" || mode == "unknown" {
				if err != nil || got.State != r.State {
					t.Fatal(got, err)
				}
			} else if err == nil {
				t.Fatal("invalid final request accepted")
			}
		})
	}
	raw, _ = json.Marshal(valid)
	if _, err := parseManagedFragmentFinalRequest(string(raw), "", c, p, replies[:len(replies)-1]); err == nil {
		t.Fatal("partial inspections authorized request")
	}
}
