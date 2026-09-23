package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"time"
)

// PlanningState is stored inside the existing atomic work/event transaction.
// No parallel persistence path: exports, rollback and optimistic locking retain
// the same boundary as tasks. Absence means legacy, never inferred adoption.
type PlanningState struct {
	QuotaAuthorization    *QuotaAuthorization            `json:"quota_authorization,omitempty"`
	ReviewerRequired      bool                           `json:"reviewer_required,omitempty"`
	Reviewer              *ReviewerConfig                `json:"reviewer,omitempty"`
	ModelRoute            *ModelRoute                    `json:"model_route,omitempty"`
	HumanReviewAuthorized string                         `json:"human_review_authorized,omitempty"`
	Checks                map[string][]ValidationControl `json:"checks,omitempty"`
	Repository            *ManagedRepository             `json:"repository,omitempty"`
	Provider              string                         `json:"provider,omitempty"`
	ProviderDigest        string                         `json:"provider_digest,omitempty"`
	Activations           int                            `json:"activations"`
	MaxActivations        int                            `json:"max_activations"`
	Failure               string                         `json:"failure,omitempty"`
	Version               int                            `json:"version"`
	Paused                bool                           `json:"paused"`
	MaxTasks              int                            `json:"max_tasks"`
	MaxDecisions          int                            `json:"max_decisions"`
	Decisions             int                            `json:"decisions"`
	Scopes                []PlanningScope                `json:"scopes"`
	Inbox                 []PlanningEvent                `json:"inbox"`
}
type PlanningScope struct {
	ModelSelection  *RoleModel        `json:"model_selection,omitempty"`
	LastModel       *RoleModel        `json:"last_model,omitempty"`
	Delivery        *PlanningDelivery `json:"delivery,omitempty"`
	Workflow        *AgentWorkflow    `json:"workflow,omitempty"`
	TaskLimit       int               `json:"max_tasks,omitempty"`
	ActivationLimit int               `json:"max_activations,omitempty"`
	Activations     int               `json:"activations"`
	ID              string            `json:"id"`
	Parent          string            `json:"parent,omitempty"`
	Objective       string            `json:"objective"`
	Requirements    []string          `json:"requirements"`
	Revision        int               `json:"revision"`
	Generation      int               `json:"generation"`
	Holder          string            `json:"holder,omitempty"`
	Until           string            `json:"lease_until,omitempty"`
	State           string            `json:"state"`
}
type PlanningEvent struct {
	Handoff   *ExchangeArtifact  `json:"handoff,omitempty"`
	ID        string             `json:"id"`
	Scope     string             `json:"scope"`
	Kind      string             `json:"kind"`
	Task      string             `json:"task,omitempty"`
	Attempt   string             `json:"attempt,omitempty"`
	Message   string             `json:"message"`
	Artifacts []ExchangeArtifact `json:"artifacts,omitempty"`
	Decision  string             `json:"decision,omitempty"`
	At        string             `json:"at"`
}
type PlanningOperation struct {
	MaxTasks       int      `json:"max_tasks,omitempty"`
	MaxActivations int      `json:"max_activations,omitempty"`
	Kind           string   `json:"kind"`
	ID             string   `json:"id"`
	Title          string   `json:"title"`
	Requirements   []string `json:"requirements"`
	Deliverable    string   `json:"deliverable,omitempty"`
	Criteria       []string `json:"criteria,omitempty"`
	Depends        []string `json:"depends,omitempty"`
	Next           string   `json:"next,omitempty"`
}
type PlanningRequest struct {
	MaxReviewCalls       int                            `json:"max_review_calls,omitempty"`
	ResultCommit         string                         `json:"result_commit,omitempty"`
	ExpectedCandidate    string                         `json:"expected_candidate,omitempty"`
	ResultTree           string                         `json:"result_tree,omitempty"`
	ConfirmRecovery      bool                           `json:"confirm_recovery,omitempty"`
	ReviewID             string                         `json:"review_id,omitempty"`
	RecoveryInstruction  string                         `json:"recovery_instruction,omitempty"`
	ReviewTimeoutSeconds int                            `json:"review_timeout_seconds,omitempty"`
	Level                string                         `json:"level,omitempty"`
	PolicyHash           string                         `json:"model_policy_hash,omitempty"`
	Checks               map[string][]ValidationControl `json:"checks,omitempty"`
	Repository           *ManagedRepositoryRequest      `json:"repository,omitempty"`
	Provider             string                         `json:"provider,omitempty"`
	MaxActivations       int                            `json:"max_activations,omitempty"`
	Agent                string                         `json:"agent_id,omitempty"`
	Task                 string                         `json:"task_id,omitempty"`
	Attempt              string                         `json:"attempt_id,omitempty"`
	Artifacts            []ExchangeArtifact             `json:"artifacts,omitempty"`
	Schema               int                            `json:"schema_version"`
	EventID              string                         `json:"event_id"`
	Revision             int                            `json:"expected_revision"`
	Scope                string                         `json:"scope,omitempty"`
	ScopeRevision        int                            `json:"scope_revision,omitempty"`
	Holder               string                         `json:"holder,omitempty"`
	Generation           int                            `json:"generation,omitempty"`
	LeaseSeconds         int                            `json:"lease_seconds,omitempty"`
	MaxTasks             int                            `json:"max_tasks,omitempty"`
	MaxDecisions         int                            `json:"max_decisions,omitempty"`
	Inputs               []string                       `json:"input_events,omitempty"`
	Operations           []PlanningOperation            `json:"operations,omitempty"`
	Reason               string                         `json:"reason,omitempty"`
}

