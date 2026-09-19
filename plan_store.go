package main

import (
	"encoding/json"
	"fmt"
	"strings"
)

func planSource(w Work, source, briefHash, responseHash string) (*Task, error) {
	t, e := w.task(source)
	if e != nil {
		return nil, e
	}
	if !t.Brainstorm || t.PlanBriefHash == "" || w.PlanningBrief == nil || t.PlanBriefHash != w.PlanningBrief.SHA256 || briefHash != t.PlanBriefHash {
		return nil, fmt.Errorf("Le brief a changé ou cette réponse n’est pas un plan structuré. Préparer un nouveau plan depuis le brief courant.")
	}
	if t.Status == "running" || !nonempty(t.Response) {
		return nil, fmt.Errorf("Attendre une réponse finale du planner.")
	}
	if responseHash != "" && responseHash != hash([]byte(t.Response)) {
		return nil, fmt.Errorf("Réponse modifiée : rouvrir la revue du plan.")
	}
	return t, nil
}
func (s *Store) readPlan(work, source string) (PlanReview, error) {
	w, e := s.get(work)
	if e != nil {
		return PlanReview{}, e
	}
	if w.PlanningBrief == nil {
		return PlanReview{}, fmt.Errorf("Adopter un brief avant de préparer un plan.")
	}
	t, e := planSource(w, source, w.PlanningBrief.SHA256, "")
	if e != nil {
		return PlanReview{}, e
	}
	p, e := parseActionPlan(t.Response)
	return PlanReview{Source: source, BriefHash: t.PlanBriefHash, ResponseHash: hash([]byte(t.Response)), Spec: p}, e
}
func (s *Store) commitPlan(work, event string, revision int, review PlanReview) (Work, error) {
	if review.ResponseHash == "" {
		return Work{}, fmt.Errorf("Empreinte de la réponse relue obligatoire.")
	}
	raw, _ := json.Marshal(review)
	return s.mutate(work, "plan.adopt", event, revision, raw, func(w *Work) error {
		for _, p := range w.Plans {
			if p.Source == review.Source {
				return fmt.Errorf("Ce plan est déjà enregistré. Retrouvez ses tâches dans le plan de travail.")
			}
		}
		t, e := planSource(*w, review.Source, review.BriefHash, review.ResponseHash)
		if e != nil {
			return e
		}
		original, parseError := parseActionPlan(t.Response)
		if parseError != nil {
			return parseError
		}
		if len(original.Questions) != len(review.Spec.Questions) {
			return fmt.Errorf("Les décisions ouvertes ne peuvent pas être supprimées. Renseigner leurs réponses.")
		}
		for i, q := range original.Questions {
			if q.Question != review.Spec.Questions[i].Question {
				return fmt.Errorf("Question source modifiée : préparer un nouveau plan si elle n’est plus applicable.")
			}
		}
		return s.materializePlan(w, review)
	})
}

// Shared mission contract for legacy review and preparation conversion.
func (s *Store) materializePlan(w *Work, review PlanReview) error {
	ordered, e := validateActionPlan(review.Spec, true)
	if e != nil {
		return e
	}
	prefix := "plan-" + hash([]byte(review.Source))[:10] + "-"
	ids := []string{}
	for _, m := range ordered {
		deps := []string{}
		for _, id := range m.Depends {
			deps = append(deps, prefix+id)
		}
		next := fmt.Sprintf("Périmètre : %s\nPreuves : %s\nGate entry : %s\nGate validation : %s\nGate delivery : %s\nConditions d’arrêt : %s\nOODA : consigner observation, orientation, décision et résultat sur blocage.\nObjectif du plan : %s\nHypothèses : %s", m.Scope, m.Proof, m.Entry, m.Validation, m.Delivery, m.Stop, review.Spec.Objective, strings.Join(review.Spec.Assumptions, " ; "))
		for _, q := range review.Spec.Questions {
			next += "\nDécision : " + q.Question + " → " + q.Answer
		}
		id := prefix + m.ID
		if e = s.apply(w, "task.add", Request{ID: id, Title: m.Title, Deliverable: m.Deliverable, Criteria: m.Criteria, Depends: deps, Owner: m.Role, Next: next}); e != nil {
			return e
		}
		task := &w.Tasks[len(w.Tasks)-1]
		task.PlanChecks = map[string]string{"plan-entry": "entry", "plan-validation": "validation", "plan-delivery": "delivery"}
		for i := range m.Criteria {
			task.PlanChecks[fmt.Sprintf("plan-criterion-%d", i+1)] = "validation"
		}
		task.Next += "\nCadre de gate obligatoire : checks plan-entry (entry), plan-validation (validation), plan-delivery (delivery) et plan-criterion-N pour chaque critère (validation). Tous mandatory=true, avec résultats et preuves réelles. Aucun PASS avant exécution.\n"
		task.PlanRole = m.Role
		task.PlanMaxAttempts = m.MaxAttempts
		task.PlanToolLimit = m.MaxToolCalls
		ids = append(ids, id)
	}
	w.Plans = append(w.Plans, ApprovedPlan{Source: review.Source, BriefHash: review.BriefHash, ResponseHash: review.ResponseHash, Spec: review.Spec, TaskIDs: ids, At: now()})
	return nil
}
