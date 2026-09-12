package main

import (
	"encoding/json"
	"fmt"
	"strings"
)

// Both CLI and terminal record exactly the same evaluation and enforce the same barème.
// name est le nom humain de l'évaluation (décision de cadrage 2026-09-12) ;
// vide pour les enregistrements antérieurs, affiché alors avec un repli.
func (s *Store) applyGateDocument(w *Work, task, phase, name string, raw json.RawMessage) error {
	t, e := w.task(task)
	if e != nil {
		return e
	}
	// Le nom humain de la gate est exigé à la source : web, console et CLI
	// passent tous par ce point unique.
	if strings.TrimSpace(name) == "" {
		return fmt.Errorf("nom de la gate requis : nom humain de cette évaluation")
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
	t.Gate = &GateRecord{Name: strings.TrimSpace(name), Document: raw, Evaluation: ev, At: now()}
	return nil
}

// gateLabel nomme une évaluation : le nom humain saisi à l'enregistrement,
// sinon un repli explicite pour les gates antérieures à la décision de cadrage.
func gateLabel(t *Task) string {
	if t.Gate == nil {
		return ""
	}
	if strings.TrimSpace(t.Gate.Name) != "" {
		return strings.TrimSpace(t.Gate.Name)
	}
	return "Gate sans nom — " + gateDocumentPath(t)
}

// gateDocumentPath indique d'où vient la preuve d'une gate, pour garder la
// traçabilité des gates enregistrées avant l'exigence de nom.
func gateDocumentPath(t *Task) string {
	var doc struct {
		Path string `json:"path"`
	}
	if e := json.Unmarshal(t.Gate.Document, &doc); e == nil && strings.TrimSpace(doc.Path) != "" {
		return strings.TrimSpace(doc.Path)
	}
	return shortID(t.Gate.Evaluation.ConfigDigest)
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
