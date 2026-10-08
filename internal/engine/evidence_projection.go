package engine

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"sort"
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
	State   string            `json:"state"`
	Items   []EvidenceControl `json:"items"`
	History []EvidenceControl `json:"history"`
	Longest []EvidenceControl `json:"longest_measured"`
}

type EvidenceControl struct {
	ID             string   `json:"id"`
	Attempt        string   `json:"attempt_id"`
	Execution      string   `json:"execution"`
	Revision       string   `json:"revision"`
	CandidateSHA   string   `json:"candidate_sha"`
	Command        []string `json:"command"`
	ExitCode       *int     `json:"exit_code"`
	Started        string   `json:"started_at"`
	Finished       string   `json:"finished_at"`
	WallDurationMS *int64   `json:"wall_duration_ms"`
	CPUDurationMS  *int64   `json:"cpu_duration_ms"`
	Cost           string   `json:"cost"`
	Tokens         string   `json:"tokens"`
	Freshness      string   `json:"freshness"`
	Result         string   `json:"result"`
	Limits         []string `json:"limits"`
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
		Started: "unknown", Finished: "unknown", WallDurationMS: nil, CPUDurationMS: nil, Cost: "unknown", Tokens: "unknown", Freshness: freshness, Result: "unknown",
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
		Controls:     EvidenceControls{State: "unknown", Items: []EvidenceControl{}, History: []EvidenceControl{}, Longest: []EvidenceControl{}},
		Acceptance:   EvidenceAcceptance{State: "not_accepted", Revision: "unknown", At: "unknown"},
		Limits:       []string{"Les valeurs unknown ne constituent pas un succès."}}

	if r := t.IndependentReview; r != nil {
		e.ReportReview = EvidenceReportReview{State: r.State, Attempt: valueOrUnknown(r.Attempt), Reviewer: valueOrUnknown(r.Reviewer), At: valueOrUnknown(r.Finished),
			Limits: []string{"La revue porte sur le rapport et ses citations ; elle n’exécute aucun contrôle et ne vaut pas acceptation."}}
		if w.Planning != nil && w.Planning.Repository != nil {
			e.ReportReview.Limits = []string{"La revue examine le candidat, les sources fournies et les reçus de contrôles ; elle n’exécute pas elle-même les commandes et ne vaut pas acceptation."}
		}
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

	a := t.AutoValidation
	if a == nil && w.Planning != nil && w.Planning.Repository != nil {
		a = s.reviewReceiptEvidence(t)
	}
	if a != nil {
		controlFresh := gateFresh
		if t.ValidationPolicy != nil && t.ValidationPolicy.Mode == "human" && len(t.ValidationPolicy.Controls) > 0 {
			_, _, evidenceErr := s.independentValidationEvidence(t)
			controlFresh = evidenceErr == nil
			e.Freshness = "stale"
			if controlFresh {
				e.Freshness = "fresh"
			}
		}
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
			if controlFresh && (attempt == "unknown" || a.Attempt == attempt) {
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
				ExitCode: exitCode, Started: valueOrUnknown(r.Started), Finished: valueOrUnknown(r.Finished), WallDurationMS: r.WallDurationMS, CPUDurationMS: r.CPUDurationMS, Cost: "unknown", Tokens: "unknown", Freshness: freshness, Result: result,
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
	e.Controls.History = s.validationControlHistory(w.ID, t, a)
	measured := append(append([]EvidenceControl{}, e.Controls.Items...), e.Controls.History...)
	measured = slicesDeleteUnmeasured(measured)
	sort.SliceStable(measured, func(i, j int) bool { return *measured[i].WallDurationMS > *measured[j].WallDurationMS })
	if len(measured) > 5 {
		measured = measured[:5]
	}
	e.Controls.Longest = measured

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

func slicesDeleteUnmeasured(items []EvidenceControl) []EvidenceControl {
	out := items[:0]
	for _, item := range items {
		if item.WallDurationMS != nil {
			out = append(out, item)
		}
	}
	return out
}

// validationControlHistory projects immutable prior receipts. It never turns an
// event into a current verdict and filters by attempts owned by the selected task.
func (s *Store) validationControlHistory(work string, t *Task, current *AutomaticValidation) []EvidenceControl {
	attempts := map[string]bool{}
	for _, attempt := range t.Attempts {
		attempts[attempt.ID] = true
	}
	// No receipt can belong to a task without an attempt. Avoid rescanning
	// the work's event history for every unstarted card in a large graph.
	if len(attempts) == 0 {
		return []EvidenceControl{}
	}
	rows, err := s.db.Query("SELECT payload FROM events WHERE work_id=? AND kind='task.auto-validation' ORDER BY revision DESC", work)
	if err != nil {
		return []EvidenceControl{}
	}
	defer rows.Close()
	history := []EvidenceControl{}
	seen := map[string]bool{}
	for rows.Next() {
		var raw []byte
		var receipt AutomaticValidation
		if rows.Scan(&raw) != nil || json.Unmarshal(raw, &receipt) != nil || !attempts[receipt.Attempt] || (current != nil && receipt.Receipt == current.Receipt) || seen[receipt.Receipt] {
			continue
		}
		seen[receipt.Receipt] = true
		for _, result := range receipt.Controls {
			state, execution := "failed", "not_executed"
			if result.Executed {
				execution = "executed"
				if result.Passed {
					state = "passed"
				}
			} else if result.Started == "" {
				state, execution = "unknown", "unknown"
			}
			exit := result.ExitCode
			history = append(history, EvidenceControl{ID: result.ID, Attempt: valueOrUnknown(receipt.Attempt), Execution: execution, Revision: strconv.Itoa(receipt.Revision), CandidateSHA: valueOrUnknown(receipt.CandidateSHA), Command: append([]string(nil), result.Command...), ExitCode: &exit, Started: valueOrUnknown(result.Started), Finished: valueOrUnknown(result.Finished), WallDurationMS: result.WallDurationMS, CPUDurationMS: result.CPUDurationMS, Cost: "unknown", Tokens: "unknown", Freshness: "historical", Result: state, Limits: []string{"Reçu historique conservé ; il ne constitue pas le verdict actuel."}})
		}
	}
	return history
}

func evidenceText(e TaskEvidence) string {
	var b strings.Builder
	fmt.Fprintf(&b, uiText("PREUVES STRUCTURÉES — tentative : %s · révision lue : %d · fraîcheur : %s\n"), e.Attempt, e.Revision, e.Freshness)
	fmt.Fprintf(&b, uiText("Revue du rapport : %s · date : %s (ne vaut ni exécution de contrôle ni acceptation)\n"), e.ReportReview.State, e.ReportReview.At)
	fmt.Fprintf(&b, uiText("Verdict actuel — contrôles : %s · acceptation : %s\n"), e.Controls.State, e.Acceptance.State)
	for _, c := range e.Controls.Items {
		command, exit, wall, cpu := "unknown", "unknown", "unknown", "unknown"
		if len(c.Command) > 0 {
			command = strings.Join(c.Command, " ")
		}
		if c.ExitCode != nil {
			exit = strconv.Itoa(*c.ExitCode)
		}
		if c.WallDurationMS != nil {
			wall = fmt.Sprintf("%d ms", *c.WallDurationMS)
		}
		if c.CPUDurationMS != nil {
			cpu = fmt.Sprintf("%d ms", *c.CPUDurationMS)
		}
		fmt.Fprintf(&b, uiText("Contrôle %s : tentative=%s · résultat=%s · exécution=%s · révision=%s · sha_candidat=%s · commande=%s · code de sortie=%s · début=%s · fin=%s · durée murale=%s · CPU mesuré=%s · coût=%s · tokens=%s · fraîcheur=%s\n"), c.ID, c.Attempt, c.Result, c.Execution, c.Revision, c.CandidateSHA, command, exit, c.Started, c.Finished, wall, cpu, c.Cost, c.Tokens, c.Freshness)
	}
	fmt.Fprintf(&b, uiText("Acceptation : %s · révision : %s · date : %s\n"), e.Acceptance.State, e.Acceptance.Revision, e.Acceptance.At)
	fmt.Fprintln(&b, uiText("Historique des contrôles (les échecs restent conservés) :"))
	for _, c := range e.Controls.History {
		fmt.Fprintf(&b, "- %s · %s · %s\n", c.Attempt, c.ID, c.Result)
	}
	fmt.Fprintln(&b, uiText("Top cinq des contrôles les plus longs (durées murales mesurées uniquement) :"))
	for _, c := range e.Controls.Longest {
		fmt.Fprintf(&b, "- %s · %s · %d ms\n", c.Attempt, c.ID, *c.WallDurationMS)
	}
	for _, limit := range append(append([]string{}, e.ReportReview.Limits...), e.Limits...) {
		fmt.Fprintf(&b, uiText("Limite : %s\n"), uiEngineText(limit))
	}
	return strings.TrimSpace(b.String())
}

// Surface verified receipts even when the independent opinion rejected the result.
// This read-only projection never creates a gate or an acceptance.
func (s *Store) reviewReceiptEvidence(t *Task) *AutomaticValidation {
	r := t.IndependentReview
	if r == nil || r.Receipt == "" || r.ReceiptDigest == "" || r.CandidateSHA == "" {
		return nil
	}
	path, e := localFile(s.root, r.Receipt)
	if e != nil {
		return nil
	}
	f, e := os.Open(path)
	if e != nil {
		return nil
	}
	defer f.Close()
	raw, e := io.ReadAll(io.LimitReader(f, 1<<20+1))
	if e != nil || len(raw) > 1<<20 || hash(raw) != r.ReceiptDigest {
		return nil
	}
	var receipt struct {
		Candidate string                               `json:"candidate_commit"`
		Attempt   string                               `json:"attempt_id"`
		Producer  string                               `json:"agent_id"`
		Controls  map[string][]ValidationControlResult `json:"controls"`
	}
	if json.Unmarshal(raw, &receipt) != nil || receipt.Candidate != r.CandidateSHA || receipt.Attempt != r.Attempt || receipt.Producer != r.Producer {
		return nil
	}
	controls := receipt.Controls[t.ID]
	if len(controls) == 0 {
		return nil
	}
	return &AutomaticValidation{Attempt: r.Attempt, CandidateSHA: r.CandidateSHA, Producer: r.Producer, Controls: controls, At: r.Started}
}
