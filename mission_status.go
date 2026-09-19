package main

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"
	"time"
)

type MissionTask struct {
	ID                string               `json:"id"`
	Title             string               `json:"title"`
	State             string               `json:"state"`
	Reason            string               `json:"reason"`
	Action            string               `json:"action"`
	Label             string               `json:"label"`
	Target            string               `json:"target,omitempty"`
	Impact            int                  `json:"impact"`
	Deliverable       string               `json:"deliverable"`
	ValidationMode    string               `json:"validation_mode"`
	ValidationReceipt string               `json:"validation_receipt,omitempty"`
	Result            ResultPresentation   `json:"result"`
	Understanding     MissionUnderstanding `json:"understanding"`
	Diagnostic        *AttemptDiagnostic   `json:"diagnostic,omitempty"`
}
type MissionUnderstanding struct {
	What      string `json:"what"`
	NextStep  string `json:"next_step"`
	Actor     string `json:"actor"`
	ActorKind string `json:"actor_kind"`
	Situation string `json:"situation"`
}
type MissionLaunchItem struct {
	ID        string `json:"id"`
	Title     string `json:"title"`
	Reason    string `json:"reason,omitempty"`
	Workspace string `json:"workspace,omitempty"`
	Normal    bool   `json:"normal,omitempty"`
}
type MissionLaunchPreview struct {
	Organization         Organization          `json:"organization"`
	Token                string                `json:"verdict_token"`
	Revision             int                   `json:"revision"`
	RequestedSlots       int                   `json:"requested_slots"`
	Immediate            int                   `json:"immediate"`
	EffectiveConcurrency int                   `json:"effective_concurrency"`
	SharedWorkspace      bool                  `json:"shared_workspace"`
	ConcurrencyMode      string                `json:"concurrency_mode"`
	ConcurrencyDetail    string                `json:"concurrency_detail"`
	Departures           []MissionLaunchItem   `json:"departures"`
	Waiting              []MissionLaunchItem   `json:"waiting"`
	Limits               []string              `json:"limits"`
	Contract             MissionLaunchContract `json:"contract"`
}
type MissionLaunchContract struct {
	Scope      string `json:"scope"`
	Budget     string `json:"budget"`
	Recovery   string `json:"recovery"`
	Validation string `json:"validation"`
}
type MissionCoordinationPhase struct {
	Kind     string `json:"kind"`
	Label    string `json:"label"`
	Summary  string `json:"summary"`
	Actor    string `json:"actor"`
	NextStep string `json:"next_step"`
	At       string `json:"at,omitempty"`
	Relative string `json:"relative,omitempty"`
}
type MissionStatus struct {
	ProviderCooldowns []ProviderCooldown         `json:"provider_cooldowns,omitempty"`
	EvidenceStage     string                     `json:"evidence_stage"`
	Organization      Organization               `json:"organization"`
	Authorized        bool                       `json:"authorized"`
	Enabled           bool                       `json:"enabled"`
	Paused            bool                       `json:"paused"`
	Summary           string                     `json:"summary"`
	Next              string                     `json:"next"`
	Validated         int                        `json:"validated"`
	Review            int                        `json:"review"`
	Running           int                        `json:"running"`
	ActiveAgents      int                        `json:"active_agents"`
	UncertainAgents   int                        `json:"uncertain_agents"`
	Total             int                        `json:"total"`
	Supervision       MissionSupervision         `json:"supervision"`
	Understanding     MissionUnderstanding       `json:"understanding"`
	Coordination      []MissionCoordinationPhase `json:"coordination"`
	Tasks             []MissionTask              `json:"tasks"`
}

func missionLaunchContract(w Work, profile LaunchProfile, budget BudgetView) MissionLaunchContract {
	scope := strings.TrimSpace(w.Scope)
	if scope == "" {
		scope = strings.TrimSpace(w.Objective)
	}
	if scope == "" {
		scope = w.Title
	}
	contract := MissionLaunchContract{
		Scope:    fmt.Sprintf("%s · %d tâche(s) · dossier %s", scope, len(w.Tasks), profile.Workspace),
		Recovery: fmt.Sprintf("Au plus %d tentatives automatiques par chaîne de reprise, premier départ compris ; un environnement inchangé attend une nouvelle vérification et les plafonds propres aux tâches restent prioritaires.", maxAutomaticAttempts),
	}
	if budget.Budget.Limit > 0 {
		contract.Budget = fmt.Sprintf("Plafond estimatif %.2f USD · %.2f USD disponibles · %.2f USD réservés par nouveau départ.", budget.Budget.Limit, budget.Remaining, budget.Budget.Reserve)
	} else {
		contract.Budget = "Aucun plafond financier configuré ; les limites d’outils et de tentatives de chaque tâche restent appliquées."
	}
	automatic, controls := 0, 0
	for i := range w.Tasks {
		if w.Tasks[i].ValidationPolicy != nil && w.Tasks[i].ValidationPolicy.Mode == "automatic" {
			automatic++
			controls += len(w.Tasks[i].ValidationPolicy.Controls)
		}
	}
	contract.Validation = fmt.Sprintf("%d tâche(s) en revue humaine · %d tâche(s) avec %d contrôle(s) automatique(s) préautorisés.", len(w.Tasks)-automatic, automatic, controls)
	return contract
}

