//go:build linux

package main

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
