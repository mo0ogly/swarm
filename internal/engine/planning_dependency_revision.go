package engine

import (
	"encoding/json"
	"fmt"
	"reflect"
)

// Preserve an explicit empty dependency array for the new operation only;
// historical task/delegate request encodings remain unchanged.
func (op PlanningOperation) MarshalJSON() ([]byte, error) {
	type plain PlanningOperation
	raw, err := json.Marshal(plain(op))
	if err == nil && op.Kind == "revise-dependencies" && op.Depends != nil && len(op.Depends) == 0 {
		raw = append(raw[:len(raw)-1], []byte(`,"depends":[]}`)...)
	}
	return raw, err
}
func applyPlanningDependencyRevision(w *Work, scope string, op PlanningOperation, r PlanningRequest) error {
	task, err := w.task(op.ID)
	if err != nil {
		return err
	}
	if task.ScopeID != scope || task.Status != "todo" || len(task.Attempts) != 0 || task.IndependentReview != nil || task.LaunchHeld {
		return fmt.Errorf("adaptation réservée à une tâche future non produite du périmètre")
	}
	if op.Depends == nil || op.Title != "" || op.Deliverable != "" || len(op.Criteria) != 0 || len(op.Requirements) != 0 || op.Next != "" || op.MaxTasks != 0 || op.MaxActivations != 0 {
		return fmt.Errorf("revise-dependencies accepte uniquement id et depends explicite")
	}
	next := *task
	next.Depends = append([]string{}, op.Depends...)
	if err = validateTaskDependencyGraph(w, &next); err != nil {
		return err
	}
	if reflect.DeepEqual(next.Depends, task.Depends) {
		return nil
	}
	before := append([]string{}, task.Depends...)
	task.Depends = next.Depends
	task.Gate = nil
	task.AutoValidation = nil
	task.Override = nil
	task.Revalidation = nil
	details, _ := json.Marshal(map[string]any{"task": task.ID, "previous_depends": before, "depends": task.Depends, "decision": r.EventID, "scope": scope, "inputs": r.Inputs, "observed_revision": w.Revision})
	w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: planningEventID(r.EventID, op.ID+"-dependencies"), Scope: scope, Task: task.ID, Kind: "task_dependencies_changed", Message: string(details), At: now()})
	return nil
}
