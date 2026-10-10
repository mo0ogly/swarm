package engine

import (
	"encoding/json"
	"fmt"
	"strings"
)

// Increments contain the public plan contract. Decisions describe recovery;
// they do not create a second scheduler or executable conditional edges.
type PreparationWorkModel struct {
	Version    int                        `json:"version"`
	Increments []PreparationWorkIncrement `json:"increments"`
	Decisions  map[string][]string        `json:"decisions"`
}

type PreparationWorkIncrement struct {
	ID    string                `json:"id"`
	Title map[string]string     `json:"title"`
	Plans map[string]ActionPlan `json:"plans"`
}

func validatePreparationWorkModel(m *PreparationWorkModel) error {
	if m == nil {
		return nil
	}
	if m.Version != 1 || len(m.Increments) == 0 {
		return fmt.Errorf("invalid work model")
	}
	seen := map[string]bool{}
	for _, inc := range m.Increments {
		if !safeName(inc.ID) || seen[inc.ID] {
			return fmt.Errorf("invalid or duplicate increment: %s", inc.ID)
		}
		seen[inc.ID] = true
		for _, lang := range []string{"fr", "en"} {
			p, exists := inc.Plans[lang]
			if !exists || inc.Title[lang] == "" || len(m.Decisions[lang]) == 0 {
				return fmt.Errorf("missing work model translation: %s", lang)
			}
			b, err := json.Marshal(p)
			if err != nil {
				return err
			}
			if _, err = parseActionPlan(string(b)); err != nil {
				return fmt.Errorf("%s/%s: %w", inc.ID, lang, err)
			}
			if err = validatePreparedProduct("ks-product", p); err != nil {
				return err
			}
		}
	}
	return nil
}

func projectPreparationWorkPlan(t PreparationTemplate, r PreparationTemplateAnswers, v *PreparationTemplateCheck) error {
	if t.WorkModel == nil {
		if r.Increment != "" {
			return fmt.Errorf("%s", uiText("Ce modèle ne propose pas de graphe de travail."))
		}
		return nil
	}
	id := r.Increment
	if id == "" {
		id = t.WorkModel.Increments[0].ID
	}
	for _, inc := range t.WorkModel.Increments {
		if inc.ID != id {
			continue
		}
		// Copy before adapting so one request never changes another projection.
		b, _ := json.Marshal(inc.Plans[r.Language])
		var p ActionPlan
		if err := json.Unmarshal(b, &p); err != nil {
			return err
		}
		context := []string{}
		for _, q := range t.Questions {
			answer := strings.TrimSpace(r.Answers[q.ID])
			if containsString(v.Missing, q.ID) {
				p.Questions = append(p.Questions, PlanQuestion{Question: q.Label[r.Language]})
			} else if answer != "" {
				context = append(context, q.Label[r.Language]+": "+answer)
			}
		}
		if len(context) > 0 {
			p.Assumptions = append(p.Assumptions, strings.Join(context, "\n"))
		}
		if v.Intervention != "create" {
			p.Product.Mode = "existing"
		}
		// The intervention remains explicit in the actual plan, including correction
		// reproduction and migration preservation already specified by the catalogue.
		if t.Subject != nil {
			for _, intervention := range t.Subject.Interventions {
				if intervention.ID == v.Intervention {
					p.Assumptions = append(p.Assumptions, intervention.Guidance[r.Language])
				}
			}
		}
		b, _ = json.Marshal(p)
		if len(b) > preparationDocumentLimit {
			return fmt.Errorf("%s", uiText("Brouillon limité à 16 000 octets."))
		}
		if _, err := validateActionPlan(p, false); err != nil {
			return err
		}
		v.WorkPlan, v.Increment = &p, inc.ID
		var graph strings.Builder
		graph.WriteString("\n## " + inc.Title[r.Language] + "\n")
		for _, task := range p.Tasks {
			graph.WriteString(task.ID + " · " + task.Title + " ← " + strings.Join(task.Depends, ", ") + "\n")
		}
		for _, decision := range t.WorkModel.Decisions[r.Language] {
			graph.WriteString("- " + decision + "\n")
		}
		v.Need += graph.String()
		if len(v.Need) > preparationDocumentLimit {
			return fmt.Errorf("%s", uiText("Brouillon limité à 16 000 octets."))
		}
		return nil
	}
	return fmt.Errorf("%s: %s", uiText("Incrément de travail inconnu."), id)
}
