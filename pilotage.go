//go:build linux

package main

import (
	"errors"
	"strings"
	"time"
)

// Projection de lecture : mêmes preuves et prédicats que les commandes.
// Aucun verdict de fraîcheur n'est conservé entre deux snapshots.
func (s *Store) pilotage(w *Work, agents []Agent, validation WorkValidation) map[string]any {
	at := now()
	edges := []map[string]any{}
	tasks := map[string]any{}
	memo := map[string]bool{}
	for i := range w.Tasks {
		t := &w.Tasks[i]
		waiting := []string{}
		for _, id := range t.Depends {
			d, _ := w.task(id)
			fresh := s.acceptedFreshMemo(w, d, map[string]bool{}, memo)
			code, label := "dependency_waiting", "Dépendance non validée ou périmée"
			if fresh {
				code, label = "dependency_satisfied", "Dépendance validée actuellement"
			} else {
				waiting = append(waiting, id)
			}
			edges = append(edges, map[string]any{"from_task_id": id, "to_task_id": t.ID, "satisfied_now": fresh, "reason_code": code, "reason_label": label, "evaluated_revision": w.Revision, "evaluated_at": at})
		}
		tasks[t.ID] = map[string]any{"ready": s.assistCanStart(w, t), "waiting_on": waiting, "delivery": validation.Tasks[t.ID]}
	}
	health := map[string]any{}
	confirmed, unknown, occupied := 0, 0, 0
	for _, a := range agents {
		desired, _ := s.desired(a.ID)
		health[a.ID] = pilotAgentHealth(a, desired, at)
		if activeAgent(a) {
			occupied++
			status := observedAgent(a)
			if status == "running" || status == "stopping" {
				confirmed++
			} else {
				unknown++
			}
		}
	}
	var total int
	complete := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&total) == nil
	return map[string]any{"schema_version": 1, "project_view_key": hash([]byte(s.root)), "snapshot_as_of": at,
		"edges": edges, "tasks": tasks, "health": health,
		"summary": map[string]any{"confirmed": confirmed, "unknown": unknown, "occupied": occupied, "total": total, "loaded": len(agents), "complete": complete}}
}

func pilotAgentHealth(a Agent, desired, at string) map[string]any {
	state := observedAgent(a)
	message := map[string]string{
		"queued/unconfirmed": "Démarrage non confirmé", "starting/unconfirmed": "Démarrage non confirmé",
		"unknown/other-host":      "État non vérifiable depuis cette session",
		"unknown/supervisor-lost": "Superviseur perdu — fin non confirmée",
		"unknown/no-heartbeat":    "Signal absent — fin non confirmée",
		"running":                 "Agent démarré", "stopping": "Arrêt demandé — confirmation attendue",
		"completed": "Exécution terminée — résultat à examiner", "failed": "Exécution en échec",
		"interrupted": "Arrêt confirmé", "queued": "Démarrage en attente", "starting": "Démarrage en cours",
	}[state]
	if message == "" {
		message = "État non confirmé"
	}
	if desired == "stop" && activeAgent(a) {
		message = "Arrêt demandé — confirmation attendue"
	}
	activity, label := "unknown", "Aucun résultat public reçu"
	limits, _ := a.Limits.normalized()
	if last, e := time.Parse(time.RFC3339Nano, a.Progress.LastResult); e == nil {
		observed, _ := time.Parse(time.RFC3339Nano, at)
		activity, label = "recent", "Résultat public récent"
		if observed.Sub(last) > time.Duration(limits.SilenceSeconds)*time.Second {
			activity, label = "old", "Dernier résultat ancien — progression non démontrée"
		}
	}
	// Silence describes a still-open execution, never a finished attempt.
	if state == "completed" || state == "failed" || state == "interrupted" {
		activity, label = "unknown", "Tentative terminée — aucun résultat public enregistré"
		if _, err := time.Parse(time.RFC3339Nano, a.Progress.LastResult); err == nil {
			activity, label = "recorded", "Résultat public reçu — tentative terminée"
		}
	}
	if a.Progress.Degraded != "" {
		activity, label = "unknown", "Flux d'activité incomplet : "+a.Progress.Degraded
	}
	if a.Mode == "terminal" {
		activity, label = "unknown", "Terminal interactif — progression et appels d’outils non mesurés"
	}
	return map[string]any{"can_cancel_pending": cancellableQueuedAgent(a), "same_host": a.Host == hostIdentity(), "agent_id": a.ID, "task_attempt_id": a.Attempt, "process_state": state, "process_label": message,
		"activity_state": activity, "activity_label": label, "last_result_at": a.Progress.LastResult,
		"threshold_seconds": limits.SilenceSeconds, "observed_at": at, "heartbeat": a.Heartbeat, "stop_requested": desired == "stop"}
}

