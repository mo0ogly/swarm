//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestCoordinatedFragmentsCoverageAndBudget(t *testing.T) {
	c := fragmentPlanFixture()
	files := []string{}
	for i := 0; i < 20; i++ {
		files = append(files, "f"+strconvI(i))
	}
	proposal := ReviewCoordinationProposal{Candidate: c.Candidate, Evidence: coordinationEvidence(c), FinalReview: "Verify interactions and all original task criteria.", Lots: []ReviewCoordinationLot{
		{ID: "second", Kind: "component", Objective: "Inspect remaining files", Files: files[10:], Criteria: coordinationCriteria(c), Depends: []string{"first"}},
		{ID: "first", Kind: "component", Objective: "Inspect prerequisite files", Files: files[:10], Criteria: coordinationCriteria(c)},
	}}
	p, e := planCoordinatedFragments(c, proposal, files, 100)
	if e != nil {
		t.Fatal(e)
	}
	if p.Packets[0].Lot != "first" || p.Packets[len(p.Packets)-1].Lot != "__global_evidence" {
		t.Fatal("DAG not followed")
	}
	if _, e = planCoordinatedFragments(c, proposal, files, len(p.Packets)+1); e == nil {
		t.Fatal("final calls not reserved")
	}
	for _, mode := range []string{"omission", "order", "scope", "criteria", "stale"} {
		t.Run(mode, func(t *testing.T) {
			raw, _ := json.Marshal(p)
			var bad managedReviewFragmentPlan
			json.Unmarshal(raw, &bad)
			switch mode {
			case "omission":
				bad.Packets[0].Artifacts = bad.Packets[0].Artifacts[:1]
			case "order":
				bad.Packets[0].Lot = "second"
			case "scope":
				bad.CoordinationFiles = append(bad.CoordinationFiles, "missing")
			case "criteria":
				bad.Coordination.Lots[0].Criteria = []string{"unknown#1"}
			case "stale":
				bad.Coordination.Evidence = "stale"
			}
			if validateManagedReviewFragments(c, bad) == nil {
				t.Fatal("altered semantic plan accepted")
			}
		})
	}
}
func strconvI(i int) string { b, _ := json.Marshal(i); return string(b) }

