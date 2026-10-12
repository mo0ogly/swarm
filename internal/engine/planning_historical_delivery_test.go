//go:build linux

package engine

import (
	"encoding/json"
	"reflect"
	"strings"
	"testing"
)

// Isolated fixture; public claims/decisions and restart establish the protocol,
// without pretending this is a real provider's autonomous decision.
func TestPlanningHistoricalInputsCannotPoisonCurrentDecision(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("producer")}
	w = planningDo(t, s, w, "decide", r)
	var err error
	w, err = s.mutate(w.ID, "test.fixture", "current-and-old-returns", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Criteria = append(w.Criteria, "Final retrospective evidence")
		w.Planning.Scopes[0].Requirements = append(w.Planning.Scopes[0].Requirements, "req-2")
		w.Tasks[0].Status = "blocked"
		w.Tasks[0].Attempts = []Attempt{{ID: "old"}, {ID: "current"}}
		w.Planning.Inbox = []PlanningEvent{
			{ID: "old-return", Scope: "root", Kind: "attempt", Task: "producer", Attempt: "old"},
			{ID: "current-return", Scope: "root", Kind: "attempt", Task: "producer", Attempt: "current"},
			{ID: "operator-retex", Scope: "root", Kind: "operator_resume", Message: "Create final RETEX"},
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	claim := func(w Work) (Work, PlanningRequest) {
		t.Helper()
		owner, _ := w.Planning.scope("root")
		w = planningDo(t, s, w, "claim", PlanningRequest{Scope: "root", ScopeRevision: owner.Revision, Holder: "history-owner", LeaseSeconds: 60})
		owner, _ = w.Planning.scope("root")
		return w, PlanningRequest{Scope: "root", ScopeRevision: owner.Revision, Generation: owner.Generation, Holder: owner.Holder, Inputs: owner.Delivery.Events, Reason: "Handle exactly the admitted inputs"}
	}
	w, r = claim(w)
	if !reflect.DeepEqual(r.Inputs, []string{"operator-retex", "current-return"}) {
		t.Fatalf("current decision poisoned by old return: %v", r.Inputs)
	}
	final := planningTask("final-retex")
	final.Requirements = []string{"req-2"}
	r.Operations = []PlanningOperation{final}
	w = planningDo(t, s, w, "decide", r)
	if len(w.Tasks) != 2 || w.Planning.Inbox[0].Decision != "" {
		t.Fatal("history lost or final task not created")
	}
	w, r = claim(w)
	if !reflect.DeepEqual(r.Inputs, []string{"old-return"}) {
		t.Fatal(r.Inputs)
	}
	owner, _ := w.Planning.scope("root")
	if !owner.Delivery.HistoricalOnly {
		t.Fatal("claim did not freeze historical-only restriction")
	}
	var schema struct {
		Properties map[string]struct {
			MaxItems *int `json:"maxItems"`
			Items    struct {
				Enum []string `json:"enum"`
			} `json:"items"`
		} `json:"properties"`
	}
	if err := json.Unmarshal([]byte(planningSchemaForDelivery(*owner.Delivery)), &schema); err != nil || schema.Properties["operations"].MaxItems == nil || *schema.Properties["operations"].MaxItems != 0 || !reflect.DeepEqual(schema.Properties["input_events"].Items.Enum, []string{"old-return"}) {
		t.Fatal("provider schema permits historical operations", schema, err)
	}
	raw, err := planningContext(w, "root")
	if err != nil {
		t.Fatal(err)
	}
	var ctx struct {
		HistoricalOnly bool `json:"historical_only"`
	}
	if err := json.Unmarshal(raw, &ctx); err != nil || !ctx.HistoricalOnly {
		t.Fatal("historical acknowledgement not distinguished", string(raw), err)
	}
	r.Operations = []PlanningOperation{planningTask("forged-history-operation")}
	r.Schema, r.EventID, r.Revision = 1, "unsafe-history", w.Revision
	if _, err := s.planningChange(w.ID, "decide", r); err == nil {
		t.Fatal("historical return authorized an operation")
	}
	unchanged, err := s.get(w.ID)
	if err != nil || unchanged.Revision != w.Revision || unchanged.Planning.Inbox[0].Decision != "" {
		t.Fatal("invalid decision mutated history", err)
	}
	r.Operations = nil
	w = planningDo(t, s, w, "decide", r)
	if w.Planning.Scopes[0].State != "ready" || w.Planning.Scopes[0].Delivery.HistoricalOnly == false {
		t.Fatal("historical acknowledgement lost continuation or delivery history")
	}
	pending := []PlanningEvent{}
	for _, event := range w.Planning.Inbox {
		if event.Decision == "" {
			pending = append(pending, event)
		}
	}
	if len(pending) != 1 || pending[0].Kind != "historical_acknowledged" || len(w.Tasks) != 2 {
		t.Fatal("no current continuation after final historical acknowledgement", pending)
	}
	second, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer second.db.Close()
	persisted, err := second.get(w.ID)
	if err != nil || len(persisted.Tasks) != 2 || persisted.Planning.Inbox[0].Decision == "" || len(persisted.Tasks[0].Attempts) != 2 || persisted.Planning.Activations != 3 || persisted.Planning.Decisions != 3 {
		t.Fatal("acknowledgement, original attempts or consumption lost after restart", persisted, err)
	}
	persisted, next := claim(persisted)
	owner, _ = persisted.Planning.scope("root")
	if owner.Delivery.HistoricalOnly || !reflect.DeepEqual(next.Inputs, []string{pending[0].ID}) {
		t.Fatal("continuation delivered as historical or missing", owner.Delivery)
	}
	next.Operations = []PlanningOperation{{Kind: "revise-instructions", ID: "final-retex", Next: "Préserver les acquittements historiques et vérifier les exigences encore ouvertes."}}
	persisted = planningDo(t, s, persisted, "decide", next)
	if len(persisted.Tasks) != 2 || len(persisted.Tasks[1].InstructionRevisions) != 1 || persisted.Planning.Scopes[0].State == "closed" {
		t.Fatal("continuation cannot act or engine invented completion", persisted)
	}
}

func TestPlanningContextMissingTaskIsHistoricalEvenWithoutAttempt(t *testing.T) {
	w := Work{Planning: &PlanningState{Scopes: []PlanningScope{{ID: "root"}}, Inbox: []PlanningEvent{
		{ID: "missing-task", Scope: "root", Task: "removed"},
		{ID: "operator", Scope: "root"},
	}}}
	raw, err := planningContext(w, "root")
	if err != nil {
		t.Fatal(err)
	}
	var ctx struct {
		Events     []PlanningEvent `json:"events"`
		Historical []PlanningEvent `json:"historical_events"`
	}
	if err := json.Unmarshal(raw, &ctx); err != nil || len(ctx.Events) != 1 || ctx.Events[0].ID != "operator" || len(ctx.Historical) != 1 || ctx.Historical[0].ID != "missing-task" {
		t.Fatal(string(raw), err)
	}
}

func TestPlanningDeliveryExcludesUnavailableProductionRetry(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("producer")}
	w = planningDo(t, s, w, "decide", r)
	var err error
	w, err = s.mutate(w.ID, "test.protocol-error", "strict-current-protocol", w.Revision, []byte(`{}`), func(w *Work) error {
		task := &w.Tasks[0]
		task.Status = "blocked"
		task.Blocker = "old semantic alias refusal"
		task.Attempts = []Attempt{{ID: "current"}}
		task.IndependentReview = &IndependentReview{ID: "current-review", Attempt: "current", State: "error", FailureCode: reviewFragmentProtocolError, Reason: "inspection de fragment : defect quote absent at original line"}
		w.Planning.Inbox = []PlanningEvent{{ID: reviewPlanningEventID(task.IndependentReview), Kind: "independent_review", Scope: "root", Task: task.ID, Attempt: "current"}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	owner, _ := w.Planning.scope("root")
	w = planningDo(t, s, w, "claim", PlanningRequest{Scope: "root", ScopeRevision: owner.Revision, Holder: "strict-owner", LeaseSeconds: 60})
	owner, _ = w.Planning.scope("root")
	kinds := planningOperationSchemaKinds(t, planningSchemaForDelivery(*owner.Delivery))
	if containsString(kinds, "retry") || !containsString(kinds, "review-retry") {
		t.Fatal("provider offered forbidden production retry", kinds)
	}
	raw, _, err := s.planningDeliveryContext(w, "root", 64000)
	if err != nil {
		t.Fatal(err)
	}
	var ctx struct {
		Tasks []struct {
			Blocker         string         `json:"blocker"`
			PreviousBlocker string         `json:"previous_blocker"`
			Review          map[string]any `json:"current_review"`
		} `json:"tasks"`
	}
	if err = json.Unmarshal(raw, &ctx); err != nil || len(ctx.Tasks) != 1 || ctx.Tasks[0].Blocker != w.Tasks[0].IndependentReview.Reason || ctx.Tasks[0].PreviousBlocker != "old semantic alias refusal" || ctx.Tasks[0].Review["state"] != "error" {
		t.Fatal("old blocker presented as current instead of attributed history", string(raw), err)
	}
	// Restore a semantic refusal with remaining attempts: ordinary correction
	// must stay available; this is a projection restriction, not new authority.
	w.Tasks[0].IndependentReview.State = "changes_requested"
	raw, d, err := s.planningDeliveryContext(w, "root", 64000)
	if err != nil {
		t.Fatal(err)
	}
	if !containsString(planningOperationSchemaKinds(t, planningSchemaForDelivery(d)), "retry") {
		t.Fatal("valid production correction was removed", string(raw), err)
	}
}

func TestPlanningDeliveryReviewRetryRequiresReceivedOwnedCurrentResult(t *testing.T) {
	for _, mode := range []string{"current", "absent", "old-id", "old-attempt", "wrong-kind", "foreign", "semantic", "launch-held"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := planningProtocolReviewFixture(t)
			task, _ := w.task(a.TaskID)
			for i := range w.Planning.Inbox {
				switch mode {
				case "absent":
					w.Planning.Inbox[i] = PlanningEvent{ID: "operator", Scope: "root", Kind: "operator_resume"}
				case "old-id":
					w.Planning.Inbox[i].ID = "older-result"
				case "old-attempt":
					w.Planning.Inbox[i].Attempt = "older-attempt"
				case "wrong-kind":
					w.Planning.Inbox[i].Kind = "handoff"
				case "foreign":
					w.Planning.Inbox[i].Scope = "foreign"
				case "semantic":
					task.IndependentReview.State = "changes_requested"
				case "launch-held":
					task.LaunchHeld = true
				}
			}
			before, _ := json.Marshal(w)
			_, d, err := s.planningDeliveryContext(w, "root", 64000)
			if err != nil {
				t.Fatal(err)
			}
			offered := containsString(planningOperationSchemaKinds(t, planningSchemaForDelivery(d)), "review-retry")
			if offered != (mode == "current") {
				t.Fatal("review retry offer does not match received current event", mode, d.Events, offered)
			}
			after, _ := json.Marshal(w)
			if string(before) != string(after) {
				t.Fatal("projection mutated state")
			}
		})
	}
}

func TestPlanningReviewRetryRestrictionTracksWholeReducedBatch(t *testing.T) {
	s, w, a := planningProtocolReviewFixture(t)
	beforeCalls := w.Planning.Reviewer.Calls
	result := w.Planning.Inbox[0]
	result.Message = strings.Repeat("original result ", 500)
	operator := PlanningEvent{ID: "latest-instruction", Scope: "root", Kind: "operator_resume", Message: "Diagnose the latest review result without inventing acceptance."}
	w.Planning.Inbox = []PlanningEvent{result, operator}
	single := w
	single.Planning = &PlanningState{}
	*single.Planning = *w.Planning
	single.Planning.Inbox = []PlanningEvent{operator}
	raw, _, err := s.planningDeliveryContext(single, "root", 64000)
	if err != nil {
		t.Fatal(err)
	}
	_, reduced, err := s.planningDeliveryContext(w, "root", len(raw)+20)
	if err != nil || !reflect.DeepEqual(reduced.Events, []string{operator.ID}) || !reduced.ReviewRetryUnavailable {
		t.Fatal("omitted current result still authorizes review retry", reduced, err)
	}
	frozen, _ := json.Marshal(reduced)
	var restored PlanningDelivery
	if err = json.Unmarshal(frozen, &restored); err != nil || !restored.ReviewRetryUnavailable {
		t.Fatal("restriction lost in receipt", err)
	}
	_, expanded, err := s.planningDeliveryContext(w, "root", 64000)
	if err != nil || expanded.ReviewRetryUnavailable || !containsString(expanded.Events, result.ID) || !restored.ReviewRetryUnavailable {
		t.Fatal("expanded result admission changed frozen restriction", expanded, err)
	}
	task, _ := w.task(a.TaskID)
	if task.IndependentReview.State != "error" || w.Planning.Reviewer.Calls != beforeCalls {
		t.Fatal("projection repaired review or erased consumption")
	}
}
