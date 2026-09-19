package main

import (
	"fmt"
	"strings"
)

type MissionTask struct {
	ID          string `json:"id"`
	Title       string `json:"title"`
	State       string `json:"state"`
	Reason      string `json:"reason"`
	Action      string `json:"action"`
	Label       string `json:"label"`
	Target      string `json:"target,omitempty"`
	Impact      int    `json:"impact"`
	Deliverable string `json:"deliverable"`
}
type MissionStatus struct {
	Enabled   bool          `json:"enabled"`
	Paused    bool          `json:"paused"`
	Summary   string        `json:"summary"`
	Next      string        `json:"next"`
	Validated int           `json:"validated"`
	Review    int           `json:"review"`
	Running   int           `json:"running"`
	Total     int           `json:"total"`
	Tasks     []MissionTask `json:"tasks"`
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
	d.Enabled = p.Enabled && s.autonomy(work) == autonomyAuto
	d.Paused = s.paused(work)
	d.Total = len(w.Tasks)
	agents, e := s.agents(work)
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
		depsReady: map[string]bool{}, priority: s.priorities(work)}
	for i := range w.Tasks {
		in.depsReady[w.Tasks[i].ID] = s.dependenciesReady(&w, &w.Tasks[i])
	}
	validation := s.validationState(&w)
	for i := range w.Tasks {
		t := &w.Tasks[i]
		v := validation.Tasks[t.ID]
		x := MissionTask{ID: t.ID, Title: t.Title, State: t.Status, Deliverable: t.Deliverable, Target: t.ID, Impact: descendantCount(&w, t.ID)}
		switch {
		case t.Status == "accepted" && v.Fresh:
			d.Validated++
			x.State = "validated"
			x.Reason = "Résultat validé sur preuves actuelles"
			x.Action = "report"
			x.Label = "Voir le résultat"
		case (t.Status == "accepted" || t.Status == "waived") && !v.Fresh:
			x.State = "intervention"
			x.Reason = "Une validation antérieure n’est plus actuelle"
			if len(v.Blockers) > 0 {
				x.Reason = v.Blockers[0]
			}
			x.Action = "reopen"
			x.Label = "Revalider les preuves"
		case t.Status == "waived":
			x.State, x.Reason, x.Action, x.Label = "waived", "Résultat dérogé ; les contrôles ne sont pas validés", "inspect", "Voir la dérogation"
		case t.Status == "abandoned":
			x.State, x.Reason, x.Action, x.Label = "abandoned", "Tâche abandonnée ; aucun résultat validé", "inspect", "Voir la décision"
		case t.Status == "submitted":
			d.Review++
			x.State = "review"
			x.Reason = "Résultat produit ; décision de validation nécessaire"
			x.Action = "report"
			x.Label = "Examiner le résultat"
			if s.validGate(t) && in.depsReady[t.ID] {
				x.Reason = "Contrôles actuels enregistrés ; acceptation après revue disponible"
				x.Action = "accepted"
				x.Label = "Valider après revue"
			}
		case t.Status == "running":
			d.Running++
			x.State = "running"
			x.Reason = "Tentative en cours ; résultat non encore validé"
			x.Action = "inspect"
			x.Label = "Suivre l’agent"
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
						if !d.Enabled {
							x.State, x.Reason = "manual", "Lancement manuel possible ; mission continue non autorisée"
						}
					}
				}
				if profileFor(in, *t) == nil && x.State == "intervention" {
					x.Action, x.Label = "configure", "Configurer la mission"
				}
				if t.LaunchHeld {
					x.State, x.Action, x.Label = "intervention", "prepare", "Autoriser le plan dans Préparer"
				}

			}
		}
		d.Tasks = append(d.Tasks, x)
	}
	d.Summary = fmt.Sprintf("%d/%d résultats validés · %d à examiner · %d en cours", d.Validated, d.Total, d.Review, d.Running)
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
	if interventions > 0 {
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
	return d, nil
}

// Classify using the actual dispatcher, projected onto this task. More specific
// explanations cover intentional silent skips; no independent ready predicate.
func missionDispatchState(in dispatchInputs, t Task) (string, string) {
	w := *in.work
	w.Tasks = []Task{t}
	in.work = &w
	// Explain durable interventions before temporary slot/pause waiting.
	if t.Status == "blocked" && strings.TrimSpace(t.Blocker) != "" {
		return "intervention", t.Blocker
	}
	failures := 0
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
		if a.Origin == originConductor && (a.Status == "failed" || a.Status == "interrupted") {
			failures++
		}
	}
	if t.LaunchHeld {
		return "intervention", "Démarrage du plan à autoriser depuis la préparation"
	}
	if failures >= maxAutomaticAttempts {
		return "intervention", fmt.Sprintf("%d tentatives automatiques infructueuses : examiner les échecs et réorienter avant reprise", failures)
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
