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
	EnvironmentRecoveryReason string            `json:"environment_recovery_reason,omitempty"`
	RecheckCompleted          bool              `json:"recheck_completed,omitempty"`
	Schema                    int               `json:"schema_version"`
	EventID                   string            `json:"event_id,omitempty"`
	Revision                  int               `json:"expected_revision"`
	TaskID                    string            `json:"task_id"`
	Intent                    string            `json:"intent"`
	Policy                    *ValidationPolicy `json:"policy,omitempty"`
	PreviewToken              string            `json:"preview_token,omitempty"`
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
	change.EnvironmentRecoveryReason = strings.TrimSpace(change.EnvironmentRecoveryReason)
	if change.EnvironmentRecoveryReason != "" && (!change.RecheckCompleted || len([]rune(change.EnvironmentRecoveryReason)) < 8 || len([]rune(change.EnvironmentRecoveryReason)) > 500) {
		return change, fmt.Errorf("environment_recovery_reason : motif explicite de 8 à 500 caractères avec recheck_completed requis")
	}
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
	if t.Status != "todo" && t.Status != "blocked" && t.Status != "submitted" {
		return preview, fmt.Errorf("configurer les validations exige une tâche à faire, bloquée ou à vérifier ; rouvrir la tâche au préalable")
	}
	if change.RecheckCompleted {
		if err = validationRecheckGuard(t, change.Policy, change.EnvironmentRecoveryReason); err != nil {
			return preview, err
		}
		var body []byte
		if err = s.db.QueryRow("SELECT body FROM agents WHERE work_id=? AND task_id=? ORDER BY rowid DESC LIMIT 1", work, change.TaskID).Scan(&body); err != nil {
			return preview, fmt.Errorf("tentative terminée absente : %w", err)
		}
		var producer Agent
		if json.Unmarshal(body, &producer) != nil || producer.Status != "completed" || producer.Attempt != latestAttemptID(t) {
			return preview, fmt.Errorf("recontrôle : dernière tentative non terminée ou remplacée")
		}
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
	if s.paused(work) && len(preview.Controls) > 0 {
		preview.Warnings = append(preview.Warnings, "La mission est en pause : les contrôles automatiques resteront suspendus jusqu’à une reprise explicite.")
	}
	switch {
	case change.Intent == "remove":
		preview.Confirmation = "Retirer la politique et revenir à la revue humaine sans exécuter de contrôle."
	case mode == "human":
		preview.Confirmation = "Enregistrer la revue humaine pour tous les critères."
		if len(preview.Controls) > 0 {
			preview.Confirmation = "Exécuter les contrôles autorisés comme preuves ; aucune acceptation automatique. La décision humaine reste obligatoire."
		}
	default:
		preview.Confirmation = "Préautoriser exactement les contrôles affichés ; ils ne pourront valider que cette tâche, sous autorisation de mission active."
	}
	if change.RecheckCompleted {
		preview.Effects = append(preview.Effects, "Recontrôle explicitement demandé : résultat existant remis aux contrôles et à la revue, sans nouvel agent ni tentative.")
	}
	preview.Token = validationPolicyPreviewToken(work, w.Revision, change)
	return preview, nil
}

func validationRecheckGuard(t *Task, policy *ValidationPolicy, recoveryReasons ...string) error {
	if t == nil || t.Status != "blocked" || t.AutoValidation == nil || t.AutoValidation.State != "blocked" || latestAttemptID(t) == "" || t.AutoValidation.Attempt != latestAttemptID(t) || policy == nil {
		return fmt.Errorf("recontrôle : résultat terminé bloqué par ses contrôles requis")
	}
	if validationPolicyDigest(*policy) == t.AutoValidation.PolicyDigest {
		environment := false
		for _, control := range t.AutoValidation.Controls {
			if control.EnvironmentFailure {
				environment = true
			}
		}
		if !environment || len(recoveryReasons) == 0 || strings.TrimSpace(recoveryReasons[0]) == "" {
			return fmt.Errorf("recontrôle : politique corrigée différente requise ; pour un échec d’environnement identifié, déclarer la précondition corrigée")
		}
	}
	return nil
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
		if t.Status != "todo" && t.Status != "blocked" && t.Status != "submitted" {
			return fmt.Errorf("la tâche doit être à faire, bloquée ou à vérifier")
		}
		if t.IndependentReview != nil && t.IndependentReview.State == "running" {
			return fmt.Errorf("attendre la fin de la revue avant de modifier la politique")
		}
		if normalized.Policy != nil {
			if coverErr := validationPolicyCoversTask(*normalized.Policy, t); coverErr != nil {
				return fmt.Errorf("validation_policy : %w", coverErr)
			}
		}
		if change.RecheckCompleted {
			if change.Intent != "replace" {
				return fmt.Errorf("recontrôle : remplacement explicite de la politique requis")
			}
			if err := validationRecheckGuard(t, normalized.Policy, normalized.EnvironmentRecoveryReason); err != nil {
				return err
			}
			t.Status, t.Blocker = "submitted", ""
			t.Next = "Recontrôler le résultat existant ; aucune nouvelle production ni acceptation."
		}
		if change.Intent == "remove" {
			t.ValidationPolicy = nil
		} else {
			policy := *normalized.Policy
			policy.Authorized, policy.Actor = now(), operatorIdentity()
			t.ValidationPolicy = &policy
		}
		archiveIndependentReview(t)
		t.Gate, t.AutoValidation, t.Override, t.Revalidation = nil, nil, nil, nil
		return nil
	}, func(tx *sql.Tx, current *Work) error {
		if err := validationPolicyChangeGuard(tx, work, change.TaskID); err != nil {
			return err
		}
		if change.RecheckCompleted {
			var body []byte
			if err := tx.QueryRow("SELECT body FROM agents WHERE work_id=? AND task_id=? ORDER BY rowid DESC LIMIT 1", work, change.TaskID).Scan(&body); err != nil {
				return fmt.Errorf("tentative terminée absente : %w", err)
			}
			var a Agent
			if err := json.Unmarshal(body, &a); err != nil || a.Status != "completed" {
				return fmt.Errorf("recontrôle : dernière tentative non terminée")
			}
			t, err := current.task(change.TaskID)
			if err != nil || len(t.Attempts) == 0 || a.Attempt != t.Attempts[len(t.Attempts)-1].ID {
				return fmt.Errorf("recontrôle : tentative remplacée")
			}
		}
		return nil
	})
}
