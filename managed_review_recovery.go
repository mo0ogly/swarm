//go:build linux

package main

import "fmt"

// Recovery is computed from verified durable evidence, never from a report label.
// This preview has no side effects and never calls a provider.
type ReviewRecovery struct {
	Revision     int    `json:"revision"`
	Task         string `json:"task_id"`
	Review       string `json:"review_id"`
	Candidate    string `json:"candidate_commit"`
	State        string `json:"state"`
	Cause        string `json:"cause"`
	Next         string `json:"next_step"`
	Actor        string `json:"actor"`
	Inspections  int    `json:"inspections_remaining"`
	Reusable     int    `json:"inspections_reusable"`
	Required     int    `json:"calls_required"`
	Available    int    `json:"calls_available"`
	Missing      int    `json:"calls_missing"`
	MinimumLimit int    `json:"minimum_review_limit"`
	Timeout      int    `json:"timeout_seconds"`
}

func fragmentRecoveryBudget(p managedReviewFragmentPlan, j managedFragmentJournal, calls, limit int) (ReviewRecovery, error) {
	reusable, err := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
	if err != nil {
		return ReviewRecovery{}, err
	}
	v := ReviewRecovery{Reusable: len(reusable), Inspections: len(p.Packets) - len(reusable), Available: max(0, limit-calls)}
	v.Required = v.Inspections + p.ReservedFinalCalls
	v.Missing = max(0, v.Required-v.Available)
	v.MinimumLimit = calls + v.Required
	return v, nil
}

func (s *Store) reviewRecoveryPreview(work, id string) (ReviewRecovery, error) {
	w, err := s.get(work)
	if err != nil {
		return ReviewRecovery{}, err
	}
	task, err := w.task(id)
	if err != nil {
		return ReviewRecovery{}, err
	}
	r := task.IndependentReview
	if r == nil || r.FragmentJournal == nil || w.Planning == nil || w.Planning.Reviewer == nil {
		return ReviewRecovery{}, fmt.Errorf("reprise de revue fragmentée indisponible pour cette tâche")
	}
	if r.State != "error" {
		return ReviewRecovery{}, fmt.Errorf("la revue doit être interrompue avant de préparer sa reprise")
	}
	if err = s.managedBatchPlanIntact(w, *r); err != nil {
		return ReviewRecovery{}, err
	}
	if err = s.managedReviewFilesIntact(*r); err != nil {
		return ReviewRecovery{}, err
	}
	p, j, err := s.readFragmentJournalAnchor(*r)
	if err != nil {
		return ReviewRecovery{}, err
	}
	// Final recovery and repartition have different contracts; do not misquote them.
	if r.FragmentJournal.FinalJournalDigest != "" {
		return ReviewRecovery{}, fmt.Errorf("décision finale déjà engagée : examiner son journal avant reprise")
	}
	for _, packet := range p.Packets {
		if !managedFragmentReplyFits(packet) {
			return ReviewRecovery{}, fmt.Errorf("redécoupage requis avant estimation de reprise")
		}
	}
	for _, entry := range j.Entries {
		if entry.State != "interrupted" && entry.State != "inspected" {
			return ReviewRecovery{}, fmt.Errorf("inspection non récupérable : %s", entry.State)
		}
	}
	cfg := w.Planning.Reviewer
	v, err := fragmentRecoveryBudget(p, j, cfg.Calls, cfg.MaxCalls)
	if err != nil {
		return v, err
	}
	v.Timeout, err = reviewTimeoutSeconds(cfg)
	if err != nil {
		return v, err
	}
	v.Revision = w.Revision
	v.Task = id
	v.Review = r.ID
	v.Candidate = r.CandidateSHA
	v.Cause = r.Reason
	v.Actor = "operator"
	v.State = "change_required"
	v.Next = "Examiner la cause et enregistrer une correction pertinente avant reprise ; aucune relance automatique identique."
	if v.Missing > 0 {
		v.State = "budget_authorization_required"
		v.Next = fmt.Sprintf("%d appels nécessaires, %d disponibles : autorisation explicite du plafond minimal %d requise. Aucun appel lancé.", v.Required, v.Available, v.MinimumLimit)
	}
	return v, nil
}
