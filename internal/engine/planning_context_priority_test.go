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
			Events  []PlanningEvent `json:"events"`
			Pending int             `json:"pending_events_total"`
		}
		if e = json.Unmarshal(raw, &context); e != nil {
			t.Fatal(e)
		}
		ids := []string{}
		for _, event := range context.Events {
			ids = append(ids, event.ID)
		}
		if !reflect.DeepEqual(ids, want) || context.Pending != 4 {
			t.Fatal(ids, context.Pending)
		}
	}
	check(1, []string{"current"})
	check(4, []string{"current", "operator", "old-1", "old-2"})
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
