//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"
	"time"
)

// Ordonnanceur des départs. Il choisit quelles tâches prêtes partent dans les
// créneaux libres ; il ne décide jamais qu'une tâche est faite.
//
// Bornes, dans cet ordre : niveau d'autonomie, suspension des départs, créneaux,
// dépendances acceptées et fraîches, profil de lancement disponible, plafond de
// tentatives automatiques, espaces de travail disjoints. Le budget est réservé
// par le moteur au moment du départ, dans la même transaction que la tentative.
const maxAutomaticAttempts = 2

// Multiple de la réserve par départ au-delà duquel une tâche cesse de partir
// seule. Cette borne empêche une dépense à venir ; elle n'annule rien de ce qui
// est déjà engagé et n'accepte ni ne refuse aucune tâche. Valeur de départ à
// vérifier à l'usage.
const costThresholdFactor = 2.0

type dispatchInputs struct {
	work               *Work
	agents             []Agent
	profile            *LaunchProfile
	depsReady          map[string]bool
	priority           map[string]int
	taskCost           map[string]CostTotal
	reserve            float64
	autonomy           string
	slots              int
	paused             bool
	occupiedWorkspaces []string
	launchBlocked      map[string]string
	at                 time.Time
}

type dispatchDecision struct {
	TaskID             string
	Profile            LaunchProfile
	Previous           string
	RecoveryCategory   string
	CauseFingerprint   string
	OperationID        string
	NextEligibleAt     string
	CorrectionFindings string
}

func automaticCorrection(t Task, a Agent) (string, string, bool) {
	validation := t.AutoValidation
	if a.Status != "completed" || t.Status != "blocked" || validation == nil ||
		validation.Attempt != a.Attempt || validation.State != "blocked" ||
		t.PlanMaxAttempts <= 0 || len(t.Attempts) >= t.PlanMaxAttempts {
		return "", "", false
	}
	findings := []string{"Reçu de contrôle : " + validation.Receipt + "."}
	fingerprintParts := []string{validation.PolicyDigest, validation.Receipt}
	for _, control := range validation.Controls {
		if control.Passed {
			continue
		}
		finding := fmt.Sprintf("Contrôle %s en échec (code %d, sortie sha256 %s) : %s.", control.ID, control.ExitCode, control.OutputHash, control.Summary)
		findings = append(findings, finding)
		fingerprintParts = append(fingerprintParts, control.ID, fmt.Sprint(control.ExitCode), control.OutputHash, control.Summary)
	}
	if len(findings) == 1 {
		return "", "", false
	}
	return strings.Join(findings, " "), hash([]byte(strings.Join(fingerprintParts, "|"))), true
}

