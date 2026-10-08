//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func finalBundleFixture(t *testing.T) (managedReviewContext, managedReviewFragmentPlan, []string) {
	t.Helper()
	c := fragmentPlanFixture()
	p, e := planManagedReviewFragments(c, 12, 2)
	if e != nil {
		t.Fatal(e)
	}
	replies := []string{}
	for _, packet := range p.Packets {
		raw, _ := json.Marshal(packet)
		r := managedFragmentInspection{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PacketDigest: hash(raw)}
		for i, a := range packet.Artifacts {
			excerpt := a.Content
			if len(excerpt) > 40 {
				excerpt = excerpt[:40]
			}
			r.Findings = append(r.Findings, managedFragmentFinding{Artifact: i, Digest: a.Digest, Verdict: "inspected", Reason: "Independent artifact inspection", Evidence: excerpt, Needs: []string{}})
		}
		raw, _ = json.Marshal(r)
		replies = append(replies, string(raw))
	}
	return c, p, replies
}
func TestManagedFragmentFinalBundlePreservesOriginalEvidence(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	b, e := managedFragmentFinalBundle(c, p, replies)
	if e != nil {
		t.Fatal(e)
	}
	n := 0
	for _, packet := range p.Packets {
		n += len(packet.Artifacts)
	}
	if len(b.Evidence) != n || len(b.ReplyDigests) != len(replies) {
		t.Fatal("coverage lost")
	}
	for _, v := range b.Evidence {
		a := p.Packets[v.Packet].Artifacts[v.Artifact]
		if !strings.Contains(a.Content, v.Excerpt) || v.Digest != a.Digest {
			t.Fatal("non-original proof")
		}
	}
}
func TestManagedFragmentFinalBundleRefusesPartialOrUnknown(t *testing.T) {
	for _, kind := range []string{"missing", "unknown", "fail", "tampered", "other-candidate"} {
		t.Run(kind, func(t *testing.T) {
			c, p, replies := finalBundleFixture(t)
			switch kind {
			case "missing":
				replies = replies[:len(replies)-1]
			case "unknown", "fail":
				var r managedFragmentInspection
				json.Unmarshal([]byte(replies[0]), &r)
				r.Findings[0].Verdict = kind
				raw, _ := json.Marshal(r)
				replies[0] = string(raw)
			case "tampered":
				replies[0] = "{}"
			case "other-candidate":
				c.Candidate = "other"
			}
			if _, e := managedFragmentFinalBundle(c, p, replies); e == nil {
				t.Fatal("unsafe bundle")
			}
		})
	}
}
