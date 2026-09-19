package main

import (
	"encoding/json"
	"fmt"
	"reflect"
	"strings"
)

// Preserve legacy request encoding while retaining an explicit dependency removal.
func (r Request) MarshalJSON() ([]byte, error) {
	type plain Request
	b, err := json.Marshal(plain(r))
	if err == nil && r.Depends != nil && len(r.Depends) == 0 {
		b = append(b[:len(b)-1], []byte(`,"depends":[]}`)...)
	}
	return b, err
}

// Definition edits change the contract that gates validate, unlike owner/next edits.
func (r Request) editsDefinition() bool {
	return r.Title != "" || r.Deliverable != "" || r.Criteria != nil || r.Depends != nil || r.MaxAttempts != 0 || r.MaxToolCalls != 0 || r.ValidationPolicy != nil
}

func updateTaskDefinition(w *Work, t *Task, r Request) error {
	if !r.editsDefinition() {
		return nil
	}
	if (t.Status != "todo" && t.Status != "blocked") || (r.Status != "" && r.Status != t.Status) {
		return fmt.Errorf("modifier le contrat exige une tâche todo ou blocked, sans transition simultanée ; rouvrir la tâche au préalable")
	}
	for _, other := range w.Tasks {
		if other.Status == "running" {
			return fmt.Errorf("terminer ou arrêter les tâches en cours avant de modifier le contrat")
		}
	}
	next := *t
	if r.ValidationPolicy != nil {
		policy, err := normalizeValidationPolicy(*r.ValidationPolicy)
		if err != nil {
			return err
		}
		policy.Authorized = now()
		policy.Actor = operatorIdentity()
		next.ValidationPolicy = &policy
	}
	if r.MaxAttempts != 0 {
		if r.MaxAttempts < 1 || r.MaxAttempts > 3 {
			return fmt.Errorf("max_attempts : 1 à 3 requis")
		}
		next.PlanMaxAttempts = r.MaxAttempts
	}
	if r.MaxToolCalls != 0 {
		if r.MaxToolCalls < 1 || r.MaxToolCalls > 100 {
			return fmt.Errorf("max_tool_calls : 1 à 100 requis")
		}
		next.PlanToolLimit = r.MaxToolCalls
	}
	if r.Title != "" {
		if !nonempty(r.Title) {
			return fmt.Errorf("title vide")
		}
		next.Title = r.Title
	}
	if r.Deliverable != "" {
		if !nonempty(r.Deliverable) {
			return fmt.Errorf("deliverable vide")
		}
		next.Deliverable = r.Deliverable
	}
	if r.Criteria != nil {
		if len(r.Criteria) == 0 || len(r.Criteria) > 12 {
			return fmt.Errorf("criteria : 1 à 12 critères requis")
		}
		for _, v := range r.Criteria {
			if !nonempty(v) {
				return fmt.Errorf("critère vide")
			}
		}
		next.Criteria = append([]string{}, r.Criteria...)
	}
	if r.Depends != nil {
		next.Depends = append([]string{}, r.Depends...)
	}
	if next.ValidationPolicy != nil {
		if err := validationPolicyCoversTask(*next.ValidationPolicy, &next); err != nil {
			return fmt.Errorf("validation_policy : %w", err)
		}
	}
	// Validate the entire resulting graph before touching the task.
	state := map[string]int{}
	var visit func(string) error
	visit = func(id string) error {
		if state[id] == 1 {
			return fmt.Errorf("cycle de dépendances : %s", id)
		}
		if state[id] == 2 {
			return nil
		}
		node, e := w.task(id)
		if e != nil {
			return e
		}
		if id == t.ID {
			node = &next
		}
		state[id] = 1
		seen := map[string]bool{}
		for _, dep := range node.Depends {
			if seen[dep] {
				return fmt.Errorf("dépendance répétée : %s", dep)
			}
			seen[dep] = true
			if e := visit(dep); e != nil {
				return e
			}
		}
		state[id] = 2
		return nil
	}
	for _, node := range w.Tasks {
		if e := visit(node.ID); e != nil {
			return e
		}
	}
	if next.Title == t.Title && next.Deliverable == t.Deliverable && reflect.DeepEqual(next.Criteria, t.Criteria) && reflect.DeepEqual(next.Depends, t.Depends) && next.PlanMaxAttempts == t.PlanMaxAttempts && next.PlanToolLimit == t.PlanToolLimit && reflect.DeepEqual(next.ValidationPolicy, t.ValidationPolicy) {
		return nil
	}
	next.Gate = nil
	next.AutoValidation = nil
	next.Override = nil
	next.Revalidation = nil
	if next.PlanChecks != nil || r.Criteria != nil {
		checks := map[string]string{"plan-entry": "entry", "plan-validation": "validation", "plan-delivery": "delivery"}
		for id, phase := range next.PlanChecks {
			if !strings.HasPrefix(id, "plan-criterion-") {
				checks[id] = phase
			}
		}
		next.PlanChecks = checks
		for i := range next.Criteria {
			next.PlanChecks[fmt.Sprintf("plan-criterion-%d", i+1)] = "validation"
		}
	}
	*t = next
	return nil
}
