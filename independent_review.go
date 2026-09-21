package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

// The reviewer is a separate, tool-free process. Its opinion never replaces
// deterministic controls or an explicitly required human acceptance.
type ReviewerConfig struct {
	TimeoutSeconds int         `json:"timeout_seconds,omitempty"`
	Failure        string      `json:"failure,omitempty"`
	Provider       string      `json:"provider"`
	ProviderDigest string      `json:"provider_digest"`
	ModelRoute     *ModelRoute `json:"model_route,omitempty"`
	MaxCalls       int         `json:"max_calls"`
	Calls          int         `json:"calls"`
	Authorized     string      `json:"authorized"`
}
type IndependentReview struct {
	ReplyPath         string              `json:"reply_path,omitempty"`
	ReplyDigest       string              `json:"reply_sha256,omitempty"`
	TimeoutSeconds    int                 `json:"timeout_seconds,omitempty"`
	Workflow          *AgentWorkflow      `json:"workflow,omitempty"`
	GitReport         string              `json:"git_report,omitempty"`
	CandidateSHA      string              `json:"candidate_commit,omitempty"`
	PreviousCandidate string              `json:"previous_candidate,omitempty"`
	Receipt           string              `json:"receipt,omitempty"`
	ReceiptDigest     string              `json:"receipt_sha256,omitempty"`
	Context           string              `json:"context,omitempty"`
	ContextDigest     string              `json:"context_sha256,omitempty"`
	ManagedTasks      []ManagedTaskReview `json:"managed_tasks,omitempty"`
	ID                string              `json:"id"`
	Attempt           string              `json:"attempt"`
	Producer          string              `json:"producer"`
	Reviewer          string              `json:"reviewer"`
	Report            string              `json:"report"`
	Digest            string              `json:"sha256"`
	Contract          string              `json:"contract"`
	State             string              `json:"state"`
	Reason            string              `json:"reason"`
	Criteria          []ReviewCriterion   `json:"criteria,omitempty"`
	Started           string              `json:"started"`
	Finished          string              `json:"finished,omitempty"`
	Usage             *Usage              `json:"usage,omitempty"`
}

// Zero keeps historical configurations at their original 90-second deadline.
func reviewTimeoutSeconds(cfg *ReviewerConfig) (int, error) {
	if cfg == nil {
		return 0, fmt.Errorf("vérificateur absent")
	}
	if cfg.TimeoutSeconds == 0 {
		return 90, nil
	}
	if cfg.TimeoutSeconds < 1 || cfg.TimeoutSeconds > 900 {
		return 0, fmt.Errorf("délai de revue : 1 à 900 secondes requis")
	}
	return cfg.TimeoutSeconds, nil
}

func (s *Store) setReviewTimeout(work string, r PlanningRequest) (Work, error) {
	if r.ReviewTimeoutSeconds < 1 || r.ReviewTimeoutSeconds > 900 || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return Work{}, fmt.Errorf("délai de revue : 1 à 900 secondes et motif explicite de 8 à 2000 caractères requis")
	}
	raw, _ := json.Marshal(r)
	return s.mutate(work, "review.timeout", r.EventID, r.Revision, raw, func(w *Work) error {
		if w.Planning == nil || w.Planning.Reviewer == nil {
			return fmt.Errorf("vérificateur absent")
		}
		for _, task := range w.Tasks {
			if task.IndependentReview != nil && task.IndependentReview.State == "running" {
				return fmt.Errorf("attendre la fin de la revue avant de modifier son délai")
			}
		}
		w.Planning.Reviewer.TimeoutSeconds = r.ReviewTimeoutSeconds
		return nil
	})
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
	if w.Planning != nil && w.Planning.Reviewer == nil {
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
	if w.Planning.Repository != nil {
		return s.managedIndependentReviewGuard(w, t)
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
	managedProducer := ""
	return s.mutateWithHook(work, "review.retry", r.EventID, r.Revision, raw, func(w *Work) error {
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
		managed := w.Planning.Repository != nil
		if (t.Status != "submitted" && !(managed && t.Status == "blocked")) || v == nil || (v.State != "error" && v.State != "stale") {
			return fmt.Errorf("seule une vérification interrompue ou périmée d’un résultat soumis peut être reprise")
		}
		if managed {
			if len(t.Attempts) == 0 || v.Attempt != t.Attempts[len(t.Attempts)-1].ID || v.CandidateSHA == "" {
				return fmt.Errorf("tentative gérée remplacée ou candidat absent")
			}
			managedProducer = v.Producer
		}
		// The former record remains in the event history. Call reservations are never refunded.
		t.IndependentReview = nil
		cfg.Failure = ""
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
		if managedProducer == "" {
			return nil
		}
		result, e := tx.Exec("UPDATE managed_attempts SET state='integrating',detail='' WHERE agent_id=? AND state IN ('conflict','integrating') AND result_commit!=''", managedProducer)
		if e != nil {
			return e
		}
		n, e := result.RowsAffected()
		if e != nil {
			return e
		}
		if n != 1 {
			return fmt.Errorf("candidat de reprise introuvable")
		}
		return nil
	})
}
