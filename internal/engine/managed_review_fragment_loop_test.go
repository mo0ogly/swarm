//go:build linux

package engine

import (
	"encoding/json"
	"path/filepath"
	"strings"
	"testing"
)

func TestRepeatedFragmentInterruptionDoesNotRefundOrForget(t *testing.T) {
	for _, tc := range []struct {
		states  []managedFragmentJournalEntry
		blocked bool
	}{
		{[]managedFragmentJournalEntry{{Packet: 0, State: "interrupted"}}, false},
		{[]managedFragmentJournalEntry{{Packet: 0, State: "interrupted"}, {Packet: 1, State: "interrupted"}}, false},
		{[]managedFragmentJournalEntry{{Packet: 0, State: "interrupted"}, {Packet: 0, State: "interrupted"}}, true},
	} {
		j := managedFragmentJournal{Entries: tc.states}
		before := len(j.Entries)
		if _, blocked := repeatedFragmentInterruption(j); blocked != tc.blocked {
			t.Fatal("wrong repeated failure decision")
		}
		if len(j.Entries) != before {
			t.Fatal("spent reservations removed")
		}
	}
}

func TestRepeatedFragmentRetryStopsBeforeMutation(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	raw, _ := json.Marshal(p.Packets[0])
	j.Entries = []managedFragmentJournalEntry{{Packet: 0, PacketDigest: hash(raw), CallID: "first", State: "interrupted"}, {Packet: 0, PacketDigest: hash(raw), CallID: "second", State: "interrupted"}}
	raw, _ = json.Marshal(j)
	if err := atomicWrite(filepath.Join(s.root, r.FragmentJournal.Journal), raw); err != nil {
		t.Fatal(err)
	}
	r.FragmentJournal.JournalDigest = hash(raw)
	r.State = "error"
	r.TimeoutSeconds = 600
	task, _ := w.task(a.TaskID)
	task.IndependentReview = &r
	task.Status = "blocked"
	w.Planning.Reviewer.TimeoutSeconds = 600
	w.Planning.Reviewer.Calls = 2
	raw, _ = json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	before, _ := s.get(w.ID)
	_, err := s.mutate(w.ID, "test.retry", "unchanged", before.Revision, []byte(`{}`), func(current *Work) error {
		t, _ := current.task(a.TaskID)
		return s.queueManagedFragmentResume(*current, t, "unchanged")
	})
	if err == nil || !strings.Contains(err.Error(), "diagnostic requis") {
		t.Fatal("unchanged repeated failure retried", err)
	}
	after, _ := s.get(w.ID)
	x, _ := json.Marshal(before)
	y, _ := json.Marshal(after)
	if string(x) != string(y) {
		t.Fatal("blocked retry modified state or accounting")
	}
}
