package main

// Oracle des actions humaines par tâche : source de vérité unique consommée
// par le cockpit web, la console TUI et l'assistant IA. Le moteur (store.go)
// reste l'arbitre final ; cet oracle ne fait que refléter ses règles pour
// qu'aucun opérateur n'ait à découvrir une transition refusée après coup.

// TaskField décrit un champ demandé à l'humain pour une action : tout champ
// porte un libellé explicite et une aide (décision de cadrage 2026-09-12).
type TaskField struct {
	Name   string `json:"name"`
	Label  string `json:"label"`
	Aide   string `json:"aide,omitempty"`
	Requis bool   `json:"requis,omitempty"`
}

// TaskAction décrit une action proposée sur une tâche : disponibilité, motif
// d'indisponibilité, statut conseillé pour l'état courant et champs attendus.
type TaskAction struct {
	Prepared   *PreparedLaunch `json:"prepared_launch,omitempty"`
	Kind       string          `json:"kind"`
	Label      string          `json:"label"`
	Disponible bool            `json:"disponible"`
	Raison     string          `json:"raison,omitempty"`
	Conseillee bool            `json:"conseillee,omitempty"`
	Champs     []TaskField     `json:"champs,omitempty"`
}

type PreparedLaunch struct {
	ID          string     `json:"id"`
	Task        string     `json:"task"`
	Ready       bool       `json:"ready"`
	Reason      string     `json:"reason,omitempty"`
	Provider    string     `json:"provider,omitempty"`
	Workspace   string     `json:"workspace,omitempty"`
	Instruction string     `json:"instruction,omitempty"`
	Timeout     int        `json:"timeout_seconds,omitempty"`
	Limits      *RunLimits `json:"limits,omitempty"`
}

func champ(name, label, aide string, requis bool) TaskField {
	return TaskField{Name: name, Label: label, Aide: aide, Requis: requis}
}

func action(kind, label string, dispo bool, raison string) TaskAction {
	return TaskAction{Kind: kind, Label: label, Disponible: dispo, Raison: raison}
}

// raisonAction retourne le motif de l'oracle, avec repli pour les tentatives
// sans tâche rattachable (oracle absent).
func raisonAction(oracle map[string]TaskAction, kind, repli string) string {
	if ta, ok := oracle[kind]; ok && ta.Raison != "" {
		return ta.Raison
	}
	return repli
}

