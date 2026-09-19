package main

import (
	"encoding/json"
	"fmt"
	"os"
)

// The reviewer is a separate, tool-free process. Its opinion never replaces
// deterministic controls or an explicitly required human acceptance.
type ReviewerConfig struct {
	Failure        string      `json:"failure,omitempty"`
	Provider       string      `json:"provider"`
	ProviderDigest string      `json:"provider_digest"`
	ModelRoute     *ModelRoute `json:"model_route,omitempty"`
	MaxCalls       int         `json:"max_calls"`
	Calls          int         `json:"calls"`
	Authorized     string      `json:"authorized"`
}
type IndependentReview struct {
	ID       string            `json:"id"`
	Attempt  string            `json:"attempt"`
	Producer string            `json:"producer"`
	Reviewer string            `json:"reviewer"`
	Report   string            `json:"report"`
	Digest   string            `json:"sha256"`
	Contract string            `json:"contract"`
	State    string            `json:"state"`
	Reason   string            `json:"reason"`
	Criteria []ReviewCriterion `json:"criteria,omitempty"`
	Started  string            `json:"started"`
	Finished string            `json:"finished,omitempty"`
	Usage    *Usage            `json:"usage,omitempty"`
}
type ReviewCriterion struct {
	Index    int    `json:"index"`
	Verdict  string `json:"verdict"`
	Evidence string `json:"evidence"`
}

func reviewContract(t *Task) string {
	b, _ := json.Marshal([]any{t.Title, t.Deliverable, t.Criteria, t.Depends, t.PlanBriefHash})
	return hash(b)
}
func (s *Store) independentReviewGuard(w *Work, t *Task) error {
	if w.Planning != nil && w.Planning.ReviewerRequired && w.Planning.Reviewer == nil {
		return fmt.Errorf("vérificateur indépendant manquant")
	}
	if w.Planning == nil || w.Planning.Reviewer == nil {
		return nil
	}
	r := t.IndependentReview
	if r == nil || r.State != "passed" {
		return fmt.Errorf("vérification IA indépendante requise avant acceptation")
	}
	if len(t.Attempts) == 0 || r.Attempt != t.Attempts[len(t.Attempts)-1].ID || r.Contract != reviewContract(t) {
		return fmt.Errorf("vérification IA périmée : tentative ou consigne modifiée")
	}
	p, e := safeReport(s.root, r.Report)
	if e != nil {
		return e
	}
	b, e := os.ReadFile(p)
	if e != nil || hash(b) != r.Digest {
		return fmt.Errorf("vérification IA périmée : rapport modifié")
	}
	return nil
}
func (s *Store) reviewerConfig(provider, level string, max int) (*ReviewerConfig, error) {
	if max < 1 || max > 100 {
		return nil, fmt.Errorf("vérification : budget de 1 à 100 appels requis")
	}
	ps, e := s.providers()
	if e != nil {
		return nil, e
	}
	p, ok := ps.Providers[provider]
	if !ok {
		return nil, fmt.Errorf("fournisseur de vérification inconnu")
	}
	if _, e = assistantProvider(p); e != nil {
		return nil, e
	}
	_, route, e := resolveModel(p, level, "planning")
	if e != nil {
		return nil, e
	}
	raw, _ := json.Marshal(p)
	return &ReviewerConfig{Provider: provider, ProviderDigest: hash(raw), ModelRoute: route, MaxCalls: max, Authorized: now()}, nil
}
func (s *Store) configureReviewer(work string, r PlanningRequest) (Work, error) {
	config, e := s.reviewerConfig(r.Provider, r.Level, r.MaxActivations)
	if e != nil {
		return Work{}, e
	}
	raw, _ := json.Marshal(r)
	return s.mutate(work, "planning.configure-reviewer", r.EventID, r.Revision, raw, func(w *Work) error {
		if w.Planning == nil {
			return fmt.Errorf("responsable de mission requis")
		}
		if w.Planning.Repository != nil {
			return fmt.Errorf("revue IA de dépôt géré non disponible ; les contrôles déterministes existants restent requis")
		}
		if w.Planning.Reviewer != nil {
			return fmt.Errorf("vérificateur déjà configuré ; sa configuration est conservée")
		}
		for _, t := range w.Tasks {
			if t.Status == "running" {
				return fmt.Errorf("attendre la fin des exécutants avant configuration")
			}
		}
		w.Planning.Reviewer = config
		w.Planning.ReviewerRequired = true
		return nil
	})
}

func reviewStateLabel(state string) string {
	switch state {
	case "running":
		return "Examen en cours"
	case "passed":
		return "Avis favorable"
	case "changes_requested":
		return "Corrections ou preuves demandées"
	case "error":
		return "Vérification interrompue"
	case "stale":
		return "Avis périmé"
	}
	return "Avis indisponible"
}

func (s *Store) retryIndependentReview(work string, r PlanningRequest) (Work, error) {
	if len(r.Reason) < 8 || len(r.Reason) > 2000 {
		return Work{}, fmt.Errorf("décrire la correction avant une nouvelle vérification (8 à 2000 caractères)")
	}
	raw, _ := json.Marshal(r)
	return s.mutate(work, "review.retry", r.EventID, r.Revision, raw, func(w *Work) error {
		if w.Planning == nil || w.Planning.Reviewer == nil {
			return fmt.Errorf("vérificateur absent")
		}
		cfg := w.Planning.Reviewer
		if cfg.Calls >= cfg.MaxCalls {
			return fmt.Errorf("budget du vérificateur atteint ; aucun appel supplémentaire autorisé")
		}
		t, e := w.task(r.Task)
		if e != nil {
			return e
		}
		v := t.IndependentReview
		if t.Status != "submitted" || v == nil || (v.State != "error" && v.State != "stale") {
			return fmt.Errorf("seule une vérification interrompue ou périmée d’un résultat soumis peut être reprise")
		}
		// The former record remains in the event history. Call reservations are never refunded.
		t.IndependentReview = nil
		cfg.Failure = ""
		return nil
	})
}
