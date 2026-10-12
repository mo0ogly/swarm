//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestPlanningIncompleteScopeDoesNotOfferClosureToProvider(t *testing.T) {
	s, w := managedFixture(t)
	w = planningDo(t, s, w, "resume", PlanningRequest{Scope: "root", Reason: "Review the unfinished candidates before any closure"})
	beforeCalls := w.Planning.Reviewer.Calls
	beforeActivations := w.Planning.Activations
	context, delivery, err := s.planningDeliveryContext(w, "root", 64000)
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(planningSchemaForDelivery(delivery), `"enum":["close"]`) {
		t.Fatal("provider still receives an impossible close operation while tasks remain unaccepted")
	}
	var values map[string]any
	if err = json.Unmarshal(context, &values); err != nil {
		t.Fatal(err)
	}
	prerequisites, ok := values["closure_prerequisites"].(map[string]any)
	if !ok || prerequisites["eligible"] != false || prerequisites["reason"] == "" {
		t.Fatal("supervisor did not receive the concrete missing closure prerequisite")
	}
	after, _ := s.get(w.ID)
	if after.Planning.Reviewer.Calls != beforeCalls || after.Planning.Activations != beforeActivations || after.Revision != w.Revision {
		t.Fatal("context inspection spent a call or changed mission state")
	}
	if !strings.Contains(planningSchemaForDelivery(delivery), `"enum":["review-plan"]`) {
		t.Fatal("preventing premature closure removed the candidate recovery operation")
	}
}
