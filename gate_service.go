package main

import (
	"encoding/json"
	"fmt"
)

// Both CLI and terminal record exactly the same evaluation and enforce the same barème.
func (s *Store) applyGateDocument(w *Work, task, phase string, raw json.RawMessage) error {
	t, e := w.task(task)
	if e != nil {
		return e
	}
	if t.Status == "accepted" || t.Status == "waived" || t.Status == "abandoned" {
		return fmt.Errorf("rouvrir la tâche avant une nouvelle évaluation")
	}
	ev, e := evaluate(raw, s.root, phase)
	if e != nil {
		return e
	}
	if ev.Scope != t.ID {
		return fmt.Errorf("scope_id doit correspondre à task_id")
	}
	if t.Gate != nil && t.Gate.Evaluation.ConfigDigest != ev.ConfigDigest {
		return fmt.Errorf("barème modifié : rouvrir une tentative après réorientation explicite")
	}
	t.Gate = &GateRecord{raw, ev, now()}
	return nil
}
