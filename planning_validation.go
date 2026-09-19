package main

import (
	"fmt"
	"strings"
)

// Only the operator's enable request defines commands. The model supplies
// requirement IDs; it cannot replace a command, its directory or its timeout.
func normalizePlanningChecks(w *Work, checks map[string][]ValidationControl) (map[string][]ValidationControl, error) {
	out := map[string][]ValidationControl{}
	for req, controls := range checks {
		known := false
		for i := range w.Criteria {
			if req == fmt.Sprintf("req-%d", i+1) {
				known = true
			}
		}
		if !known {
			return nil, fmt.Errorf("contrôles pour une exigence inconnue : %s", req)
		}
		for i := range controls {
			controls[i].Criteria = []int{1}
		}
		p, e := normalizeValidationPolicy(ValidationPolicy{Mode: "automatic", Controls: controls})
		if e != nil {
			return nil, e
		}
		out[req] = p.Controls
	}
	return out, nil
}
func inheritPlanningChecks(w *Work, t *Task) error {
	if w.Planning.HumanReviewAuthorized != "" {
		t.ValidationPolicy = &ValidationPolicy{Mode: "human", Authorized: w.Planning.HumanReviewAuthorized, Actor: "autorisation initiale de la mission"}
		return nil
	}
	if len(w.Planning.Checks) == 0 {
		return nil
	}
	for _, req := range t.Requirements {
		if len(w.Planning.Checks[req]) == 0 {
			return nil
		}
	}
	policy := ValidationPolicy{Mode: "automatic", Authorized: now(), Actor: "autorisation initiale de la mission"}
	criteria := []string{}
	for i, req := range t.Requirements {
		for n, text := range w.Criteria {
			if req == fmt.Sprintf("req-%d", n+1) {
				criteria = append(criteria, text)
			}
		}
		for _, control := range w.Planning.Checks[req] {
			control.Criteria = []int{i + 1}
			control.ID = "check-" + hash([]byte(req + ":" + control.ID))[:24]
			policy.Controls = append(policy.Controls, control)
		}
	}
	normalized, err := normalizeValidationPolicy(policy)
	if err != nil {
		return err
	}
	// A model may suggest additional local checks, but only the original
	// requirements carry the operator's automatic acceptance authorization.
	t.Next += "\nPrécisions de réalisation proposées : " + strings.Join(t.Criteria, " ; ")
	t.Criteria = criteria
	normalized.Authorized, normalized.Actor = policy.Authorized, policy.Actor
	t.ValidationPolicy = &normalized
	return validationPolicyCoversTask(normalized, t)
}
