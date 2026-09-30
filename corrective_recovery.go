package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"
)

// A single explicit operator grant, separate from the plan's ordinary retries.
// Kept with the task and in the atomic event journal; never issued by a planner.
type CorrectiveRecovery struct {
	MissingReport bool   `json:"missing_report,omitempty"`
	Result        string `json:"result_commit,omitempty"`
	Event         string `json:"event_id"`
	Revision      int    `json:"authorized_revision"`
	Review        string `json:"review_id"`
	Attempt       string `json:"attempt_id"`
	Actor         string `json:"actor"`
	At            string `json:"at"`
	Reason        string `json:"reason"`
	Instruction   string `json:"instruction"`
}

func correctiveRecoveryReason(w *Work, t *Task, agents []Agent) string {
	return correctiveRecoveryReasonFor(w, t, agents, missingReportRecovery(w, t, agents))
}

func correctiveRecoveryReasonFor(w *Work, t *Task, agents []Agent, delivery bool) string {
	if t.CorrectiveRecovery != nil {
		return "L’essai correctif exceptionnel a déjà été autorisé. Aucun autre essai ne peut être ajouté."
	}
	if t.Status != "blocked" || t.PlanMaxAttempts != 3 || len(t.Attempts) != 3 {
		return "La reprise corrective exige une tâche bloquée après trois tentatives consommées."
	}
	r := t.IndependentReview
	if !delivery && (r == nil || r.ID == "" || r.State != "changes_requested" || r.Attempt != t.Attempts[len(t.Attempts)-1].ID) {
		return "Un refus indépendant de la dernière tentative est nécessaire pour préparer sa correction."
	}
	if w.Planning == nil {
		return "La reprise corrective exige une mission avec responsable et vérificateur indépendant."
	}
	if err := reviewerLaunchGuard(*w); err != nil {
		return err.Error()
	}
	for _, a := range agents {
		if a.TaskID == t.ID && activeAgent(a) {
			return "Attendre la fin de l’agent et la libération de sa tentative."
		}
	}
	return ""
}