// Toujours inclure les intentions actives, même au-delà de l'historique récent.
// Garder aussi la dernière tentative par tâche pour permettre sa relance même
// lorsque les 200 sessions récentes appartiennent toutes à d'autres tâches.
func (s *Store) pilotAgents(work string) ([]Agent, error) {
	rows, e := s.db.Query("SELECT body,status,desired FROM agents WHERE work_id=? AND (status IN ('queued','starting','running','stopping') OR id IN (SELECT id FROM agents WHERE work_id=? ORDER BY rowid DESC LIMIT 200) OR rowid IN (SELECT max(rowid) FROM agents WHERE work_id=? AND task_id!='' GROUP BY task_id)) ORDER BY rowid DESC", work, work, work)
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	out := []Agent{}
	for rows.Next() {
		var raw []byte
		var status, desired string
		if e = rows.Scan(&raw, &status, &desired); e != nil {
			return nil, e
		}
		a, err := decodeAgentRow(raw, status, desired)
		if err != nil {
			return nil, err
		}
		out = append(out, a)
	}
	return out, rows.Err()
}

func (s *Store) launchEligibility(r webRequest) map[string]any {
	result := map[string]any{"eligible": false, "reason_code": "configuration_required", "reason_label": "Choisir un fournisseur et un espace de travail", "evaluated_revision": r.Revision, "evaluated_at": now()}
	if strings.TrimSpace(r.Provider) == "" || strings.TrimSpace(r.Workspace) == "" {
		return result
	}
	_, _, e := s.prepareLaunch(r.Work, Launch{Mode: r.Mode, Schema: 1, EventID: newID("preview-"), Revision: r.Revision, TaskID: r.Task, Provider: r.Provider, Workspace: r.Workspace, Role: r.Role, Level: r.Level, ModelPolicyHash: r.ModelPolicyHash, Instruction: r.Instruction}, true)
	if e != nil {
		result["reason_code"] = "launch_refused"
		result["reason_label"] = e.Error()
		var busy *WorkspaceBusyError
		if errors.As(e, &busy) {
			busy.Title = busy.TaskID
			if w, err := s.get(busy.WorkID); err == nil {
				if t, _ := w.task(busy.TaskID); t != nil {
					busy.Title = t.Title
				}
			}
			result["reason_code"] = "workspace_busy"
			result["reason_label"] = "En attente de l’espace de travail — « " + busy.Title + " » utilise actuellement ce répertoire."
			result["blocker"] = busy
			result["next_label"] = "Après sa fin, vérifiez à nouveau les conditions puis lancez cette tâche."
			if policy, err := s.missionPolicy(r.Work); err == nil && policy.Enabled && s.autonomy(r.Work) == autonomyAuto {
				if s.paused(r.Work) {
					result["next_label"] = "La mission est en pause. Reprenez-la pour autoriser les départs après libération de l’espace."
				} else {
					result["next_label"] = "La mission continue réexaminera cette tâche après libération de l’espace. Elle démarrera si ses autres conditions sont réunies."
				}
			}
		}
	} else {
		result["eligible"] = true
		result["reason_code"] = "eligible_now"
		result["reason_label"] = "Conditions réunies à cet instant ; vérifiées à nouveau au lancement"
	}
	return result
}