// missionLaunchPreview asks the dispatcher for the first wave using the
// proposed common profile. It is read-only: confirmation must evaluate the
// current revision again before persisting the authorization.
func (s *Store) missionLaunchPreview(work string, profile LaunchProfile, slots int) (MissionLaunchPreview, error) {
	preview := MissionLaunchPreview{RequestedSlots: slots, Departures: []MissionLaunchItem{}, Waiting: []MissionLaunchItem{}, Limits: []string{}}
	if slots < 1 || slots > slotsMax {
		return preview, fmt.Errorf("Créneaux : 1 à %d", slotsMax)
	}
	providers, err := s.providers()
	if err != nil {
		return preview, err
	}
	provider, known := providers.Providers[profile.Provider]
	if !known {
		return preview, fmt.Errorf("Fournisseur inconnu : %s", profile.Provider)
	}
	if _, _, err = resolveModel(provider, profile.Level, "work"); err != nil {
		return preview, err
	}
	profile.Workspace, err = resolveWorkspace(s.root, profile.Workspace)
	if err != nil {
		return preview, err
	}
	w, err := s.get(work)
	if err != nil {
		return preview, err
	}
	preview.Revision = w.Revision
	preview.Organization = organization(w)
	if preview.Organization.Ready {
		if e := s.reviewerAvailable(w); e != nil {
			preview.Organization.Ready = false
			preview.Organization.Issues = append(preview.Organization.Issues, e.Error())
			preview.Organization.Next = "Rétablir le vérificateur avant le lancement."
		}
	}
	agents, err := s.agents(work)
	if err != nil {
		return preview, err
	}
	occupiedWorkspaces, err := s.activeWorkspaces()
	if err != nil {
		return preview, err
	}
	cost, err := s.costSummary(work)
	if err != nil {
		return preview, err
	}
	budget, err := s.budget(work)
	if err != nil {
		return preview, err
	}
	preview.Contract = missionLaunchContract(w, profile, budget)
	if !preview.Organization.Ready {
		preview.Limits = append(preview.Limits, preview.Organization.Issues...)
		preview.ConcurrencyDetail = preview.Organization.Next
		return preview, nil
	}

	in := dispatchInputs{work: &w, agents: agents, profile: &profile, taskCost: cost.ByTask,
		reserve: budget.Budget.Reserve, autonomy: autonomyAuto, slots: slots, paused: false,
		depsReady: map[string]bool{}, priority: s.priorities(work), occupiedWorkspaces: occupiedWorkspaces,
		launchBlocked: map[string]string{}}
	for i := range w.Tasks {
		in.depsReady[w.Tasks[i].ID] = s.dependenciesReady(&w, &w.Tasks[i])
		task := w.Tasks[i]
		if (task.Status != "todo" && task.Status != "blocked") || !in.depsReady[task.ID] || profileFor(in, task) == nil {
			continue
		}
		candidate := *profileFor(in, task)
		launch := Launch{Schema: 1, EventID: "preview-" + hash([]byte(work + "|" + task.ID + "|" + fmt.Sprint(w.Revision)))[:20],
			Revision: w.Revision, TaskID: task.ID, Provider: candidate.Provider, Role: candidate.Role,
			Workspace: candidate.Workspace, Instruction: candidate.Instruction, Level: candidate.Level,
			Timeout: candidate.Timeout, Capture: candidate.Capture, Limits: candidate.Limits, Origin: originMissionPreview}
		if _, _, err := s.prepareLaunch(work, launch, true); err != nil {
			in.launchBlocked[task.ID] = err.Error()
		}
	}
	plan, _ := planDispatch(in)
	starts := map[string]dispatchDecision{}
	remaining := budget.Remaining
	for _, decision := range plan {
		if budget.Budget.Limit > 0 && budget.Budget.Reserve > remaining {
			in.launchBlocked[decision.TaskID] = "Budget estimatif insuffisant : nouveaux départs suspendus ; examiner le budget."
			continue
		}
		if budget.Budget.Limit > 0 {
			remaining -= budget.Budget.Reserve
		}
		starts[decision.TaskID] = decision
		task, _ := w.task(decision.TaskID)
		title := decision.TaskID
		if task != nil {
			title = task.Title
		}
		preview.Departures = append(preview.Departures, MissionLaunchItem{ID: decision.TaskID, Title: title, Workspace: decision.Profile.Workspace})
	}
	workspaces := []string{}
	for _, task := range w.Tasks {
		if task.Status != "todo" && task.Status != "blocked" {
			continue
		}
		if p := profileFor(in, task); p != nil {
			for _, workspace := range workspaces {
				if workspaceOverlap(workspace, p.Workspace) {
					preview.SharedWorkspace = true
					break
				}
			}
			workspaces = append(workspaces, p.Workspace)
		}
		if _, starting := starts[task.ID]; starting {
			continue
		}
		state, reason := missionDispatchState(in, task)
		if blocked := in.launchBlocked[task.ID]; blocked != "" {
			state, reason = "intervention", blocked
		}
		normal := state == "waiting"
		if state == "ready" {
			reason = "Attend un créneau libre ou la libération de l’espace partagé"
			normal = true
		}
		preview.Waiting = append(preview.Waiting, MissionLaunchItem{ID: task.ID, Title: task.Title, Reason: reason, Normal: normal})
	}
	preview.Immediate = len(preview.Departures)
	preview.EffectiveConcurrency = len(preview.Departures)
	switch {
	case preview.SharedWorkspace && preview.EffectiveConcurrency <= 1:
		preview.ConcurrencyMode = "sequential_shared"
		preview.ConcurrencyDetail = "Mode séquentiel sur les espaces partagés ou imbriqués : les créneaux sont un plafond, jamais une promesse, et le moteur n’autorise qu’un écrivain à la fois dans un même arbre de fichiers."
	case preview.EffectiveConcurrency > 1:
		preview.ConcurrencyMode = "parallel_isolated_unintegrated"
		preview.ConcurrencyDetail = "Départs parallèles possibles uniquement sur des espaces déjà configurés et disjoints. Swarm ne crée, ne nettoie ni n’intègre ces copies : leurs changements doivent être contrôlés et intégrés séparément."
	default:
		preview.ConcurrencyMode = "bounded"
		preview.ConcurrencyDetail = "La concurrence annoncée est la première vague calculée maintenant ; créneaux, dépendances, budget et espaces disponibles peuvent la borner."
	}
	preview.Limits = append(preview.Limits,
		fmt.Sprintf("Au plus %d départ(s) simultané(s), selon les espaces réellement disponibles.", slots),
		"Un même espace de travail est utilisé par un seul agent à la fois.",
		fmt.Sprintf("Après %d tentatives automatiques infructueuses, une décision humaine est requise.", maxAutomaticAttempts),
		"Les dépendances, budgets, arrêts demandés et validations restent bloquants.")
	tokenPayload := struct {
		Preview    MissionLaunchPreview `json:"preview"`
		Budget     BudgetView           `json:"budget"`
		Workspaces []string             `json:"occupied_workspaces"`
	}{Preview: preview, Budget: budget, Workspaces: occupiedWorkspaces}
	raw, _ := json.Marshal(tokenPayload)
	preview.Token = hash(raw)
	return preview, nil
}

