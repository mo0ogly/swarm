//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func validCoordination() ReviewCoordinationProposal {
	return ReviewCoordinationProposal{Candidate: "sha", Evidence: "proof", FinalReview: "Examiner toutes les interactions entre les lots.", Lots: []ReviewCoordinationLot{{ID: "api", Kind: "component", Objective: "Examiner le contrat API", Files: []string{"api.go"}, Criteria: []string{"t#1"}}, {ID: "ui", Kind: "dependency", Objective: "Examiner le consommateur", Files: []string{"ui.js"}, Criteria: []string{"t#2"}, Depends: []string{"api"}}}}
}
func TestReviewCoordinationCoverageDAGAndIdentity(t *testing.T) {
	for _, mode := range []string{"valid", "missing-file", "missing-criterion", "foreign", "cycle", "duplicate-id", "stale", "no-final", "unknown-kind"} {
		t.Run(mode, func(t *testing.T) {
			p := validCoordination()
			switch mode {
			case "missing-file":
				p.Lots[1].Files = []string{"api.go"}
			case "missing-criterion":
				p.Lots[1].Criteria = []string{"t#1"}
			case "foreign":
				p.Lots[1].Files = []string{"elsewhere"}
			case "cycle":
				p.Lots[0].Depends = []string{"ui"}
			case "duplicate-id":
				p.Lots[1].ID = "api"
			case "stale":
				p.Evidence = "old"
			case "no-final":
				p.FinalReview = ""
			case "unknown-kind":
				p.Lots[0].Kind = "whatever"
			}
			order, err := validateReviewCoordination(p, "sha", "proof", []string{"api.go", "ui.js"}, []string{"t#1", "t#2"})
			if mode == "valid" {
				if err != nil || len(order) != 2 || order[0] != "api" {
					t.Fatal(order, err)
				}
			} else if err == nil {
				t.Fatal("invalid plan accepted")
			}
		})
	}
}
func TestReviewCoordinationThroughVersionedPlannerDecision(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	w, r := planningClaim(t, s, w, task.ScopeID)
	preview, err := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
	if err != nil {
		t.Fatal(err)
	}
	context, _, contextErr := s.planningDeliveryContext(w, task.ScopeID, 64000, s.planningReviewInputs(w, task.ScopeID))
	if contextErr != nil || !strings.Contains(string(context), "review_planning_inputs") || !strings.Contains(string(context), preview.Evidence) {
		t.Fatal("planner evidence missing", contextErr)
	}
	p := ReviewCoordinationProposal{Candidate: preview.Candidate, Evidence: preview.Evidence, FinalReview: "Revoir toutes les interactions et tous les critères du candidat.", Lots: []ReviewCoordinationLot{{ID: "whole", Kind: "component", Objective: "Examiner les changements bornés", Files: preview.Files, Criteria: preview.Criteria}}}
	raw, _ := json.Marshal(p)
	r.Operations = []PlanningOperation{{Kind: "review-plan", ID: a.TaskID, Deliverable: string(raw)}}
	after, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = after.task(a.TaskID)
	if task.ReviewCoordination == nil || task.ReviewCoordination.State != "validated_not_executed" || task.Status != "blocked" || managedReviewCalls(t, s) != 0 {
		t.Fatal("plan became paid execution or acceptance")
	}
	if _, err = s.planningChange(w.ID, "decide", r); err != nil {
		t.Fatal("replay", err)
	}
	again, _ := s.get(w.ID)
	if again.Revision != after.Revision {
		t.Fatal("replay mutated work")
	}
}
