//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"testing"
)

func TestManagedReviewOrphanReservationSettlesWithoutAnotherCall(t *testing.T) {
	for _, phase := range []string{"inspection", "final"} {
		t.Run(phase, func(t *testing.T) {
			var s *Store
			var w Work
			var a Agent
			var r IndependentReview
			if phase == "inspection" {
				var p managedReviewFragmentPlan
				var j managedFragmentJournal
				s, w, a, r, p, j = fragmentStoreFixture(t)
				if err := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "orphan-inspection-reserve", reserveFragment(p, j)); err != nil {
					t.Fatal(err)
				}
			} else {
				var p managedReviewFragmentPlan
				var j managedFragmentJournal
				var c managedReviewContext
				var replies []string
				var prefix string
				s, w, a, r, p, j, c, replies, prefix = finalStoreFixture(t)
				f := finalReservation(t, c, p, j, replies, prefix)
				if err := s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "", "orphan-final-reserve", f); err != nil {
					t.Fatal(err)
				}
			}
			before, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			task, err := before.task(a.TaskID)
			if err != nil {
				t.Fatal(err)
			}
			r = *task.IndependentReview
			originalPath := r.FragmentJournal.Journal
			if phase == "final" {
				originalPath = r.FragmentJournal.FinalJournal
			}
			original, err := os.ReadFile(filepath.Join(s.root, originalPath))
			if err != nil {
				t.Fatal(err)
			}
			receipt, err := os.ReadFile(filepath.Join(s.root, r.Receipt))
			if err != nil {
				t.Fatal(err)
			}
			if err = s.reviewManagedCandidate(before, a, r.CandidateSHA, r.Receipt, receipt); err == nil {
				t.Fatal("orphan implicitly resumed")
			}
			after, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			task, err = after.task(a.TaskID)
			if err != nil {
				t.Fatal(err)
			}
			r = *task.IndependentReview
			if r.State != "error" || after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls || len(task.Attempts) != len(before.Tasks[0].Attempts) {
				t.Fatal("orphan settlement spent, accepted or changed history", r.State)
			}
			p, j, err := s.readFragmentJournalAnchor(r)
			if err != nil {
				t.Fatal(err)
			}
			if phase == "inspection" {
				if j.Entries[len(j.Entries)-1].State != "interrupted" {
					t.Fatal("orphan inspection reservation remains active")
				}
			} else {
				f, _, _, err := s.readManagedFragmentFinal(r, p, j)
				if err != nil {
					t.Fatal(err)
				}
				if f.Calls[len(f.Calls)-1].State != "interrupted" {
					t.Fatal("orphan final reservation remains active")
				}
			}
			retained, err := os.ReadFile(filepath.Join(s.root, originalPath))
			if err != nil || string(retained) != string(original) {
				t.Fatal("original journal changed", err)
			}
			route := planningReviewFailureRouting(after, after.Planning.Inbox)[a.TaskID]
			if r.FailureCode != reviewFragmentInterruptedError || route.Code != reviewFragmentInterruptedError || route.Owner != "reviewer_runtime" || route.Operation != "review-retry" {
				t.Fatal("orphan incident did not reach its runtime owner", route)
			}
			if !s.recoveredReviewAllowsCorrection(after, &r, true) {
				t.Fatal("verified orphan blocks an explicit changed-result repair")
			}
		})
	}
}

func TestManagedReviewInterruptionDirectiveReachesReviewer(t *testing.T) {
	d := &ManagedReviewRetryDirective{Decision: "owner-decision", Scope: "root", ResultEvent: "current-incident", Review: "current-review", Candidate: "candidate", Attempt: "current-attempt", FailureCode: reviewFragmentInterruptedError, Cause: "Owned engine exited with a paid reservation", Instruction: "Resume only after verified process reconciliation and preserve the original paid call"}
	if _, err := managedReviewRecoveryPrefix("original-prefix", d); err != nil {
		t.Fatal("runtime owner decision cannot reach reviewer", err)
	}
}
