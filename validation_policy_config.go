//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"
)

// ValidationPolicyChange is the shared web/CLI contract.  A preview never
// authorizes anything; apply requires the opaque token emitted for the exact
// work revision and normalized policy.
type ValidationPolicyChange struct {
	Schema       int               `json:"schema_version"`
	EventID      string            `json:"event_id,omitempty"`
	Revision     int               `json:"expected_revision"`
	TaskID       string            `json:"task_id"`
	Intent       string            `json:"intent"`
	Policy       *ValidationPolicy `json:"policy,omitempty"`
	PreviewToken string            `json:"preview_token,omitempty"`
}

type ValidationCriterionPreview struct {
	Index      int      `json:"index"`
	Text       string   `json:"text"`
	ControlIDs []string `json:"control_ids"`
	Review     string   `json:"review"`
}

type ValidationPolicyPreview struct {
	Schema       int                          `json:"schema_version"`
	WorkID       string                       `json:"work_id"`
	TaskID       string                       `json:"task_id"`
	TaskTitle    string                       `json:"task_title"`
	Revision     int                          `json:"revision"`
	Intent       string                       `json:"intent"`
	Mode         string                       `json:"mode"`
	Scope        string                       `json:"scope"`
	Criteria     []ValidationCriterionPreview `json:"criteria"`
	Controls     []ValidationControl          `json:"controls"`
	Limits       []string                     `json:"limits"`
	Effects      []string                     `json:"effects"`
	Warnings     []string                     `json:"warnings"`
	Confirmation string                       `json:"confirmation"`
	Token        string                       `json:"preview_token"`
}

func normalizeValidationPolicyChange(change ValidationPolicyChange) (ValidationPolicyChange, error) {
	change.TaskID = strings.TrimSpace(change.TaskID)
	change.Intent = strings.TrimSpace(change.Intent)
	if change.Schema != 1 || !safeName(change.TaskID) {
		return change, fmt.Errorf("schema_version/task_id invalide")
	}
	if change.Intent != "replace" && change.Intent != "remove" {
		return change, fmt.Errorf("intent : replace ou remove requis")
	}
	if change.Intent == "remove" {
		if change.Policy != nil {
			return change, fmt.Errorf("retrait : policy doit être omise")
		}
		return change, nil
	}
	if change.Policy == nil {
		return change, fmt.Errorf("policy requise pour replace")
	}
	policy, err := normalizeValidationPolicy(*change.Policy)
	if err != nil {
		return change, err
	}
	change.Policy = &policy
	return change, nil
}

func validationPolicyPreviewToken(work string, revision int, change ValidationPolicyChange) string {
	copy := change
	copy.EventID, copy.PreviewToken = "", ""
	raw, _ := json.Marshal(struct {
		Work     string                 `json:"work"`
		Revision int                    `json:"revision"`
		Change   ValidationPolicyChange `json:"change"`
	}{work, revision, copy})
	return hash(raw)
}

