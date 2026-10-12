package engine

import (
	"encoding/json"
	"fmt"
)

type planningOperationError struct {
	Index    int
	Kind, ID string
	Err      error
}

func (e *planningOperationError) Error() string {
	return fmt.Sprintf("opération %d %s[%s] : %s", e.Index+1, e.Kind, e.ID, e.Err)
}
func (e *planningOperationError) Unwrap() error { return e.Err }

// Ignore leases, inference counters and the diagnostic itself: none constitutes
// a changed precondition. Owned task/review state, public inputs and explicit
// policy changes do. Other scopes remain independently eligible.
func planningRejectionState(w Work, owner *PlanningScope) string {
	tasks := []Task{}
	existingIDs := []string{}
	for _, task := range w.Tasks {
		existingIDs = append(existingIDs, task.ID)
		if planningScopeWithin(w.Planning, task.ScopeID, owner.ID) {
			tasks = append(tasks, task)
		}
	}
	inputs := []PlanningEvent{}
	for _, event := range w.Planning.Inbox {
		if event.Scope == owner.ID && event.Decision == "" && event.Kind != "planning_rejected" {
			inputs = append(inputs, event)
		}
	}
	children := []map[string]any{}
	for _, child := range w.Planning.Scopes {
		if child.Parent == owner.ID {
			children = append(children, map[string]any{"id": child.ID, "state": child.State, "requirements": child.Requirements})
		}
	}
	var reviewer *ReviewerConfig
	if w.Planning.Reviewer != nil {
		copy := *w.Planning.Reviewer
		copy.Calls = 0
		reviewer = &copy
	}
	raw, _ := json.Marshal(map[string]any{"existing_task_ids": existingIDs, "objective": owner.Objective, "requirements": owner.Requirements, "criteria": w.Criteria, "tasks": tasks, "inputs": inputs, "children": children, "provider": w.Planning.Provider, "provider_digest": w.Planning.ProviderDigest, "route": w.Planning.ModelRoute, "scope_model": owner.ModelSelection, "reviewer": reviewer, "repository": w.Planning.Repository, "max_tasks": w.Planning.MaxTasks, "max_activations": w.Planning.MaxActivations, "max_decisions": w.Planning.MaxDecisions, "scope_task_limit": owner.TaskLimit, "scope_activation_limit": owner.ActivationLimit})
	return hash(raw)
}
func planningRejectionUnchanged(w Work, scope *PlanningScope) bool {
	return scope.RejectionWait != nil && scope.RejectionWait.StateSHA256 == planningRejectionState(w, scope)
}
