//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func journalFixture() (managedReviewFragmentPlan, managedFragmentJournal) {
	packet, reply := inspectionReplyFixture()
	pr, _ := json.Marshal(packet)
	rr, _ := json.Marshal(reply)
	p := managedReviewFragmentPlan{Version: 1, Candidate: packet.Candidate, ContextDigest: packet.ContextDigest, Packets: []managedReviewFragmentPacket{packet}, AvailableCalls: 4, ReservedFinalCalls: 2}
	raw, _ := json.Marshal(p)
	j := managedFragmentJournal{Version: 1, Attempt: "attempt", ProviderDigest: "provider", PlanDigest: hash(raw), Entries: []managedFragmentJournalEntry{{Packet: 0, PacketDigest: hash(pr), CallID: "call-1", State: "inspected", Reply: string(rr), ReplyDigest: hash(rr)}}}
	return p, j
}
func TestManagedFragmentJournalReadAfterRestart(t *testing.T) {
	p, j := journalFixture()
	raw, _ := json.Marshal(j)
	path := filepath.Join(t.TempDir(), "journal.json")
	if e := atomicWrite(path, raw); e != nil {
		t.Fatal(e)
	}
	_, reused, e := readManagedFragmentJournal(path, hash(raw), p, "attempt", "provider")
	if e != nil || len(reused) != 1 {
		t.Fatal(reused, e)
	}
	if _, _, e = readManagedFragmentJournal(path, "", p, "attempt", "provider"); e == nil {
		t.Fatal("unanchored")
	}
	if e = os.WriteFile(path, append(raw, ' '), 0600); e != nil {
		t.Fatal(e)
	}
	if _, _, e = readManagedFragmentJournal(path, hash(raw), p, "attempt", "provider"); e == nil {
		t.Fatal("tampered file")
	}
}
func TestManagedFragmentJournalRefusesDuplicateAndStale(t *testing.T) {
	for _, kind := range []string{"call", "packet", "reply", "state", "attempt", "provider", "plan"} {
		t.Run(kind, func(t *testing.T) {
			p, j := journalFixture()
			switch kind {
			case "call":
				j.Entries = append(j.Entries, j.Entries[0])
			case "packet":
				j.Entries[0].PacketDigest = "other"
			case "reply":
				j.Entries[0].Reply += "changed"
			case "state":
				j.Entries[0].State = "accepted"
			case "attempt":
				j.Attempt = "other"
			case "provider":
				j.ProviderDigest = "other"
			case "plan":
				p.Candidate = "other"
			}
			if _, e := validateManagedFragmentJournal(j, p, "attempt", "provider"); e == nil {
				t.Fatal("invalid journal")
			}
		})
	}
}
func TestManagedFragmentJournalInterruptedCallNotRefunded(t *testing.T) {
	p, j := journalFixture()
	completed := j.Entries[0]
	j.Entries[0].State = "interrupted"
	j.Entries[0].Reply = ""
	j.Entries[0].ReplyDigest = ""
	reused, e := validateManagedFragmentJournal(j, p, "attempt", "provider")
	if e != nil || len(reused) != 0 {
		t.Fatal(reused, e)
	}
	completed.CallID = "call-2"
	j.Entries = append(j.Entries, completed)
	reused, e = validateManagedFragmentJournal(j, p, "attempt", "provider")
	if e != nil || len(reused) != 1 || len(j.Entries) != 2 {
		t.Fatal(reused, e)
	}
	j.Entries[0].State = "reserved"
	if _, e = validateManagedFragmentJournal(j, p, "attempt", "provider"); e == nil {
		t.Fatal("live reservation reused")
	}
}
