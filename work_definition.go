package main

import "fmt"

// Reframe an unstarted work without replacing its tasks or event history.
func updateWorkDefinition(w *Work, r Request) error {
	if r.Title == "" && r.Objective == "" && r.Scope == "" && r.Criteria == nil {
		return fmt.Errorf("title, objective, scope ou criteria requis")
	}
	if r.Status != "" || r.ID != "" || r.Depends != nil || r.Deliverable != "" || r.MaxAttempts != 0 || r.MaxToolCalls != 0 {
		return fmt.Errorf("champs réservés aux tâches")
	}
	for _, task := range w.Tasks {
		if task.Status != "todo" && task.Status != "blocked" {
			return fmt.Errorf("rouvrir les tâches avant de modifier le contrat du travail")
		}
	}
	for name, value := range map[string]string{"title": r.Title, "objective": r.Objective, "scope": r.Scope} {
		if value != "" && !nonempty(value) {
			return fmt.Errorf("%s vide", name)
		}
	}
	if r.Criteria != nil {
		if len(r.Criteria) == 0 {
			return fmt.Errorf("criteria vide")
		}
		for _, v := range r.Criteria {
			if !nonempty(v) {
				return fmt.Errorf("critère vide")
			}
		}
	}
	if r.Title != "" {
		w.Title = r.Title
	}
	if r.Objective != "" {
		w.Objective = r.Objective
	}
	if r.Scope != "" {
		w.Scope = r.Scope
	}
	if r.Criteria != nil {
		w.Criteria = r.Criteria
	}
	for i := range w.Tasks {
		w.Tasks[i].Gate = nil
		w.Tasks[i].Override = nil
		w.Tasks[i].Revalidation = nil
	}
	return nil
}