// taskActions calcule les actions disponibles, leur motif et l'action
// conseillée pour l'état courant de la tâche. Les conditions reprennent les
// transitions de store.go et les vérifications de launch/override/report.
func (s *Store) taskActions(w *Work, t *Task, agents []Agent) []TaskAction {
	taskActive := false
	stopAvailable := false
	for _, a := range agents {
		if a.TaskID == t.ID && activeAgent(a) {
			taskActive = true
			desired, _ := s.desired(a.ID)
			if desired != "stop" {
				stopAvailable = true
			}
		}
	}
	gates := s.gateFiles(t.ID)
	canStart := s.assistCanStart(w, t)
	accepted := t.Status == "accepted"
	acceptedFresh := accepted && s.validGate(t)

	startRaison := map[string]string{
		"accepted":  "Tâche acceptée : rouvrez-la (action conseillée) avant de relancer un agent.",
		"waived":    "Tâche acceptée par dérogation : rouvrez-la avant de relancer un agent.",
		"abandoned": "Tâche abandonnée : rouvrez-la avant de relancer un agent.",
	}[t.Status]
	var lastFinished *Agent
	for i := range agents {
		if agents[i].TaskID == t.ID && !activeAgent(agents[i]) {
			lastFinished = &agents[i]
			break
		}
	}
	if lastFinished != nil && requiresEnvironmentVerification(*lastFinished) {
		startRaison = "Échec d’environnement identifié : utilisez la reprise et décrivez une vérification nouvelle des préconditions."
	}
	if t.Status == "running" {
		if taskActive {
			startRaison = "Un agent est actif sur cette tâche : attendre sa fin ou l'arrêter avant de relancer."
		} else {
			startRaison = "Aucun agent actif : réconciliez l'état observé avant de relancer."
		}
	}
	if startRaison == "" && !canStart {
		startRaison = s.startBlockReason(w, t)
	}

	// Relancer exige une tentative terminée sur cette tâche (comme le garde
	// de la console : sans elle, le web n'a aucun agent à reprendre) et les
	// mêmes préconditions qu'un départ : la relance reprend la tentative et
	// repasse la tâche en running via task.update.
	stopRaison := ""
	retryDispo, retryRaison := false, ""
	if t.LaunchHeld {
		retryRaison = s.startBlockReason(w, t)
	} else if taskActive {
		retryRaison = "Une tentative est en cours : attendez sa fin ou arrêtez-la."
	} else if !hasFinishedAttempt(t, agents) {
		retryRaison = "Aucune tentative à relancer : choisir Lancer un agent."
	} else if !canStart {
		retryRaison = s.startBlockReason(w, t)
	} else {
		retryDispo = true
	}
	if !taskActive {
		stopRaison = "Aucune tentative active sur cette tâche."
	} else if !stopAvailable {
		stopRaison = "Arrêt déjà demandé ; confirmation du superviseur attendue."
	}
	reconcileRaison := ""
	if !taskActive {
		reconcileRaison = "Aucune tentative active sur cette tâche."
	}

	submitRaison := ""
	if taskActive {
		submitRaison = "Une tentative est en cours : attendez sa fin avant de soumettre un rapport."
	} else if t.Status != "todo" && t.Status != "blocked" {
		submitRaison = "Un rapport se soumet depuis une tâche « À faire » ou « Bloquée » qui vient de produire son livrable."
	}

	gateRaison := ""
	if t.Status == "accepted" || t.Status == "waived" || t.Status == "abandoned" {
		gateRaison = "Tâche validée : rouvrez-la avant une nouvelle évaluation."
	} else if t.Revalidation != nil && t.Status != "submitted" {
		gateRaison = "Revalidation en attente : soumettez un nouveau rapport avant la gate."
	}

	acceptRaison := ""
	if t.Status != "submitted" {
		acceptRaison = "Un rapport doit être soumis avant l'acceptation."
	} else if !s.validGate(t) {
		acceptRaison = "Examinez puis enregistrez une gate delivery avant d'accepter."
	}

	if e := s.independentReviewGuard(w, t); e != nil && t.Status == "submitted" {
		acceptRaison = e.Error()
	}
	overrideRaison := ""
	if taskActive || t.Status == "running" {
		overrideRaison = "Arrêtez et réconciliez la tentative avant dérogation."
	} else if t.Status == "waived" {
		overrideRaison = "Dérogation déjà enregistrée : rouvrez la tâche pour la modifier."
	} else if t.Status == "abandoned" {
		overrideRaison = "Rouvrez la tâche abandonnée avant dérogation."
	} else {
		for _, id := range t.Depends {
			d, e := w.task(id)
			if e != nil || !s.acceptedFresh(w, d, map[string]bool{}) {
				overrideRaison = "Acceptez d'abord la dépendance " + id + " (revue ou dérogation)."
				break
			}
		}
	}

	reopenRaison := ""
	if !accepted && t.Status != "waived" && t.Status != "abandoned" {
		reopenRaison = "Seule une tâche acceptée, dérogée ou abandonnée peut être rouverte."
	}

	actions := []TaskAction{
		action("start", "Lancer un agent", canStart && startRaison == "", startRaison),
		action("retry", "Relancer une tentative", retryDispo, retryRaison),
		action("stop", "Arrêter la tentative", stopAvailable, stopRaison),
		action("reconcile", "Réconcilier l'état observé", taskActive, reconcileRaison),
		action("report", "Lire le rapport", true, ""),
		action("submit", "Soumettre le rapport", submitRaison == "", submitRaison),
		action("gate", "Examiner et enregistrer une gate", gateRaison == "", gateRaison),
		action("accepted", "Accepter après revue", acceptRaison == "", acceptRaison),
		action("override", "Accepter par dérogation motivée", overrideRaison == "", overrideRaison),
		action("reopen", "Rouvrir la tâche", reopenRaison == "", reopenRaison),
		action("assign", "Changer le responsable", true, ""),
	}
	if t.Status == "blocked" && t.PlanMaxAttempts > 0 && len(t.Attempts) >= t.PlanMaxAttempts {
		reason := attemptExtensionReason(t, agents)
		actions = append(actions, action("extend-attempt", "Autoriser une tentative supplémentaire", reason == "", reason))
		if t.PlanMaxAttempts >= 3 {
			reason = correctiveRecoveryReason(w, t, agents)
			actions = append(actions, action("authorize-recovery", "Préparer un essai correctif", reason == "", reason))
		}
	}

	champs := map[string][]TaskField{
		"start": {
			champ("provider", "Fournisseur", "Programme agent à lancer (défini dans .swarm/providers.json).", true),
			champ("role", "Rôle", "Worker réalise la tâche ; planner et subplanner planifient sans coder.", false),
			champ("workspace", "Espace de travail", "Répertoire dans lequel l'agent travaillera.", true),
			champ("instruction", "Consigne pour l'agent", "Transmise telle quelle ; préremplie avec la prochaine action prévue.", true),
			champ("capture", "Capture détaillée des sorties", "Désactivée par défaut : activité structurée seulement.", false),
		},
		"retry": {
			champ("agent", "Tentative à relancer", "Reprend le fournisseur et l'espace de travail de cette tentative.", true),
			champ("precondition_evidence", "Vérification nouvelle des préconditions", "Obligatoire après un échec d’environnement : contrôle observé des droits, du montage ou de la ressource. Ne désactivez aucune protection.", false),
			champ("instruction", "Consigne pour l'agent", "Ajoutée au handoff de la tentative précédente.", false),
			champ("capture", "Capture détaillée des sorties", "Désactivée par défaut : activité structurée seulement.", false),
		},
		"stop":      {champ("agent", "Tentative à arrêter", "Demande d'arrêt ; vérifiez ensuite l'état observé.", true)},
		"reconcile": {champ("agent", "Tentative à réconcilier", "Rapproche l'état enregistré du processus réellement observé.", true)},
		"report":    {champ("path", "Rapport à lire", "Fichier listé parmi les rapports détectés.", true)},
		"submit":    {champ("path", "Chemin du rapport soumis", "Fichier produit par l'agent, listé parmi les rapports détectés.", true)},
		"gate": {
			champ("path", "Document de gate (preuve d'évaluation)", "Fichier .evidence.json calculé par le moteur ; aperçu avant enregistrement.", true),
			champ("name", "Nom de la gate", "Nom humain qui identifie cette évaluation dans la console et l'assistant.", true),
		},
		"override": {champ("note", "Motif de la dérogation", "Conservé avec votre compte local et la date ; les contrôles restent non PASS.", true)},
		"assign":   {champ("owner", "Responsable", "Humain ou fournisseur chargé du prochain geste sur la tâche.", false)},
		// accepted, reopen et todo ne demandent aucun champ : la décision se
		// prend après lecture de l'aperçu (rapport de revue, historique).
	}
	for i := range actions {
		actions[i].Champs = champs[actions[i].Kind]
	}

	conseillee := conseilleePour(t.Status, acceptedFresh, taskActive, len(gates) > 0)
	if !taskActive {
		if pending, e := s.preparedLaunchForTask(*w, t); e == nil && pending != nil {
			resume := action("resume-launch", "Reprendre le lancement préparé", pending.Ready && canStart, pending.Reason)
			resume.Prepared = pending
			if resume.Raison == "" && !canStart {
				resume.Raison = s.startBlockReason(w, t)
			}
			actions = append(actions, resume)
			for i := range actions {
				if actions[i].Kind == "start" || actions[i].Kind == "retry" {
					actions[i].Disponible = false
					actions[i].Raison = "Un lancement est déjà préparé. Reprendre cette opération conserve sa copie et évite un doublon."
				}
			}
			conseillee = "resume-launch"
		}
	}
	if t.Status == "blocked" && attemptExtensionReason(t, agents) == "" {
		conseillee = "extend-attempt"
	}
	if correctiveRecoveryReason(w, t, agents) == "" {
		conseillee = "authorize-recovery"
	}
	for i := range actions {
		actions[i].Conseillee = actions[i].Kind == conseillee && actions[i].Disponible
		if actions[i].Conseillee {
			actions[i].Raison = ""
		}
	}
	return actions
}

