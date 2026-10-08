//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestFragmentRetryFeedbackIsDiagnosticOnly(t *testing.T) {
	s := storeTest(t)
	p, reply := inspectionReplyFixture()
	raw, _ := json.Marshal(p)
	r := IndependentReview{Context: ".swarm/review/context.json"}
	j := managedFragmentJournal{Entries: []managedFragmentJournalEntry{{Packet: p.Index, PacketDigest: hash(raw), CallID: "call-1", State: "interrupted"}}}
	reply.Findings[0].Evidence = "invented quote"
	raw, _ = json.Marshal(reply)
	path := filepath.Join(s.root, ".swarm/review", "call-1-raw-"+hash(raw)+".json")
	os.MkdirAll(filepath.Dir(path), 0700)
	os.WriteFile(path, raw, 0600)
	before, _ := json.Marshal(j)
	feedback := s.fragmentRetryFeedback(r, j, p)
	if !strings.Contains(feedback, "[0]") || strings.Contains(feedback, "invented quote") {
		t.Fatal(feedback)
	}
	after, _ := json.Marshal(j)
	if string(before) != string(after) {
		t.Fatal("diagnostic promoted to proof")
	}
	os.WriteFile(path, []byte("tampered"), 0600)
	if s.fragmentRetryFeedback(r, j, p) != "" {
		t.Fatal("tampered diagnostic trusted")
	}
}

func TestFragmentRetryFeedbackRejectsInventedOperatorInCompactReply(t *testing.T) {
	s := storeTest(t)
	original := "diff --git a/check.go b/check.go\n+\tif calls > maximum {\n+\t\treturn budgetError\n+\t}\n"
	a := managedReviewFragmentArtifact{Kind: "diff", Name: "check.go", Content: original, Digest: hash([]byte(original))}
	p := managedReviewFragmentPacket{Version: 2, Candidate: "candidate", ContextDigest: "context", Artifacts: []managedReviewFragmentArtifact{a}}
	packet, _ := json.Marshal(p)
	d := managedFragmentDefect{Artifact: 0, Line: 3, Quote: "+\tif calls >= maximum {", Explanation: "The claimed boundary condition incorrectly rejects equality.", Reproduction: "Use calls equal to the maximum permitted count.", Expected: "Accept exactly the authorized maximum."}
	reply := map[string]any{"candidate_commit": p.Candidate, "context_sha256": p.ContextDigest, "packet_sha256": hash(packet), "findings": map[string]any{"0": map[string]any{"v": "fail", "r": "Content examined", "e": 0, "n": []string{}}}, "defects": []managedFragmentDefect{d}}
	raw, _ := json.Marshal(reply)
	r := IndependentReview{Context: ".swarm/review/context.json"}
	j := managedFragmentJournal{Entries: []managedFragmentJournalEntry{{Packet: 0, PacketDigest: hash(packet), CallID: "bad-operator", State: "interrupted"}}}
	path := filepath.Join(s.root, ".swarm/review", "bad-operator-raw-"+hash(raw)+".json")
	os.MkdirAll(filepath.Dir(path), 0700)
	if err := os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	before, _ := json.Marshal(j)
	feedback := s.fragmentRetryFeedback(r, j, p)
	if !strings.Contains(feedback, "SWARM_REJECTED_DEFECT_DIAGNOSTIC") {
		t.Fatal("compact reply silently ignored", feedback)
	}
	parts := strings.Split(feedback, "SWARM_REJECTED_DEFECT_DIAGNOSTIC\n")
	var diagnostic []struct {
		Rejected string `json:"rejected_quote"`
		Lines    []struct {
			Line int    `json:"line"`
			Text string `json:"original_text"`
		} `json:"original_nearby_lines"`
	}
	if len(parts) != 2 || json.Unmarshal([]byte(parts[1]), &diagnostic) != nil || len(diagnostic) != 1 {
		t.Fatal("missing diagnostic")
	}
	if diagnostic[0].Rejected != d.Quote || len(diagnostic[0].Lines) < 2 || diagnostic[0].Lines[1].Text != "+\tif calls > maximum {" || diagnostic[0].Lines[1].Line != 2 {
		t.Fatal("original operator or line altered", feedback)
	}
	// Feedback must never turn the fabricated defect into an accepted finding.
	if _, _, err := parseManagedFragmentInspection(string(raw), p); err == nil {
		t.Fatal("fabricated defect accepted")
	}
	after, _ := json.Marshal(j)
	if string(before) != string(after) {
		t.Fatal("journal changed")
	}
	p.Candidate = "another-candidate"
	if s.fragmentRetryFeedback(r, j, p) != "" {
		t.Fatal("foreign diagnostic reused")
	}
}