func (p *PlanningState) scope(id string) (*PlanningScope, error) {
	for i := range p.Scopes {
		if p.Scopes[i].ID == id {
			return &p.Scopes[i], nil
		}
	}
	return nil, fmt.Errorf("périmètre inconnu : %s", id)
}
func planningError(code, message string) error { return &CommandError{Code: code, Message: message} }

// All mutations, including lease changes, share the work revision. Replays are
// checked by mutate before executing this closure, even after lease expiry.
func (s *Store) planningChange(work, action string, r PlanningRequest) (Work, error) {
	if r.Schema != 1 {
		return Work{}, fmt.Errorf("schema_version doit valoir 1")
	}
	if r.MaxReviewCalls != 0 && (action != "enable" || r.Provider == "" || r.MaxReviewCalls < 1 || r.MaxReviewCalls > 100) {
		return Work{}, planningError("invalid_review_budget", "max_review_calls doit être compris entre 1 et 100, uniquement lors de enable avec un fournisseur")
	}
	if action == "retry-integration" {
		return s.retryManagedIntegration(work, r)
	}
	if action == "retry-review" {
		return s.retryIndependentReview(work, r)
	}
	if action == "configure-reviewer" {
		return s.configureReviewer(work, r)
	}
	if action == "review-timeout" {
		return s.setReviewTimeout(work, r)
	}
	if action == "extend-attempt" {
		return s.extendAttempt(work, r)
	}
	if action == "authorize-recovery" {
		return s.authorizeCorrectiveRecovery(work, r)
	}
	if action == "revise-recovered-result" {
		return s.recoverResult(work, r, true)
	}
	if action == "submit-recovered-result" {
		return s.submitRecoveredResult(work, r)
	}
	raw, err := json.Marshal(r)
	if err != nil {
		return Work{}, err
	}
	if len(raw) > 65536 {
		return Work{}, fmt.Errorf("décision limitée à 64 Kio")
	}
	var repository *ManagedRepository
	if action == "enable" && r.Repository != nil {
		current, e := s.get(work)
		if e != nil {
			return Work{}, e
		}
		if current.Planning == nil {
			if current.Revision != r.Revision || len(current.Tasks) > 0 {
				return Work{}, fmt.Errorf("révision ou mission non disponible")
			}
			repository, e = s.configureManagedRepository(work, *r.Repository)
			if e != nil {
				return Work{}, e
			}
		}
	}
	return s.mutateWithHook(work, "planning."+action, r.EventID, r.Revision, raw, func(w *Work) error {
		if action == "claim" && w.Planning != nil && w.Planning.Provider != "" {
			if e := s.providerCooldownGuard(w.Planning.Provider); e != nil {
				return e
			}
		}
		if err := s.applyPlanning(w, action, r, time.Now().UTC()); err != nil {
			return err
		}
		var providerDigest string
		if action == "enable" && r.Provider != "" {
			ps, e := s.providers()
			if e != nil {
				return e
			}
			provider, ok := ps.Providers[r.Provider]
			if !ok {
				return fmt.Errorf("fournisseur inconnu")
			}
			if _, e = assistantProvider(provider); e != nil {
				return e
			}
			_, route, routeErr := resolveModel(provider, r.Level, "planning")
			if routeErr != nil {
				return routeErr
			}
			if r.PolicyHash != "" && (route == nil || route.PolicyHash != r.PolicyHash) {
				return fmt.Errorf("politique de modèles modifiée")
			}
			w.Planning.ModelRoute = route
			encoded, _ := json.Marshal(provider)
			providerDigest = hash(encoded)
		}

		if action == "enable" {
			w.Planning.ProviderDigest = providerDigest
			w.Planning.Repository = repository
			if r.Provider != "" {
				maxReviews := r.MaxReviewCalls
				if maxReviews == 0 {
					maxReviews = min(w.Planning.MaxActivations, 100)
				}
				config, e := s.reviewerConfig(r.Provider, r.Level, maxReviews)
				if e != nil {
					return e
				}
				w.Planning.Reviewer = config
				w.Planning.ReviewerRequired = true
			}
		}

		return nil
	}, func(tx *sql.Tx, w *Work) error {
		if action == "claim" || action == "decide" {
			var paused int
			err := tx.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", work).Scan(&paused)
			if err != nil && err != sql.ErrNoRows {
				return err
			}
			if paused != 0 {
				return fmt.Errorf("mission en pause")
			}
			var archived int
			if err = tx.QueryRow("SELECT count(*) FROM mission_lifecycle WHERE work_id=?", work).Scan(&archived); err != nil {
				return err
			}
			if archived > 0 {
				return fmt.Errorf("mission archivée")
			}
		}
		if action == "claim" {
			if err := reservePlanningCall(tx, work, r.Scope, r.EventID); err != nil {
				return err
			}
		}

		if action == "handoff" {
			var body []byte
			if e := tx.QueryRow("SELECT body FROM agents WHERE id=? AND work_id=?", r.Agent, work).Scan(&body); e != nil {
				return e
			}
			var agent Agent
			if e := json.Unmarshal(body, &agent); e != nil {
				return e
			}
			if agent.TaskID != r.Task || agent.Attempt != r.Attempt || agent.Role != "worker" {
				return fmt.Errorf("origine de remise non attribuable à cette tentative")
			}
			if e := s.verifyPlanningHandoffArtifacts(agent, r.Artifacts); e != nil {
				return e
			}
		}
		for _, op := range r.Operations {
			if op.Kind == "retry" {
				if w.Planning.Repository != nil {
					var pending int
					if e := tx.QueryRow("SELECT count(*) FROM managed_attempts m JOIN agents a ON a.id=m.agent_id WHERE m.work_id=? AND m.task_id=? AND m.state IN ('ready','integrating') AND a.status='completed'", work, op.ID).Scan(&pending); e != nil {
						return e
					}
					if pending > 0 {
						return fmt.Errorf("attendre le verdict du contrôleur avant reprise")
					}
				}
				var active int
				if e := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')", work, op.ID).Scan(&active); e != nil {
					return e
				}
				if active > 0 {
					return fmt.Errorf("tentative encore active")
				}
			}
			if op.Kind == "close" {
				var count int
				if e := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')", work).Scan(&count); e != nil {
					return e
				}
				if count > 0 {
					return fmt.Errorf("tentative encore active ; clôture refusée")
				}
			}
		}
		return nil
	})
}
func (s *Store) applyPlanning(w *Work, action string, r PlanningRequest, at time.Time) error {
	if action == "enable" {
		if w.Planning != nil || len(w.Tasks) > 0 {
			return fmt.Errorf("activer sur une mission neuve sans tâches ; adoption historique non implicite")
		}
		if r.MaxActivations < 1 || r.MaxActivations > 200 {
			return fmt.Errorf("max_activations : 1 à 200 requis")
		}
		if r.MaxTasks < 1 || r.MaxTasks > 100 || r.MaxDecisions < 1 || r.MaxDecisions > 100 {
			return fmt.Errorf("max_tasks et max_decisions : 1 à 100 requis")
		}
		if len(w.Criteria) == 0 || len(w.Criteria) > 100 {
			return fmt.Errorf("1 à 100 exigences requises")
		}
		requirements := []string{}
		for i := range w.Criteria {
			requirements = append(requirements, fmt.Sprintf("req-%d", i+1))
		}
		w.Planning = &PlanningState{Provider: r.Provider, MaxActivations: r.MaxActivations, Version: 1, MaxTasks: r.MaxTasks, MaxDecisions: r.MaxDecisions, Scopes: []PlanningScope{{ID: "root", Objective: w.Objective, Requirements: requirements, Revision: 1, State: "ready"}}, Inbox: []PlanningEvent{{ID: r.EventID, Scope: "root", Kind: "brief", Message: w.Objective, At: at.Format(time.RFC3339Nano)}}}
		checks, e := normalizePlanningChecks(w, r.Checks)
		if e != nil {
			return e
		}
		if r.Repository != nil {
			for _, req := range requirements {
				if len(checks[req]) == 0 {
					return fmt.Errorf("autoriser les contrôles de %s avant de gérer les intégrations", req)
				}
			}
		}
		w.Planning.Checks = checks
		return nil
	}
	p := w.Planning
	if p == nil {
		return fmt.Errorf("planification hiérarchique non activée")
	}
	if action == "pause" || action == "resume" {
		p.Paused = action == "pause"
		if action == "resume" {
			p.Failure = ""
			if nonempty(r.Reason) {
				if len(r.Reason) > 4000 {
					return fmt.Errorf("motif limité à 4000 octets")
				}
				scopeID := r.Scope
				if scopeID == "" {
					scopeID = "root"
				}
				owner, e := p.scope(scopeID)
				if e != nil {
					return e
				}
				owner.State = "ready"
				p.Inbox = append(p.Inbox, PlanningEvent{ID: r.EventID, Scope: scopeID, Kind: "operator_resume", Message: r.Reason, At: at.Format(time.RFC3339Nano)})
			}
		}
		// Revoke all outstanding activations; a paused response must not land later.
		for i := range p.Scopes {
			p.Scopes[i].Generation++
			p.Scopes[i].Holder = ""
			p.Scopes[i].Until = ""
		}
		return nil
	}
	if action == "handoff" {
		task, e := w.task(r.Task)
		if e != nil {
			return e
		}
		if r.Scope != "" && r.Scope != task.ScopeID {
			return fmt.Errorf("remise hors périmètre : seul le responsable propriétaire %s peut la recevoir", task.ScopeID)
		}
		if !currentTaskAttempt(task, r.Attempt) || task.ScopeID == "" || !nonempty(r.Reason) || len(r.Reason) > 4000 || len(r.Artifacts) == 0 || len(r.Artifacts) > 32 {
			return fmt.Errorf("remise : tentative actuelle, constats et artefacts requis")
		}
		if e = s.verifyExchangeArtifacts(r.Artifacts); e != nil {
			return e
		}
		for _, event := range p.Inbox {
			if event.Kind == "handoff" && event.Attempt == r.Attempt {
				return fmt.Errorf("une remise existe déjà pour cette tentative")
			}
		}
		scope, e := p.scope(task.ScopeID)
		if e != nil {
			return e
		}
		scope.State = "ready"
		p.Inbox = append(p.Inbox, PlanningEvent{ID: r.EventID, Scope: scope.ID, Kind: "handoff", Task: task.ID, Attempt: r.Attempt, Message: r.Reason, Artifacts: r.Artifacts, At: at.Format(time.RFC3339Nano)})
		return nil
	}
	if p.Paused {
		return planningError("planning_paused", "planification en pause")
	}
	scope, err := p.scope(r.Scope)
	if err != nil {
		return err
	}
	if scope.Revision != r.ScopeRevision {
		return planningError("revision_conflict", "périmètre modifié ; relire le contexte")
	}
	switch action {
	case "claim":
		if !safeName(r.Holder) || r.LeaseSeconds < 5 || r.LeaseSeconds > 300 {
			return fmt.Errorf("holder requis, lease_seconds entre 5 et 300")
		}
		until, _ := time.Parse(time.RFC3339Nano, scope.Until)
		if scope.Holder != "" && at.Before(until) {
			return planningError("lease_busy", "un planificateur possède déjà ce périmètre")
		}
		pending := false
		for _, event := range p.Inbox {
			if event.Scope == scope.ID && event.Decision == "" {
				pending = true
			}
		}
		if !pending || scope.State == "closed" {
			return fmt.Errorf("aucun retour à traiter")
		}
		if p.Activations >= p.MaxActivations {
			return fmt.Errorf("budget d’activations atteint")
		}
		if p.Failure != "" {
			return fmt.Errorf("planification arrêtée : %s", p.Failure)
		}
		if p.Decisions >= p.MaxDecisions {
			return fmt.Errorf("budget de décisions atteint")
		}
		if err := checkScopeActivation(p, scope.ID); err != nil {
			return err
		}
		workflow, workflowPrompt, err := agentWorkflow(planningWorkflowRole(scope))
		if err != nil {
			return err
		}
		_, delivery, err := s.planningDeliveryContext(*w, scope.ID, 64000-len(workflowPrompt)-3000)
		if err != nil {
			return err
		}
		scope.Delivery = &delivery
		scope.Workflow = &workflow
		selectedModel := effectivePlanningScope(p, scope)
		scope.LastModel = &RoleModel{Provider: selectedModel.Provider, ProviderDigest: selectedModel.ProviderDigest, Route: selectedModel.ModelRoute, At: now()}
		for _, owner := range planningAncestors(p, scope.ID) {
			owner.Activations++
		}
		p.Activations++
		scope.Generation++
		scope.Holder = r.Holder
		scope.Until = at.Add(time.Duration(r.LeaseSeconds) * time.Second).Format(time.RFC3339Nano)
		return nil
	case "decide":
		until, _ := time.Parse(time.RFC3339Nano, scope.Until)
		if scope.Holder == "" || scope.Holder != r.Holder || scope.Generation != r.Generation || !at.Before(until) {
			return planningError("stale_lease", "activation expirée ou remplacée ; décision refusée")
		}
		if p.Decisions >= p.MaxDecisions {
			return fmt.Errorf("budget de décisions atteint")
		}
		if len(r.Inputs) == 0 || len(r.Inputs) > 100 || len(r.Operations) > 20 || !nonempty(r.Reason) || len(r.Reason) > 4000 {
			return fmt.Errorf("événements d’entrée et justification requis ; maximum 20 opérations")
		}
		seen := map[string]bool{}
		for _, id := range r.Inputs {
			if seen[id] {
				return fmt.Errorf("événement répété")
			}
			seen[id] = true
			found := false
			for _, event := range p.Inbox {
				if event.ID == id && event.Scope == scope.ID && event.Decision == "" {
					if event.Task != "" {
						task, e := w.task(event.Task)
						if (e != nil || (event.Attempt != "" && !currentTaskAttempt(task, event.Attempt))) && len(r.Operations) > 0 {
							return fmt.Errorf("retour d’une tentative périmée : %s", id)
						}
					}
					if e := s.verifyExchangeArtifacts(event.Artifacts); e != nil && len(r.Operations) > 0 {
						return e
					}
					found = true
				}
			}
			if !found {
				return fmt.Errorf("événement absent, déjà traité ou hors périmètre : %s", id)
			}
			if scope.Delivery != nil && !containsString(scope.Delivery.Events, id) {
				return fmt.Errorf("événement non fourni dans cette activation : %s", id)
			}
		}
		scopeID := scope.ID
		for _, op := range r.Operations {
			if err := s.applyPlanningOperation(w, scopeID, op, r, at); err != nil {
				return err
			}
		}
		scope, _ = p.scope(scopeID) // delegation can reallocate the slice
		if scope.Delivery != nil {
			scope.Delivery.Decision = r.EventID
		}
		for i := range p.Inbox {
			if seen[p.Inbox[i].ID] {
				p.Inbox[i].Decision = r.EventID
			}
		}
		scope.Revision++
		scope.Generation++
		scope.Holder = ""
		scope.Until = ""
		p.Decisions++
		if scope.State != "closed" {
			scope.State = "waiting"
			for _, event := range p.Inbox {
				if event.Scope == scopeID && event.Decision == "" {
					scope.State = "ready"
				}
			}
		}
		return nil
	default:
		return fmt.Errorf("action de planification inconnue : %s", action)
	}
}

