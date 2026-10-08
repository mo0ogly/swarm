package engine

import "fmt"

// Operator-owned ordering rules survive a planner omitting task dependencies.
// Configure them before creating tasks; they are never a planning operation.
func validateRequirementPrerequisites(w *Work) error {
	if len(w.RequirementPrerequisites) == 0 {
		return nil
	}
	known := map[string]bool{}
	for i := range w.Criteria {
		known[fmt.Sprintf("req-%d", i+1)] = true
	}
	for _, task := range w.Tasks {
		for _, id := range task.Requirements {
			if !known[id] {
				return fmt.Errorf("prérequis métier : exigence de tâche inconnue %s", id)
			}
		}
	}
	visiting, done := map[string]bool{}, map[string]bool{}
	var visit func(string) error
	visit = func(id string) error {
		if !known[id] {
			return fmt.Errorf("prérequis métier : exigence inconnue %s", id)
		}
		if visiting[id] {
			return fmt.Errorf("prérequis métier : cycle sur %s", id)
		}
		if done[id] {
			return nil
		}
		visiting[id] = true
		seen := map[string]bool{}
		for _, dep := range w.RequirementPrerequisites[id] {
			if seen[dep] {
				return fmt.Errorf("prérequis métier dupliqué : %s", dep)
			}
			seen[dep] = true
			if err := visit(dep); err != nil {
				return err
			}
		}
		visiting[id], done[id] = false, true
		return nil
	}
	for id := range w.RequirementPrerequisites {
		if err := visit(id); err != nil {
			return err
		}
	}
	return nil
}

func (s *Store) requirementPrerequisiteGuard(w *Work, t *Task) error {
	if err := validateRequirementPrerequisites(w); err != nil {
		return err
	}
	if len(w.RequirementPrerequisites) > 0 && len(t.Requirements) == 0 {
		return &CommandError{Code: "requirement_prerequisite", Message: "Prérequis métier configurés : tâche sans exigence déclarée, lancement refusé"}
	}
	needed := prerequisiteRequirements(w, t)
	for id := range needed {
		covered := false
		for i := range w.Tasks {
			prior := &w.Tasks[i]
			if prior.ID == t.ID || prior.Status != "accepted" {
				continue
			}
			for _, req := range prior.Requirements {
				if req == id && s.acceptedFresh(w, prior, map[string]bool{}) {
					covered = true
					break
				}
			}
			if covered {
				break
			}
		}
		if !covered {
			return &CommandError{Code: "requirement_prerequisite", Message: fmt.Sprintf("%s : prérequis métier %s sans preuve acceptée fraîche", t.ID, id)}
		}
	}
	return nil
}

func prerequisiteRequirements(w *Work, t *Task) map[string]bool {
	needed := map[string]bool{}
	var collect func(string)
	collect = func(id string) {
		for _, dep := range w.RequirementPrerequisites[id] {
			if !needed[dep] {
				needed[dep] = true
				collect(dep)
			}
		}
	}
	for _, id := range t.Requirements {
		collect(id)
	}
	return needed
}
