//go:build linux

package main

import (
	"encoding/json"
	"fmt"
)

// A model change is an explicit operator decision, not permission to reuse the
// former reviewer's opinions. Restart the review, retaining all spent calls and
// immutable evidence. The normal integration path reruns candidate checks.
func fragmentReviewerChanged(w Work, r IndependentReview) bool {
	if r.FragmentJournal == nil || w.Planning == nil || w.Planning.Reviewer == nil {
		return false
	}
	cfg := w.Planning.Reviewer
	raw, _ := json.Marshal(cfg.ModelRoute)
	return cfg.ProviderDigest != r.BatchProviderDigest || hash(raw) != r.FragmentJournal.ModelConfigDigest
}

func (s *Store) fragmentReplacementPreview(w Work, task *Task) (ReviewRecovery, error) {
	r := task.IndependentReview
	cfg := w.Planning.Reviewer
	v := ReviewRecovery{}
	if r == nil || r.State != "error" || !fragmentReviewerChanged(w, *r) || cfg.ModelSelection == nil || cfg.ModelSelection.Provider != cfg.Provider || cfg.ModelSelection.ProviderDigest != cfg.ProviderDigest {
		return v, fmt.Errorf("remplacement du vérificateur non autorisé ou revue non interrompue")
	}
	if w.Planning.Repository == nil || r.Contract != reviewContract(task) || !currentTaskAttempt(task, r.Attempt) || r.PreviousCandidate != w.Planning.Repository.Candidate {
		return v, fmt.Errorf("candidat, contrat ou tentative remplacé")
	}
	if err := s.managedReviewFilesIntact(*r); err != nil {
		return v, err
	}
	p, j, err := s.readFragmentJournalAnchor(*r)
	if err != nil {
		return v, err
	}
	if r.FragmentJournal.FinalJournalDigest != "" || r.FragmentJournal.FinalJournal != "" {
		return v, fmt.Errorf("décision finale déjà engagée : examiner son journal avant remplacement")
	}
	for _, entry := range j.Entries {
		if entry.State != "interrupted" && entry.State != "inspected" && entry.State != "unknown" {
			return v, fmt.Errorf("avis défavorable ou réservation active : remplacement refusé")
		}
	}
	// Validate the new provider configuration without assigning old evidence to it.
	fresh := *r
	fresh.BatchProviderDigest = cfg.ProviderDigest
	if err = s.managedBatchProviderIntact(cfg, fresh); err != nil {
		return v, err
	}
	timeout, err := reviewTimeoutSeconds(cfg)
	if err != nil {
		return v, err
	}
	required := len(p.Packets) + p.ReservedFinalCalls
	available := max(0, cfg.MaxCalls-cfg.Calls)
	v = ReviewRecovery{Revision: w.Revision, Task: task.ID, Review: r.ID, Candidate: r.CandidateSHA, State: "reviewer_replacement_ready", Cause: r.Reason, Actor: "operator", Inspections: len(p.Packets), Required: required, Available: available, Missing: max(0, required-available), MinimumLimit: cfg.Calls + required, Timeout: timeout, Next: "Nouvelle revue avec le vérificateur choisi ; anciennes inspections et dépenses conservées dans l’historique, sans réutilisation des avis."}
	if v.Missing > 0 {
		v.State = "budget_authorization_required"
		v.Next = fmt.Sprintf("%d appels nécessaires, %d disponibles : plafond minimal %d à autoriser pour le nouveau vérificateur.", required, available, v.MinimumLimit)
	}
	return v, nil
}