func (s *Store) applyPlanningOperation(w *Work, id string, op PlanningOperation, r PlanningRequest, at time.Time) error {
	p := w.Planning
	scope, _ := p.scope(id)
	if scope.State == "closed" {
		return fmt.Errorf("périmètre déjà clos")
	}
	switch op.Kind {
	case "task", "delegate":
		if !safeName(op.ID) {
			return fmt.Errorf("identifiant de l’opération invalide : %q", op.ID)
		}
		if !nonempty(op.Title) || len([]rune(op.Title)) > 500 {
			return fmt.Errorf("titre requis, limité à 500 caractères (reçu : %d)", len([]rune(op.Title)))
		}
		if len(op.Requirements) == 0 {
			return fmt.Errorf("exigences requises pour %s", op.ID)
		}
		if len(op.Next) > 4000 {
			return fmt.Errorf("consigne limitée à 4000 octets")
		}
		seen := map[string]bool{}
		for _, req := range op.Requirements {
			if seen[req] || !containsString(scope.Requirements, req) {
				return fmt.Errorf("exigence répétée ou non possédée : %s", req)
			}
			seen[req] = true
		}
		if op.Kind == "delegate" {
			if _, err := p.scope(op.ID); err == nil {
				return fmt.Errorf("périmètre déjà présent")
			}
			depth := 1
			parent := scope
			for parent.Parent != "" {
				depth++
				parent, _ = p.scope(parent.Parent)
			}
			if depth >= 3 || len(p.Scopes) >= 20 {
				return fmt.Errorf("limite de délégation atteinte : profondeur 3, 20 périmètres")
			}
			for _, task := range w.Tasks {
				if task.ScopeID == id {
					for _, req := range task.Requirements {
						if seen[req] {
							return fmt.Errorf("exigence déjà confiée à une tâche")
						}
					}
				}
			}
			taskLimit := scope.TaskLimit
			if taskLimit == 0 {
				taskLimit = p.MaxTasks
			}
			activationLimit := scope.ActivationLimit
			if activationLimit == 0 {
				activationLimit = p.MaxActivations
			}
			if op.MaxTasks < 0 || op.MaxTasks > taskLimit || op.MaxActivations < 0 || op.MaxActivations > activationLimit {
				return fmt.Errorf("budget enfant supérieur au budget parent")
			}
			if op.MaxTasks > 0 {
				taskLimit = op.MaxTasks
			}
			if op.MaxActivations > 0 {
				activationLimit = op.MaxActivations
			}
			kept := []string{}
			for _, req := range scope.Requirements {
				if !seen[req] {
					kept = append(kept, req)
				}
			}
			scope.Requirements = kept
			p.Scopes = append(p.Scopes, PlanningScope{TaskLimit: taskLimit, ActivationLimit: activationLimit, ID: op.ID, Parent: id, Objective: op.Title + "\n" + op.Next, Requirements: op.Requirements, Revision: 1, State: "ready"})
			p.Inbox = append(p.Inbox, PlanningEvent{ID: planningEventID(r.EventID, op.ID), Scope: op.ID, Kind: "delegation", Message: op.Title, At: at.Format(time.RFC3339Nano)})
			return nil
		}
		// A requirement already confided to a task that is now blocked must be
		// resumed via "retry" on that task, never re-delegated to a fresh "task"
		// op: otherwise unbounded new tasks pay for fresh activations on the same
		// requirement instead of correcting and retrying the one already stuck.
		// Tasks that are not blocked may still legitimately share a requirement
		// (e.g. an initial batch, or a discovery handoff from a running task).
		for _, task := range w.Tasks {
			if task.ScopeID == id && task.Status == "blocked" {
				for _, req := range task.Requirements {
					if seen[req] {
						return fmt.Errorf("exigence déjà confiée à la tâche bloquée %s ; utiliser retry", task.ID)
					}
				}
			}
		}
		if err := checkScopeTask(w, id); err != nil {
			return err
		}
		if len(w.Tasks) >= p.MaxTasks {
			return fmt.Errorf("budget de tâches atteint")
		}
		if len(op.Criteria) > 12 || len(op.Depends) > 100 || len(op.Deliverable) > 4000 || len(op.Next) > 4000 {
			return fmt.Errorf("contrat de tâche trop grand")
		}
		for _, criterion := range op.Criteria {
			if !nonempty(criterion) || len(criterion) > 1000 {
				return fmt.Errorf("critère invalide")
			}
		}
		for _, dep := range op.Depends {
			task, err := w.task(dep)
			if err != nil || task.ScopeID != id {
				return fmt.Errorf("prérequis hors périmètre : %s", dep)
			}
		}
		err := s.apply(w, "task.add", Request{ID: op.ID, Title: op.Title, Deliverable: op.Deliverable, Criteria: op.Criteria, Depends: op.Depends, Next: op.Next, Owner: id})
		if err != nil {
			return err
		}
		task, _ := w.task(op.ID)
		task.ScopeID = id
		task.Requirements = op.Requirements
		task.PlanRole = "worker"
		task.PlanMaxAttempts = 2
		return inheritPlanningChecks(w, task)
	case "retry":
		task, err := w.task(op.ID)
		if err != nil {
			return err
		}
		if task.ScopeID != id || task.Status != "blocked" || task.LaunchHeld || task.PlanningRetry {
			return fmt.Errorf("reprise non disponible")
		}
		if len(task.Attempts) >= task.PlanMaxAttempts || !nonempty(op.Next) || op.Next == task.Next || len(op.Next) > 4000 {
			return fmt.Errorf("reprise bornée exigeant une nouvelle correction explicite")
		}
		task.PlanningRetry = true
		task.Next = op.Next
		return nil
	case "close":
		for _, event := range p.Inbox {
			if event.Scope == id && event.Decision == "" && !containsString(r.Inputs, event.ID) {
				return fmt.Errorf("retour non traité : %s", event.ID)
			}
		}
		for _, child := range p.Scopes {
			if child.Parent == id && child.State != "closed" {
				return fmt.Errorf("périmètre enfant non terminé")
			}
		}
		for _, task := range w.Tasks {
			if task.ScopeID == "" {
				return fmt.Errorf("tâche sans périmètre")
			}
			owner, err := p.scope(task.ScopeID)
			if err != nil {
				return err
			}
			for owner.ID != id && owner.Parent != "" {
				owner, _ = p.scope(owner.Parent)
			}
			if owner.ID == id && (task.Status != "accepted" || !s.acceptedFresh(w, &task, map[string]bool{})) {
				return fmt.Errorf("preuve descendante non valide : %s", task.ID)
			}
		}
		covered := map[string]bool{}
		for i := range w.Tasks {
			task := &w.Tasks[i]
			if task.ScopeID == id {
				if task.Status != "accepted" || !s.acceptedFresh(w, task, map[string]bool{}) {
					return fmt.Errorf("tâche non validée ou preuve périmée : %s", task.ID)
				}
				for _, req := range task.Requirements {
					covered[req] = true
				}
			}
		}
		for _, req := range scope.Requirements {
			if !covered[req] {
				return fmt.Errorf("exigence sans preuve : %s", req)
			}
		}
		scope.State = "closed"
		if scope.Parent != "" {
			parent, _ := p.scope(scope.Parent)
			parent.State = "ready"
			p.Inbox = append(p.Inbox, PlanningEvent{ID: planningEventID(r.EventID, "closed", id), Scope: scope.Parent, Kind: "scope_closed", Message: op.Title, At: at.Format(time.RFC3339Nano)})
		}
		return nil
	default:
		return fmt.Errorf("opération non autorisée : %s", op.Kind)
	}
}