func descendantCount(w *Work, id string) int {
	seen := map[string]bool{id: true}
	changed := true
	for changed {
		changed = false
		for _, t := range w.Tasks {
			if seen[t.ID] {
				continue
			}
			for _, dep := range t.Depends {
				if seen[dep] {
					seen[t.ID] = true
					changed = true
					break
				}
			}
		}
	}
	return len(seen) - 1
}
func (s *Store) missionStatus(work string) (MissionStatus, error) {
	d := MissionStatus{Tasks: []MissionTask{}}
	w, e := s.get(work)
	if e != nil {
		return d, e
	}
	p, e := s.missionPolicy(work)
	if e != nil {
		return d, e
	}
	d.Authorized = p.Enabled && s.autonomy(work) == autonomyAuto
	d.Paused = s.paused(work)
	d.Supervision, e = s.missionSupervision(work, time.Now())
	if e != nil {
		return d, e
	}
	d.Enabled = d.Authorized && !d.Paused && d.Supervision.State == "active"
	d.Total = len(w.Tasks)
	agents, e := s.agents(work)
	if e != nil {
		return d, e
	}
	for _, agent := range agents {
		if !activeAgent(agent) {
			continue
		}
		switch observedAgent(agent) {
		case "queued", "starting", "running", "stopping":
			d.ActiveAgents++
		default:
			d.UncertainAgents++
		}
	}
	occupiedWorkspaces, e := s.activeWorkspaces()
	if e != nil {
		return d, e
	}
	cost, e := s.costSummary(work)
	if e != nil {
		return d, e
	}
	budget, e := s.budget(work)
	if e != nil {
		return d, e
	}
	in := dispatchInputs{work: &w, agents: agents, profile: w.Profile, taskCost: cost.ByTask,
		reserve: budget.Budget.Reserve, autonomy: s.autonomy(work), slots: s.slots(work), paused: d.Paused,
		depsReady: map[string]bool{}, priority: s.priorities(work), occupiedWorkspaces: occupiedWorkspaces}
	for i := range w.Tasks {
		in.depsReady[w.Tasks[i].ID] = s.dependenciesReady(&w, &w.Tasks[i])
	}
	validation := s.validationState(&w)
	for i := range w.Tasks {
		t := &w.Tasks[i]
		v := validation.Tasks[t.ID]
		x := MissionTask{ID: t.ID, Title: t.Title, State: t.Status, Deliverable: t.Deliverable, Target: t.ID, Impact: descendantCount(&w, t.ID), ValidationMode: "human"}
		if t.ValidationPolicy != nil {
			x.ValidationMode = t.ValidationPolicy.Mode
		}
		if t.AutoValidation != nil {
			x.ValidationReceipt = t.AutoValidation.Receipt
		}
		x.Result = s.resultPresentation(&w, t, agents, v)
		switch {
		case x.Result.State == "validated":
			d.Validated++
			x.State = "validated"
			x.Reason = x.Result.Reason
			x.Action = "report"
			x.Label = "Voir le résultat"
		case x.Result.State == "validation_stale":
			x.State = "intervention"
			x.Reason = x.Result.Reason
			x.Action = "reopen"
			x.Label = "Revalider les preuves"
		case x.Result.State == "waived":
			x.State, x.Reason, x.Action, x.Label = "waived", x.Result.Reason, "inspect", "Voir la dérogation"
		case x.Result.State == "abandoned":
			x.State, x.Reason, x.Action, x.Label = "abandoned", x.Result.Reason, "inspect", "Voir la décision"
		case x.Result.State == "result_to_review" || x.Result.State == "validation_withheld":
			d.Review++
			x.State = "review"
			x.Reason = x.Result.Reason
			x.Action = "report"
			x.Label = "Examiner le résultat"
			if x.ValidationMode == "automatic" && x.Result.ValidationState == "missing_gate" {
				x.Reason = "Contrôles automatiques préautorisés retenus ; vérifier pause, autorisation et preuves"
			}
			if x.Result.ValidationState == "ready_for_decision" {
				x.Action = "accepted"
				x.Label = "Valider après revue"
			}
		case x.Result.State == "in_progress" || x.Result.State == "execution_unconfirmed":
			d.Running++
			x.State = "running"
			x.Reason = x.Result.Reason
			x.Action = "inspect"
			x.Label = "Suivre l’agent"
		case x.Result.State == "stopped_early" || x.Result.State == "completed_unproven":
			x.State = "intervention"
			x.Reason = x.Result.Reason
			x.Action = "inspect"
			x.Label = "Examiner avant reprise"
		default:
			x.Action = "inspect"
			x.Label = "Comprendre et résoudre"
			x.Reason = t.Blocker
			for _, dep := range t.Depends {
				parent, _ := w.task(dep)
				if !s.acceptedFresh(&w, parent, map[string]bool{}) {
					x.State = "waiting"
					x.Target = dep
					x.Reason = "Attend la validation de " + dep
					if parent != nil {
						x.Reason = "Attend " + parent.Title
					}
					x.Label = "Examiner le prérequis"
					break
				}
			}
			if x.State != "waiting" {
				x.State, x.Reason = missionDispatchState(in, *t)
				if x.State == "ready" {
					if !s.assistCanStart(&w, t) {
						x.State, x.Reason = "intervention", s.startBlockReason(&w, t)
					} else if profile := profileFor(in, *t); profile != nil {
						var occupied int
						e = s.db.QueryRow("SELECT count(*) FROM agents WHERE status IN ('queued','starting','running','stopping') AND (cwd=? OR instr(cwd, ? || '/')=1 OR instr(?, cwd || '/')=1)", profile.Workspace, profile.Workspace, profile.Workspace).Scan(&occupied)
						if e != nil {
							return d, e
						}
						if occupied > 0 {
							x.State, x.Reason = "waiting", "Espace occupé par une tentative ; reprise après sa libération"
						}
					}
					if x.State == "ready" {
						x.Action, x.Label = "start", "Lancer cette tâche"
						if !d.Authorized {
							x.State, x.Reason = "manual", "Lancement manuel possible ; mission continue non autorisée"
						} else if !d.Enabled {
							x.State, x.Action, x.Label = "waiting", "supervise", "Voir comment reprendre"
							if d.Supervision.State == "error" {
								x.Reason = "Conducteur présent, mais sa dernière vérification est en erreur"
							} else {
								x.Reason = "Autorisation enregistrée ; aucun conducteur actif observé"
							}
						}
					}
				}
				if profileFor(in, *t) == nil && x.State == "intervention" {
					x.State, x.Reason, x.Action, x.Label = "configure", "Configuration initiale à préparer avant le premier départ", "configure", "Préparer le lancement"
				}
				if t.LaunchHeld {
					x.State, x.Action, x.Label = "intervention", "prepare", "Autoriser le plan dans Préparer"
				}

			}
		}
		x.Understanding = taskUnderstanding(x, d, agents)
		for _, agent := range agents {
			if agent.TaskID != t.ID {
				continue
			}
			diagnostic := fallbackAttemptDiagnostic(agent)
			if len(diagnostic.Items) > 0 {
				x.Diagnostic = &diagnostic
			}
			break
		}
		d.Tasks = append(d.Tasks, x)
	}
	d.Summary = fmt.Sprintf("%d/%d résultats validés · %d à examiner · %d tâche(s) en cours · %d agent(s) actif(s) confirmé(s) · %d état(s) agent incertain(s)", d.Validated, d.Total, d.Review, d.Running, d.ActiveAgents, d.UncertainAgents)
	d.Next = "Choisissez Poursuivre la mission pour enchaîner automatiquement les tâches autorisées."
	if d.Enabled {
		d.Next = "Les tâches prêtes démarrent automatiquement ; les espaces occupés sont réessayés à leur libération."
	}
	interventions := 0
	for _, task := range d.Tasks {
		if task.State == "intervention" {
			interventions++
		}
	}
	configurations := 0
	for _, task := range d.Tasks {
		if task.State == "configure" {
			configurations++
		}
	}
	if configurations > 0 {
		d.Next = fmt.Sprintf("Une configuration commune préparera le lancement de %d tâche(s).", configurations)
	} else if interventions > 0 {
		d.Next = fmt.Sprintf("%d intervention(s) nécessaire(s) : consultez les actions signalées pour reprendre la mission.", interventions)
	}
	if d.Review > 0 {
		d.Next = "Examinez les résultats signalés. Après validation, la mission continue reprend seule si elle est active."
	}
	if d.Paused {
		d.Next = "Mission en pause : aucun nouveau départ automatique. Les agents déjà lancés restent supervisés."
	}
	if d.Total > 0 && d.Validated == d.Total {
		d.Next = "Tous les résultats sont actuellement validés. Consultez les livrables ci-dessous."
	}
	if d.Authorized && !d.Paused && d.Validated != d.Total {
		switch d.Supervision.State {
		case "error":
			d.Next += " Conducteur présent, mais dernière vérification en erreur : " + d.Supervision.LastError + ". Prochaine vérification : " + d.Supervision.NextCheckRelative + "."
		case "absent":
			d.Next += " Autorisation enregistrée, mais aucun conducteur actif observé : démarrer le serveur web ou `swarm mission watch " + work + "`."
		}
	}
	d.Understanding = missionUnderstanding(d)
	if w.Planning != nil {
		root, _ := w.Planning.scope("root")
		if w.Planning.Failure != "" {
			d.Understanding = understanding(w.Planning.Failure, "Corrigez la cause puis reprenez la planification.", "Vous", "user", "decision_humaine")
		} else if root != nil && root.State != "closed" && (w.Planning.Activations >= w.Planning.MaxActivations || w.Planning.Decisions >= w.Planning.MaxDecisions) {
			d.Understanding = understanding("Le plafond de planification est atteint.", "Examinez les décisions et préparez une nouvelle mission bornée pour le travail restant.", "Vous", "user", "decision_humaine")
		} else if w.Planning.Paused {
			d.Understanding = understanding("La planification est en pause.", "Reprenez lorsque vous êtes prêt.", "Vous", "user", "decision_humaine")
		} else if root != nil && root.State != "closed" && (d.Total == 0 || d.Total == d.Validated) {
			d.Understanding = understanding("Les résultats attendent une décision du responsable du périmètre.", "Le responsable examine les retours avant de clore ou compléter le plan.", "Le planificateur", "supervisor", "attente_normale")
		}
		if d.Understanding.Situation != "termine" {
			d.Next = d.Understanding.NextStep
		}
	}
	exchanges, exchangeErr := s.agentExchanges(work)
	if exchangeErr != nil {
		return d, exchangeErr
	}
	d.EvidenceStage = "Parcours de cette mission non encore vérifié jusqu’à la clôture. Une recette isolée ou une installation ne vaut pas cette preuve."
	if w.Planning != nil && d.Total > 0 && d.Validated == d.Total {
		if root, err := w.Planning.scope("root"); err == nil && root.State == "closed" {
			d.EvidenceStage = "Résultats validés et responsabilité racine clôturée dans cette mission."
		}
	}
	d.Organization = organization(w)
	if !d.Organization.Ready {
		d.Enabled = false
		d.Understanding = understanding(d.Organization.Label, d.Organization.Next, "Vous", "user", "decision_humaine")
		d.Next = d.Organization.Next
	}
	d.ProviderCooldowns, e = s.missionProviderCooldowns(w, time.Now())
	if e != nil {
		return d, e
	}
	if d.Organization.Ready && d.Understanding.Situation != "termine" && len(d.ProviderCooldowns) > 0 {
		messages := []string{}
		unknown := false
		for _, c := range d.ProviderCooldowns {
			messages = append(messages, c.message())
			unknown = unknown || c.ResetAt == 0
		}
		next := "Attendre l’échéance ; les plafonds de tentatives et les autres blocages restent applicables."
		actor, kind, situation := "Le fournisseur IA", "none", "attente_normale"
		if unknown {
			next = "Vérifier la disponibilité du compte puis lever explicitement l’attente fournisseur sans heure de reprise."
			actor, kind, situation = "Vous", "user", "decision_humaine"
		}
		if d.Paused {
			next += " La mission reste en pause ; aucun redémarrage automatique n’est autorisé."
		}
		exhausted := []string{}
		for _, task := range w.Tasks {
			if task.Status == "blocked" && task.PlanMaxAttempts > 0 && len(task.Attempts) >= task.PlanMaxAttempts {
				exhausted = append(exhausted, task.Title)
			}
		}
		if len(exhausted) > 0 {
			messages = append(messages, "Plafond de tentatives atteint : "+strings.Join(exhausted, ", ")+". La fin du quota ne débloquera pas ces tâches.")
			next += " Examiner les tâches ayant épuisé leurs tentatives ; aucun nouveau départ n’est autorisé pour elles."
			actor, kind, situation = "Vous", "user", "decision_humaine"
		}
		d.Understanding = understanding(strings.Join(messages, "\n"), next, actor, kind, situation)
		d.Next = next
	}
	d.Coordination = missionCoordinationPhases(&w, d, exchanges, time.Now())
	return d, nil
}