// conseilleePour retourne l'action pertinente pour l'état courant (table de
// vérité du design, ajustée au moteur : une gate exige une tâche rouverte,
// donc une acceptation périmée conseille d'abord la réouverture).
func conseilleePour(status string, acceptedFresh, taskActive, gates bool) string {
	switch status {
	case "todo":
		return "start"
	case "running":
		if taskActive {
			return "stop"
		}
		return "reconcile"
	case "submitted":
		if gates {
			return "gate"
		}
		return "report"
	case "accepted":
		return "reopen"
	case "blocked":
		return "start"
	case "waived", "abandoned":
		return "reopen"
	}
	return ""
}

// hasFinishedAttempt indique si la tâche compte au moins une tentative
// terminée (non active) : précondition du retry, la relance reprend le
// fournisseur et l'espace de travail de cette tentative.
func hasFinishedAttempt(t *Task, agents []Agent) bool {
	for _, a := range agents {
		if a.TaskID == t.ID && !activeAgent(a) {
			return true
		}
	}
	return false
}

// startBlockReason explique pourquoi un départ est refusé alors que l'état de
// la tâche l'autoriserait (conditions assistCanStart, miroir du moteur).
func (s *Store) startBlockReason(w *Work, t *Task) string {
	if t.LaunchHeld {
		return "Démarrage non autorisé : ouvrir la préparation puis autoriser les missions de ce plan."
	}
	if s.paused(w.ID) {
		return "Départs suspendus : réautoriez les départs depuis l'en-tête du cockpit."
	}
	for _, id := range t.Depends {
		d, e := w.task(id)
		if e != nil || !s.acceptedFresh(w, d, map[string]bool{}) {
			return "Dépendance non validée ou périmée : " + id + "."
		}
	}
	if t.PlanBriefHash != "" && (w.PlanningBrief == nil || w.PlanningBrief.SHA256 != t.PlanBriefHash) {
		return "Brief du plan modifié : relisez le brief avant de lancer."
	}
	if t.PlanMaxAttempts > 0 {
		var count int
		if e := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=?", w.ID, t.ID).Scan(&count); e == nil && count >= t.PlanMaxAttempts {
			return "Plafond de tentatives du plan atteint."
		}
	}
	var overlaps int
	if e := s.db.QueryRow("SELECT count(*) FROM agents WHERE status IN ('queued','starting','running','stopping') AND (cwd=? OR instr(cwd, ? || '/')=1 OR instr(?, cwd || '/')=1)", s.root, s.root, s.root).Scan(&overlaps); e == nil && overlaps > 0 {
		return "Un agent occupe déjà cet espace de travail."
	}
	b, e := s.budget(w.ID)
	if e != nil || (b.Budget.Limit != 0 && b.Remaining < b.Budget.Reserve) {
		return "Budget estimatif insuffisant pour un nouveau départ."
	}
	return "Conditions de départ non réunies (dépendances, budget ou espace de travail)."
}