// A process ending wakes the owner, but is deliberately not a validation.
// Called in the same transaction as task settlement; recovery cannot lose it.
func planningAttemptEnded(w *Work, a Agent, outcome string, artifacts ...ExchangeArtifact) {
	if w.Planning == nil {
		return
	}
	task, err := w.task(a.TaskID)
	if err != nil || task.ScopeID == "" {
		return
	}
	id := planningEventID("attempt-ended", a.Attempt)
	for _, event := range w.Planning.Inbox {
		if event.ID == id {
			return
		}
	}
	scope, err := w.Planning.scope(task.ScopeID)
	if err != nil {
		return
	}
	scope.State = "ready"
	message := outcome + " : " + a.Activity
	if len(artifacts) > 0 {
		message += fmt.Sprintf(" · Bilan moteur conservé : %d/%d appels d’outils, %d résultats reçus. Ce bilan ne valide aucun critère et ne remplace pas le rapport du producteur.", a.Progress.ToolCalls, a.Limits.MaxToolCalls, a.Progress.ToolResults)
	}
	w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: id, Scope: scope.ID, Kind: "attempt_ended", Task: task.ID, Attempt: a.Attempt, Message: message, Artifacts: artifacts, At: now()})
}

// Validation is a distinct input. A planner that has already read a process
// result must wake again when the controller validates (or reopens) the task.
func planningValidationSignals(w *Work, before map[string]string, eventID string) {
	if w.Planning == nil {
		return
	}
	for _, task := range w.Tasks {
		if task.ScopeID == "" || before[task.ID] == task.Status || (task.Status != "accepted" && before[task.ID] != "accepted") {
			continue
		}
		scope, err := w.Planning.scope(task.ScopeID)
		if err != nil {
			continue
		}
		attempt := ""
		if len(task.Attempts) > 0 {
			attempt = task.Attempts[len(task.Attempts)-1].ID
		}
		scope.State = "ready"
		w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: planningEventID(eventID, "validation", task.ID), Scope: scope.ID, Kind: "validation_changed", Task: task.ID, Attempt: attempt, Message: task.Status, At: now()})
		for scope.Parent != "" {
			scope, _ = w.Planning.scope(scope.Parent)
			scope.State = "ready"
		}
	}
}

