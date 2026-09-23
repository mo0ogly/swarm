package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"io"
)

type RoleModel struct {
	Provider       string      `json:"provider"`
	ProviderDigest string      `json:"provider_digest"`
	Route          *ModelRoute `json:"route"`
	Actor          string      `json:"actor"`
	At             string      `json:"at"`
}
type RoleModelRequest struct {
	Schema     int    `json:"schema_version"`
	EventID    string `json:"event_id"`
	Revision   int    `json:"expected_revision"`
	Scope      string `json:"scope"`
	Reviewer   bool   `json:"reviewer"`
	Provider   string `json:"provider"`
	Level      string `json:"level"`
	PolicyHash string `json:"model_policy_hash"`
	Inherit    bool   `json:"inherit"`
}

func effectivePlanningScope(p *PlanningState, scope *PlanningScope) *PlanningState {
	copy := *p
	if scope.ModelSelection != nil {
		m := scope.ModelSelection
		copy.Provider = m.Provider
		copy.ProviderDigest = m.ProviderDigest
		copy.ModelRoute = m.Route
	}
	return &copy
}
func (s *Store) prepareRoleModel(w *Work, r RoleModelRequest) error {
	if r.Schema != 1 || r.Revision < 1 {
		return fmt.Errorf("Version de demande ou révision invalide.")
	}
	p := w.Planning
	if p == nil {
		return fmt.Errorf("Cette mission ne dispose pas de planification hiérarchique.")
	}
	if r.Reviewer && r.Scope != "" || !r.Reviewer && r.Scope == "" {
		return fmt.Errorf("Choisissez un responsable ou le vérificateur.")
	}
	for _, scope := range p.Scopes {
		if scope.Holder != "" {
			return fmt.Errorf("Une décision est en cours ; attendez sa fin avant de changer le modèle.")
		}
	}
	for _, t := range w.Tasks {
		if t.IndependentReview != nil && (t.IndependentReview.State == "running" || len(t.IndependentReview.Batches) > 0 && t.IndependentReview.State != "passed") || t.BatchReviewResume != nil {
			return fmt.Errorf("Une revue est active ou reprend des lots ; terminez-la avant de changer le modèle.")
		}
	}
	var scope *PlanningScope
	if r.Reviewer {
		if p.Reviewer == nil {
			return fmt.Errorf("Aucun vérificateur configuré ; configurez son identité avant son budget.")
		}
		if r.Inherit {
			return fmt.Errorf("Choisissez explicitement le modèle du vérificateur.")
		}
	} else {
		var err error
		scope, err = p.scope(r.Scope)
		if err != nil {
			return err
		}
		if r.Inherit {
			scope.ModelSelection = nil
			scope.Revision++
			return nil
		}
	}
	providers, err := s.providers()
	if err != nil {
		return err
	}
	provider, ok := providers.Providers[r.Provider]
	if !ok {
		return fmt.Errorf("fournisseur inconnu : %s", r.Provider)
	}
	if _, err = assistantProvider(provider); err != nil {
		return err
	}
	_, route, err := resolveModel(provider, r.Level, "planning")
	if err != nil {
		return err
	}
	if route == nil || r.PolicyHash == "" || route.PolicyHash != r.PolicyHash {
		return fmt.Errorf("Politique de modèle modifiée ; examiner à nouveau le choix.")
	}
	raw, _ := json.Marshal(provider)
	selection := &RoleModel{Provider: r.Provider, ProviderDigest: hash(raw), Route: route, Actor: operatorIdentity(), At: now()}
	if r.Reviewer {
		p.Reviewer.Provider = r.Provider
		p.Reviewer.ProviderDigest = selection.ProviderDigest
		p.Reviewer.ModelRoute = route
		p.Reviewer.ModelSelection = selection
	} else {
		scope.ModelSelection = selection
		scope.Revision++
	}
	return nil
}
func (s *Store) previewRoleModel(work string, r RoleModelRequest) (any, error) {
	w, e := s.get(work)
	if e != nil {
		return nil, e
	}
	if w.Revision != r.Revision {
		return nil, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	if !s.paused(work) {
		return nil, fmt.Errorf("Mettez la mission en pause avant de modifier le modèle d’un responsable ou du vérificateur.")
	}
	if e = s.prepareRoleModel(&w, r); e != nil {
		return nil, e
	}
	return w, nil
}
func (s *Store) configureRoleModel(work string, r RoleModelRequest) (Work, error) {
	raw, e := json.Marshal(r)
	if e != nil {
		return Work{}, e
	}
	return s.mutateWithHook(work, "role.model", r.EventID, r.Revision, raw, func(w *Work) error { return s.prepareRoleModel(w, r) }, func(tx *sql.Tx, w *Work) error {
		var paused bool
		if e := tx.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", work).Scan(&paused); e != nil {
			return e
		}
		if !paused {
			return fmt.Errorf("Mettez la mission en pause avant de modifier le modèle d’un responsable ou du vérificateur.")
		}
		return nil
	})
}
func (s *Store) roleModelCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 3 {
		return fmt.Errorf("swarm role-model show|preview|apply WORK --input request.json")
	}
	if pos[1] == "show" {
		return s.taskModelCLI([]string{"task-model", "show", pos[2]}, "", out)
	}
	raw, e := readInput(input)
	if e != nil {
		return e
	}
	var r RoleModelRequest
	if e = strict(raw, &r); e != nil {
		return e
	}
	if pos[1] == "preview" {
		v, e := s.previewRoleModel(pos[2], r)
		if e != nil {
			return e
		}
		return printJSON(out, v)
	}
	if pos[1] != "apply" {
		return fmt.Errorf("action inconnue")
	}
	v, e := s.configureRoleModel(pos[2], r)
	if e != nil {
		return e
	}
	return printJSON(out, v)
}
