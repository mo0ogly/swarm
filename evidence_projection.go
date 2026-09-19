package main

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
)

// TaskEvidence is the common CLI/web read model for validation facts.  It does
// not upgrade claims found in a report or a gate document into observed command
// executions. Unknown values are intentionally serialized instead of omitted.
type TaskEvidence struct {
	Attempt      string               `json:"attempt_id"`
	Revision     int                  `json:"revision"`
	Freshness    string               `json:"freshness"`
	ObservedAt   string               `json:"observed_at"`
	ReportReview EvidenceReportReview `json:"report_review"`
	Controls     EvidenceControls     `json:"controls"`
	Acceptance   EvidenceAcceptance   `json:"acceptance"`
	Limits       []string             `json:"limits"`
}

type EvidenceReportReview struct {
	State    string   `json:"state"`
	Attempt  string   `json:"attempt_id"`
	Reviewer string   `json:"reviewer"`
	At       string   `json:"at"`
	Limits   []string `json:"limits"`
}

type EvidenceControls struct {
	State string            `json:"state"`
	Items []EvidenceControl `json:"items"`
}

type EvidenceControl struct {
	ID           string   `json:"id"`
	Attempt      string   `json:"attempt_id"`
	Execution    string   `json:"execution"`
	Revision     string   `json:"revision"`
	CandidateSHA string   `json:"candidate_sha"`
	Command      []string `json:"command"`
	ExitCode     *int     `json:"exit_code"`
	Started      string   `json:"started_at"`
	Finished     string   `json:"finished_at"`
	Freshness    string   `json:"freshness"`
	Result       string   `json:"result"`
	Limits       []string `json:"limits"`
}

type EvidenceAcceptance struct {
	State    string `json:"state"`
	Revision string `json:"revision"`
	At       string `json:"at"`
}

func unknownEvidenceControl(id, freshness string) EvidenceControl {
	if strings.TrimSpace(id) == "" {
		id = "unknown"
	}
	return EvidenceControl{ID: id, Attempt: "unknown", Execution: "unknown", Revision: "unknown", CandidateSHA: "unknown", Command: []string{}, ExitCode: nil,
		Started: "unknown", Finished: "unknown", Freshness: freshness, Result: "unknown",
		Limits: []string{"Aucun reçu d’exécution moteur : une citation du rapport ou un statut de gate ne démontre ni la commande ni son code de sortie."}}
}

func gateResultIDs(raw json.RawMessage) []string {
	doc, err := decodeAny(raw)
	if err != nil {
		return nil
	}
	rows, ok := doc["results"].([]any)
	if !ok {
		return nil
	}
	ids := []string{}
	for _, row := range rows {
		m, ok := row.(map[string]any)
		if !ok {
			continue
		}
		if id, ok := m["id"].(string); ok && strings.TrimSpace(id) != "" && id != "deliverable" {
			ids = append(ids, id)
		}
	}
	return ids
}

func latestAttemptID(t *Task) string {
	if len(t.Attempts) == 0 || strings.TrimSpace(t.Attempts[len(t.Attempts)-1].ID) == "" {
		return "unknown"
	}
	return t.Attempts[len(t.Attempts)-1].ID
}

