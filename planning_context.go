package main

import (
	"encoding/json"
	"fmt"
)

// Exact bounded projections: no gate documents or past transcripts are copied
// into each activation. Pending events are batched, never marked consumed by
// compression. The remainder will cause another activation after this one.
func planningContext(w Work, id string) ([]byte, error) {
	return planningContextLimit(w, id, 8)
}

func planningContextLimit(w Work, id string, eventLimit int) ([]byte, error) {
	p := w.Planning
	scope, e := p.scope(id)
	if e != nil {
		return nil, e
	}
	events := []PlanningEvent{}
	pending := 0
	relevant := map[string]bool{}
	for _, event := range p.Inbox {
		if event.Scope == id && event.Decision == "" {
			pending++
			if len(events) < eventLimit {
				events = append(events, event)
				relevant[event.Task] = true
			}
		}
	}
	tasks := []map[string]any{}
	counts := map[string]int{}
	omitted := 0
	for _, task := range w.Tasks {
		if task.ScopeID != id {
			continue
		}
		counts[task.Status]++
		if !relevant[task.ID] && len(tasks) >= 20 {
			omitted++
			continue
		}
		tasks = append(tasks, map[string]any{"id": task.ID, "title": task.Title, "status": task.Status, "requirements": task.Requirements, "dependencies": task.Depends, "next": guardBlock(task.Next, 1000), "attempts_used": len(task.Attempts), "attempts_max": task.PlanMaxAttempts, "blocker": task.Blocker})
	}
	requirements := map[string]string{}
	for _, req := range scope.Requirements {
		for i, text := range w.Criteria {
			if req == fmt.Sprintf("req-%d", i+1) {
				requirements[req] = text
			}
		}
	}
	delegated := map[string]string{}
	children := []map[string]any{}
	for _, child := range p.Scopes {
		if child.ID != id && planningScopeWithin(p, child.ID, id) {
			for _, req := range child.Requirements {
				for i, text := range w.Criteria {
					if req == fmt.Sprintf("req-%d", i+1) {
						delegated[req] = text
					}
				}
			}
		}
		if child.Parent == id {
			children = append(children, map[string]any{"id": child.ID, "state": child.State, "requirements": child.Requirements})
		}
	}
	return json.Marshal(map[string]any{"scope": scope, "requirements": requirements, "events": events, "pending_events_total": pending, "tasks": tasks, "task_counts": counts, "tasks_not_in_projection": omitted, "children": children, "task_capacity_remaining": p.MaxTasks - len(w.Tasks), "delegated_requirements": delegated, "projection_note": "Historique et documents de preuve exclus. Les événements restants seront remis au prochain tour ; next est un extrait d’affichage limité à 1000 caractères. La clôture est contrôlée sur l’état complet."})
}

func planningScopeWithin(p *PlanningState, ownerID, ancestor string) bool {
	seen := map[string]bool{}
	for !seen[ownerID] {
		seen[ownerID] = true
		owner, err := p.scope(ownerID)
		if err != nil {
			return false
		}
		if owner.ID == ancestor {
			return true
		}
		if owner.Parent == "" {
			return false
		}
		ownerID = owner.Parent
	}
	return false
}