func (s *Store) authorizeCorrectiveRecovery(work string, r PlanningRequest) (Work, error) {
	modes := 0
	if r.ReviewID != "" {
		modes++
	}
	if r.ResultCommit != "" {
		modes++
	}
	if r.MissingReport {
		modes++
	}
	if !r.ConfirmRecovery || modes != 1 || r.Attempt == "" {
		return Work{}, fmt.Errorf("confirmation explicite, tentative et une seule cause de reprise requises : avis indépendant, résultat incomplet immuable ou rapport absent")
	}
	if len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 || len(strings.TrimSpace(r.RecoveryInstruction)) < 20 || len(r.RecoveryInstruction) > 16000 {
		return Work{}, fmt.Errorf("préciser le motif (8 à 2000 caractères) et la correction prévue (20 à 16000 caractères)")
	}
	// Keep the immutable result binding stable through the operator grant.
	// This path consumes no reviewer call just to establish a known incomplete delivery.
	if r.ResultCommit != "" {
		unlock, err := managedLock(s.root, work)
		if err != nil {
			return Work{}, err
		}
		defer unlock()
		current, err := s.get(work)
		if err != nil {
			return Work{}, err
		}
		task, err := current.task(r.Task)
		if err != nil {
			return Work{}, err
		}
		if task.CorrectiveRecovery == nil || task.CorrectiveRecovery.Event != r.EventID {
			if why := correctiveRecoveryReasonFor(&current, task, nil, true); why != "" {
				return Work{}, fmt.Errorf("%s", why)
			}
			h, err := s.managedRecoveryHandoff(current, task)
			if err != nil {
				return Work{}, err
			}
			if h == nil || h.Result == "" || h.Result != r.ResultCommit || h.PreviousAttempt != r.Attempt || h.Review != nil {
				return Work{}, fmt.Errorf("résultat incomplet absent, remplacé ou déjà revu")
			}
			_, err = s.managedDelivery(current, task, Agent{ID: h.PreviousAgent, Attempt: h.PreviousAttempt, DeliveryVersion: 1}, h.Result)
			if err == nil || commandFailure(err).Code != "delivery_incomplete" || !strings.HasPrefix(task.Blocker, "Livraison incomplète :") {
				return Work{}, fmt.Errorf("refus de complétude non démontré sur ce résultat")
			}
		}
	}
	raw, _ := json.Marshal(r)
	return s.mutateWithHook(work, "task.corrective-recovery", r.EventID, r.Revision, raw, func(w *Work) error {
		t, err := w.task(r.Task)
		if err != nil {
			return err
		}
		if reason := correctiveRecoveryReasonFor(w, t, nil, r.ResultCommit != "" || r.MissingReport); reason != "" {
			return fmt.Errorf("%s", reason)
		}
		if r.Attempt != t.Attempts[len(t.Attempts)-1].ID || (r.ResultCommit == "" && !r.MissingReport && (t.IndependentReview.ID != r.ReviewID || t.IndependentReview.Attempt != r.Attempt)) {
			return fmt.Errorf("l’avis ou la tentative a changé ; relire la correction avant de confirmer")
		}
		instruction := strings.TrimSpace(r.RecoveryInstruction)
		if instruction == strings.TrimSpace(t.Next) || (t.Profile != nil && instruction == strings.TrimSpace(t.Profile.Instruction)) {
			return fmt.Errorf("préciser une correction nouvelle ; répéter la consigne précédente est refusé")
		}
		t.CorrectiveRecovery = &CorrectiveRecovery{MissingReport: r.MissingReport, Result: r.ResultCommit, Event: r.EventID, Revision: w.Revision, Review: r.ReviewID, Attempt: r.Attempt, Actor: operatorIdentity(), At: now(), Reason: strings.TrimSpace(r.Reason), Instruction: instruction}
		t.PlanMaxAttempts = 4
		t.PlanningRetry = true
		t.Next = instruction
		if t.Profile != nil {
			profile := *t.Profile
			profile.Instruction, profile.Actor, profile.Updated = instruction, operatorIdentity(), now()
			t.Profile = &profile
		}
		return nil
	}, func(tx *sql.Tx, current *Work) error {
		if r.MissingReport {
			task, err := current.task(r.Task)
			if err != nil {
				return err
			}
			rows, err := tx.Query("SELECT body FROM agents WHERE work_id=? AND task_id=?", work, r.Task)
			if err != nil {
				return err
			}
			var agents []Agent
			for rows.Next() {
				var raw []byte
				if err = rows.Scan(&raw); err != nil {
					rows.Close()
					return err
				}
				var a Agent
				if err = json.Unmarshal(raw, &a); err != nil {
					rows.Close()
					return err
				}
				agents = append(agents, a)
			}
			err = rows.Err()
			rows.Close()
			if err != nil {
				return err
			}
			if !missingReportRecovery(current, task, agents) {
				return fmt.Errorf("absence de rapport non démontrée par le conducteur pour cette tentative")
			}
		}
		var active int
		if err := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')", work, r.Task).Scan(&active); err != nil {
			return err
		}
		if active != 0 {
			return fmt.Errorf("attendre la fin de l’agent avant d’autoriser une tentative supplémentaire")
		}
		return nil
	})
}

// A conductor refusal is an operational failure, never a fabricated review.
// Restrict this route to unmanaged deliveries; managed copies use commit-bound recovery.
func missingReportRecovery(w *Work, t *Task, agents []Agent) bool {
	if w.Planning == nil || w.Planning.Repository != nil || len(t.Attempts) == 0 || (t.IndependentReview != nil && t.IndependentReview.State == "running") {
		return false
	}
	latest := t.Attempts[len(t.Attempts)-1].ID
	for _, a := range agents {
		if a.WorkID == w.ID && a.TaskID == t.ID && a.Attempt == latest && a.Status == "completed" && strings.HasPrefix(a.Relay, "Relais refusé par le conducteur : aucun rapport produit par cette tentative") {
			return true
		}
	}
	return false
}
