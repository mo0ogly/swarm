package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"
)

// A restart is an operator-authorized new production, never a refund or an
// acceptance. Old attempts, reports and costs remain durable.
type TaskRestart struct {
	Event           string           `json:"event_id"`
	Attempt         string           `json:"previous_attempt"`
	Candidate       string           `json:"base_candidate"`
	Reason          string           `json:"reason"`
	Actor           string           `json:"actor"`
	At              string           `json:"at"`
	PreviousBlocker string           `json:"previous_blocker"`
	Recovered       *RecoveredResult `json:"previous_recovered_result,omitempty"`
}

func (s *Store) restartTask(work string, r PlanningRequest) (Work, error) {
	if !r.ConfirmRecovery || r.Attempt == "" || r.ExpectedCandidate == "" || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 || len(strings.TrimSpace(r.RecoveryInstruction)) < 20 || len(r.RecoveryInstruction) > 16000 {
		return Work{}, fmt.Errorf("confirmation, tentative, candidat de départ, motif et nouvelle consigne requis")
	}
	unlock, err := managedLock(s.root, work)
	if err != nil {
		return Work{}, err
	}
	defer unlock()
	raw, _ := json.Marshal(r)
	return s.mutateWithHook(work, "task.restart", r.EventID, r.Revision, raw, func(w *Work) error {
		t, e := w.task(r.Task)
		if e != nil {
			return e
		}
		if t.Status != "blocked" || t.PlanningRetry || len(t.Attempts) == 0 || t.Attempts[len(t.Attempts)-1].ID != r.Attempt {
			return fmt.Errorf("redémarrage réservé à la dernière tentative bloquée, sans reprise déjà préparée")
		}
		if w.Planning == nil || w.Planning.Repository == nil || w.Planning.Repository.Candidate != r.ExpectedCandidate {
			return fmt.Errorf("candidat de départ modifié ou absent")
		}
		if e = reviewerLaunchGuard(*w); e != nil {
			return e
		}
		for _, task := range w.Tasks {
			if task.IndependentReview != nil && task.IndependentReview.State == "running" {
				return fmt.Errorf("attendre la fin de la revue active")
			}
		}
		if t.Profile == nil || t.PlanMaxAttempts != len(t.Attempts) {
			return fmt.Errorf("profil requis et tentatives existantes à terminer avant redémarrage")
		}
		if strings.TrimSpace(r.RecoveryInstruction) == strings.TrimSpace(t.Profile.Instruction) {
			return fmt.Errorf("une nouvelle consigne de redémarrage est requise")
		}
		t.Restarts = append(t.Restarts, TaskRestart{Event: r.EventID, Attempt: r.Attempt, Candidate: r.ExpectedCandidate, Reason: strings.TrimSpace(r.Reason), Actor: operatorIdentity(), At: now(), PreviousBlocker: t.Blocker, Recovered: t.RecoveredResult})
		if t.IndependentReview != nil {
			t.PreviousReviews = append(t.PreviousReviews, *t.IndependentReview)
		}
		t.IndependentReview = nil
		t.BatchReviewResume = nil
		t.RecoveredResult = nil
		t.AutoValidation = nil
		t.Gate = nil
		t.Override = nil
		t.PlanMaxAttempts++
		t.PlanningRetry = true
		t.Status = "todo"
		t.Blocker = ""
		t.Next = strings.TrimSpace(r.RecoveryInstruction)
		profile := *t.Profile
		profile.Instruction = t.Next
		profile.Actor = operatorIdentity()
		profile.Updated = now()
		t.Profile = &profile
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
		var n int
		if e := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')", work).Scan(&n); e != nil {
			return e
		}
		if n != 0 {
			return fmt.Errorf("attendre la fin des agents actifs avant redémarrage")
		}
		return nil
	})
}
