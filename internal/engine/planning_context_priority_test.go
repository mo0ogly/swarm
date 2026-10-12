package engine

import (
	"encoding/json"
	"reflect"
	"testing"
)

func TestPlanningContextPrioritizesCurrentAttemptWithoutConsumingHistory(t *testing.T) {
	w := Work{Planning: &PlanningState{Scopes: []PlanningScope{{ID: "root"}}, Inbox: []PlanningEvent{{ID: "old-1", Scope: "root", Task: "task", Attempt: "old"}, {ID: "old-2", Scope: "root", Task: "task", Attempt: "old"}, {ID: "current", Scope: "root", Task: "task", Attempt: "new"}, {ID: "operator", Scope: "root"}}}, Tasks: []Task{{ID: "task", ScopeID: "root", Attempts: []Attempt{{ID: "old"}, {ID: "new"}}}}}
	before, _ := json.Marshal(w)
	check := func(limit int, want []string) {
		raw, e := planningContextLimit(w, "root", limit)
		if e != nil {
			t.Fatal(e)
		}
		var context struct {
			Events         []PlanningEvent `json:"events"`
			Historical     []PlanningEvent `json:"historical_events"`
			HistoricalOnly bool            `json:"historical_only"`
			Pending        int             `json:"pending_events_total"`
		}
		if e = json.Unmarshal(raw, &context); e != nil {
			t.Fatal(e)
		}
		ids := []string{}
		for _, event := range context.Events {
			ids = append(ids, event.ID)
		}
		if !reflect.DeepEqual(ids, want) || context.Pending != 4 || context.HistoricalOnly || len(context.Historical) != min(limit, 2) {
			t.Fatal(ids, context.Pending)
		}
	}
	check(1, []string{"current"})
	check(4, []string{"current", "operator"})
	after, _ := json.Marshal(w)
	if string(before) != string(after) {
		t.Fatal("projection mutated history")
	}
}

func TestPlanningContextPrioritizesBlockedTaskOverAcceptedCurrentAttempt(t *testing.T) {
	w := Work{Planning: &PlanningState{Scopes: []PlanningScope{{ID: "root"}}, Inbox: []PlanningEvent{
		{ID: "accepted", Scope: "root", Task: "done", Attempt: "latest"},
		{ID: "blocked", Scope: "root", Task: "blocked", Attempt: "latest"},
	}}, Tasks: []Task{
		{ID: "done", ScopeID: "root", Status: "accepted", Attempts: []Attempt{{ID: "latest"}}},
		{ID: "blocked", ScopeID: "root", Status: "blocked", Attempts: []Attempt{{ID: "latest"}}},
	}}
	before, _ := json.Marshal(w)
	raw, e := planningContextLimit(w, "root", 1)
	if e != nil {
		t.Fatal(e)
	}
	var got struct {
		Events  []PlanningEvent `json:"events"`
		Pending int             `json:"pending_events_total"`
	}
	if e = json.Unmarshal(raw, &got); e != nil {
		t.Fatal(e)
	}
	if len(got.Events) != 1 || got.Events[0].ID != "blocked" || got.Pending != 2 {
		t.Fatal(string(raw))
	}
	after, _ := json.Marshal(w)
	if string(before) != string(after) {
		t.Fatal("history changed")
	}
	raw, e = planningContextLimit(w, "root", 8)
	if e != nil {
		t.Fatal(e)
	}
	if e = json.Unmarshal(raw, &got); e != nil {
		t.Fatal(e)
	}
	if len(got.Events) != 2 || got.Events[1].ID != "accepted" {
		t.Fatal("accepted event lost", string(raw))
	}
}

func TestPlanningContextNewestOperatorResumePrecedesBlockedReturns(t *testing.T) {
	w := Work{Planning: &PlanningState{Scopes: []PlanningScope{{ID: "root"}}, Inbox: []PlanningEvent{
		{ID: "earlier-instruction", Scope: "root", Kind: "operator_resume"},
		{ID: "blocked-return", Scope: "root", Task: "task", Attempt: "current"},
		{ID: "latest-instruction", Scope: "root", Kind: "operator_resume"},
	}}, Tasks: []Task{{ID: "task", ScopeID: "root", Status: "blocked", Attempts: []Attempt{{ID: "current"}}}}}
	before, _ := json.Marshal(w)
	raw, err := planningContextLimit(w, "root", 1)
	if err != nil {
		t.Fatal(err)
	}
	var ctx struct {
		Events  []PlanningEvent `json:"events"`
		Pending int             `json:"pending_events_total"`
	}
	if err = json.Unmarshal(raw, &ctx); err != nil || len(ctx.Events) != 1 || ctx.Events[0].ID != "latest-instruction" || ctx.Pending != 3 {
		t.Fatal("latest correction not delivered first", string(raw), err)
	}
	after, _ := json.Marshal(w)
	if string(after) != string(before) {
		t.Fatal("priority silently consumed history")
	}
}

func TestPlanningContextCurrentReviewPrecedesOlderResumes(t *testing.T) {
	w := Work{Planning: &PlanningState{Scopes: []PlanningScope{{ID: "root"}}, Inbox: []PlanningEvent{
		{ID: "older-instruction", Scope: "root", Kind: "operator_resume"},
		{ID: "old-blocker", Scope: "root", Task: "task", Attempt: "current"},
		{ID: "current-result", Scope: "root", Kind: "independent_review", Task: "task", Attempt: "current"},
		{ID: "latest-instruction", Scope: "root", Kind: "operator_resume"},
	}}, Tasks: []Task{{ID: "task", ScopeID: "root", Status: "blocked", Attempts: []Attempt{{ID: "current"}}, IndependentReview: &IndependentReview{ID: "review", ResultEvent: "current-result", Attempt: "current", State: "error", FailureCode: reviewFragmentProtocolError}}}}
	before, _ := json.Marshal(w)
	raw, err := planningContextLimit(w, "root", 2)
	if err != nil {
		t.Fatal(err)
	}
	var ctx struct {
		Events  []PlanningEvent `json:"events"`
		Pending int             `json:"pending_events_total"`
	}
	if err = json.Unmarshal(raw, &ctx); err != nil || len(ctx.Events) != 2 || ctx.Events[0].ID != "latest-instruction" || ctx.Events[1].ID != "current-result" || ctx.Pending != 4 {
		t.Fatal("current review displaced by old resume or blocker", string(raw), err)
	}
	after, _ := json.Marshal(w)
	if string(before) != string(after) {
		t.Fatal("priority consumed history")
	}
}