func (s *Store) missionProviderCooldowns(w Work, at time.Time) ([]ProviderCooldown, error) {
	names := map[string]bool{}
	if w.Planning != nil {
		names[w.Planning.Provider] = true
		if w.Planning.Reviewer != nil {
			names[w.Planning.Reviewer.Provider] = true
		}
	}
	if w.Profile != nil {
		names[w.Profile.Provider] = true
	}
	for _, task := range w.Tasks {
		if task.Profile != nil && task.Status != "accepted" && task.Status != "abandoned" && task.Status != "waived" {
			names[task.Profile.Provider] = true
		}
	}
	delete(names, "")
	ordered := make([]string, 0, len(names))
	for name := range names {
		ordered = append(ordered, name)
	}
	sort.Strings(ordered)
	var active []ProviderCooldown
	for _, name := range ordered {
		c, _, err := s.providerCooldown(name)
		if err != nil {
			return nil, err
		}
		if c != nil && c.active(at) {
			active = append(active, *c)
		}
	}
	return active, nil
}

func missionCoordinationPhases(w *Work, d MissionStatus, exchanges []AgentExchange, at time.Time) []MissionCoordinationPhase {
	phases := []MissionCoordinationPhase{{Kind: "work", Label: "Travail"}, {Kind: "wait", Label: "Attente"}, {Kind: "exchange", Label: "Échanges"}, {Kind: "validation", Label: "Vérification"}}
	if d.Running > 0 {
		phases[0].Summary = fmt.Sprintf("%d tâche(s) en cours ; %d agent(s) actif(s) confirmé(s).", d.Running, d.ActiveAgents)
		phases[0].Actor, phases[0].NextStep = "Les agents", "Attendre le résultat ou ouvrir leur journal."
	} else {
		phases[0].Summary, phases[0].Actor, phases[0].NextStep = "Aucune tâche n’est actuellement déclarée en cours.", "Personne pour le moment", "Suivre la prochaine action indiquée."
	}
	waiting := []MissionTask{}
	for _, task := range d.Tasks {
		if task.State == "waiting" || task.State == "ready" {
			waiting = append(waiting, task)
		}
	}
	if len(waiting) > 0 {
		first := waiting[0]
		phases[1].Summary = first.Title + " — " + first.Reason
		if len(waiting) > 1 {
			phases[1].Summary += fmt.Sprintf(" · %d autre(s) attente(s)", len(waiting)-1)
		}
		phases[1].Actor, phases[1].NextStep = first.Understanding.Actor, first.Understanding.NextStep
	} else {
		phases[1].Summary, phases[1].Actor, phases[1].NextStep = "Aucune attente normale signalée.", "Personne pour le moment", "Aucune résolution d’attente requise."
	}
	if len(exchanges) == 0 {
		phases[2].Summary, phases[2].Actor, phases[2].NextStep = "Aucun échange structuré enregistré.", "Personne pour le moment", "Les remises et demandes d’aide apparaîtront ici."
	} else {
		x := exchanges[len(exchanges)-1]
		for i := len(exchanges) - 1; i >= 0; i-- {
			if exchanges[i].State == "pending" || exchanges[i].State == "acknowledged" || exchanges[i].State == "escalated" || exchanges[i].State == "stale" {
				x = exchanges[i]
				break
			}
		}
		source, recipient := x.SourceTask, x.RecipientTask
		if task, err := w.task(source); err == nil {
			source = task.Title
		}
		if task, err := w.task(recipient); err == nil {
			recipient = task.Title
		}
		action := map[string]string{"handoff": "remet un résultat à", "help_request": "demande de l’aide à", "help_answer": "répond à"}[x.Kind]
		if action == "" {
			action = "échange avec"
		}
		stateLabel := map[string]string{"pending": "à traiter", "acknowledged": "pris en charge", "consumed": "traité", "answered": "réponse envoyée", "escalated": "délai dépassé", "stale": "résultat périmé"}[x.State]
		if stateLabel == "" {
			stateLabel = "à examiner"
		}
		phases[2].Summary = fmt.Sprintf("%s %s %s · %s.", source, action, recipient, stateLabel)
		if x.Need != "" {
			phases[2].Summary += " " + x.Need
		}
		phases[2].Actor, phases[2].NextStep, phases[2].At = recipient, "Le destinataire doit traiter cet échange.", x.CreatedAt
		switch x.State {
		case "consumed", "answered":
			phases[2].Actor, phases[2].NextStep = "Personne pour cet échange", "Échange terminé ; aucune action supplémentaire."
		case "acknowledged":
			phases[2].NextStep = "Le destinataire a pris la demande en charge ; sa réponse est attendue."
		case "escalated":
			phases[2].Actor, phases[2].NextStep = "Vous", "La demande n’a pas reçu de réponse à temps ; examiner le besoin."
		case "stale":
			phases[2].Actor, phases[2].NextStep = source, "Fournir un résultat à jour avant de poursuivre."
		}
		if stamp, err := time.Parse(time.RFC3339Nano, x.CreatedAt); err == nil {
			phases[2].Relative = supervisionPastRelative(at.Sub(stamp))
		}
	}
	// A completed mission can retain unconsumed exchanges for provenance.
	// Do not turn their historical state into a new instruction or mutate it.
	if d.Understanding.Situation == "termine" && len(exchanges) > 0 {
		phases[2].Summary = "Échanges conservés dans l’historique de la mission terminée."
		phases[2].Actor = "Personne pour ces échanges"
		phases[2].NextStep = "Aucune action requise ; les messages et leurs états restent consultables."
	}
	checking := 0
	for _, task := range d.Tasks {
		if task.ValidationMode == "automatic" && task.State == "review" {
			checking++
		}
	}
	if checking > 0 {
		phases[3].Summary = fmt.Sprintf("%d résultat(s) attendent leurs contrôles automatiques autorisés.", checking)
		phases[3].Actor, phases[3].NextStep = "Le superviseur", "Exécuter les contrôles puis publier une décision avec des preuves fraîches."
	} else if d.Review > 0 {
		phases[3].Summary = fmt.Sprintf("%d résultat(s) attendent une revue humaine.", d.Review)
		phases[3].Actor, phases[3].NextStep = "Vous", "Examiner le résultat et ses preuves."
	} else {
		phases[3].Summary, phases[3].Actor, phases[3].NextStep = "Aucun résultat n’attend actuellement de vérification.", "Personne pour le moment", "Les contrôles apparaîtront après une remise de résultat."
	}
	return phases
}

