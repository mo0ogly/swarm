package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"time"
)

// Thresholds observe the run; only an attributed owner decision may stop it.
// Frozen observations and decisions share the existing work/event transaction.
type RunObservation struct {
	Diagnostic       *AttemptDiagnostic `json:"diagnostic,omitempty"`
	Agent            string             `json:"agent_id"`
	Trigger          string             `json:"trigger"`
	ThresholdSeconds int                `json:"threshold_seconds"`
	Progress         AgentProgress      `json:"progress"`
}

func observeRunTime(a *Agent, started, at time.Time) {
	a.Progress.ElapsedSeconds = int(at.Sub(started).Seconds())
	a.Progress.TimeThresholdExceeded = a.Timeout > 0 && a.Progress.ElapsedSeconds >= a.Timeout
}

// No hidden timer retry: each threshold is emitted once per attempt. A revision
// race is reconsidered on the next ordinary tick, with fresh state and counters.
func (s *Store) observeRun(a Agent) (string, error) {
	w, err := s.get(a.WorkID)
	if err != nil {
		return "", err
	}
	if w.Planning == nil {
		return "", nil
	} // Standalone telemetry remains in agent.progress.
	task, err := w.task(a.TaskID)
	if err != nil || task.ScopeID == "" || !currentTaskAttempt(task, a.Attempt) {
		return "", nil
	}
	if decision := runStopDecision(w, a); decision != "" {
		return decision, nil
	}

	observations := []RunObservation{}
	if a.Progress.TimeThresholdExceeded {
		observations = append(observations, RunObservation{Agent: a.ID, Trigger: "elapsed_time", ThresholdSeconds: a.Timeout, Progress: a.Progress})
	}
	if a.Progress.ToolTimeThresholdExceeded {
		observations = append(observations, RunObservation{Agent: a.ID, Trigger: "tool_wait", ThresholdSeconds: a.Limits.ToolSeconds, Progress: a.Progress})
	}
	if a.Progress.OutputState == "silent" {
		observations = append(observations, RunObservation{Agent: a.ID, Trigger: "output_silence", ThresholdSeconds: a.Limits.SilenceSeconds, Progress: a.Progress})
	}
	for _, item := range a.Diagnostic.Items {
		if item.Category == "environment" {
			diagnostic := a.Diagnostic
			observations = append(observations, RunObservation{Agent: a.ID, Trigger: "infrastructure", Progress: a.Progress, Diagnostic: &diagnostic})
			break
		}
	}
	ordinaryErrors := 0
	for _, item := range a.Diagnostic.Items {
		if item.Category != "environment" && item.Category != "limit" {
			ordinaryErrors += item.Count
		}
	}
	if ordinaryErrors > 0 {
		diagnostic := a.Diagnostic
		observations = append(observations, RunObservation{Agent: a.ID, Trigger: "tool_failure", Progress: a.Progress, Diagnostic: &diagnostic})
	}
	for _, observation := range observations {
		id := planningEventID("run-observation", a.ID, a.Attempt, observation.Trigger)
		if observation.Trigger == "infrastructure" {
			id = planningEventID("run-observation", a.ID, a.Attempt, observation.Trigger, fmt.Sprint(infrastructureErrorCount(*observation.Diagnostic)))
		}
		if observation.Trigger == "tool_failure" {
			id = planningEventID("run-observation", a.ID, a.Attempt, observation.Trigger, fmt.Sprint(ordinaryErrors))
		}
		exists := false
		for _, event := range w.Planning.Inbox {
			if event.ID == id {
				exists = true
				break
			}
		}
		if exists {
			continue
		}
		message := "Seuil de durée observé ; processus actif, décision du responsable requise. Aucune preuve de fin ou de non-progrès."
		if observation.Trigger == "infrastructure" {
			message = "Incident d’environnement transmis au superviseur ; résultat conservé, décision de conduite requise. Les détails techniques sont disponibles dans la console."
		}
		if observation.Trigger == "tool_failure" {
			message = "Échec d’outil transmis au responsable de la tentative ; diagnostiquer la cause et vérifier toute correction avant reprise. Ce constat ne valide aucun critère."
		}
		raw, _ := json.Marshal(observation)
		w, err = s.mutate(a.WorkID, "planning.run-observation", id, w.Revision, raw, func(current *Work) error {
			if current.Planning == nil {
				return fmt.Errorf("plan de supervision absent")
			}
			t, e := current.task(a.TaskID)
			if e != nil || t.ScopeID != task.ScopeID || t.Status != "running" || !currentTaskAttempt(t, a.Attempt) {
				return fmt.Errorf("tentative de supervision périmée")
			}
			scope, e := current.Planning.scope(t.ScopeID)
			if e != nil || scope.State == "closed" {
				return fmt.Errorf("responsable de supervision indisponible")
			}
			current.Planning.Inbox = append(current.Planning.Inbox, PlanningEvent{ID: id, Scope: t.ScopeID, Kind: "run_observation", Task: t.ID, Attempt: a.Attempt, Message: message, RunObservation: &observation, At: now()})
			// Do not revoke a live activation: the following event awaits its next batch.
			if scope.Holder == "" {
				scope.State = "ready"
			}
			return nil
		})
		if err != nil {
			if commandFailure(err).Code == "revision_conflict" {
				return "", nil
			}
			return "", err
		}
	}
	return "", nil
}

