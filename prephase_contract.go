package main

import (
	"fmt"
	"strings"
	"unicode/utf8"
)

const preparationDocumentLimit = 16000

// Preparation documents are independent of Work tasks and process activity.
// Reading or editing a preparation never makes anything eligible for dispatch.
type PreparationDocument struct {
	SourceTurn string `json:"source_turn,omitempty"`
	Kind       string `json:"kind"`
	Text       string `json:"text"`
	Revision   int    `json:"revision"`
	Hash       string `json:"sha256"`
	At         string `json:"at"`
	BriefHash  string `json:"brief_hash,omitempty"`
	MethodHash string `json:"method_hash,omitempty"`
	NeedHash   string `json:"need_hash,omitempty"`
}

type Preparation struct {
	Sources           []PreparationSource            `json:"sources,omitempty"`
	Budget            *Budget                        `json:"budget,omitempty"`
	Conversion        *PreparationConversion         `json:"conversion,omitempty"`
	ID                string                         `json:"id"`
	WorkID            string                         `json:"work_id,omitempty"`
	Title             string                         `json:"title"`
	Method            string                         `json:"method"`
	MethodHash        string                         `json:"method_hash,omitempty"`
	Revision          int                            `json:"revision"`
	CreatedAt         string                         `json:"created_at"`
	UpdatedAt         string                         `json:"updated_at"`
	Documents         map[string]PreparationDocument `json:"documents"`
	Brief             *PreparationDocument           `json:"adopted_brief,omitempty"`
	Verdict           *PreparationVerdict            `json:"verdict,omitempty"`
	PlanReady         bool                           `json:"plan_ready"`
	ReceiptHistorical bool                           `json:"receipt_historical,omitempty"`
}

type PreparationVerdict struct {
	PlanHash   string `json:"plan_hash"`
	BriefHash  string `json:"brief_hash"`
	MethodHash string `json:"method_hash"`
	At         string `json:"at"`
}

type PreparationRequest struct {
	Organization *PreparationOrganization `json:"organization,omitempty"`
	Source       *PreparationSourceRef    `json:"source,omitempty"`
	Budget       *Budget                  `json:"budget,omitempty"`
	AdoptBrief   bool                     `json:"adopt_brief,omitempty"`
	WorkRevision *int                     `json:"expected_work_revision,omitempty"`
	Decisions    []PlanQuestion           `json:"decisions,omitempty"`
	Turn         string                   `json:"turn_id,omitempty"`
	Version      int                      `json:"version"`
	ID           string                   `json:"preparation_id,omitempty"`
	Event        string                   `json:"event_id"`
	Action       string                   `json:"action"`
	Revision     *int                     `json:"expected_revision"`
	WorkID       string                   `json:"work_id,omitempty"`
	Title        string                   `json:"title,omitempty"`
	Method       string                   `json:"method,omitempty"`
	Document     string                   `json:"document,omitempty"`
	Text         string                   `json:"text,omitempty"`
	Hash         string                   `json:"sha256,omitempty"`
}

type PreparationError struct{ Code, Message string }

func (e *PreparationError) Error() string { return e.Message }
func preparationError(code, message string) error {
	return &PreparationError{code, message}
}

func (r PreparationRequest) validate() error {
	if r.Version != 1 || r.Revision == nil || *r.Revision < 0 || !preparationKey(r.Event) {
		return preparationError("invalid_request", "version 1, event_id et expected_revision positif ou nul requis.")
	}
	if r.Action != "create" && !preparationKey(r.ID) {
		return preparationError("invalid_request", "Identifiant de préparation requis.")
	}
	if !utf8.ValidString(r.Text) || len(r.Text) > preparationDocumentLimit {
		return preparationError("invalid_document", "Document UTF-8 limité à 16 000 octets ; texte non enregistré.")
	}
	if len(r.Title) > 240 || !utf8.ValidString(r.Title) {
		return preparationError("invalid_request", "Titre UTF-8 limité à 240 octets.")
	}
	if r.WorkID != "" && !preparationKey(r.WorkID) {
		return preparationError("invalid_request", "Identifiant de travail invalide.")
	}
	if r.AdoptBrief && r.Action != "use-proposal" {
		return preparationError("invalid_request", "Adoption liée uniquement à une proposition de brief.")
	}
	if (r.Action == "budget") != (r.Budget != nil) || (r.Action == "source-add" || r.Action == "source-remove") != (r.Source != nil) {
		return preparationError("invalid_request", "Budget ou source incompatible avec cette action.")
	}
	if r.Organization != nil && r.Action != "create-missions" && r.Action != "authorize-plan" {
		return preparationError("invalid_request", "Organisation autorisée uniquement lors de la création.")
	}
	switch r.Action {
	case "source-add", "source-remove", "budget", "create", "method", "save", "adopt-brief", "validate-plan", "use-proposal", "answer-questions", "create-missions", "authorize-plan", "revise-missions", "release-plan":
	default:
		return preparationError("invalid_action", fmt.Sprintf("Action de préparation indisponible : %s", r.Action))
	}
	if r.Action != "create" && (r.WorkID != "" || r.Title != "") ||
		r.Action != "create" && r.Action != "method" && r.Method != "" ||
		r.Action != "create" && r.Action != "save" && r.Text != "" ||
		r.Action != "save" && r.Document != "" ||
		r.Action != "adopt-brief" && r.Action != "validate-plan" && r.Action != "answer-questions" && r.Action != "create-missions" && r.Action != "authorize-plan" && r.Action != "revise-missions" && r.Action != "release-plan" && r.Hash != "" {
		return preparationError("invalid_request", "Champ incompatible avec cette action ; aucune modification enregistrée.")
	}
	if r.Action != "answer-questions" && r.Decisions != nil || r.Action == "answer-questions" && (r.Hash == "" || len(r.Decisions) == 0 || len(r.Decisions) > 16) {
		return preparationError("invalid_request", "Décisions et empreinte du plan requises uniquement pour répondre aux questions.")
	}
	for _, q := range r.Decisions {
		if !utf8.ValidString(q.Question) || !utf8.ValidString(q.Answer) || len(q.Question) > preparationDocumentLimit || len(q.Answer) > 4000 {
			return preparationError("invalid_request", "Chaque réponse doit être en UTF-8 et limitée à 4 000 octets.")
		}
	}
	if r.Action != "create-missions" && r.Action != "authorize-plan" && r.Action != "revise-missions" && r.Action != "release-plan" && r.WorkRevision != nil {
		return preparationError("invalid_request", "Révision du travail incompatible avec cette action.")
	}
	if (r.Action == "create-missions" || r.Action == "authorize-plan" || r.Action == "revise-missions" || r.Action == "release-plan") && (r.Hash == "" || r.WorkRevision == nil || *r.WorkRevision < 0) {
		return preparationError("invalid_request", "Empreinte du plan et expected_work_revision requis.")
	}
	if r.Action == "use-proposal" && !preparationKey(r.Turn) || r.Action != "use-proposal" && r.Turn != "" {
		return preparationError("invalid_request", "Échange requis uniquement pour utiliser une proposition.")
	}
	return nil
}

func preparationKey(s string) bool {
	if len(s) < 1 || len(s) > 128 {
		return false
	}
	for _, c := range s {
		if !(c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' || c == '-' || c == '_') {
			return false
		}
	}
	return true
}

func preparationDocumentKind(s string) bool {
	return s == "besoin" || s == "brief" || s == "plan"
}

func preparationTitle(s string) string {
	if strings.TrimSpace(s) == "" {
		return "Nouvelle préparation"
	}
	return strings.TrimSpace(s)
}
