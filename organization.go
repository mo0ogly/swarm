package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"
)

// Organization is a factual precondition, not a claim of delivery or quality.
type Organization struct {
	Ready        bool     `json:"ready"`
	Label        string   `json:"label"`
	Issues       []string `json:"issues"`
	Next         string   `json:"next"`
	Verification string   `json:"verification"`
}

func reviewerLaunchGuard(w Work) error {
	if p := w.Planning; p != nil && (p.Reviewer == nil || p.Reviewer.Provider == "" || p.Reviewer.Authorized == "" || p.Reviewer.MaxCalls < 1) {
		return &CommandError{Code: "reviewer_required", Message: "Vérificateur IA indépendant absent : configurer la revue avant le lancement autonome."}
	}
	if p := w.Planning; p != nil && (p.Reviewer.Calls >= p.Reviewer.MaxCalls || p.Reviewer.Failure != "") {
		return &CommandError{Code: "reviewer_unavailable", Message: "Vérificateur IA indisponible ou budget épuisé : aucun nouveau départ autorisé."}
	}
	return nil
}

func (s *Store) reviewerAvailable(w Work) error {
	if e := reviewerLaunchGuard(w); e != nil {
		return e
	}
	if w.Planning == nil {
		return nil
	}
	cfg := w.Planning.Reviewer
	ps, e := s.providers()
	if e != nil {
		return e
	}
	p, ok := ps.Providers[cfg.Provider]
	raw, _ := json.Marshal(p)
	if !ok || cfg.ProviderDigest == "" || hash(raw) != cfg.ProviderDigest {
		return &CommandError{Code: "reviewer_unavailable", Message: "Configuration du vérificateur absente ou modifiée ; départ refusé."}
	}
	_, e = assistantProvider(p)
	return e
}

func organization(w Work) Organization {
	o := Organization{Ready: true, Label: "Organisation autonome configurée", Issues: []string{}, Next: "Consulter les responsables et les contrôles autorisés.", Verification: "Contrôles configurés ; aucune revue IA indépendante implicite."}
	p := w.Planning
	if p == nil {
		o.Issues = append(o.Issues, "Responsable de mission absent : planification hiérarchique non configurée.")
	} else {
		// Autonomous work never inherits permission to skip independent review
		// from a missing legacy flag or from the repository execution mode.
		// This is structural readiness. Runtime capacity belongs to launch
		// preview/reservation, not to the conductor's ability to finish an
		// already reviewed result or let its planner close the scope.
		if p.Reviewer == nil || p.Reviewer.Provider == "" || p.Reviewer.Authorized == "" || p.Reviewer.MaxCalls < 1 {
			o.Issues = append(o.Issues, "Vérificateur IA indépendant absent : configurer la revue avant le lancement autonome.")
		}
		if err := validatePlanningState(&w); err != nil {
			o.Issues = append(o.Issues, "Organisation invalide : "+err.Error())
		}
		if p.HumanReviewAuthorized != "" {
			o.Verification = "Revue humaine explicitement autorisée ; les résultats attendront votre acceptation."
		}
		root, e := p.scope("root")
		if e != nil || root.Parent != "" {
			o.Issues = append(o.Issues, "Responsable racine absent ou invalide.")
		}
		if p.Provider == "" {
			o.Issues = append(o.Issues, "Fournisseur du planificateur absent.")
		}
		for i := range w.Criteria {
			if len(p.Checks[fmt.Sprintf("req-%d", i+1)]) == 0 && p.HumanReviewAuthorized == "" {
				o.Issues = append(o.Issues, fmt.Sprintf("Exigence %d sans contrôles de validation autorisés.", i+1))
			}
		}
		for _, sc := range p.Scopes {
			if sc.ID != "root" {
				if _, e := p.scope(sc.Parent); e != nil {
					o.Issues = append(o.Issues, "Responsable parent absent : "+sc.ID)
				}
			}
		}
		for i := range w.Tasks {
			t := &w.Tasks[i]
			if _, e := p.scope(t.ScopeID); e != nil {
				o.Issues = append(o.Issues, "Tâche sans responsable : "+t.ID)
			}
			if t.ValidationPolicy == nil || (t.ValidationPolicy.Mode != "automatic" && t.ValidationPolicy.Mode != "human") || t.ValidationPolicy.Authorized == "" {
				o.Issues = append(o.Issues, "Politique de validation non autorisée : "+t.ID)
			} else if e := validationPolicyCoversTask(*t.ValidationPolicy, t); e != nil {
				o.Issues = append(o.Issues, "Validation incomplète : "+t.ID)
			}
		}
	}
	if p != nil && p.Reviewer != nil {
		o.Verification = fmt.Sprintf("Vérificateur IA indépendant : %s · %d/%d appels. Avis requis avant acceptation. ", p.Reviewer.Provider, p.Reviewer.Calls, p.Reviewer.MaxCalls) + o.Verification
	}
	if len(o.Issues) > 0 {
		o.Ready = false
		o.Verification = "Politique incomplète ; aucune vérification de mission démontrée."
		o.Label = "Organisation autonome non configurée"
		o.Next = "Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement."
	}
	return o
}
func organizationGuard(w Work) error {
	o := organization(w)
	if !o.Ready {
		return &CommandError{Code: "organization_incomplete", Message: o.Label + " : " + strings.Join(o.Issues, " ") + " " + o.Next}
	}
	return nil
}
func organizationGuardTx(tx *sql.Tx, id string) error {
	var raw []byte
	if e := tx.QueryRow("SELECT body FROM works WHERE id=?", id).Scan(&raw); e != nil {
		return e
	}
	var w Work
	if e := json.Unmarshal(raw, &w); e != nil {
		return e
	}
	return organizationGuard(w)
}