func runStopDecision(w Work, a Agent) string {
	if w.Planning == nil {
		return ""
	}
	task, err := w.task(a.TaskID)
	if err != nil || !currentTaskAttempt(task, a.Attempt) {
		return ""
	}
	for _, event := range w.Planning.Inbox {
		if event.Scope == task.ScopeID && event.Task == task.ID && event.Attempt == a.Attempt && event.RunObservation != nil && event.RunObservation.Agent == a.ID && event.Decision != "" && event.RunDecision == "stop-run" {
			return "Arrêt décidé par le superviseur : " + event.RunReason
		}
	}
	return ""
}

func (s *Store) runStopRequest(a Agent) (string, error) {
	w, err := s.get(a.WorkID)
	if err != nil {
		return "", err
	}
	return runStopDecision(w, a), nil
}

func applyRunDecision(w *Work, scope string, op PlanningOperation, r PlanningRequest) error {
	if !nonempty(op.Next) || len(op.Next) > 2000 || op.Title != "" || op.Deliverable != "" || len(op.Criteria)+len(op.Depends)+len(op.Requirements) != 0 || op.MaxTasks != 0 || op.MaxActivations != 0 {
		return fmt.Errorf("conduite : id et next requis, autres champs vides ou zéro")
	}
	task, err := w.task(op.ID)
	if err != nil || task.ScopeID != scope || task.Status != "running" {
		return fmt.Errorf("conduite hors périmètre ou tentative inactive")
	}
	found := false
	for i := range w.Planning.Inbox {
		event := &w.Planning.Inbox[i]
		if !containsString(r.Inputs, event.ID) || event.Kind != "run_observation" || event.Task != task.ID || event.Scope != scope {
			continue
		}
		if event.RunObservation == nil || event.Decision != "" || event.RunDecision != "" || !currentTaskAttempt(task, event.Attempt) {
			return fmt.Errorf("observation périmée ou déjà décidée")
		}
		event.RunDecision, event.RunReason = op.Kind, op.Next
		found = true
	}
	if !found {
		return fmt.Errorf("observation courante effectivement fournie requise")
	}
	return nil
}

// Fence the actual live process inside the same decision transaction, including
// the stop/result race; a task projection alone cannot authorize process control.
func validateRunDecisionTx(tx *sql.Tx, w *Work, op PlanningOperation, r PlanningRequest) error {
	for _, event := range w.Planning.Inbox {
		if !containsString(r.Inputs, event.ID) || event.Task != op.ID || event.RunDecision != op.Kind || event.RunObservation == nil {
			continue
		}
		var body []byte
		if err := tx.QueryRow("SELECT body FROM agents WHERE id=? AND work_id=? AND task_id=? AND status='running' AND desired!='stop'", event.RunObservation.Agent, w.ID, op.ID).Scan(&body); err != nil {
			return fmt.Errorf("processus de supervision indisponible : %w", err)
		}
		var a Agent
		if err := json.Unmarshal(body, &a); err != nil {
			return err
		}
		if a.ID != event.RunObservation.Agent || a.WorkID != w.ID || a.TaskID != op.ID || a.Attempt != event.Attempt || a.Role != "worker" {
			return fmt.Errorf("processus de supervision non attribuable")
		}
		return nil
	}
	return fmt.Errorf("décision de conduite non attribuable")
}

func infrastructureErrorCount(d AttemptDiagnostic) int {
	count := 0
	for _, item := range d.Items {
		if item.Category == "environment" {
			count += item.Count
		}
	}
	return count
}
