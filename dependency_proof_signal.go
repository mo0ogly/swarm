//go:build linux

package main

import (
	"encoding/json"
	"fmt"
)

// Notify the dependent owner even while its scope is open. Identical proof
// drift has one durable event, including after that event was acknowledged.
func (s *Store) signalDependencyProofDrift(w Work) (Work, error) {
	if w.Planning == nil {
		return w, nil
	}
	events := []PlanningEvent{}
	for _, task := range w.Tasks {
		if task.Status != "todo" && task.Status != "blocked" {
			continue
		}
		dependencies := append([]string(nil), task.Depends...)
		for req := range prerequisiteRequirements(&w, &task) {
			fresh := false
			candidates := []string{}
			for i := range w.Tasks {
				prior := &w.Tasks[i]
				if prior.ID == task.ID || prior.Status != "accepted" || !containsString(prior.Requirements, req) {
					continue
				}
				if s.acceptedFresh(&w, prior, map[string]bool{}) {
					fresh = true
					break
				}
				candidates = append(candidates, prior.ID)
			}
			if !fresh {
				for _, id := range candidates {
					if !containsString(dependencies, id) {
						dependencies = append(dependencies, id)
					}
				}
			}
		}
		for _, id := range dependencies {
			dep, err := w.task(id)
			if err != nil || dep.Status != "accepted" || s.acceptedFresh(&w, dep, map[string]bool{}) {
				continue
			}
			observed := map[string]string{}
			if dep.Gate != nil {
				for path := range dep.Gate.Evaluation.Artifacts {
					local, err := safeReport(s.root, path)
					if err != nil {
						observed[path] = "unavailable:" + err.Error()
					} else {
						digest, err := fileDigest(local)
						if err != nil {
							observed[path] = "unavailable:" + err.Error()
						} else {
							observed[path] = digest
						}
					}
				}
			}
			raw, _ := json.Marshal(struct {
				Task     Task
				Observed map[string]string
			}{*dep, observed})
			eventID := planningEventID("dependency-stale", task.ID, id, hash(raw))
			exists := false
			for _, event := range w.Planning.Inbox {
				if event.ID == eventID {
					exists = true
					break
				}
			}
			if exists {
				continue
			}
			owner, err := w.Planning.scope(task.ScopeID)
			if err != nil {
				continue
			}
			events = append(events, PlanningEvent{ID: eventID, Scope: owner.ID, Kind: "dependency_stale", Task: task.ID,
				Message: fmt.Sprintf("%s : preuve acceptée de %s périmée ; revalidation requise, aucun nouveau producteur imposé", task.ID, id), At: now()})
		}
	}
	if len(events) == 0 {
		return w, nil
	}
	raw, _ := json.Marshal(events)
	return s.mutate(w.ID, "planning.dependency-stale", newID("dependency-stale-"), w.Revision, raw, func(current *Work) error {
		for _, event := range events {
			current.Planning.Inbox = append(current.Planning.Inbox, event)
			scope, err := current.Planning.scope(event.Scope)
			if err != nil {
				return err
			}
			for {
				if scope.State == "closed" {
					scope.State = "ready"
					scope.Generation++
					scope.Holder = ""
					scope.Until = ""
				}
				if scope.Parent == "" {
					break
				}
				scope, err = current.Planning.scope(scope.Parent)
				if err != nil {
					return err
				}
			}
		}
		return nil
	})
}
