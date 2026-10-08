package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"
)

func attemptExtensionReason(t *Task, agents []Agent) string {
	if t.Status != "blocked" || t.PlanMaxAttempts < 1 || len(t.Attempts) != t.PlanMaxAttempts {
		return "Une tentative supplémentaire se prépare après épuisement du plafond d’une tâche bloquée."
	}
	if t.PlanMaxAttempts >= 3 || len(t.Attempts) >= 3 {
		return "Trois tentatives déjà autorisées : revoir le périmètre et les preuves avant une autre décision."
	}
	if t.IndependentReview != nil && t.IndependentReview.State == "running" {
		return "Attendre la fin de la revue indépendante avant de préparer une nouvelle tentative."
	}
	for _, a := range agents {
		if a.TaskID == t.ID && activeAgent(a) {
			return "Attendre la fin de l’agent et la libération de sa tentative."
		}
	}
	return ""
}

// An explicit operator decision changes one allowance, not the task contract,
// past opinions or production history. It never starts a provider by itself.
func (s *Store) extendAttempt(work string, r PlanningRequest) (Work, error) {
	if len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 || len(strings.TrimSpace(r.RecoveryInstruction)) < 20 || len(r.RecoveryInstruction) > 16000 {
		return Work{}, fmt.Errorf("préciser le motif (8 à 2000 caractères) et la correction prévue (20 à 16000 caractères)")
	}
	raw, _ := json.Marshal(r)
	return s.mutateWithHook(work, "task.attempt-extension", r.EventID, r.Revision, raw, func(w *Work) error {
		t, err := w.task(r.Task)
		if err != nil {
			return err
		}
		if reason := attemptExtensionReason(t, nil); reason != "" {
			return fmt.Errorf("%s", reason)
		}
		t.PlanMaxAttempts++
		t.Next = strings.TrimSpace(r.RecoveryInstruction)
		if t.Profile != nil {
			profile := *t.Profile
			profile.Instruction = t.Next
			profile.Actor = operatorIdentity()
			profile.Updated = now()
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
