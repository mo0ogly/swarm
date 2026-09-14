//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"
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
	work      *Work
	agents    []Agent
	profile   *LaunchProfile
	depsReady map[string]bool
	priority  map[string]int
	taskCost  map[string]CostTotal
	reserve   float64
	autonomy  string
	slots     int
	paused    bool
}

type dispatchDecision struct {
	TaskID  string
	Profile LaunchProfile
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
	busy, workspaces := 0, map[string]bool{}
	for _, a := range in.agents {
		if activeAgent(a) {
			busy++
			workspaces[a.CWD] = true
		}
	}
	free := in.slots - busy
	if free <= 0 {
		return nil, fmt.Sprintf("Aucun départ automatique : %d créneau(x) occupé(s) sur %d", busy, in.slots)
	}

	failures := map[string]int{}
	for _, a := range in.agents {
		if a.Origin == originConductor && (a.Status == "failed" || a.Status == "interrupted") {
			failures[a.TaskID]++
		}
	}
	refused := map[string]bool{}
	held := map[string]bool{}
	for _, a := range in.agents {
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
		switch {
		case t.Status != "todo" && t.Status != "blocked":
			continue
		case t.Status == "blocked" && refused[t.ID]:
			// Tentative terminée dont le handoff n'a pas pu être relayé :
			// relancer ne produirait pas la preuve manquante.
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
		case failures[t.ID] >= maxAutomaticAttempts:
			reasons = append(reasons, fmt.Sprintf("%s : %d tentatives automatiques infructueuses, décision humaine attendue", t.ID, failures[t.ID]))
			continue
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
		if workspaces[p.Workspace] {
			reasons = append(reasons, t.ID+" : espace de travail déjà occupé par une tentative active")
			continue
		}
		workspaces[p.Workspace] = true
		out = append(out, dispatchDecision{TaskID: t.ID, Profile: p})
	}
	if len(out) == 0 && len(reasons) == 0 {
		reasons = append(reasons, "aucune tâche candidate")
	}
	return out, strings.Join(reasons, " ; ")
}

// Le profil de la tâche prime ; celui du travail sert de repli.
func profileFor(in dispatchInputs, t Task) *LaunchProfile {
	if t.Profile != nil {
		return t.Profile
	}
	return in.profile
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
func (s *Store) dispatch(work string) ([]dispatchDecision, error) {
	w, e := s.get(work)
	if e != nil {
		return nil, e
	}
	agents, e := s.agents(work)
	if e != nil {
		return nil, e
	}
	in := dispatchInputs{work: &w, agents: agents, profile: w.Profile,
		autonomy: s.autonomy(work), slots: s.slots(work), paused: s.paused(work),
		depsReady: map[string]bool{}, priority: s.priorities(work)}
	// Le seuil s'appuie sur ce que les fournisseurs ont réellement rapporté ;
	// sans réserve configurée, aucun plafond n'est déduit.
	if summary, err := s.costSummary(work); err == nil {
		in.taskCost = summary.ByTask
	}
	if v, err := s.budget(work); err == nil {
		in.reserve = v.Budget.Reserve
	}
	for i := range w.Tasks {
		in.depsReady[w.Tasks[i].ID] = s.dependenciesReady(&w, &w.Tasks[i])
	}
	plan, reason := planDispatch(in)
	if len(plan) == 0 {
		if nonempty(reason) {
			_ = s.controlEvent(work, "dispatch", reason)
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
		r := Launch{Schema: 1, EventID: automaticEventID(work, d.TaskID, len(agents)), Revision: current.Revision,
			TaskID: d.TaskID, Provider: p.Provider, Role: p.Role, Workspace: p.Workspace,
			Instruction: p.Instruction, Level: p.Level, Timeout: p.Timeout, Capture: p.Capture, Limits: p.Limits, Origin: originConductor}
		a, created, e := s.prepare(work, r)
		if e != nil {
			_ = s.controlEvent(work, "dispatch", d.TaskID+" : départ automatique refusé — "+e.Error())
			return done, nil
		}
		if !created {
			continue
		}
		if e = s.spawnAgent(a); e != nil {
			_ = s.controlEvent(work, "dispatch", d.TaskID+" : superviseur non démarré — "+e.Error())
			return done, nil
		}
		_ = s.controlEvent(work, "dispatch", fmt.Sprintf("%s : départ automatique · %s · rôle %s · espace %s", d.TaskID, p.Provider, p.Role, p.Workspace))
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

// dispatchAfterSettle relance l'ordonnanceur après la fin d'une tentative : le
// créneau libéré doit servir sans attendre une action humaine. Un échec
// d'ordonnancement n'échoue jamais la fin de la tentative.
func (s *Store) dispatchAfterSettle(work string) {
	_, _ = s.dispatch(work)
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
