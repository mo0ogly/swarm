//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestManagedFragmentFinalOrphanReservationResume(t *testing.T) {
	for _, budget := range []bool{true, false} {
		name := "available"
		if !budget {
			name = "exhausted"
		}
		t.Run(name, func(t *testing.T) {
			s, w, a, r, p, j, c, replies, prefix := finalStoreFixture(t)
			f := finalReservation(t, c, p, j, replies, prefix)
			if e := s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "", "orphan-reservation", f); e != nil {
				t.Fatal(e)
			}
			before, _ := s.get(w.ID)
			if !budget {
				before.Planning.Reviewer.MaxCalls = before.Planning.Reviewer.Calls
				raw, _ := json.Marshal(before)
				if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
					t.Fatal(e)
				}
			}
			_, e := s.mutate(w.ID, "fixture.resume", "orphan-resume", before.Revision, []byte(`{}`), func(current *Work) error {
				task, err := current.task(a.TaskID)
				if err != nil {
					return err
				}
				current.Planning.Reviewer.TimeoutSeconds = 600
				return s.queueManagedFragmentResume(*current, task, "orphan-resume")
			})
			if (e == nil) != budget {
				t.Fatal(e)
			}
			after, _ := s.get(w.ID)
			task, _ := after.task(a.TaskID)
			if after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls {
				t.Fatal("orphan reservation refunded or charged")
			}
			got, _, _, e := s.readManagedFragmentFinal(*task.IndependentReview, p, j)
			if e != nil {
				t.Fatal(e)
			}
			if budget {
				if task.IndependentReview.TimeoutSeconds != 600 {
					t.Fatal("configured deadline ignored")
				}
				if got.Calls[0].State != "interrupted" || len(got.ResumeCalls) != 1 || got.ResumeCalls[0] != f.Calls[0].CallID || task.IndependentReview.State != "queued" {
					t.Fatal("orphan authorization lost")
				}
			} else if got.Calls[0].State != "reserved" || len(got.ResumeCalls) != 0 {
				t.Fatal("budget refusal mutated proof")
			}
		})
	}
}

func TestManagedFragmentDurableVerdictResumesWithoutBudget(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	if e := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fragmentRuntimeProviderFixture), 0700); e != nil {
		t.Fatal(e)
	}
	r, e := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if e != nil {
		t.Fatal(e)
	}
	if state, _, e := s.runManagedFragmentReview(w, a, r); e != nil || state != "passed" {
		t.Fatal(state, e)
	}
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	saved := *task.IndependentReview
	saved.State = "error"
	saved.Reason = "Fixture crash after durable final reply"
	if e = s.saveManagedReview(w.ID, a, saved); e != nil {
		t.Fatal(e)
	}
	current, _ = s.get(w.ID)
	current.Planning.Reviewer.MaxCalls = current.Planning.Reviewer.Calls
	raw, _ := json.Marshal(current)
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	calls := managedReviewCalls(t, s)
	request := PlanningRequest{Schema: 1, EventID: "recover-durable-verdict", Revision: current.Revision, Task: a.TaskID, Reason: "Finalize the existing durable verdict without any provider call"}
	if _, e = s.planningChange(w.ID, "retry-review", request); e != nil {
		t.Fatal(e)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	task, _ = after.task(a.TaskID)
	if task.Status != "accepted" || managedReviewCalls(t, s) != calls || after.Planning.Reviewer.Calls != current.Planning.Reviewer.Calls {
		t.Fatal("durable verdict lost or charged", task.Blocker)
	}
}

func TestManagedFragmentInspectionOrphanResume(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	packet, _ := json.Marshal(p.Packets[0])
	j.Entries = append(j.Entries, managedFragmentJournalEntry{Packet: 0, PacketDigest: hash(packet), CallID: "orphan-inspection", State: "reserved"})
	if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "orphan-inspection-reserve", j); e != nil {
		t.Fatal(e)
	}
	before, _ := s.get(w.ID)
	_, e := s.mutate(w.ID, "fixture.resume", "orphan-inspection-resume", before.Revision, []byte(`{}`), func(current *Work) error {
		task, err := current.task(a.TaskID)
		if err != nil {
			return err
		}
		current.Planning.Reviewer.TimeoutSeconds = 600
		return s.queueManagedFragmentResume(*current, task, "orphan-inspection-resume")
	})
	if e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.IndependentReview.TimeoutSeconds != 600 {
		t.Fatal("configured deadline ignored")
	}
	_, got, e := s.readFragmentJournalAnchor(*task.IndependentReview)
	if e != nil {
		t.Fatal(e)
	}
	if got.Entries[0].State != "interrupted" || len(task.IndependentReview.FragmentJournal.ResumeCalls) != 1 || after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls {
		t.Fatal("orphan inspection not retained")
	}
}

func TestManagedFragmentResumeRejectsInvalidDeadline(t *testing.T) {
	s, w, a, r, _, _ := fragmentStoreFixture(t)
	task, _ := w.task(a.TaskID)
	task.IndependentReview = &r
	w.Planning.Reviewer.TimeoutSeconds = 901
	if err := s.queueManagedFragmentResume(w, task, "invalid-deadline"); err == nil {
		t.Fatal("invalid deadline accepted")
	}
}
