//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
)

// Only persisted engine executions are evidence; imported gate results and
// prose claims never become execution receipts. Failed/stale checks retain the
// task without spending a review call.
func (s *Store) independentValidationEvidence(t *Task) (string, map[string]string, error) {
	if t.ValidationPolicy == nil || t.ValidationPolicy.Mode != "automatic" {
		return "", nil, nil
	}
	a := t.AutoValidation
	if a == nil || a.Attempt != latestAttemptID(t) || a.Controller != validationController || a.PolicyDigest != validationPolicyDigest(*t.ValidationPolicy) || !s.validGate(t) {
		return "", nil, fmt.Errorf("contrôles moteur courants requis avant la revue")
	}
	if a.State != "pending_review" && a.State != "accepted" {
		return "", nil, fmt.Errorf("contrôles non réussis")
	}
	if len(a.Controls) != len(t.ValidationPolicy.Controls) || a.Receipt == "" || a.Artifacts[a.Receipt] == "" {
		return "", nil, fmt.Errorf("reçu moteur incomplet")
	}
	for i, r := range a.Controls {
		c := t.ValidationPolicy.Controls[i]
		command, _ := json.Marshal(c.Command)
		actual, _ := json.Marshal(r.Command)
		if r.ID != c.ID || string(command) != string(actual) || !r.Executed || !r.Passed || r.ExitCode != 0 || r.Started == "" || r.Finished == "" {
			return "", nil, fmt.Errorf("contrôle %s non démontré", c.ID)
		}
	}
	if err := s.currentReportArtifacts(a.Artifacts); err != nil {
		return "", nil, err
	}
	raw, err := json.Marshal(a)
	return string(raw), a.Artifacts, err
}

// Reuse the frozen controls, never run them again just because the reviewer
// finished. Acceptance still checks authorization, current evidence and review.
func (s *Store) acceptReviewedValidation(w Work, t *Task, a Agent) (bool, string) {
	if _, _, err := s.independentValidationEvidence(t); err != nil {
		return false, err.Error()
	}
	if err := s.independentReviewGuard(&w, t); err != nil {
		return false, err.Error()
	}
	payload, _ := json.Marshal(map[string]string{"attempt": a.Attempt})
	_, err := s.mutateWithHook(w.ID, "task.auto-validation-reviewed", newID("auto-reviewed-"), w.Revision, payload, func(current *Work) error {
		task, err := current.task(t.ID)
		if err != nil {
			return err
		}
		if task.Status != "submitted" || task.AutoValidation == nil || task.AutoValidation.Attempt != a.Attempt || task.AutoValidation.State != "pending_review" {
			return fmt.Errorf("validation remplacée")
		}
		if _, _, err = s.independentValidationEvidence(task); err != nil {
			return err
		}
		if err = s.independentReviewGuard(current, task); err != nil {
			return err
		}
		for _, dep := range task.Depends {
			parent, _ := current.task(dep)
			if !s.acceptedFresh(current, parent, map[string]bool{}) {
				return fmt.Errorf("dépendance %s non validée ou périmée", dep)
			}
		}
		task.AutoValidation.State = "accepted"
		task.Status, task.Blocker, task.Next = "accepted", "", "Contrôles moteur et revue indépendante réussis."
		return nil
	}, func(tx *sql.Tx, _ *Work) error { return automaticValidationGuard(tx, w.ID) })
	if err != nil {
		return false, err.Error()
	}
	return true, "contrôles préautorisés et revue indépendante réussis"
}
