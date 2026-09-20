package main

import (
	"fmt"
	"strings"
)

// ResultPresentation is the single read-time projection used by the CLI and
// the web UI. It deliberately keeps process exit, report presence and delivery
// validation separate: none of these facts is a substitute for another one.
type ResultPresentation struct {
	ProcessLabel    string `json:"process_label"`
	ReportLabel     string `json:"report_label"`
	ValidationLabel string `json:"validation_label"`
	State           string `json:"state"`
	Label           string `json:"label"`
	ProcessState    string `json:"process_state"`
	ReportState     string `json:"report_state"`
	ValidationState string `json:"validation_state"`
	Reason          string `json:"reason"`
	NextStep        string `json:"next_step"`
	AttemptID       string `json:"attempt_id,omitempty"`
	ReportID        string `json:"report_id,omitempty"`
	GateID          string `json:"gate_id,omitempty"`
	ReceiptID       string `json:"receipt_id,omitempty"`
}

func submittedReport(t *Task) string {
	if t.Revalidation != nil && strings.TrimSpace(t.Revalidation.Report) != "" {
		return strings.TrimSpace(t.Revalidation.Report)
	}
	const marker = "handoff : "
	if at := strings.LastIndex(t.Next, marker); at >= 0 {
		return strings.TrimSpace(t.Next[at+len(marker):])
	}
	return ""
}

func latestTaskAgent(agents []Agent, task string) *Agent {
	for i := range agents { // Store.agents returns newest first.
		if agents[i].TaskID == task {
			return &agents[i]
		}
	}
	return nil
}