func TestCoordinatedReviewExecutesAndPersists(t *testing.T) {
	for _, mode := range []string{"pass", "decision-fail", "exit-second"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, c, path, receipt := fragmentBeginFixture(t)
			w, _ = s.get(w.ID)
			task, _ := w.task(a.TaskID)
			w, r := planningClaim(t, s, w, task.ScopeID)
			preview, e := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
			if e != nil {
				t.Fatal(e)
			}
			proposal := ReviewCoordinationProposal{Candidate: preview.Candidate, Evidence: preview.Evidence, FinalReview: "Verify all interactions and original acceptance criteria.", Lots: []ReviewCoordinationLot{{ID: "bounded", Kind: "component", Objective: "Inspect the bounded candidate", Files: preview.Files, Criteria: preview.Criteria}}}
			raw, _ := json.Marshal(proposal)
			r.Operations = []PlanningOperation{{Kind: "review-plan", ID: a.TaskID, Deliverable: string(raw)}}
			w, e = s.planningChange(w.ID, "decide", r)
			if e != nil {
				t.Fatal(e)
			}
			if _, e = s.prepareManagedPreflightRetry(w.ID, a.TaskID); e != nil {
				t.Fatal(e)
			}
			if e = os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fragmentRuntimeProviderFixture), 0700); e != nil {
				t.Fatal(e)
			}
			managedReviewMode(t, s, mode)
			review, e := s.beginManagedFragmentReview(w, a, c, path, receipt)
			if e != nil {
				t.Fatal(e)
			}
			p, _, e := s.readFragmentJournalAnchor(review)
			if e != nil || p.Coordination == nil {
				t.Fatal("semantic plan not durable", e)
			}
			state, records, runErr := s.runManagedFragmentReview(w, a, review)
			if mode == "pass" && (runErr != nil || state != "passed" || len(records) != len(c.Tasks)) {
				t.Fatal(state, runErr)
			}
			if mode != "pass" && state == "passed" {
				t.Fatal("failure accepted")
			}
			after, _ := s.get(w.ID)
			task, _ = after.task(a.TaskID)
			if task.Status == "accepted" {
				t.Fatal("inspection bypassed publication")
			}
			expected := len(p.Packets) + 2
			if mode == "exit-second" {
				expected = 2
			}
			if managedReviewCalls(t, s) != expected {
				t.Fatal("wrong paid count", managedReviewCalls(t, s), expected)
			}
			_, journal, e := s.readFragmentJournalAnchor(*task.IndependentReview)
			if e != nil || len(journal.Entries) == 0 {
				t.Fatal("missing durable results", e)
			}
			if mode == "exit-second" {
				if e = s.finishManagedFragmentReview(w.ID, a, review.ID, runErr); e == nil {
					t.Fatal("interruption reported success")
				}
				managedReviewMode(t, s, "pass")
				current, _ := s.get(w.ID)
				req := PlanningRequest{Schema: 1, EventID: "resume-coordinated", Revision: current.Revision, Task: a.TaskID, Reason: "Retry interrupted inspection preserving the first durable observation."}
				if _, e = s.planningChange(w.ID, "retry-review", req); e != nil {
					t.Fatal(e)
				}
				if e = s.reconcileKnownMissionResult(a, "fixture-resume"); e != nil {
					t.Fatal(e)
				}
				current, _ = s.get(w.ID)
				task, _ = current.task(a.TaskID)
				if task.Status != "accepted" || managedReviewCalls(t, s) != len(p.Packets)+3 {
					t.Fatal("resume repeated successful inspections", task.Blocker, managedReviewCalls(t, s))
				}
			}

		})
	}
}
func TestCoordinationGitPaths(t *testing.T) {
	for _, path := range []string{"space name.go", "é.go", "tab\tname.go"} {
		header := "diff --git " + gitCoordinationQuote("a/"+path) + " " + gitCoordinationQuote("b/"+path)
		if !coordinationHeaderMatches(header, path) {
			t.Fatal(path)
		}
		a := managedReviewFragmentArtifact{Kind: "diff", Content: header + "\n--- " + gitCoordinationQuote("a/"+path) + "\n+++ " + gitCoordinationQuote("b/"+path) + "\n@@ -1 +1 @@\n"}
		if got := coordinationDiffPath(a); got != path {
			t.Fatal(path, got)
		}
	}
	if !strings.Contains(gitCoordinationQuote("é"), `\303`) {
		t.Fatal("Git byte quoting")
	}
}

func TestCoordinatedReviewPublicRetryPublishesSameCandidate(t *testing.T) {
	s, w, a, _, _, _ := fragmentBeginFixture(t)
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	w, r := planningClaim(t, s, w, task.ScopeID)
	preview, e := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
	if e != nil {
		t.Fatal(e)
	}
	p := ReviewCoordinationProposal{Candidate: preview.Candidate, Evidence: preview.Evidence, FinalReview: "Verify cross-lot interactions before any publication.", Lots: []ReviewCoordinationLot{{ID: "whole", Kind: "component", Objective: "Review this bounded change", Files: preview.Files, Criteria: preview.Criteria}}}
	raw, _ := json.Marshal(p)
	r.Operations = []PlanningOperation{{Kind: "review-plan", ID: a.TaskID, Deliverable: string(raw)}}
	w, e = s.planningChange(w.ID, "decide", r)
	if e != nil {
		t.Fatal(e)
	}
	if e = os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fragmentRuntimeProviderFixture), 0700); e != nil {
		t.Fatal(e)
	}
	cost, e := s.managedReviewCostPreview(w.ID, a.TaskID)
	if e != nil || !cost.TransportReady {
		t.Fatal(cost, e)
	}
	retry := PlanningRequest{Schema: 1, EventID: "coordinated-retry", Revision: w.Revision, Task: a.TaskID, Reason: "Execute the approved bounded review plan."}
	if _, e = s.planningChange(w.ID, "retry-review", retry); e != nil {
		t.Fatal(e)
	}
	if managedReviewCalls(t, s) != 0 {
		t.Fatal("queue spent calls")
	}
	if e = s.reconcileKnownMissionResult(a, "fixture-conductor"); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	task, _ = after.task(a.TaskID)
	if task.Status != "accepted" || task.IndependentReview == nil || task.IndependentReview.CandidateSHA != preview.Candidate || after.Planning.Repository.Candidate != preview.Candidate {
		t.Fatal(task.Status, task.Blocker)
	}
	if managedReviewCalls(t, s) != cost.CallsRequired {
		t.Fatal("estimate differs", managedReviewCalls(t, s), cost.CallsRequired)
	}
	if _, e = s.planningChange(w.ID, "retry-review", retry); e != nil {
		t.Fatal(e)
	}
	if e = s.reconcileKnownMissionResult(a, "fixture-conductor"); e != nil || managedReviewCalls(t, s) != cost.CallsRequired {
		t.Fatal("duplicate spend", e)
	}
}