// planDispatch rend les départs à effectuer et, quand il n'en rend aucun, le
// motif à journaliser. Aucune décision n'est prise sur des données absentes :
// une information manquante empêche le départ, elle ne le suppose pas.
func planDispatch(in dispatchInputs) ([]dispatchDecision, string) {
	if in.autonomy != autonomyAuto {
		return nil, "Aucun départ automatique : niveau d'autonomie « " + autonomyLabel(in.autonomy) + " »"
	}
	if in.paused {
		return nil, "Aucun départ automatique : départs suspendus par l'opérateur"
	}
	busy, workspaces := 0, append([]string{}, in.occupiedWorkspaces...)
	for _, a := range in.agents {
		if activeAgent(a) {
			busy++
			workspaces = append(workspaces, a.CWD)
		}
	}
	free := in.slots - busy
	if free <= 0 {
		return nil, fmt.Sprintf("Aucun départ automatique : %d créneau(x) occupé(s) sur %d", busy, in.slots)
	}

	decisionAt := in.at
	if decisionAt.IsZero() {
		decisionAt = time.Now()
	}
	refused := map[string]bool{}
	held := map[string]bool{}
	latest := map[string]Agent{}
	for _, a := range in.agents {
		if _, seen := latest[a.TaskID]; !seen {
			latest[a.TaskID] = a
		}
		if a.Status == "completed" {
			refused[a.TaskID] = true
		}
		// Un arrêt demandé par l'opérateur est une décision, pas un incident :
		// l'ordonnanceur ne la contredit pas en relançant aussitôt.
		if a.Status == "interrupted" && a.StopKind == originOperator {
			held[a.TaskID] = true
		}
	}

	reasons := []string{}
	candidates := []Task{}
	for _, t := range in.work.Tasks {
		last := latest[t.ID]
		_, _, correction := automaticCorrection(t, last)
		switch {
		case t.Status != "todo" && t.Status != "blocked":
			continue
		case t.Status == "blocked" && refused[t.ID] && !correction && !t.PlanningRetry:
			// Tentative terminée dont le handoff n'a pas pu être relayé :
			// relancer ne produirait pas la preuve manquante.
			continue
		case t.LaunchHeld:
			reasons = append(reasons, t.ID+" : missions créées, démarrage à autoriser depuis la préparation")
			continue
		case held[t.ID]:
			reasons = append(reasons, t.ID+" : arrêt demandé par l'opérateur, reprise à la main")
			continue
		case in.reserve > 0 && in.taskCost[t.ID].Reported > costThresholdFactor*in.reserve:
			reasons = append(reasons, fmt.Sprintf(
				"%s : %.2f USD rapportés pour une réserve de %.2f par départ ; prochain départ suspendu, la dépense engagée n'est pas annulée",
				t.ID, in.taskCost[t.ID].Reported, in.reserve))
			continue
		case !in.depsReady[t.ID]:
			continue
		case in.launchBlocked[t.ID] != "":
			reasons = append(reasons, t.ID+" : "+in.launchBlocked[t.ID])
			continue
		}
		if last, ok := latest[t.ID]; ok && (last.Status == "failed" || last.Status == "interrupted") {
			recovery := assessRecovery(last, t, decisionAt)
			if recovery.Disposition != recoveryDispositionRetry && recovery.Disposition != recoveryDispositionCorrection && !(t.PlanningRetry && !requiresEnvironmentVerification(last)) {
				reasons = append(reasons, t.ID+" : "+recovery.Reason)
				continue
			}
		}
		if profileFor(in, t) == nil {
			reasons = append(reasons, t.ID+" : aucun profil de lancement enregistré")
			continue
		}
		candidates = append(candidates, t)
	}

	depth := graphDepth(in.work.Tasks)
	sort.SliceStable(candidates, func(i, j int) bool {
		a, b := candidates[i], candidates[j]
		if in.priority[a.ID] != in.priority[b.ID] {
			return in.priority[a.ID] > in.priority[b.ID]
		}
		if depth[a.ID] != depth[b.ID] {
			return depth[a.ID] < depth[b.ID]
		}
		return a.ID < b.ID
	})

	out := []dispatchDecision{}
	for _, t := range candidates {
		if len(out) >= free {
			break
		}
		p := *profileFor(in, t)
		occupied := false
		for _, workspace := range workspaces {
			if workspaceOverlap(workspace, p.Workspace) {
				occupied = true
				break
			}
		}
		if occupied {
			reasons = append(reasons, t.ID+" : espace de travail déjà occupé par une tentative active")
			continue
		}
		workspaces = append(workspaces, p.Workspace)
		decision := dispatchDecision{TaskID: t.ID, Profile: p}
		if last, ok := latest[t.ID]; ok && (last.Status == "failed" || last.Status == "interrupted") {
			recovery := assessRecovery(last, t, decisionAt)
			decision.Previous = last.ID
			decision.RecoveryCategory = recovery.Category
			decision.CauseFingerprint = recovery.CauseFingerprint
			decision.OperationID = recovery.OperationID
			if !recovery.NextEligibleAt.IsZero() {
				decision.NextEligibleAt = recovery.NextEligibleAt.UTC().Format(time.RFC3339Nano)
			}
		} else if last, ok := latest[t.ID]; ok {
			if findings, cause, correction := automaticCorrection(t, last); correction {
				decision.Previous = last.ID
				decision.RecoveryCategory = recoveryBusiness
				decision.CauseFingerprint = cause
				decision.OperationID = last.Recovery.OperationID
				if decision.OperationID == "" {
					decision.OperationID = last.ID
				}
				decision.CorrectionFindings = findings
			}
		}
		out = append(out, decision)
	}
	if len(out) == 0 && len(reasons) == 0 {
		reasons = append(reasons, "aucune tâche candidate")
	}
	return out, strings.Join(reasons, " ; ")
}