func understanding(what, next, actor, actorKind, situation string) MissionUnderstanding {
	return MissionUnderstanding{What: what, NextStep: next, Actor: actor, ActorKind: actorKind, Situation: situation}
}

func taskUnderstanding(t MissionTask, mission MissionStatus, agents []Agent) MissionUnderstanding {
	switch t.State {
	case "validated":
		return understanding(t.Reason, "Aucune action requise ; le résultat reste consultable.", "Personne pour le moment", "none", "termine")
	case "waived", "abandoned":
		return understanding(t.Reason, "Consultez la décision si vous devez vérifier sa portée.", "Personne pour le moment", "none", "termine")
	case "review":
		if t.ValidationMode == "automatic" {
			if mission.Paused || !mission.Authorized {
				return understanding("Le résultat attend ses contrôles automatiques autorisés.", "Reprenez ou autorisez la mission pour permettre les contrôles.", "Vous", "user", "attente_normale")
			}
			if !mission.Enabled {
				return understanding("Le résultat attend ses contrôles ; aucun superviseur actif n’est confirmé.", "Vérifiez le superviseur et les conditions de validation.", "Vous", "user", "information_manquante")
			}
			return understanding("Le résultat relève des contrôles automatiques préautorisés.", "Le superviseur vérifie les conditions avant de contrôler et valider le résultat.", "Le superviseur", "supervisor", "attente_normale")
		}
		return understanding(t.Reason, t.Label+".", "Vous", "user", "decision_humaine")
	case "intervention", "configure":
		if t.State == "intervention" {
			for _, agent := range agents {
				if agent.TaskID != t.ID {
					continue
				}
				if recoveryCategoryFor(agent) == recoveryEnvironment {
					return understanding("L’environnement de l’agent refuse l’accès à une ressource. Cette tâche ne sera pas relancée automatiquement.", "Faire vérifier l’accès signalé dans le diagnostic, puis enregistrer une nouvelle vérification avant de reprendre.", "Vous", "user", "blocage")
				}
				break
			}
		}
		if t.Action == "reopen" || t.Action == "accepted" || t.Action == "prepare" {
			return understanding(t.Reason, t.Label+".", "Vous", "user", "decision_humaine")
		}
		return understanding(t.Reason, t.Label+".", "Vous", "user", "blocage")
	case "manual":
		return understanding(t.Reason, t.Label+".", "Vous", "user", "decision_humaine")
	case "waiting":
		if mission.Paused {
			return understanding(t.Reason, "Reprenez la mission lorsque vous voulez autoriser la suite.", "Vous", "user", "attente_normale")
		}
		if t.Action == "supervise" {
			return understanding(t.Reason, "Rétablissez le superviseur avant le prochain départ.", "Vous", "user", "information_manquante")
		}
		if mission.Enabled {
			return understanding(t.Reason, "La tâche reprendra automatiquement lorsque la condition sera remplie.", "Le superviseur", "supervisor", "attente_normale")
		}
		return understanding(t.Reason, "Traitez d’abord le prérequis indiqué, puis revenez à cette tâche.", "Vous", "user", "attente_normale")
	case "ready":
		return understanding(t.Reason, "Le prochain contrôle lancera la tâche si les conditions restent remplies.", "Le superviseur", "supervisor", "attente_normale")
	case "running":
		for _, agent := range agents {
			if agent.TaskID != t.ID || !activeAgent(agent) {
				continue
			}
			switch observedAgent(agent) {
			case "running":
				return understanding("Un agent travaille sur cette tâche.", "Attendez son résultat ou ouvrez sa session pour voir les faits reçus.", "L’agent", "agent", "en_cours")
			case "queued", "starting", "stopping":
				return understanding("Une tentative est enregistrée, mais aucun travail en cours n’est affirmé.", "Le superviseur vérifie son démarrage ou son arrêt.", "Le superviseur", "supervisor", "attente_normale")
			default:
				return understanding("La tâche est marquée en cours, mais l’état de son agent n’est pas confirmé.", "Le superviseur doit vérifier la tentative avant toute reprise.", "Le superviseur", "supervisor", "information_manquante")
			}
		}
		return understanding("La tâche est marquée en cours, mais aucun agent actif n’est visible.", "Le superviseur doit vérifier la tentative avant toute reprise.", "Le superviseur", "supervisor", "information_manquante")
	default:
		return understanding(t.Reason, t.Label+".", "Vous", "user", "information_manquante")
	}
}