func revokePlanningTx(tx *sql.Tx, work string) error {
	var raw []byte
	if err := tx.QueryRow("SELECT body FROM works WHERE id=?", work).Scan(&raw); err != nil {
		return err
	}
	var w Work
	if err := json.Unmarshal(raw, &w); err != nil {
		return err
	}
	if w.Planning == nil {
		return nil
	}
	for i := range w.Planning.Scopes {
		w.Planning.Scopes[i].Generation++
		w.Planning.Scopes[i].Holder = ""
		w.Planning.Scopes[i].Until = ""
	}
	w.Revision++
	w.Updated = now()
	body, err := json.Marshal(w)
	if err != nil {
		return err
	}
	if _, err = tx.Exec("UPDATE works SET revision=?,body=? WHERE id=?", w.Revision, body, work); err != nil {
		return err
	}
	request := []byte(`{"reason":"mission paused: planner leases revoked"}`)
	_, err = tx.Exec("INSERT INTO events VALUES(?,?,?,?,?,?,?)", newID("planning-revoke-"), work, w.Revision, "planning.revoke", w.Updated, request, request)
	return err
}

// Imported history never supplies an unchecked tree to ownership traversal.
func validatePlanningState(w *Work) error {
	p := w.Planning
	if p == nil {
		return nil
	}
	if p.Version != 1 || len(p.Scopes) < 1 || len(p.Scopes) > 20 || p.MaxTasks < 1 || p.MaxTasks > 100 || len(w.Tasks) > p.MaxTasks || p.MaxDecisions < 1 || p.MaxDecisions > 100 || p.MaxActivations < 1 || p.MaxActivations > 200 || p.Activations < 0 || p.Activations > p.MaxActivations || p.Decisions < 0 || p.Decisions > p.MaxDecisions {
		return fmt.Errorf("contrat de planification invalide")
	}
	ids := map[string]bool{}
	requirements := map[string]bool{}
	for _, scope := range p.Scopes {
		if !safeName(scope.ID) || ids[scope.ID] || scope.Revision < 1 {
			return fmt.Errorf("périmètre invalide")
		}
		ids[scope.ID] = true
		for _, req := range scope.Requirements {
			if requirements[req] {
				return fmt.Errorf("exigence possédée plusieurs fois")
			}
			requirements[req] = true
		}
	}
	root, e := p.scope("root")
	if e != nil || root.Parent != "" {
		return fmt.Errorf("racine absente ou invalide")
	}
	for _, scope := range p.Scopes {
		seen := map[string]bool{}
		node := scope
		for node.ID != "root" {
			if seen[node.ID] {
				return fmt.Errorf("cycle de propriété")
			}
			seen[node.ID] = true
			parent, e := p.scope(node.Parent)
			if e != nil {
				return e
			}
			node = *parent
		}
		if len(seen) > 2 {
			return fmt.Errorf("profondeur de délégation excessive")
		}
	}
	if len(requirements) != len(w.Criteria) {
		return fmt.Errorf("couverture des exigences incomplète")
	}
	for i := range w.Criteria {
		if !requirements[fmt.Sprintf("req-%d", i+1)] {
			return fmt.Errorf("exigence non possédée")
		}
	}
	for _, task := range w.Tasks {
		scope, e := p.scope(task.ScopeID)
		if e != nil {
			return e
		}
		// In a hierarchical mission, planners are represented by scopes and run
		// through the tool-free planning path. Executable tasks are workers only.
		// Failing closed here also prevents an imported or historical payload from
		// turning a planner label into a coding-agent launch.
		if task.PlanRole != "" && task.PlanRole != "worker" {
			return fmt.Errorf("tâche hiérarchique non exécutante : %s", task.ID)
		}
		if len(task.Requirements) == 0 {
			return fmt.Errorf("tâche sans exigence")
		}
		for _, req := range task.Requirements {
			if !containsString(scope.Requirements, req) {
				return fmt.Errorf("tâche hors propriété")
			}
		}
	}
	events := map[string]bool{}
	for _, event := range p.Inbox {
		if !safeName(event.ID) || events[event.ID] || !ids[event.Scope] {
			return fmt.Errorf("événement de planification invalide")
		}
		events[event.ID] = true
	}
	return nil
}

