//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func inspectionReplyFixture() (managedReviewFragmentPacket, managedFragmentInspection) {
	p := managedReviewFragmentPacket{Version: 1, Candidate: "candidate", ContextDigest: "context", Index: 0}
	for _, content := range []string{"original proof alpha", "original proof beta"} {
		p.Artifacts = append(p.Artifacts, managedReviewFragmentArtifact{Kind: "source", Name: content, Content: content, Digest: hash([]byte(content))})
	}
	raw, _ := json.Marshal(p)
	r := managedFragmentInspection{Candidate: p.Candidate, ContextDigest: p.ContextDigest, PacketDigest: hash(raw)}
	for i, a := range p.Artifacts {
		r.Findings = append(r.Findings, managedFragmentFinding{Artifact: i, Digest: a.Digest, Verdict: "inspected", Reason: "This artifact was examined", Evidence: a.Content, Needs: []string{}})
	}
	return p, r
}
func TestManagedFragmentInspectionNeverAcceptsTask(t *testing.T) {
	p, r := inspectionReplyFixture()
	raw, _ := json.Marshal(r)
	state, _, e := parseManagedFragmentInspection(string(raw), p)
	if e != nil || state != "inspected" {
		t.Fatal(state, e)
	}
	r.Findings[0].Verdict = "unknown"
	r.Findings[0].Needs = []string{"other.go"}
	raw, _ = json.Marshal(r)
	state, _, e = parseManagedFragmentInspection(string(raw), p)
	if e != nil || state != "unknown" {
		t.Fatal(state, e)
	}
	r.Findings[1].Verdict = "fail"
	raw, _ = json.Marshal(r)
	state, _, e = parseManagedFragmentInspection(string(raw), p)
	if e != nil || state != "changes_requested" {
		t.Fatal(state, e)
	}
}
func TestManagedFragmentInspectionRejectsFalseProof(t *testing.T) {
	for _, kind := range []string{"candidate", "context", "packet", "missing", "duplicate", "digest", "invented", "needs", "accepted", "oversized", "unknown-field"} {
		t.Run(kind, func(t *testing.T) {
			p, r := inspectionReplyFixture()
			switch kind {
			case "candidate":
				r.Candidate = "other"
			case "context":
				r.ContextDigest = "other"
			case "packet":
				r.PacketDigest = "other"
			case "missing":
				r.Findings = r.Findings[:1]
			case "duplicate":
				r.Findings[1] = r.Findings[0]
			case "digest":
				r.Findings[0].Digest = "other"
			case "invented":
				r.Findings[0].Evidence = "invented evidence"
			case "needs":
				r.Findings[0].Needs = []string{"needed.go"}
			case "accepted":
				r.Findings[0].Verdict = "accepted"
			case "oversized":
				r.Findings[0].Reason = strings.Repeat("x", managedFragmentReplyLimit)
			}
			raw, _ := json.Marshal(r)
			reply := string(raw)
			if kind == "unknown-field" {
				reply = strings.TrimSuffix(reply, "}") + `,"accepted":true}`
			}
			if _, _, e := parseManagedFragmentInspection(reply, p); e == nil {
				t.Fatal("invalid inspection accepted")
			}
		})
	}
}
