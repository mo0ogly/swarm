package main

import (
	"encoding/json"
	"fmt"
	"strings"
)

// Operator supplied configuration; never accepted from the model's plan JSON.
type PreparationOrganization struct {
	WorkerProvider string                         `json:"worker_provider,omitempty"`
	Provider       string                         `json:"provider"`
	Level          string                         `json:"level,omitempty"`
	PolicyHash     string                         `json:"model_policy_hash,omitempty"`
	Workspace      string                         `json:"workspace"`
	Validation     string                         `json:"validation"`
	Controls       map[string][]ValidationControl `json:"controls,omitempty"`
	MaxTasks       int                            `json:"max_tasks"`
	MaxCalls       int                            `json:"max_calls"`
}

func (s *Store) configurePreparedOrganization(w *Work, p Preparation, config PreparationOrganization) error {
	if w.Planning != nil {
		return fmt.Errorf("organisation déjà configurée ; conserver sa politique lors de la révision")
	}
	if config.Validation != "human" && config.Validation != "automatic" {
		return fmt.Errorf("choisir la revue humaine ou des contrôles automatiques explicites")
	}
	if config.MaxTasks < len(w.Tasks) || config.MaxTasks > 100 || config.MaxCalls < 1 || config.MaxCalls > 100 {
		return fmt.Errorf("plafonds : nombre de tâches courant à 100 tâches, 1 à 100 appels")
	}
	ps, err := s.providers()
	if err != nil {
		return err
	}
	provider, ok := ps.Providers[config.Provider]
	if !ok {
		return fmt.Errorf("fournisseur inconnu")
	}
	if _, err = assistantProvider(provider); err != nil {
		return err
	}
	_, route, err := resolveModel(provider, config.Level, "planning")
	if err != nil {
		return err
	}
	if config.PolicyHash != "" && (route == nil || route.PolicyHash != config.PolicyHash) {
		return fmt.Errorf("politique de modèles modifiée ; relire l’aperçu")
	}
	workerID := config.WorkerProvider
	if workerID == "" {
		workerID = config.Provider
	}
	worker, ok := ps.Providers[workerID]
	if !ok || worker.APIConnectionID != "" {
		return fmt.Errorf("Choisissez un agent avec outils pour les exécutants ; une connexion API peut rester responsable et vérificateur.")
	}
	if _, _, err = resolveModel(worker, config.Level, "work"); err != nil {
		return err
	}
	workspace, err := resolveWorkspace(s.root, config.Workspace)
	if err != nil {
		return err
	}
	digest, _ := json.Marshal(provider)
	planning := &PlanningState{Version: 1, Provider: config.Provider, ProviderDigest: hash(digest), ModelRoute: route, MaxTasks: config.MaxTasks, MaxDecisions: config.MaxCalls, MaxActivations: config.MaxCalls, Checks: map[string][]ValidationControl{}, Inbox: []PlanningEvent{}}
	root := PlanningScope{ID: "root", Objective: w.Objective, Revision: 1, State: "waiting", Requirements: []string{}}
	w.Criteria = nil
	prefix := "plan-" + hash([]byte(p.ID))[:10] + "-"
	for i := range w.Tasks {
		t := &w.Tasks[i]
		if !strings.HasPrefix(t.ID, prefix) {
			return fmt.Errorf("organisation : le travail contient des tâches extérieures à cette préparation")
		}
		if t.PlanRole != "worker" {
			return fmt.Errorf("%s : une mission organisée réserve planner/subplanner aux responsables de périmètre ; la tâche exécutable doit avoir le rôle worker", t.Title)
		}
		t.ScopeID = "root"
		t.Requirements = nil
		policy := ValidationPolicy{Mode: config.Validation, Controls: config.Controls[strings.TrimPrefix(t.ID, prefix)]}
		policy, err = normalizeValidationPolicy(policy)
		if err != nil {
			return fmt.Errorf("%s : %w", t.Title, err)
		}
		if err = validationPolicyCoversTask(policy, t); err != nil {
			return fmt.Errorf("%s : %w", t.Title, err)
		}
		policy.Authorized = now()
		policy.Actor = operatorIdentity()
		t.ValidationPolicy = &policy
		for n, criterion := range t.Criteria {
			w.Criteria = append(w.Criteria, criterion)
			req := fmt.Sprintf("req-%d", len(w.Criteria))
			t.Requirements = append(t.Requirements, req)
			root.Requirements = append(root.Requirements, req)
			for _, control := range policy.Controls {
				if containsInt(control.Criteria, n+1) {
					control.Criteria = []int{1}
					planning.Checks[req] = append(planning.Checks[req], control)
				}
			}
		}
	}
	if config.Validation == "human" {
		planning.HumanReviewAuthorized = now()
	}
	planning.Scopes = []PlanningScope{root}
	configReview, e := s.reviewerConfig(config.Provider, config.Level, config.MaxCalls)
	if e != nil {
		return e
	}
	planning.Reviewer = configReview
	planning.ReviewerRequired = true
	w.Planning = planning
	w.Profile = &LaunchProfile{Provider: workerID, Level: config.Level, Role: "worker", Workspace: workspace, Updated: now(), Actor: originOperator}
	return organizationGuard(*w)
}
func containsInt(values []int, value int) bool {
	for _, v := range values {
		if v == value {
			return true
		}
	}
	return false
}
