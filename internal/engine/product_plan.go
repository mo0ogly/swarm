package engine

import (
	"fmt"
	"reflect"
	"regexp"
)

// Product organisation belongs to the adopted plan. It never creates tasks,
// reviewers, budgets or a second acceptance state.
type ProductPlan struct {
	Mode     string        `json:"mode"`
	Journeys []UserJourney `json:"journeys"`
	Stories  []UserStory   `json:"stories"`
}

type UserJourney struct {
	ID       string   `json:"id"`
	Title    string   `json:"title"`
	Goal     string   `json:"goal"`
	StoryIDs []string `json:"story_ids"`
}

type UserStory struct {
	ID         string   `json:"id"`
	Title      string   `json:"title"`
	User       string   `json:"user"`
	Value      string   `json:"value"`
	Criteria   []string `json:"criteria"`
	Complexity int      `json:"complexity"`
	Risk       string   `json:"risk,omitempty"`
	Depends    []string `json:"depends"`
	TaskIDs    []string `json:"task_ids"`
}

var storyName = regexp.MustCompile(`^s[0-9]+-[a-z0-9]+(?:-[a-z0-9]+)*$`)

func validatePreparedProduct(method string, p ActionPlan) error {
	if method == "ks-product" && p.Product == nil {
		return fmt.Errorf("%s", uiText("Le parcours application complète exige la structure product : parcours, stories et liens vers les tâches. Le brouillon est conservé."))
	}
	return nil
}

func productPhase(phase string) bool {
	return containsString([]string{"frame", "requirements", "stories", "story-review", "architecture", "design-system", "research", "design", "plan", "implement", "review", "deliver"}, phase)
}

func validateProductPlan(p ActionPlan) error {
	tasks := map[string]bool{}
	for _, t := range p.Tasks {
		if t.Phase != "" && !productPhase(t.Phase) {
			return fmt.Errorf("%s : étape produit inconnue : %s", t.ID, t.Phase)
		}
		tasks[t.ID] = true
	}
	if p.Product == nil {
		return nil
	}
	product := p.Product
	if !containsString([]string{"existing", "greenfield", "replacement"}, product.Mode) || len(product.Journeys) < 1 || len(product.Stories) < 1 {
		return fmt.Errorf("Structure produit : mode existing, greenfield ou replacement, parcours et stories requis.")
	}
	stories := map[string]UserStory{}
	for _, s := range product.Stories {
		if !storyName.MatchString(s.ID) || len(s.ID) > 64 || !nonempty(s.Title) || !nonempty(s.User) || !nonempty(s.Value) || len(s.Criteria) == 0 || s.Depends == nil || s.TaskIDs == nil {
			return fmt.Errorf("Story %s : identifiant sN-slug, titre, utilisateur, valeur, critères et listes requis.", s.ID)
		}
		if _, exists := stories[s.ID]; exists || s.Complexity < 1 || s.Complexity > 4 || s.Complexity == 4 && !nonempty(s.Risk) {
			return fmt.Errorf("Story %s : doublon ou complexité invalide ; découper un 5, expliciter le risque d’un 4.", s.ID)
		}
		for _, criterion := range s.Criteria {
			if !nonempty(criterion) {
				return fmt.Errorf("Story %s : critère vide.", s.ID)
			}
		}
		if err := productReferences(s.ID, s.TaskIDs, tasks); err != nil {
			return err
		}
		stories[s.ID] = s
	}
	known := map[string]bool{}
	for id := range stories {
		known[id] = true
	}
	journeys, covered := map[string]bool{}, map[string]bool{}
	for _, j := range product.Journeys {
		if !safeName(j.ID) || len(j.ID) > 64 || journeys[j.ID] || !nonempty(j.Title) || !nonempty(j.Goal) || len(j.StoryIDs) == 0 {
			return fmt.Errorf("Parcours %s : identifiant unique, titre, objectif et stories requis.", j.ID)
		}
		journeys[j.ID] = true
		if err := productReferences(j.ID, j.StoryIDs, known); err != nil {
			return err
		}
		for _, id := range j.StoryIDs {
			covered[id] = true
		}
	}
	state := map[string]int{}
	var visit func(string) error
	visit = func(id string) error {
		if state[id] == 1 {
			return fmt.Errorf("Cycle de stories : %s", id)
		}
		if state[id] == 2 {
			return nil
		}
		state[id] = 1
		s := stories[id]
		if !covered[id] {
			return fmt.Errorf("Story %s sans parcours.", id)
		}
		if err := productReferences(id, s.Depends, known); err != nil {
			return err
		}
		for _, dep := range s.Depends {
			if err := visit(dep); err != nil {
				return err
			}
		}
		state[id] = 2
		return nil
	}
	for _, s := range product.Stories {
		if err := visit(s.ID); err != nil {
			return err
		}
	}
	return nil
}

func productReferences(owner string, refs []string, known map[string]bool) error {
	seen := map[string]bool{}
	for _, id := range refs {
		if !known[id] || seen[id] {
			return fmt.Errorf("%s : référence inconnue ou répétée : %s", owner, id)
		}
		seen[id] = true
	}
	return nil
}

// A changed story contract changes the work required of its linked tasks.
// Presentation changes to journey labels do not invalidate execution evidence.
func productTaskContract(p *ProductPlan, task string) []UserStory {
	var out []UserStory
	if p != nil {
		for _, story := range p.Stories {
			if containsString(story.TaskIDs, task) {
				out = append(out, story)
			}
		}
	}
	return out
}

func productTaskChanged(before, after *ProductPlan, task string) bool {
	if before != nil && after != nil && before.Mode != after.Mode {
		return true
	}
	return !reflect.DeepEqual(productTaskContract(before, task), productTaskContract(after, task))
}