func missionUnderstanding(d MissionStatus) MissionUnderstanding {
	if d.Total == 0 {
		return understanding("Cette mission ne contient aucune tâche.", "Préparez les tâches à réaliser.", "Vous", "user", "decision_humaine")
	}
	if d.Total == d.Validated {
		return understanding("Tous les résultats sont validés sur les preuves actuelles.", "Aucune action requise ; les livrables restent consultables.", "Personne pour le moment", "none", "termine")
	}
	if d.Review > 0 {
		human := 0
		var automatic MissionUnderstanding
		for _, task := range d.Tasks {
			if task.State != "review" {
				continue
			}
			if task.ValidationMode == "automatic" {
				automatic = taskUnderstanding(task, d, nil)
			} else {
				human++
			}
		}
		if human == 0 && automatic.What != "" {
			return automatic
		}
		return understanding(fmt.Sprintf("%d résultat(s) attendent une décision de validation.", human), "Examinez les résultats signalés.", "Vous", "user", "decision_humaine")
	}
	blocked := 0
	configured := 0
	for _, task := range d.Tasks {
		if task.State == "intervention" {
			blocked++
		}
		if task.State == "configure" {
			configured++
		}
	}
	if blocked == 1 {
		for _, task := range d.Tasks {
			if task.State == "intervention" && task.Understanding.What != "" {
				result := task.Understanding
				result.What = task.Title + " — " + result.What
				return result
			}
		}
	}
	if blocked > 0 {
		return understanding(fmt.Sprintf("%d tâche(s) ne peuvent pas continuer sans intervention.", blocked), "Ouvrez les tâches signalées et choisissez la reprise.", "Vous", "user", "blocage")
	}
	if configured > 0 {
		return understanding(fmt.Sprintf("%d tâche(s) attendent leur configuration de lancement.", configured), "Préparez la configuration commune.", "Vous", "user", "decision_humaine")
	}
	if d.Paused {
		return understanding("La mission est en pause ; aucun nouveau départ automatique n’est autorisé.", "Reprenez la mission lorsque vous voulez continuer.", "Vous", "user", "attente_normale")
	}
	if d.Running > 0 {
		if d.ActiveAgents > 0 {
			return understanding(fmt.Sprintf("%d tâche(s) sont en cours et %d tentative(s) active(s) sont observées.", d.Running, d.ActiveAgents), "Attendez les résultats ou ouvrez les sessions pour voir les faits reçus.", "L’agent", "agent", "en_cours")
		}
		return understanding(fmt.Sprintf("%d tâche(s) sont marquées en cours, sans agent actif confirmé.", d.Running), "Le superviseur doit vérifier les tentatives avant toute reprise.", "Le superviseur", "supervisor", "information_manquante")
	}
	if d.Enabled {
		return understanding("La mission attend que les conditions du prochain départ soient réunies.", "Le superviseur vérifiera et lancera les tâches devenues prêtes.", "Le superviseur", "supervisor", "attente_normale")
	}
	if d.Authorized {
		if d.Supervision.State == "error" {
			return understanding("La mission est autorisée, mais la dernière vérification du superviseur a échoué.", "Consultez l’erreur et la prochaine vérification annoncée.", "Le superviseur", "supervisor", "information_manquante")
		}
		return understanding("La mission est autorisée, mais aucun superviseur actif n’est observé.", "Démarrez le serveur web ou la veille de mission.", "Vous", "user", "information_manquante")
	}
	return understanding("La suite automatique n’est pas autorisée.", "Lancez une tâche ou autorisez la mission continue.", "Vous", "user", "decision_humaine")
}

