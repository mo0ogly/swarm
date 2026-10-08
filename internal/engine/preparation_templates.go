package engine

import (
	"encoding/json"
	"fmt"
	"strings"
	resources "swarm.local/companion"
)

// Templates are data, never executable commands or launch permissions.
var preparationTemplateFiles = resources.Preparations

type PreparationTemplate struct {
	ID                string                        `json:"id"`
	Version           int                           `json:"version"`
	Title             map[string]string             `json:"title"`
	Need              map[string]string             `json:"need"`
	Phases            []string                      `json:"phases"`
	RecommendedMethod string                        `json:"recommended_method"`
	Questions         []PreparationTemplateQuestion `json:"questions"`
	Team              []PreparationTemplateRole     `json:"team"`
}

type PreparationTemplateQuestion struct {
	ID       string            `json:"id"`
	Label    map[string]string `json:"label"`
	Line     map[string]string `json:"line"`
	Required bool              `json:"required"`
}
type PreparationTemplateRole struct {
	Role           string            `json:"role"`
	Title          map[string]string `json:"title"`
	Responsibility map[string]string `json:"responsibility"`
	MethodGuidance map[string]string `json:"method_guidance"`
	Workflow       *AgentWorkflow    `json:"workflow"`
}
type PreparationTemplateAnswers struct {
	TemplateID string            `json:"template_id"`
	Language   string            `json:"language"`
	Answers    map[string]string `json:"answers"`
}
type PreparationTemplateCheck struct {
	TemplateID       string                    `json:"template_id"`
	Version          int                       `json:"version"`
	Title            string                    `json:"title"`
	Need             string                    `json:"need"`
	Missing          []string                  `json:"missing"`
	Answered         int                       `json:"answered"`
	Total            int                       `json:"total"`
	NeedComplete     bool                      `json:"need_complete"`
	LaunchAuthorized bool                      `json:"launch_authorized"`
	Team             []PreparationTemplateRole `json:"proposed_team"`
}

// This checks draft completeness only. It neither creates agents nor grants
// readiness/authorization to a plan; normal plan validation remains mandatory.
func checkPreparationTemplate(r PreparationTemplateAnswers) (PreparationTemplateCheck, error) {
	t, e := preparationTemplate(r.TemplateID)
	if e != nil {
		return PreparationTemplateCheck{}, e
	}
	if r.Language == "" {
		r.Language = "fr"
	}
	if r.Language != "fr" && r.Language != "en" {
		return PreparationTemplateCheck{}, fmt.Errorf("%s", uiText("Langue attendue : fr ou en."))
	}
	known := map[string]bool{}
	for _, q := range t.Questions {
		known[q.ID] = true
	}
	for key, answer := range r.Answers {
		if !known[key] || len(answer) > 1000 {
			return PreparationTemplateCheck{}, fmt.Errorf("%s: %s", uiText("Réponse inconnue ou trop longue (1 000 octets maximum)."), key)
		}
	}
	v := PreparationTemplateCheck{TemplateID: t.ID, Version: t.Version, Title: t.Title[r.Language], Need: t.Need[r.Language], Missing: []string{}, Total: len(t.Questions), Team: t.Team}
	for _, q := range t.Questions {
		answer := strings.TrimSpace(r.Answers[q.ID])
		if answer == "" || strings.Contains(answer, "[à préciser") || strings.Contains(answer, "[define") || strings.Contains(answer, "[describe") {
			if q.Required {
				v.Missing = append(v.Missing, q.ID)
			}
			continue
		}
		line := q.Line[r.Language]
		start, end := strings.Index(line, "["), strings.LastIndex(line, "]")
		if start < 0 || end < start || strings.Count(v.Need, line) != 1 {
			return PreparationTemplateCheck{}, fmt.Errorf("%s: %s", uiText("Question du modèle invalide."), q.ID)
		}
		filled := line[:start] + answer + line[end+1:]
		v.Need = strings.Replace(v.Need, line, filled, 1)
		v.Answered++
	}
	if len(v.Need) > 16000 {
		return PreparationTemplateCheck{}, fmt.Errorf("%s", uiText("Brouillon limité à 16 000 octets."))
	}
	v.NeedComplete = len(v.Missing) == 0
	return v, nil
}

func preparationTemplates() ([]PreparationTemplate, error) {
	b, e := preparationTemplateFiles.ReadFile("tools/agent-workflows/templates/preparations.json")
	if e != nil {
		return nil, e
	}
	var v []PreparationTemplate
	e = json.Unmarshal(b, &v)
	if e != nil {
		return nil, e
	}
	// Use the same frozen role framing as agent launches, not a second list.
	for i := range v {
		for j := range v[i].Team {
			w, _, err := agentWorkflow(v[i].Team[j].Role)
			if err != nil {
				return nil, err
			}
			v[i].Team[j].Workflow = &w
		}
	}
	return v, e
}
func preparationTemplate(id string) (PreparationTemplate, error) {
	// Historical catalogue IDs remain readable; newly returned IDs are neutral.
	if current, ok := map[string]string{"ks-feature": "application", "ks-bugfix": "correction", "ks-interface": "interface", "ks-product": "product"}[id]; ok {
		id = current
	}
	ts, e := preparationTemplates()
	if e != nil {
		return PreparationTemplate{}, e
	}
	for _, t := range ts {
		if t.ID == id {
			return t, nil
		}
	}
	return PreparationTemplate{}, preparationError("not_found", uiText("Modèle de mission introuvable."))
}