func (s *Store) resultPresentation(w *Work, t *Task, agents []Agent, validation TaskValidation) (p ResultPresentation) {
	defer func() {
		p.ProcessLabel = resultFactLabel(p.ProcessState)
		p.ReportLabel = resultFactLabel(p.ReportState)
		p.ValidationLabel = resultFactLabel(p.ValidationState)
	}()
	p = ResultPresentation{
		State: "not_started", Label: "Pas encore commencé", ProcessState: "not_started",
		ReportState: "absent", ValidationState: "not_started",
		Reason:   "Aucune tentative terminée ni validation actuelle n’est établie.",
		NextStep: "Lancer la tâche lorsque ses conditions sont réunies.",
	}
	a := latestTaskAgent(agents, t.ID)
	if a != nil {
		p.AttemptID = a.Attempt
		if p.AttemptID == "" {
			p.AttemptID = a.ID
		}
		p.ProcessState = observedAgent(*a)
	}
	p.ReportID = submittedReport(t)
	if p.ReportID != "" || t.Status == "submitted" || t.Status == "accepted" {
		p.ReportState = "submitted"
	}
	if t.Gate != nil {
		p.GateID = strings.TrimSpace(t.Gate.Name)
		if p.GateID == "" {
			p.GateID = t.Gate.Evaluation.Scope
		}
	}
	if t.AutoValidation != nil {
		p.ReceiptID = t.AutoValidation.Receipt
	}

	// A current acceptance is the only positive delivery verdict.
	if t.Status == "accepted" && validation.Fresh {
		p.State, p.Label = "validated", "Validé"
		p.ValidationState = "fresh"
		p.Reason = "Gate delivery et preuves actuelles ; la tâche est acceptée."
		p.NextStep = "Consulter le résultat ; revalider si une preuve change."
		return p
	}
	if t.Status == "accepted" && !validation.Fresh {
		p.State, p.Label = "validation_stale", "Validation à refaire"
		p.ValidationState = "stale"
		p.Reason = "L’acceptation historique n’est plus fondée sur des preuves actuelles."
		if len(validation.Blockers) > 0 {
			p.Reason = validation.Blockers[0]
		}
		p.NextStep = "Rouvrir la tâche, renouveler les preuves et enregistrer une nouvelle revue de validation."
		return p
	}
	if t.Status == "waived" {
		p.State, p.Label = "waived", "Dérogation — non validé"
		p.ValidationState = "waived"
		p.Reason = "Une décision explicite permet la suite sans transformer les contrôles en PASS."
		p.NextStep = "Consulter le motif et la portée de la dérogation."
		return p
	}
	if t.Status == "abandoned" {
		p.State, p.Label = "abandoned", "Abandonné — non validé"
		p.ValidationState = "abandoned"
		p.Reason = "La tâche est abandonnée ; aucun résultat validé n’est revendiqué."
		p.NextStep = "Consulter la décision d’abandon."
		return p
	}

	if a != nil && activeAgent(*a) {
		switch p.ProcessState {
		case "running":
			p.State, p.Label = "in_progress", "En cours"
			p.Reason = "Le processus est actif ; aucun résultat final n’est encore affirmé."
			p.NextStep = "Voir la session ou attendre la fin de la tentative."
		default:
			p.State, p.Label = "execution_unconfirmed", "État à vérifier"
			p.Reason = "La tentative est enregistrée, mais son exécution n’est pas confirmée."
			p.NextStep = "Réconcilier la tentative avant toute relance."
		}
		if reports := s.taskReports(t.ID); len(reports) > 0 {
			p.ReportState = "early_unverified"
			p.ReportID = reports[0]
			p.Reason += " Un rapport existe déjà, mais il peut être précoce ou incomplet."
		}
		p.ValidationState = "pending_process"
		return p
	}

	if a != nil && (a.Status == "failed" || a.Status == "interrupted") {
		p.State, p.Label = "stopped_early", "Arrêté avant la fin"
		p.ValidationState = "not_validated"
		p.Reason = strings.TrimSpace(a.Activity)
		if p.Reason == "" {
			if a.Status == "failed" {
				p.Reason = "Le processus s’est terminé en échec."
			} else {
				p.Reason = "Le processus a été interrompu."
			}
		}
		if reports := s.taskReports(t.ID); len(reports) > 0 && p.ReportState == "absent" {
			p.ReportState, p.ReportID = "partial_possible", reports[0]
			p.Reason += " Un rapport existe, mais l’arrêt ne permet pas d’en affirmer la complétude."
		}
		p.NextStep = "Examiner le motif, les traces et le rapport éventuel avant de décider d’une reprise."
		return p
	}
	if a != nil && a.Status == "completed" && s.managedReviewPresentation(w, t, a, &p) {
		return p
	}

	if t.Status == "submitted" {
		if a == nil {
			p.ProcessState = "unknown"
		}
		p.ValidationState = "missing_gate"
		p.State, p.Label = "result_to_review", "Résultat à vérifier"
		p.Reason = "Un rapport est soumis ; sa complétude et les critères restent à examiner. Aucune revue de validation n’est enregistrée."
		p.NextStep = "Lire le rapport, vérifier les critères et enregistrer la revue de validation."
		if t.Gate != nil {
			if !s.validGate(t) || len(validation.Blockers) > 0 {
				p.State, p.Label = "validation_withheld", "Validation retenue"
				p.ValidationState = "failed_or_stale"
				p.Reason = "La revue de validation n’autorise pas une validation actuelle."
				if len(validation.Blockers) > 0 {
					p.Reason = validation.Blockers[0]
				}
				p.NextStep = "Corriger ou renouveler les preuves, puis enregistrer une revue de validation actuelle."
			} else {
				p.ValidationState = "ready_for_decision"
				p.Reason = "La revue de validation et ses preuves sont actuelles ; la décision d’acceptation reste à enregistrer."
				p.NextStep = "Examiner le rapport puis enregistrer la décision de validation."
			}
		}
		return p
	}

	if a != nil && a.Status == "completed" {
		p.ProcessState = "completed"
		p.ValidationState = "not_started"
		report, reportReason := s.provenReport(t.ID, a.Started)
		if report != "" {
			p.State, p.Label = "result_to_review", "Résultat à vérifier"
			p.ReportState, p.ReportID = "attributable_unsubmitted", report
			p.Reason = "Le processus est terminé et un rapport lui est attribuable ; sa complétude n’est pas démontrée."
			p.NextStep = "Soumettre le rapport, vérifier les critères et enregistrer la revue de validation."
			return p
		}
		p.State, p.Label = "completed_unproven", "Terminé, résultat non démontré"
		p.ReportState = "absent_or_unattributed"
		p.Reason = "Le processus est terminé normalement, mais " + reportReason + "."
		if strings.TrimSpace(a.Relay) != "" {
			p.Reason = a.Relay
		}
		p.NextStep = "Désigner ou compléter un rapport avant toute validation ou relance."
		return p
	}

	if t.Gate != nil && !s.validGate(t) {
		p.ValidationState = "failed_or_stale"
	}
	return p
}