// Proof drift can happen without a database write. Reopen a closed owner once,
// leaving the controller's original record intact and never relaunching by fiat.
func (s *Store) reconcilePlanningProofs(w Work) (Work, error) {
	if w.Planning == nil {
		return w, nil
	}
	stale := []string{}
	for _, task := range w.Tasks {
		owner, e := w.Planning.scope(task.ScopeID)
		if e == nil && owner.State == "closed" && !s.acceptedFresh(&w, &task, map[string]bool{}) {
			stale = append(stale, task.ID)
		}
	}
	if len(stale) == 0 {
		return w, nil
	}
	event := newID("planning-proof-")
	raw, _ := json.Marshal(map[string]any{"stale_tasks": stale})
	return s.mutate(w.ID, "planning.proof-stale", event, w.Revision, raw, func(current *Work) error {
		for _, id := range stale {
			task, _ := current.task(id)
			scope, _ := current.Planning.scope(task.ScopeID)
			scope.State = "ready"
			scope.Generation++
			scope.Holder = ""
			scope.Until = ""
			attempt := ""
			if len(task.Attempts) > 0 {
				attempt = task.Attempts[len(task.Attempts)-1].ID
			}
			current.Planning.Inbox = append(current.Planning.Inbox, PlanningEvent{ID: planningEventID(event, id), Scope: scope.ID, Kind: "proof_stale", Task: id, Attempt: attempt, Message: "La preuve n’est plus actuelle ; décision de reprise requise.", At: now()})
			for scope.Parent != "" {
				scope, _ = current.Planning.scope(scope.Parent)
				scope.State = "ready"
			}
		}
		return nil
	})
}

func planningEventID(parts ...string) string {
	raw, _ := json.Marshal(parts)
	return "plan-" + hash(raw)
}
