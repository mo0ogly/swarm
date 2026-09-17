package main

import (
	"encoding/json"
	"fmt"
	"regexp"
	"strings"
)

// AssistantAnswer v1 validation. A schema does not make a free text true: it
// only guarantees that every reference is resolvable and every proposed action
// exists. What the model asserts in prose remains its own claim, bound to the
// facts it cites.

var htmlShape = regexp.MustCompile(`(?i)<\s*/?\s*[a-z!]|javascript:|data:text/html`)

const (
	maxAnswerFacts  = 12
	maxAnswerSteps  = 3
	maxAnswerList   = 8
	maxAnswerQuest  = 5
	maxAnswerText   = 800
	maxAnswerReason = 600
)

// One repair attempt at most: a model that wraps its JSON in a code fence or
// adds a sentence is recoverable; anything else is refused with an explanation.
func extractAnswerJSON(raw string) (string, bool) {
	trimmed := strings.TrimSpace(raw)
	if strings.HasPrefix(trimmed, "{") && strings.HasSuffix(trimmed, "}") {
		return trimmed, false
	}
	body := trimmed
	if fence := regexp.MustCompile("(?s)```(?:json)?\\s*(\\{.*?\\})\\s*```").FindStringSubmatch(body); fence != nil {
		return fence[1], true
	}
	start := strings.Index(body, "{")
	end := strings.LastIndex(body, "}")
	if start < 0 || end <= start {
		return "", true
	}
	return body[start : end+1], true
}

func refuse(code, kind, message, detail string) *AssistRefusal {
	return &AssistRefusal{Code: code, Kind: kind, Message: message, Detail: guardBlock(detail, 600)}
}

// Parse and validate a provider reply against the exact context it was grounded
// on. Returns either an answer or a refusal, never both.
func validateAssistantReply(raw string, ctx PageContext, templateID string) (*AssistantAnswer, *AssistRefusal, bool) {
	if strings.TrimSpace(raw) == "" {
		return nil, refuse("empty_reply", refusalService, "Le fournisseur n’a renvoyé aucune réponse exploitable.", ""), false
	}
	if len(raw) > maxAnswerBytes*4 {
		return nil, refuse("oversized_reply", refusalContract, "Réponse du fournisseur hors limite de taille.", fmt.Sprintf("%d octets reçus", len(raw))), false
	}
	body, repaired := extractAnswerJSON(raw)
	if body == "" {
		return nil, refuse("invalid_json", refusalContract, "Réponse non conforme : aucun objet JSON exploitable. Une seule réparation de format est tentée, puis la réponse est refusée.", shortText(raw, 300)), repaired
	}
	var answer AssistantAnswer
	if e := strict([]byte(body), &answer); e != nil {
		return nil, refuse("invalid_json", refusalContract, "Réponse non conforme au contrat AssistantAnswer v1 : "+e.Error(), shortText(body, 300)), repaired
	}
	var fields map[string]json.RawMessage
	_ = json.Unmarshal([]byte(body), &fields)
	for _, name := range []string{"version", "template_id", "context_hash", "facts", "interpretation", "missing_information", "next_steps", "limitations", "questions"} {
		v, ok := fields[name]
		if !ok || string(v) == "null" {
			return nil, refuse("contract_violation", refusalContract, "Champ obligatoire absent ou nul : "+name, ""), repaired
		}
	}
	if len(body) > maxAnswerBytes {
		return nil, refuse("oversized_answer", refusalContract, fmt.Sprintf("Réponse supérieure à %d octets.", maxAnswerBytes), ""), repaired
	}
	if e := checkAnswer(&answer, ctx, templateID); e != nil {
		return nil, e, repaired
	}
	return &answer, nil, repaired
}