func TestCoordinatedFragmentsCountsLotContractBeforePacking(t *testing.T) {
	c := fragmentPlanFixture()
	c.Tasks[0].Criteria[0] = strings.Repeat("Check this invariant with evidence. ", 900)
	files := []string{}
	for i := 0; i < 20; i++ {
		files = append(files, "f"+strconvI(i))
	}
	proposal := ReviewCoordinationProposal{Candidate: c.Candidate, Evidence: coordinationEvidence(c), FinalReview: "Verify all interactions using the original evidence.", Lots: []ReviewCoordinationLot{{ID: "bounded", Kind: "component", Objective: "Review the bounded files and criteria", Files: files, Criteria: coordinationCriteria(c)}}}
	p, err := planCoordinatedFragments(c, proposal, files, 100)
	if err != nil {
		t.Fatal("lot contract must participate in packing", err)
	}
	_, method, err := agentWorkflow("reviewer")
	if err != nil {
		t.Fatal(err)
	}
	if err = preflightManagedFragmentCalls(managedReviewPrefix(method), c, p); err != nil {
		t.Fatal(err)
	}
}

func TestCoordinatedTokenPackingKeepsLotBoundaries(t *testing.T) {
	c := fragmentPlanFixture()
	files := []string{}
	for i := 0; i < 20; i++ {
		files = append(files, "f"+strconvI(i))
	}
	proposal := ReviewCoordinationProposal{Candidate: c.Candidate, Evidence: coordinationEvidence(c), FinalReview: "Check interactions before the final decision.", Lots: []ReviewCoordinationLot{{ID: "first", Kind: "component", Objective: "Inspect first component", Files: files[:10], Criteria: coordinationCriteria(c)}, {ID: "second", Kind: "component", Objective: "Inspect dependent component", Files: files[10:], Criteria: coordinationCriteria(c), Depends: []string{"first"}}}}
	old, e := planCoordinatedFragments(c, proposal, files, 100)
	if e != nil {
		t.Fatal(e)
	}
	p, e := planTokenManagedFragments(c, old, reviewTokenBudgetFixture(), "", 100)
	if e != nil {
		t.Fatal(e)
	}
	if p.Version != 4 || p.Coordination == nil || len(p.Packets) >= len(old.Packets) {
		t.Fatal("capacity adaptation lost semantic plan")
	}
	seen := map[string]bool{}
	last := ""
	for _, packet := range p.Packets {
		if packet.Lot != last {
			if seen[packet.Lot] {
				t.Fatal("lot interleaved")
			}
			seen[packet.Lot] = true
			last = packet.Lot
		}
		if packet.Lot != "__global_evidence" && packet.LotContract == nil {
			t.Fatal("contract omitted")
		}
	}
	if len(seen) != 3 {
		t.Fatal("lot omitted")
	}
	if _, e = planTokenManagedFragments(c, old, reviewTokenBudgetFixture(), "", len(p.Packets)+1); e == nil {
		t.Fatal("budget exceeded")
	}
}
