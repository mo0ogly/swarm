package engine

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
	diagnostics, operator, reviewResults, actionable, current, historical := []PlanningEvent{}, []PlanningEvent{}, []PlanningEvent{}, []PlanningEvent{}, []PlanningEvent{}, []PlanningEvent{}
	for _, event := range p.Inbox {
		if event.Scope != id || event.Decision != "" {
			continue
		}
		stale, needsAction, currentReview := !explorationCurrent(w, event.Exploration), false, false
		if event.Task != "" {
			task, err := w.task(event.Task)
			stale = err != nil || (event.Attempt != "" && !currentTaskAttempt(task, event.Attempt))
			needsAction = !stale && task.Status == "blocked"
			currentReview = needsAction && task.ScopeID == id && event.Kind == "independent_review" && task.IndependentReview != nil && event.ID == reviewPlanningEventID(task.IndependentReview) && event.Attempt == task.IndependentReview.Attempt
		}
		if stale {
			historical = append(historical, event)
		} else if event.Kind == "planning_rejected" {
			diagnostics = append(diagnostics, event)
		} else if event.Kind == "operator_resume" {
			operator = append(operator, event)
		} else if currentReview {
			reviewResults = append(reviewResults, event)
		} else if needsAction {
			actionable = append(actionable, event)
		} else {
			current = append(current, event)
		}
	}
	// Historical returns are read-only context while current inputs need action.
	// Mixing them in the admitted batch would forbid every operational proposal.
	// Once current inputs are acknowledged, explicitly admit a historical-only
	// batch for acknowledgement. Projection never consumes an event itself.
	pending := len(diagnostics) + len(operator) + len(reviewResults) + len(actionable) + len(current) + len(historical)
	// Public resumption instructions change the precondition for this activation.
	// Prefer the latest committed instruction even when reports reduce the batch
	// to a single event. Older inputs remain pending; timestamps are not authority.
	selected := []PlanningEvent{}
	for i := len(diagnostics) - 1; i >= 0; i-- {
		selected = append(selected, diagnostics[i])
	}
	if len(operator) > 0 {
		selected = append(selected, operator[len(operator)-1])
	}
	// The current result must accompany the new instruction before old resumes
	// displace it. A review-retry cannot use a result outside the received batch.
	selected = append(selected, reviewResults...)
	selected = append(selected, actionable...)
	for i := len(operator) - 2; i >= 0; i-- {
		selected = append(selected, operator[i])
	}
	selected = append(selected, current...)
	historicalOnly := len(selected) == 0 && len(historical) > 0
	readonlyHistory := []PlanningEvent{}
	if historicalOnly {
		selected = historical
	} else {
		for _, event := range historical {
			if len(readonlyHistory) < eventLimit {
				readonlyHistory = append(readonlyHistory, event)
			}
		}
	}
	relevant := map[string]bool{}
	for _, event := range selected {
		if len(events) < eventLimit {
			events = append(events, event)
			relevant[event.Task] = true
		}
	}
	unresolved := []PlanningEvent{}
	if !historicalOnly && len(events) > 0 {
		unresolved = planningUnresolvedReviewIncidents(w, id)
	}
	reviewInputs := append(append([]PlanningEvent(nil), events...), unresolved...)
	reviewRetryAvailable := false
	for taskID := range planningReviewFailureRouting(w, reviewInputs) {
		task, _ := w.task(taskID)
		if task.Status == "blocked" && !task.LaunchHeld {
			reviewRetryAvailable = true
		}
	}
	existingIDs := []string{}
	for _, task := range w.Tasks {
		existingIDs = append(existingIDs, task.ID)
	}
	tasks := []map[string]any{}
	counts := map[string]int{}
	omitted := 0
	productionRetryAvailable := false
	for _, task := range w.Tasks {
		if task.ScopeID != id {
			continue
		}
		counts[task.Status]++
		if task.Status == "blocked" && !task.LaunchHeld && !task.PlanningRetry && len(task.Attempts) < task.PlanMaxAttempts && !currentReviewProtocolFailure(&task) {
			productionRetryAvailable = true
		}
		if !relevant[task.ID] && len(tasks) >= 20 {
			omitted++
			continue
		}
		row := map[string]any{"id": task.ID, "title": task.Title, "status": task.Status, "requirements": task.Requirements, "dependencies": task.Depends, "next": guardBlock(task.Next, 1000), "review_coordination": task.ReviewCoordination, "instruction_revisions_count": len(task.InstructionRevisions), "attempts_used": len(task.Attempts), "attempts_max": task.PlanMaxAttempts, "blocker": guardBlock(task.Blocker, 1000)}
		if len(task.MemorySelections) > 0 {
			row["memory_selections"] = task.MemorySelections
			row["memory_decision"] = task.MemoryDecision
		}
		if review := task.IndependentReview; review != nil {
			row["current_review"] = map[string]any{"id": review.ID, "state": review.State, "candidate_commit": review.CandidateSHA, "reason": guardBlock(review.Reason, 1000)}
			if task.Status == "blocked" && review.State == "error" && currentTaskAttempt(&task, review.Attempt) {
				if task.Blocker != "" && task.Blocker != review.Reason {
					row["previous_blocker"] = guardBlock(task.Blocker, 1000)
				}
				row["blocker"] = guardBlock(review.Reason, 1000)
			}
		}
		tasks = append(tasks, row)
	}
	requirements := map[string]string{}
	for _, req := range scope.Requirements {
		for i, text := range w.Criteria {
			if req == fmt.Sprintf("req-%d", i+1) {
				requirements[req] = text
			}
		}
	}
	// Rebuild the root's live cycle from durable state; full historical reasons
	// remain in the work/event log, outside this bounded activation.
	var cycle map[string]any
	if id == "root" && p.Judge != nil {
		j := p.Judge
		cycle = map[string]any{"cycle": j.Cycle, "max_cycles": j.MaxCycles, "verdict": j.Verdict, "reason": j.Reason, "evidence_ref": j.EvidenceRef, "closed": j.Closed, "history_records": len(j.History)}
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
	return json.Marshal(map[string]any{"global_cycle": cycle, "exploration_repository": planningExplorationRepository(w), "existing_task_ids": existingIDs, "task_creation_rule": "task crée seulement un identifiant global nouveau. Ne pas recréer ni remplacer une tâche existante pour reprendre son travail ; proposer uniquement l’opération adaptée à son état actuel.", "delivery_evidence_contract": deliveryEvidenceBoundary, "review_source_contract": reviewSourceBoundary, "scope": scope, "requirements": requirements, "events": events, "unresolved_review_incidents": unresolved, "historical_events": readonlyHistory, "historical_events_total": len(historical), "historical_only": historicalOnly, "production_retry_unavailable": !productionRetryAvailable, "review_retry_unavailable": !reviewRetryAvailable, "pending_events_total": pending, "tasks": tasks, "task_counts": counts, "tasks_not_in_projection": omitted, "children": children, "task_capacity_remaining": p.MaxTasks - len(w.Tasks), "delegated_requirements": delegated, "projection_note": "Historique et documents de preuve exclus. Les événements restants seront remis au prochain tour ; next est un extrait d’affichage limité à 1000 caractères. La clôture est contrôlée sur l’état complet."})
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
