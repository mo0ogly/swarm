//go:build linux

package main

import (
	"os"
	"strings"
	"testing"
	"time"
)

func TestManagedPublicationLaneRetainsSiblingWithoutPaidReview(t *testing.T) {
	s, w := managedFixture(t)
	first := managedCompleted(t, s, w, "first", "initial\n")
	w, _ = s.get(w.ID)
	second := managedCompleted(t, s, w, "second", "initial\n")
	// Both copies retain the same starting tree; reports are independent changes.
	ps, _ := s.providers()
	p := ps.Providers["managed-review-fixture"]
	code, err := os.ReadFile(p.Command)
	if err != nil {
		t.Fatal(err)
	}
	code = []byte(strings.Replace(string(code), "import json, os, sys", "import json, os, sys, time\nopen(__file__+'.entered','w').close()\nwhile not os.path.exists(__file__+'.release'): time.sleep(.01)", 1))
	if err = os.WriteFile(p.Command, code, 0700); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { done <- s.integrateManagedAttempt(first) }()
	defer os.WriteFile(p.Command+".release", nil, 0600)
	deadline := time.Now().Add(10 * time.Second)
	for {
		if _, err = os.Stat(p.Command + ".entered"); err == nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("review did not start")
		}
		time.Sleep(10 * time.Millisecond)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	before, _ := other.get(w.ID)
	if err = other.integrateManagedAttempt(second); err != nil {
		t.Fatal(err)
	}
	after, _ := other.get(w.ID)
	if after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls || after.Tasks[1].IndependentReview != nil {
		t.Fatal("sibling started a review against an obsolete base")
	}
	// The mission Git lock stays available while review is pending.
	unlock, err := managedLock(s.root, w.ID)
	if err != nil {
		t.Fatal(err)
	}
	unlock()
	if err = os.WriteFile(p.Command+".release", nil, 0600); err != nil {
		t.Fatal(err)
	}
	if err = <-done; err != nil {
		t.Fatal(err)
	}
	if err = other.integrateManagedAttempt(second); err != nil {
		t.Fatal(err)
	}
	final, _ := other.get(w.ID)
	for i := range final.Tasks {
		if final.Tasks[i].Status != "accepted" || len(final.Tasks[i].Attempts) != 1 {
			t.Fatal("sibling not integrated without reproduction", final.Tasks[i].ID)
		}
		if err = other.independentReviewGuard(&final, &final.Tasks[i]); err != nil {
			t.Fatal(err)
		}
	}
	if final.Planning.Reviewer.Calls != 2 {
		t.Fatal("unexpected paid calls", final.Planning.Reviewer.Calls)
	}
}
