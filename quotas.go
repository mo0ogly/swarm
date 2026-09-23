package main

import (
	"encoding/json"
	"fmt"
	"io"
	"strings"
)

type QuotaValues struct {
	Planning  int  `json:"planning_activations"`
	Decisions int  `json:"planning_decisions"`
	Reviews   *int `json:"review_calls"`
}
type QuotaAuthorization struct {
	Before QuotaValues `json:"before"`
	After  QuotaValues `json:"after"`
	Actor  string      `json:"actor"`
	At     string      `json:"at"`
	Reason string      `json:"reason"`
}
type QuotaChange struct {
	Schema   int         `json:"schema_version"`
	EventID  string      `json:"event_id"`
	Revision int         `json:"expected_revision"`
	Limits   QuotaValues `json:"limits"`
	Reason   string      `json:"reason"`
}
type QuotaView struct {
	Revision      int                 `json:"revision"`
	Limits        QuotaValues         `json:"limits"`
	Consumed      QuotaValues         `json:"consumed"`
	Remaining     QuotaValues         `json:"remaining"`
	Authorization *QuotaAuthorization `json:"last_authorization,omitempty"`
	Scopes        []PlanningScope     `json:"scopes"`
}

func quotaValues(p *PlanningState) (QuotaValues, QuotaValues) {
	limits := QuotaValues{Planning: p.MaxActivations, Decisions: p.MaxDecisions}
	used := QuotaValues{Planning: p.Activations, Decisions: p.Decisions}
	if p.Reviewer != nil {
		a, b := p.Reviewer.MaxCalls, p.Reviewer.Calls
		limits.Reviews = &a
		used.Reviews = &b
	}
	return limits, used
}
func quotaView(w Work) (QuotaView, error) {
	if w.Planning == nil {
		return QuotaView{}, fmt.Errorf("Cette mission ne dispose pas de planification hiérarchique.")
	}
	limits, used := quotaValues(w.Planning)
	remaining := QuotaValues{Planning: max(0, limits.Planning-used.Planning), Decisions: max(0, limits.Decisions-used.Decisions)}
	if limits.Reviews != nil {
		n := max(0, *limits.Reviews-*used.Reviews)
		remaining.Reviews = &n
	}
	return QuotaView{Revision: w.Revision, Limits: limits, Consumed: used, Remaining: remaining, Authorization: w.Planning.QuotaAuthorization, Scopes: w.Planning.Scopes}, nil
}
func applyQuotaChange(w *Work, r QuotaChange) error {
	if r.Schema != 1 || r.Revision < 1 {
		return fmt.Errorf("Version de demande ou révision invalide.")
	}
	if len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return fmt.Errorf("Expliquez la modification des plafonds (8 à 2 000 caractères).")
	}
	p := w.Planning
	if p == nil {
		return fmt.Errorf("Cette mission ne dispose pas de planification hiérarchique.")
	}
	if r.Limits.Planning < max(1, p.Activations) || r.Limits.Planning > 200 || r.Limits.Decisions < max(1, p.Decisions) || r.Limits.Decisions > 100 {
		return fmt.Errorf("Plafonds incompatibles avec la consommation : planification 1–200, décisions 1–100.")
	}
	if p.Reviewer == nil && r.Limits.Reviews != nil {
		return fmt.Errorf("Aucun vérificateur configuré ; configurez son identité avant son budget.")
	}
	if p.Reviewer != nil && (r.Limits.Reviews == nil || *r.Limits.Reviews < max(1, p.Reviewer.Calls) || *r.Limits.Reviews > 100) {
		return fmt.Errorf("Plafond de vérification requis entre la consommation actuelle (au moins 1) et 100.")
	}
	for _, scope := range p.Scopes {
		if scope.Holder != "" {
			return fmt.Errorf("Un planificateur détient encore une session ; attendez sa libération avant de modifier les plafonds.")
		}
	}
	for _, task := range w.Tasks {
		if task.IndependentReview != nil && task.IndependentReview.State == "running" {
			return fmt.Errorf("Une vérification est en cours ; attendez sa fin avant de modifier les plafonds.")
		}
	}
	before, _ := quotaValues(p)
	p.MaxActivations = r.Limits.Planning
	p.MaxDecisions = r.Limits.Decisions
	if p.Reviewer != nil {
		p.Reviewer.MaxCalls = *r.Limits.Reviews
	}
	after, _ := quotaValues(p)
	p.QuotaAuthorization = &QuotaAuthorization{Before: before, After: after, Actor: operatorIdentity(), At: now(), Reason: strings.TrimSpace(r.Reason)}
	// Existing leases, failures, pauses, sub-scope allocations and all attempt
	// counters are intentionally preserved. Increasing a cap is not acceptance.
	return nil
}
func (s *Store) previewQuotas(work string, r QuotaChange) (any, error) {
	w, err := s.get(work)
	if err != nil {
		return nil, err
	}
	if w.Revision != r.Revision {
		return nil, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	current, err := quotaView(w)
	if err != nil {
		return nil, err
	}
	// get returns a detached Work; never persist this preview.
	if err = applyQuotaChange(&w, r); err != nil {
		return nil, err
	}
	proposed, err := quotaView(w)
	if err != nil {
		return nil, err
	}
	return map[string]any{"current": current, "proposed": proposed}, nil
}
func (s *Store) configureQuotas(work string, r QuotaChange) (Work, error) {
	raw, err := json.Marshal(r)
	if err != nil {
		return Work{}, err
	}
	return s.mutate(work, "quotas.configure", r.EventID, r.Revision, raw, func(w *Work) error { return applyQuotaChange(w, r) })
}
func (s *Store) quotasCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 3 {
		return fmt.Errorf("swarm quotas show|preview|apply <work> [--input quotas.json]")
	}
	if pos[1] == "show" {
		w, err := s.get(pos[2])
		if err != nil {
			return err
		}
		v, err := quotaView(w)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	if pos[1] != "preview" && pos[1] != "apply" {
		return fmt.Errorf("Action de plafonds inconnue.")
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	var r QuotaChange
	if err = strict(raw, &r); err != nil {
		return err
	}
	if pos[1] == "preview" {
		v, err := s.previewQuotas(pos[2], r)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	w, err := s.configureQuotas(pos[2], r)
	if err != nil {
		return err
	}
	return printJSON(out, w)
}