func workspaceOverlap(a, b string) bool {
	a, b = strings.TrimSuffix(a, "/"), strings.TrimSuffix(b, "/")
	return a == b || strings.HasPrefix(a, b+"/") || strings.HasPrefix(b, a+"/")
}

func (s *Store) activeWorkspaces() ([]string, error) {
	rows, err := s.db.Query("SELECT cwd FROM agents WHERE status IN ('queued','starting','running','stopping') ORDER BY rowid")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	workspaces := []string{}
	for rows.Next() {
		var workspace string
		if err := rows.Scan(&workspace); err != nil {
			return nil, err
		}
		workspaces = append(workspaces, workspace)
	}
	return workspaces, rows.Err()
}

// Le profil de la tâche prime ; celui du travail sert de repli.
func profileFor(in dispatchInputs, t Task) *LaunchProfile {
	p := in.profile
	if t.Profile != nil {
		p = t.Profile
	}
	if p == nil {
		return nil
	}
	if in.work.Planning != nil && in.work.Planning.Repository != nil {
		copy := *p
		copy.Workspace = managedCopyPath(in.work.Planning.Repository, t.ID, len(t.Attempts)+1)
		return &copy
	}
	return p
}

// Profondeur dans le graphe : une racine part avant ce qui en dépend.
func graphDepth(tasks []Task) map[string]int {
	byID := map[string]Task{}
	for _, t := range tasks {
		byID[t.ID] = t
	}
	depth := map[string]int{}
	var visit func(string, map[string]bool) int
	visit = func(id string, seen map[string]bool) int {
		if d, ok := depth[id]; ok {
			return d
		}
		if seen[id] {
			return 0
		}
		seen[id] = true
		d := 0
		for _, parent := range byID[id].Depends {
			if x := visit(parent, seen) + 1; x > d {
				d = x
			}
		}
		depth[id] = d
		return d
	}
	for _, t := range tasks {
		visit(t.ID, map[string]bool{})
	}
	return depth
}