// Managed reports live beside the candidate receipt, not in the host's docs/.
// Present the current review without authorizing a retry or an acceptance.
func (s *Store) managedReviewPresentation(w *Work, t *Task, a *Agent, p *ResultPresentation) bool {
	r := t.IndependentReview
	if w.Planning == nil || w.Planning.Repository == nil || r == nil || r.CandidateSHA == "" ||
		!currentTaskAttempt(t, r.Attempt) || r.Attempt != a.Attempt || r.Producer != a.ID {
		return false
	}
	p.ReportID, p.ReceiptID = r.Report, r.Receipt
	p.ReportState = "submitted"
	p.State, p.Label, p.ValidationState = "review_blocked", reviewStateLabel(r.State), "review_unvalidated"
	p.Reason = fmt.Sprintf("Revue indépendante : %s", r.Reason)
	if r.State == "error" && strings.HasPrefix(r.Reason, "délai du planificateur dépassé") {
		p.Reason = "Le vérificateur indépendant n’a pas répondu dans le délai imparti."
	}
	p.NextStep = "Examiner le rapport et le motif de la revue avant de choisir une correction ou une reprise autorisée."
	if _, err := s.artifactDigest(ExchangeArtifact{Path: r.Report, SHA256: r.Digest}); err != nil {
		p.ReportState = "stale"
		p.ValidationState = "failed_or_stale"
		p.Label = "Preuves à renouveler"
		p.Reason = "Le rapport transmis au vérificateur est indisponible ou modifié ; ses preuves doivent être réexaminées."
		return true
	}
	if r.Contract != reviewContract(t) || r.PreviousCandidate != w.Planning.Repository.Candidate {
		p.ValidationState = "failed_or_stale"
		p.Label = "Avis périmé"
		p.Reason = "Le contrat ou la révision de base a changé depuis cette revue."
		return true
	}
	switch r.State {
	case "stale":
		p.ValidationState = "failed_or_stale"
	case "changes_requested":
		p.ValidationState = "review_changes_requested"
	case "running":
		p.State, p.ValidationState = "review_in_progress", "pending_review"
		p.Reason = "Le rapport est conservé ; le vérificateur indépendant examine le candidat testé."
		p.NextStep = "Attendre l’avis indépendant ; le résultat n’est pas encore accepté."
	case "passed":
		p.State, p.ValidationState = "review_awaiting_publication", "pending_publication"
		p.Reason = "Un avis favorable est enregistré ; la publication du candidat reste à confirmer par le moteur."
		p.NextStep = "Attendre la décision du moteur sur les preuves actuelles."
	}
	return true
}

func resultFactLabel(state string) string {
	labels := map[string]string{
		"pending_review": "Revue indépendante en cours", "pending_publication": "Publication non confirmée",
		"review_unvalidated": "Avis indépendant non obtenu", "review_changes_requested": "Avis indépendant défavorable ou incomplet",
		"not_started": "Pas encore commencé", "absent": "Aucun rapport soumis", "unknown": "Exécution non documentée", "running": "Agent actif", "completed": "Processus terminé", "failed": "Processus en échec", "interrupted": "Processus interrompu", "queued": "Démarrage en attente", "starting": "Démarrage en cours", "stopping": "Arrêt en cours", "submitted": "Rapport soumis, contenu à vérifier", "fresh": "Preuves actuelles", "stale": "Preuves à renouveler", "waived": "Dérogation, contrôles non validés", "abandoned": "Abandonné", "early_unverified": "Rapport détecté, peut-être incomplet", "pending_process": "En attente de la fin du processus", "not_validated": "Non validé", "partial_possible": "Rapport présent, peut-être partiel", "missing_gate": "Contrôles de validation non enregistrés", "failed_or_stale": "Contrôles en échec ou preuves périmées", "ready_for_decision": "Contrôles actuels, décision à enregistrer", "attributable_unsubmitted": "Rapport détecté, à soumettre et vérifier", "absent_or_unattributed": "Rapport absent ou non attribuable",
	}
	if label, ok := labels[state]; ok {
		return label
	}
	return "État non confirmé"
}