func checkAnswer(a *AssistantAnswer, ctx PageContext, templateID string) *AssistRefusal {
	if a.Version != assistAnswerVersion {
		return refuse("contract_violation", refusalContract, fmt.Sprintf("Version de réponse inattendue : %d.", a.Version), assistAnswerName)
	}
	if a.TemplateID != templateID {
		return refuse("contract_violation", refusalContract, "Gabarit renvoyé différent du gabarit demandé.", a.TemplateID)
	}
	if a.ContextHash != ctx.Hash {
		return refuse("stale_context", refusalEvidence, "La réponse ne porte pas sur le contexte envoyé : elle est refusée comme périmée.", a.ContextHash)
	}
	if len(a.Facts) == 0 || len(a.Facts) > maxAnswerFacts {
		return refuse("contract_violation", refusalContract, fmt.Sprintf("Nombre de faits hors bornes : 1 à %d attendus.", maxAnswerFacts), "")
	}
	known := ctx.factIDs()
	for i := range a.Facts {
		f := &a.Facts[i]
		f.Text = strings.TrimSpace(f.Text)
		if f.Text == "" || len(f.Text) > maxAnswerText {
			return refuse("contract_violation", refusalContract, "Fait vide ou trop long dans la réponse.", f.Text)
		}
		if htmlShape.MatchString(f.Text) {
			return refuse("unsafe_output", refusalContract, "Contenu balisé ou exécutable refusé dans la réponse.", f.Text)
		}
		if len(f.SourceIDs) == 0 || len(f.SourceIDs) > 8 {
			return refuse("unknown_reference", refusalEvidence, "Un fait sans référence de source exploitable a été refusé.", f.Text)
		}
		for _, id := range f.SourceIDs {
			if !known[id] {
				return refuse("unknown_reference", refusalEvidence, "Référence inconnue citée par la réponse : "+shortText(id, 40)+". Réponse refusée.", f.Text)
			}
		}
	}
	if len(a.NextSteps) > maxAnswerSteps {
		return refuse("contract_violation", refusalContract, fmt.Sprintf("Au plus %d prochaines actions.", maxAnswerSteps), "")
	}
	for i := range a.NextSteps {
		step := &a.NextSteps[i]
		action, ok := ctx.action(step.ActionID)
		if !ok {
			return refuse("unknown_action", refusalEvidence, "Action hors catalogue proposée : "+shortText(step.ActionID, 60)+". Réponse refusée.", step.Why)
		}
		if !action.Available {
			return refuse("unknown_action", refusalEvidence, "Action indisponible sur cette page proposée : "+action.ID+". Réponse refusée.", action.Precondition)
		}
		step.Why = strings.TrimSpace(step.Why)
		if step.Why == "" || len(step.Why) > maxAnswerReason {
			return refuse("contract_violation", refusalContract, "Justification d’action vide ou trop longue.", step.ActionID)
		}
		if htmlShape.MatchString(step.Why) {
			return refuse("unsafe_output", refusalContract, "Contenu balisé refusé dans une prochaine action.", step.Why)
		}
		if len(step.SourceIDs) == 0 || len(step.SourceIDs) > 8 {
			return refuse("unknown_reference", refusalEvidence, "Une action nécessite 1 à 8 références connues.", step.ActionID)
		}
		for _, id := range step.SourceIDs {
			if !known[id] {
				return refuse("unknown_reference", refusalEvidence, "Référence inconnue dans une prochaine action : "+shortText(id, 40)+".", step.ActionID)
			}
		}
	}
	a.Interpretation = strings.TrimSpace(a.Interpretation)
	if templateID == "mission_advice.v1" {
		lines := strings.Split(a.Interpretation, "\n")
		if len(lines) < 2 || len(lines) > 3 {
			return refuse("contract_violation", refusalContract, "Résumé attendu en deux ou trois phrases courtes.", "")
		}
		for _, line := range lines {
			if strings.TrimSpace(line) == "" || len([]rune(line)) > 220 {
				return refuse("contract_violation", refusalContract, "Une phrase courte par ligne, 220 caractères maximum.", "")
			}
		}
	}
	if templateID == "report_summary.v1" {
		lines := strings.Split(a.Interpretation, "\n")
		if len(lines) != 2 || strings.TrimSpace(lines[0]) == "" || strings.TrimSpace(lines[1]) == "" || len([]rune(lines[0])) > 320 || len([]rune(lines[1])) > 320 {
			return refuse("contract_violation", refusalContract, "Synthèse attendue en deux lignes non vides, de 320 caractères maximum chacune.", "")
		}
	}
	if len(a.Interpretation) > 2000 || htmlShape.MatchString(a.Interpretation) {
		return refuse("contract_violation", refusalContract, "Interprétation trop longue ou balisée.", "")
	}
	for _, list := range [][]string{a.MissingInformation, a.Limitations} {
		if len(list) > maxAnswerList {
			return refuse("contract_violation", refusalContract, fmt.Sprintf("Au plus %d entrées par liste.", maxAnswerList), "")
		}
		for _, item := range list {
			if len(item) > maxAnswerText || htmlShape.MatchString(item) {
				return refuse("contract_violation", refusalContract, "Entrée de liste trop longue ou balisée.", item)
			}
		}
	}
	if len(a.Questions) > maxAnswerQuest {
		return refuse("contract_violation", refusalContract, fmt.Sprintf("Au plus %d questions.", maxAnswerQuest), "")
	}
	for _, q := range a.Questions {
		if len(q) > maxAnswerText || htmlShape.MatchString(q) {
			return refuse("contract_violation", refusalContract, "Question trop longue ou balisée.", q)
		}
	}
	return nil
}

func answerBytes(a *AssistantAnswer) int {
	if a == nil {
		return 0
	}
	raw, _ := json.Marshal(a)
	return len(raw)
}