// dispatch applique le plan : chaque départ est enregistré puis lancé. Un refus
// du moteur (budget, workspace, révision) interrompt la série sans annuler les
// départs déjà effectués ; il est journalisé et laissé à l'escalade.
func (s *Store) dispatch(work string, conductors ...string) ([]dispatchDecision, error) {
	if archived, err := s.archived(work); err != nil {
		return nil, err
	} else if archived {
		return nil, &CommandError{Code: "mission_archived", Message: "mission archivée ; restaurer avant de lancer le conducteur"}
	}
	w, e := s.get(work)
	if e != nil {
		return nil, e
	}
	agents, e := s.agents(work)
	if e != nil {
		return nil, e
	}
	occupiedWorkspaces, e := s.activeWorkspaces()
	if e != nil {
		return nil, e
	}
	in := dispatchInputs{work: &w, agents: agents, profile: w.Profile,
		autonomy: s.autonomy(work), slots: s.slots(work), paused: s.paused(work),
		depsReady: map[string]bool{}, priority: s.priorities(work), occupiedWorkspaces: occupiedWorkspaces,
		launchBlocked: map[string]string{}}
	// Le seuil s'appuie sur ce que les fournisseurs ont réellement rapporté ;
	// sans réserve configurée, aucun plafond n'est déduit. Une lecture qui
	// échoue n'est pas une absence de dépassement : elle désactiverait le
	// plafond en silence, donc elle arrête l'ordonnancement comme les autres
	// lectures de cette fonction.
	summary, e := s.costSummary(work)
	if e != nil {
		return nil, e
	}
	in.taskCost = summary.ByTask
	v, e := s.budget(work)
	if e != nil {
		return nil, e
	}
	in.reserve = v.Budget.Reserve
	for i := range w.Tasks {
		in.depsReady[w.Tasks[i].ID] = s.dependenciesReady(&w, &w.Tasks[i])
	}
	// Register the mission's next runnable contenders before applying global
	// occupancy. Their AUTOINCREMENT order is the durable, cross-mission FIFO;
	// disjoint path trees never block one another.
	probe := in
	probe.occupiedWorkspaces = nil
	probe.launchBlocked = map[string]string{}
	probe.slots = len(w.Tasks) + len(agents) + 1
	contenders, _ := planDispatch(probe)
	retainedTurns := map[string]bool{}
	for _, contender := range contenders {
		retainedTurns[contender.TaskID] = true
		if e = s.enqueueWorkspaceTurn(work, contender.TaskID, contender.Profile.Workspace); e != nil {
			return nil, e
		}
	}
	if e = s.releaseObsoleteWorkspaceTurns(work, retainedTurns); e != nil {
		return nil, e
	}
	turns, e := s.workspaceTurns()
	if e != nil {
		return nil, e
	}
	in.launchBlocked = workspaceTurnBlockers(turns, work)
	plan, reason := planDispatch(in)
	if len(plan) == 0 {
		if nonempty(reason) {
			_ = s.dispatchEvent(work, reason)
		}
		return nil, nil
	}
	done := []dispatchDecision{}
	for _, d := range plan {
		current, e := s.get(work)
		if e != nil {
			return done, e
		}
		p := d.Profile
		if task, err := current.task(d.TaskID); err == nil && task.Profile == nil {
			p.Instruction = "Mission : " + task.Title + "\nLivrable : " + task.Deliverable + "\nProchaine action : " + task.Next + "\nConsignes communes : " + p.Instruction
		}
		conductor := ""
		if len(conductors) > 0 {
			conductor = conductors[0]
		}
		r := Launch{Schema: 1, EventID: automaticEventID(work, d.TaskID, len(agents)), Revision: current.Revision, ConductorID: conductor,
			TaskID: d.TaskID, Provider: p.Provider, Role: p.Role, Workspace: p.Workspace,
			Instruction: p.Instruction, Level: p.Level, Timeout: p.Timeout, Capture: p.Capture, Limits: p.Limits, Origin: originConductor,
			Previous: d.Previous, recoveryCategory: d.RecoveryCategory, recoveryCause: d.CauseFingerprint,
			recoveryOperation: d.OperationID, recoveryNext: d.NextEligibleAt}
		if d.Previous != "" {
			r.Instruction += "\n" + recoveryInstruction(d.RecoveryCategory, d.OperationID, d.CauseFingerprint)
			if d.CorrectionFindings != "" {
				r.Instruction += " Constats objectifs à corriger : " + d.CorrectionFindings
			}
		}
		a, created, e := s.prepare(work, r)
		if e != nil {
			_ = s.dispatchEvent(work, d.TaskID+" : départ automatique refusé — "+e.Error())
			return done, nil
		}
		if !created {
			continue
		}
		if e = s.spawnAgent(a); e != nil {
			_ = s.dispatchEvent(work, d.TaskID+" : superviseur non démarré — "+e.Error())
			return done, nil
		}
		_ = s.dispatchEvent(work, fmt.Sprintf("%s : départ automatique · %s · rôle %s · espace %s", d.TaskID, p.Provider, p.Role, p.Workspace))
		_ = s.log(a.ID, "lifecycle", fmt.Sprintf("Départ automatique par le %s ; profil : %s · rôle %s · espace %s", conductorAuthor, p.Provider, p.Role, p.Workspace))
		done = append(done, d)
	}
	return done, nil
}

