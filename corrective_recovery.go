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
	Result      string `json:"result_commit,omitempty"`
	Event       string `json:"event_id"`
	Revision    int    `json:"authorized_revision"`
	Review      string `json:"review_id"`
	Attempt     string `json:"attempt_id"`
	Actor       string `json:"actor"`
	At          string `json:"at"`
	Reason      string `json:"reason"`
	Instruction string `json:"instruction"`
}

func correctiveRecoveryReason(w *Work, t *Task, agents []Agent) string {
	return correctiveRecoveryReasonFor(w, t, agents, false)
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
	if !r.ConfirmRecovery || (r.ReviewID == "") == (r.ResultCommit == "") || r.Attempt == "" {
		return Work{}, fmt.Errorf("confirmation explicite, tentative et soit avis indépendant soit résultat incomplet immuable requis")
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
		if reason := correctiveRecoveryReasonFor(w, t, nil, r.ResultCommit != ""); reason != "" {
			return fmt.Errorf("%s", reason)
		}
		if r.Attempt != t.Attempts[len(t.Attempts)-1].ID || (r.ResultCommit == "" && (t.IndependentReview.ID != r.ReviewID || t.IndependentReview.Attempt != r.Attempt)) {
			return fmt.Errorf("l’avis ou la tentative a changé ; relire la correction avant de confirmer")
		}
		instruction := strings.TrimSpace(r.RecoveryInstruction)
		if instruction == strings.TrimSpace(t.Next) || (t.Profile != nil && instruction == strings.TrimSpace(t.Profile.Instruction)) {
			return fmt.Errorf("préciser une correction nouvelle ; répéter la consigne précédente est refusé")
		}
		t.CorrectiveRecovery = &CorrectiveRecovery{Result: r.ResultCommit, Event: r.EventID, Revision: w.Revision, Review: r.ReviewID, Attempt: r.Attempt, Actor: operatorIdentity(), At: now(), Reason: strings.TrimSpace(r.Reason), Instruction: instruction}
		t.PlanMaxAttempts = 4
		t.PlanningRetry = true
		t.Next = instruction
		if t.Profile != nil {
			profile := *t.Profile
			profile.Instruction, profile.Actor, profile.Updated = instruction, operatorIdentity(), now()
			t.Profile = &profile
		}
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
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