func (s *Store) previewValidationPolicy(work string, change ValidationPolicyChange) (ValidationPolicyPreview, error) {
	var preview ValidationPolicyPreview
	change, err := normalizeValidationPolicyChange(change)
	if err != nil {
		return preview, err
	}
	w, err := s.get(work)
	if err != nil {
		return preview, err
	}
	if change.Revision != w.Revision {
		return preview, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; relire la tâche et demander un nouvel aperçu.", Retryable: true}
	}
	t, err := w.task(change.TaskID)
	if err != nil {
		return preview, err
	}
	if t.Status != "todo" && t.Status != "blocked" {
		return preview, fmt.Errorf("configurer les validations exige une tâche à faire ou bloquée ; rouvrir la tâche au préalable")
	}
	mode := "none"
	if change.Policy != nil {
		mode = change.Policy.Mode
		if err = validationPolicyCoversTask(*change.Policy, t); err != nil {
			return preview, fmt.Errorf("validation_policy : %w", err)
		}
	}
	preview = ValidationPolicyPreview{Schema: 1, WorkID: work, TaskID: t.ID, TaskTitle: t.Title,
		Revision: w.Revision, Intent: change.Intent, Mode: mode, Scope: "Cette autorisation concerne uniquement la tâche " + t.ID + " et reste applicable à ses tentatives jusqu’à modification ou retrait explicite.",
		Limits:   []string{"1 à 8 contrôles ; 32 arguments maximum par commande", "300 secondes cumulées ; 64 Kio de sortie retenue par contrôle", "Programmes autorisés : go, git, node, npm, python, python3, pytest", "Exécution sans shell implicite, dans le projet uniquement"},
		Effects:  []string{"Toute gate, tout reçu automatique et toute preuve de validation antérieurs seront invalidés.", "La configuration n’exécute aucun contrôle et ne lance aucun agent."},
		Warnings: []string{"Une suggestion ou un texte produit par une IA n’est jamais une autorisation.", "Un critère qualitatif doit conserver la revue humaine : choisir le mode humain pour toute la tâche."}}
	covered := map[int][]string{}
	if change.Policy != nil {
		preview.Controls = append([]ValidationControl{}, change.Policy.Controls...)
		for _, control := range change.Policy.Controls {
			for _, criterion := range control.Criteria {
				covered[criterion] = append(covered[criterion], control.ID)
			}
		}
	}
	for i, text := range t.Criteria {
		review := "revue humaine"
		if mode == "automatic" {
			review = "contrôle structuré préautorisé"
		}
		preview.Criteria = append(preview.Criteria, ValidationCriterionPreview{Index: i + 1, Text: text, ControlIDs: covered[i+1], Review: review})
	}
	if s.paused(work) && mode == "automatic" {
		preview.Warnings = append(preview.Warnings, "La mission est en pause : les contrôles automatiques resteront suspendus jusqu’à une reprise explicite.")
	}
	switch {
	case change.Intent == "remove":
		preview.Confirmation = "Retirer la politique et revenir à la revue humaine sans exécuter de contrôle."
	case mode == "human":
		preview.Confirmation = "Enregistrer la revue humaine pour tous les critères."
	default:
		preview.Confirmation = "Préautoriser exactement les contrôles affichés ; ils ne pourront valider que cette tâche, sous autorisation de mission active."
	}
	preview.Token = validationPolicyPreviewToken(work, w.Revision, change)
	return preview, nil
}

func validationPolicyChangeGuard(tx *sql.Tx, work, task string) error {
	var count int
	if err := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')", work, task).Scan(&count); err != nil {
		return err
	}
	if count > 0 {
		return &CommandError{Code: "active_agent", Message: "arrêter et réconcilier l’agent de cette tâche avant de modifier sa politique"}
	}
	return nil
}

func (s *Store) applyValidationPolicy(work string, change ValidationPolicyChange) (Work, error) {
	normalized, err := normalizeValidationPolicyChange(change)
	if err != nil {
		return Work{}, err
	}
	if change.PreviewToken == "" || change.PreviewToken != validationPolicyPreviewToken(work, change.Revision, normalized) {
		return Work{}, fmt.Errorf("l’aperçu confirmé n’est plus courant ; examiner un nouvel aperçu")
	}
	if !safeName(change.EventID) {
		return Work{}, fmt.Errorf("event_id obligatoire (lettres, chiffres, tirets)")
	}
	request, _ := json.Marshal(normalized)
	return s.mutateWithHook(work, "validation-policy.change", change.EventID, change.Revision, request, func(w *Work) error {
		t, findErr := w.task(change.TaskID)
		if findErr != nil {
			return findErr
		}
		if t.Status != "todo" && t.Status != "blocked" {
			return fmt.Errorf("la tâche doit être à faire ou bloquée")
		}
		if normalized.Policy != nil {
			if coverErr := validationPolicyCoversTask(*normalized.Policy, t); coverErr != nil {
				return fmt.Errorf("validation_policy : %w", coverErr)
			}
		}
		if change.Intent == "remove" {
			t.ValidationPolicy = nil
		} else {
			policy := *normalized.Policy
			policy.Authorized, policy.Actor = now(), operatorIdentity()
			t.ValidationPolicy = &policy
		}
		t.Gate, t.AutoValidation, t.Override, t.Revalidation = nil, nil, nil, nil
		return nil
	}, func(tx *sql.Tx, _ *Work) error { return validationPolicyChangeGuard(tx, work, change.TaskID) })
}
