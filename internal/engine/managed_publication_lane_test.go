//go:build linux

package engine

import (
	"os"
	"path/filepath"
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
	retained, err := other.managedAttempt(second.ID)
	if err != nil || retained.Result == "" || retained.State != "integrating" {
		t.Fatal("completed sibling could not freeze its result while the publication lane was occupied", retained, err)
	}
	if after.Revision != before.Revision {
		t.Fatal("retaining a result modified acceptance or planning state")
	}
	// Replays use the retained Git object, even if the original copy changes.
	if err = os.WriteFile(filepath.Join(second.CWD, "docs", second.TaskID+".md"), []byte("unsubmitted workspace change"), 0600); err != nil {
		t.Fatal(err)
	}
	if err = other.integrateManagedAttempt(second); err != nil {
		t.Fatal(err)
	}
	stable, err := other.managedAttempt(second.ID)
	replayed, workErr := other.get(w.ID)
	if err != nil || workErr != nil || stable.Result != retained.Result || replayed.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls {
		t.Fatal("staging replay replaced a result or spent a call", stable, err)
	}
	bare := filepath.Join(before.Planning.Repository.Storage, "repository.git")
	if report := gitTest(t, bare, "show", retained.Result+":docs/second.md"); report != "Rapport nouveau second" {
		t.Fatal("retained report was not immutable", report)
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

func TestManagedPublicationStagingPreservesPauseAndIdentity(t *testing.T) {
	for _, kind := range []string{"global-pause", "planning-pause", "active", "foreign-attempt", "foreign-copy"} {
		t.Run(kind, func(t *testing.T) {
			s, w := managedFixture(t)
			a := managedCompleted(t, s, w, "first", "initial\n")
			w, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			switch kind {
			case "global-pause":
				if err = s.pause(w.ID, true); err != nil {
					t.Fatal(err)
				}
			case "planning-pause":
				w = planningDo(t, s, w, "pause", PlanningRequest{Reason: "Keep pending worker results paused"})
			case "active":
				a.Status = "running"
			case "foreign-attempt":
				a.Attempt = "other-attempt"
			case "foreign-copy":
				if _, err = s.db.Exec("UPDATE managed_attempts SET path=? WHERE agent_id=?", filepath.Join(s.root, "other-copy"), a.ID); err != nil {
					t.Fatal(err)
				}
			}
			before, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			release, acquired, err := managedPublicationLane(s.root, w.ID)
			if err != nil || !acquired {
				t.Fatal(err)
			}
			defer release()
			err = s.integrateManagedAttempt(a)
			paused := kind == "global-pause" || kind == "planning-pause"
			if (err == nil) != paused {
				t.Fatal("unexpected staging eligibility", kind, err)
			}
			item, err := s.managedAttempt(a.ID)
			if err != nil || item.Result != "" || item.State != "ready" {
				t.Fatal("ineligible result staged", item, err)
			}
			after, err := s.get(w.ID)
			if err != nil || after.Revision != before.Revision || after.Planning.Reviewer.Calls != 0 || after.Tasks[0].IndependentReview != nil {
				t.Fatal("staging refusal changed work or spent a call", after, err)
			}
		})
	}
}