// assistCanStart vérifie qu'un départ est possible : conditions moteur
// (dépendances fraîches, brief du plan, plafond de tentatives, espace de
// travail, budget). Conservée comme condition de l'oracle.
func (s *Store) assistCanStart(w *Work, t *Task) bool {
	if t.LaunchHeld {
		return false
	}
	if s.paused(w.ID) || (t.Status != "todo" && t.Status != "blocked" && t.Status != "submitted") {
		return false
	}
	for _, id := range t.Depends {
		d, e := w.task(id)
		if e != nil || !s.acceptedFresh(w, d, map[string]bool{}) {
			return false
		}
	}
	if t.PlanBriefHash != "" && (w.PlanningBrief == nil || w.PlanningBrief.SHA256 != t.PlanBriefHash) {
		return false
	}
	if t.PlanMaxAttempts > 0 {
		var count int
		if e := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=?", w.ID, t.ID).Scan(&count); e != nil || count >= t.PlanMaxAttempts {
			return false
		}
	}
	// Sans fournisseur/workspace choisi, ceci est une préparation de départ.
	// Le formulaire vérifie la proposition complète via prepareLaunch en aperçu.
	var occupied int
	if e := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')", w.ID, t.ID).Scan(&occupied); e != nil || occupied > 0 {
		return false
	}
	b, e := s.budget(w.ID)
	if e != nil {
		return false
	}
	return b.Budget.Limit == 0 || b.Remaining >= b.Budget.Reserve
}