// Identifiant déterministe : deux ordonnancements concurrents ne lancent pas
// deux fois la même tentative.
func automaticEventID(work, task string, generation int) string {
	return "auto-" + hash([]byte(fmt.Sprintf("%s|%s|%d", work, task, generation)))[:20]
}

func (s *Store) dependenciesReady(w *Work, t *Task) bool {
	for _, id := range t.Depends {
		dep, e := w.task(id)
		if e != nil || !s.acceptedFresh(w, dep, map[string]bool{}) {
			return false
		}
	}
	return true
}

func dispatchedIDs(list []dispatchDecision) []string {
	out := []string{}
	for _, d := range list {
		out = append(out, d.TaskID)
	}
	return out
}

// dispatchAfterSettle persists the wake-up, then tries a short-lived ownership
// claim for backward-compatible immediate chaining. If a durable conductor is
// already active, it alone observes the occupancy change on its next bounded
// poll. Thus the supervisor never bypasses lease ownership.
func (s *Store) dispatchAfterSettle(work string) {
	_ = s.controlEvent(work, "resource-released", "Une tentative a libéré ses ressources ; réévaluation automatique par le conducteur")
	conductor := newID("release-conductor-")
	owned, err := s.claimMissionSupervision(work, conductor, "libération de ressource", time.Now())
	if err != nil || !owned {
		return
	}
	defer s.stopMissionSupervision(work, conductor, time.Now())
	_ = s.recordCoordinationEvent("lease:"+work+":"+conductor, work, "", conductor,
		"lease-acquired", "Possession éphémère acquise après libération d’une ressource")
	_, _ = s.dispatch(work, conductor)
}

// setProfile enregistre un profil de lancement sans lancer : un plan complet
// peut ainsi être confié à l'ordonnanceur sans première tentative manuelle.
// Tâche vide : profil du travail, repli de toutes ses tâches.
func (s *Store) setProfile(work, task string, p LaunchProfile, expected int) error {
	if !nonempty(p.Provider) {
		return fmt.Errorf("fournisseur requis dans le profil de lancement")
	}
	providers, e := s.providers()
	if e != nil {
		return e
	}
	if _, known := providers.Providers[p.Provider]; !known {
		return fmt.Errorf("fournisseur inconnu : %s", p.Provider)
	}
	if p.Role == "" {
		p.Role = "worker"
	}
	if p.Role != "worker" && p.Role != "planner" && p.Role != "subplanner" {
		return fmt.Errorf("rôle inconnu : %s (worker, planner ou subplanner)", p.Role)
	}
	cwd, e := resolveWorkspace(s.root, p.Workspace)
	if e != nil {
		return e
	}
	p.Workspace = cwd
	if p.Limits != nil {
		limits, e := p.Limits.normalized()
		if e != nil {
			return e
		}
		p.Limits = &limits
	}
	p.Updated = now()
	p.Actor = originOperator
	w, e := s.get(work)
	if e != nil {
		return e
	}
	if expected < 0 {
		expected = w.Revision
	}
	r := Request{Schema: 1, EventID: newID("profil-"), Revision: expected, ID: task}
	raw, _ := json.Marshal(r)
	_, e = s.mutate(work, "task.update", r.EventID, expected, raw, func(w *Work) error {
		if !nonempty(task) {
			w.Profile = &p
			return nil
		}
		for i := range w.Tasks {
			if w.Tasks[i].ID == task {
				w.Tasks[i].Profile = &p
				return nil
			}
		}
		return fmt.Errorf("tâche inconnue : %s", task)
	})
	return e
}

// Keep a stable waiting reason visible without emitting the same event each tick.
func (s *Store) dispatchEvent(work, message string) error {
	var prior string
	if e := s.db.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='dispatch' ORDER BY rowid DESC LIMIT 1", work).Scan(&prior); e == nil && prior == message {
		return nil
	}
	return s.controlEvent(work, "dispatch", message)
}
