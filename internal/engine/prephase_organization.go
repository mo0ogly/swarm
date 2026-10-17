package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"reflect"
	"strings"
)

func (s *Store) attachPreparedOrganization(tx *sql.Tx, w *Work, p *Preparation, r PreparationRequest) error {
	s.preparationFreshness(p)
	if !p.PlanReady || r.Hash != p.Documents["plan"].Hash {
		return preparationError("stale_plan", "Vérifier le plan courant avant de configurer son organisation.")
	}
	if p.Conversion.ReleasedAt != "" || w.Planning != nil || w.Profile != nil || len(w.Tasks) != len(p.Conversion.TaskIDs) {
		return preparationError("already_created", "Configurer uniquement une préparation non démarrée et sans organisation ; les tâches existantes restent conservées.")
	}
	if w.Objective != p.Conversion.Spec.Objective || w.Scope != p.Brief.Text || !reflect.DeepEqual(w.Criteria, preparationWorkCriteria()) {
		return preparationError("stale_plan", "Le contrat du travail a changé ; le réconcilier avec le plan avant configuration.")
	}
	var agents int
	if err := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents); err != nil {
		return err
	}
	if agents != 0 {
		return preparationError("already_created", "Une tentative existe déjà ; aucune adoption d’historique implicite.")
	}
	expected := Work{Schema: 1, ID: w.ID, Tasks: []Task{}}
	if err := s.materializePlan(&expected, PlanReview{Source: p.ID, BriefHash: p.Conversion.BriefHash, ResponseHash: p.Conversion.PlanHash, Spec: p.Conversion.Spec}); err != nil {
		return err
	}
	for _, id := range p.Conversion.TaskIDs {
		task, err := w.task(id)
		if err != nil {
			return err
		}
		original, err := expected.task(id)
		if err != nil {
			return err
		}
		original.LaunchHeld = true
		if !reflect.DeepEqual(*task, *original) {
			return preparationError("stale_plan", "Les tâches doivent correspondre au plan verrouillé sans tentative, décision, politique ou historique antérieur.")
		}
		var held int
		if err := tx.QueryRow("SELECT count(*) FROM preparation_launch_locks WHERE work_id=? AND task_id=? AND preparation_id=? AND released=0", w.ID, id, p.ID).Scan(&held); err != nil {
			return err
		}
		if held != 1 {
			return preparationError("conflict", "Verrou du plan absent ; aucune organisation enregistrée.")
		}
	}
	if err := s.configurePreparedOrganization(w, *p, *r.Organization, r.repositoryFinish); err != nil {
		return err
	}
	encoded, err := json.Marshal(r.Organization)
	if err != nil {
		return err
	}
	p.Conversion.OrganizationDigest = hash(encoded)
	if r.Action == "authorize-plan" {
		p.Conversion.ReleasedAt = now()
		for i := range w.Tasks {
			w.Tasks[i].LaunchHeld = false
		}
	}
	return nil
}

// Operator supplied configuration; never accepted from the model's plan JSON.
type PreparationOrganization struct {
	Repository     *ManagedRepositoryRequest      `json:"repository,omitempty"`
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

func (s *Store) configurePreparedOrganization(w *Work, p Preparation, config PreparationOrganization, repositoryFinish *func(bool) error) error {
	if w.Planning != nil {
		return fmt.Errorf("organisation déjà configurée ; conserver sa politique lors de la révision")
	}
	if config.Validation != "human" && config.Validation != "automatic" {
		return fmt.Errorf("choisir la revue humaine ou des contrôles automatiques explicites")
	}
	if config.MaxTasks < max(1, len(w.Tasks)) || config.MaxCalls < 1 {
		return fmt.Errorf("plafonds : nombre de tâches existantes au minimum et budget d'appels positif requis")
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
	if config.Repository != nil {
		if config.Validation != "automatic" {
			return fmt.Errorf("dépôt géré : autoriser des contrôles automatiques explicites")
		}
		repository, err := s.configurePreparedRepository(w.ID, *config.Repository, repositoryFinish)
		if err != nil {
			return err
		}
		planning.Repository = repository
	}
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