func (s *Store) taskEvidence(w *Work, t *Task, acceptedFresh bool) TaskEvidence {
	attempt := latestAttemptID(t)
	e := TaskEvidence{Attempt: attempt, Revision: w.Revision, Freshness: "unknown", ObservedAt: "unknown",
		ReportReview: EvidenceReportReview{State: "not_configured", Attempt: attempt, Reviewer: "unknown", At: "unknown", Limits: []string{"Aucune revue indépendante du rapport n’est configurée."}},
		Controls:     EvidenceControls{State: "unknown", Items: []EvidenceControl{}},
		Acceptance:   EvidenceAcceptance{State: "not_accepted", Revision: "unknown", At: "unknown"},
		Limits:       []string{"Les valeurs unknown ne constituent pas un succès."}}

	if r := t.IndependentReview; r != nil {
		e.ReportReview = EvidenceReportReview{State: r.State, Attempt: valueOrUnknown(r.Attempt), Reviewer: valueOrUnknown(r.Reviewer), At: valueOrUnknown(r.Finished),
			Limits: []string{"La revue porte sur le rapport et ses citations ; elle n’exécute aucun contrôle et ne vaut pas acceptation."}}
		if r.Finished == "" {
			e.ReportReview.At = valueOrUnknown(r.Started)
		}
	} else if w.Planning != nil && w.Planning.Reviewer != nil {
		// A reviewer is configured for this work but has not produced a
		// verdict for this attempt yet (not started, or waiting on its
		// producer). That is not the same fact as no reviewer configured
		// at all, so it must not be reported as not_configured.
		e.ReportReview = EvidenceReportReview{State: "pending", Attempt: attempt, Reviewer: "reviewer://" + w.Planning.Reviewer.Provider, At: "unknown",
			Limits: []string{"Vérificateur indépendant configuré pour ce travail ; aucune revue n’a encore produit de verdict pour cette tentative."}}
	}

	gateFresh := t.Gate != nil && s.validGate(t)
	if t.Gate != nil {
		e.ObservedAt = valueOrUnknown(t.Gate.At)
		if gateFresh {
			e.Freshness = "fresh"
		} else {
			e.Freshness = "stale"
		}
	}

	if a := t.AutoValidation; a != nil {
		e.Attempt = valueOrUnknown(a.Attempt)
		e.ObservedAt = valueOrUnknown(a.At)
		hasFailed, hasUnknown, hasPassed := false, false, false
		for _, r := range a.Controls {
			result, execution := "failed", "not_executed"
			var exitCode *int
			switch {
			case r.Executed && r.Passed:
				result, execution = "passed", "executed"
			case r.Executed:
				execution = "executed"
			case r.Started == "":
				result, execution = "unknown", "unknown"
			}
			if execution == "executed" || execution == "not_executed" {
				exit := r.ExitCode
				exitCode = &exit
			}
			switch result {
			case "failed":
				hasFailed = true
			case "unknown":
				hasUnknown = true
			case "passed":
				hasPassed = true
			}
			freshness := "stale"
			if gateFresh && (attempt == "unknown" || a.Attempt == attempt) {
				freshness = "fresh"
			}
			command := append([]string(nil), r.Command...)
			if len(command) == 0 {
				for _, policy := range a.Policy.Controls {
					if policy.ID == r.ID {
						command = append([]string(nil), policy.Command...)
						break
					}
				}
			}
			testedRevision := "unknown"
			if a.Revision > 0 {
				testedRevision = strconv.Itoa(a.Revision)
			}
			e.Controls.Items = append(e.Controls.Items, EvidenceControl{ID: r.ID, Attempt: valueOrUnknown(a.Attempt), Execution: execution, Revision: testedRevision, CandidateSHA: valueOrUnknown(a.CandidateSHA), Command: command,
				ExitCode: exitCode, Started: valueOrUnknown(r.Started), Finished: valueOrUnknown(r.Finished), Freshness: freshness, Result: result,
				Limits: []string{fmt.Sprintf("Sortie non exposée ; seule son empreinte SHA-256 est conservée. Plafond %d octets.", maxValidationOutput)}})
		}
		// Aggregate independently of item order: a failure is never masked by a
		// later unknown item, and passed requires at least one executed,
		// passing control — never the default for an empty or all-unknown list.
		switch {
		case hasFailed:
			e.Controls.State = "failed"
		case hasUnknown:
			e.Controls.State = "unknown"
		case hasPassed:
			e.Controls.State = "passed"
		default:
			e.Controls.State = "unknown"
		}
		if a.Revision > 0 {
			e.Limits = append(e.Limits, "Contrôles exécutés sur la révision "+strconv.Itoa(a.Revision)+" ; révision de lecture "+strconv.Itoa(w.Revision)+".")
		} else {
			e.Limits = append(e.Limits, "Révision d’exécution inconnue pour cet ancien reçu.")
		}
	} else if t.Gate != nil {
		for _, id := range gateResultIDs(t.Gate.Document) {
			e.Controls.Items = append(e.Controls.Items, unknownEvidenceControl(id, e.Freshness))
		}
		if len(e.Controls.Items) == 0 {
			e.Controls.Items = append(e.Controls.Items, unknownEvidenceControl("unknown", e.Freshness))
		}
		e.Limits = append(e.Limits, "Gate enregistrée sans reçu moteur : les contrôles effectivement exécutés restent unknown.")
	} else {
		e.Controls.Items = append(e.Controls.Items, unknownEvidenceControl("unknown", "unknown"))
	}

	switch {
	case t.Status == "accepted" && acceptedFresh:
		e.Acceptance.State = "accepted"
	case t.Status == "accepted":
		e.Acceptance.State = "stale"
	case t.Status == "waived":
		e.Acceptance.State = "waived"
		if t.Override != nil {
			e.Acceptance.At = valueOrUnknown(t.Override.At)
		}
	case t.Status == "submitted" && gateFresh:
		e.Acceptance.State = "pending"
	}
	return e
}

func valueOrUnknown(value string) string {
	if strings.TrimSpace(value) == "" {
		return "unknown"
	}
	return value
}

func evidenceText(e TaskEvidence) string {
	var b strings.Builder
	fmt.Fprintf(&b, "PREUVES STRUCTURÉES — tentative : %s · révision lue : %d · fraîcheur : %s\n", e.Attempt, e.Revision, e.Freshness)
	fmt.Fprintf(&b, "Revue du rapport : %s · date : %s (ne vaut ni exécution de contrôle ni acceptation)\n", e.ReportReview.State, e.ReportReview.At)
	for _, c := range e.Controls.Items {
		command, exit := "unknown", "unknown"
		if len(c.Command) > 0 {
			command = strings.Join(c.Command, " ")
		}
		if c.ExitCode != nil {
			exit = strconv.Itoa(*c.ExitCode)
		}
		fmt.Fprintf(&b, "Contrôle %s : tentative=%s · exécution=%s · révision=%s · sha_candidat=%s · commande=%s · code de sortie=%s · début=%s · fin=%s · fraîcheur=%s\n", c.ID, c.Attempt, c.Execution, c.Revision, c.CandidateSHA, command, exit, c.Started, c.Finished, c.Freshness)
	}
	fmt.Fprintf(&b, "Acceptation : %s · révision : %s · date : %s\n", e.Acceptance.State, e.Acceptance.Revision, e.Acceptance.At)
	for _, limit := range append(append([]string{}, e.ReportReview.Limits...), e.Limits...) {
		fmt.Fprintf(&b, "Limite : %s\n", limit)
	}
	return strings.TrimSpace(b.String())
}