// Classify using the actual dispatcher, projected onto this task. More specific
// explanations cover intentional silent skips; no independent ready predicate.
func missionDispatchState(in dispatchInputs, t Task) (string, string) {
	w := *in.work
	w.Tasks = []Task{t}
	in.work = &w
	// Explain durable recovery decisions before temporary slot/pause waiting.
	var latest *Agent
	recoveryAllowed := false
	for _, a := range in.agents {
		if a.TaskID == t.ID {
			candidate := a
			latest = &candidate
			break
		}
	}
	if latest != nil && (latest.Status == "failed" || latest.Status == "interrupted") && !(latest.Status == "interrupted" && latest.StopKind == originOperator) {
		recovery := assessRecovery(*latest, t, func() time.Time {
			if in.at.IsZero() {
				return time.Now()
			}
			return in.at
		}())
		switch recovery.Disposition {
		case recoveryDispositionRetry, recoveryDispositionCorrection:
			recoveryAllowed = true
		case recoveryDispositionWait:
			if recovery.Category == recoveryEnvironment {
				return "intervention", recovery.Reason
			}
			return "waiting", recovery.Reason
		default:
			return "intervention", recovery.Reason
		}
	}
	if !recoveryAllowed && t.Status == "blocked" && strings.TrimSpace(t.Blocker) != "" {
		return "intervention", t.Blocker
	}
	for _, a := range in.agents {
		if a.TaskID != t.ID {
			continue
		}
		if t.Status == "blocked" && a.Status == "completed" {
			return "intervention", "Tentative terminée sans rapport relayé : examiner le rapport avant toute reprise"
		}
		if a.Status == "interrupted" && a.StopKind == originOperator {
			return "intervention", "Arrêt demandé par l’opérateur : reprise explicite nécessaire"
		}
	}
	if t.LaunchHeld {
		return "intervention", "Démarrage du plan à autoriser depuis la préparation"
	}
	if in.reserve > 0 && in.taskCost[t.ID].Reported > costThresholdFactor*in.reserve {
		return "intervention", "Seuil de coût de la tâche dépassé : examiner les dépenses avant un nouveau départ"
	}
	if profileFor(in, t) == nil {
		return "intervention", "Aucun profil de lancement : configurer Poursuivre la mission ou le profil de cette tâche"
	}
	if !in.depsReady[t.ID] {
		return "waiting", "Dépendance non validée ou périmée"
	}
	if in.launchBlocked[t.ID] != "" {
		return "intervention", in.launchBlocked[t.ID]
	}
	plan, reason := planDispatch(in)
	if len(plan) > 0 {
		return "ready", "Prête pour un départ automatique"
	}
	if in.paused {
		return "waiting", "Mission en pause : reprendre la mission pour autoriser les départs"
	}
	if in.autonomy != autonomyAuto {
		return "manual", "Départs automatiques désactivés ; poursuivre la mission ou lancer explicitement"
	}
	if strings.Contains(reason, "occupé") {
		return "waiting", reason
	}
	if reason == "" {
		reason = "Départ automatique retenu : examiner les conditions de lancement"
	}
	return "intervention", reason
}
