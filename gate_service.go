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
	if e := requiredPlanChecks(t, raw); e != nil {
		return e
	}
	ev, e := evaluate(raw, s.root, phase)
	if e != nil {
		return e
	}
	if t.Revalidation != nil && t.Status != "submitted" {
		return fmt.Errorf("Revalidation : soumettre un nouveau rapport de contrôles avant la gate")
	}
	if err := revalidationGate(t, ev); err != nil {
		return err
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

// A generated plan cannot be accepted by replacing its required checks with a
// single unrelated green score. The operator still reviews the evidence itself.
func requiredPlanChecks(t *Task, raw json.RawMessage) error {
	if len(t.PlanChecks) == 0 {
		return nil
	}
	var doc struct {
		Checks []struct {
			ID        string `json:"id"`
			Gate      string `json:"gate"`
			Mandatory bool   `json:"mandatory"`
		}
	}
	if e := json.Unmarshal(raw, &doc); e != nil {
		return e
	}
	found := map[string]bool{}
	for _, c := range doc.Checks {
		if expected, ok := t.PlanChecks[c.ID]; ok && c.Mandatory && c.Gate == expected {
			found[c.ID] = true
		}
	}
	for id, phase := range t.PlanChecks {
		if !found[id] {
			return fmt.Errorf("Contrôle obligatoire du plan absent ou modifié : %s, gate %s, mandatory=true.", id, phase)
		}
	}
	return nil
}

func revalidationGate(t *Task, ev Evaluation) error {
	if t.Revalidation == nil {
		return nil
	}
	r := t.Revalidation
	if r.Report == "" || ev.Artifacts[r.Report] != r.ReportHash {
		return fmt.Errorf("Revalidation : soumettre un nouveau rapport de contrôles et le référencer dans les preuves de la gate ; changer les empreintes ne suffit pas")
	}
	if ev.ConfigDigest != r.Config {
		return fmt.Errorf("Revalidation : les contrôles et leur barème doivent être conservés")
	}
	return nil
}
