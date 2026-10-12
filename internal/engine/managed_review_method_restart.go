//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

type preparedReviewMethodRestart struct {
	Revision int
	Review   string
	Producer string
	Workflow string
	Provider string
	Model    string
	Preview  ReviewRecovery
}

func (s *Store) fragmentReviewMethodChanged(r *IndependentReview) (bool, string, error) {
	if r == nil || r.FragmentJournal == nil {
		return false, "", nil
	}
	workflow, prompt, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		return false, "", err
	}
	raw, _ := json.Marshal([]any{workflow, prompt})
	digest := hash(raw)
	if r.FragmentJournal.WorkflowDigest != digest {
		return true, digest, nil
	}
	// Completed historical opinions keep their own wire contract. Only an
	// interrupted review can adopt a corrected citation contract, by a fresh
	// explicit full review. Read immutable bytes only: this check also runs
	// inside a mutation and must not acquire another database connection.
	if r.State != "error" || r.FragmentJournal.FinalJournalDigest != "" {
		return false, digest, nil
	}
	path, err := safeReport(s.root, r.FragmentJournal.Plan)
	if err != nil {
		return false, digest, err
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return false, digest, err
	}
	if r.FragmentJournal.PlanDigest == "" || hash(data) != r.FragmentJournal.PlanDigest {
		return false, digest, fmt.Errorf("plan original de contrat de citation modifié")
	}
	var plan managedReviewFragmentPlan
	if err := strict(data, &plan); err != nil {
		return false, digest, err
	}
	if plan.Candidate != r.CandidateSHA || plan.ContextDigest != r.ContextDigest {
		return false, digest, fmt.Errorf("identité du contrat de citation différente")
	}
	changed := false
	for _, packet := range plan.Packets {
		if !supportedFragmentDefectCitationContract(packet.DefectCitationContractVersion) {
			return false, digest, fmt.Errorf("contrat de citation inconnu")
		}
		changed = changed || (packet.Version >= 2 && packet.DefectCitationContractVersion != managedFragmentDefectCitationContractVersion)
	}
	return changed, digest, nil
}

// Validate immutable old evidence and preflight a COMPLETE current review.
// Old findings are retained as history, never reused under the new method.
// This read-only preparation cannot reserve a call or change an acceptance.
func (s *Store) prepareReviewMethodRestart(w Work, task *Task) (*preparedReviewMethodRestart, error) {
	r := task.IndependentReview
	changed, digest, err := s.fragmentReviewMethodChanged(r)
	if err != nil || !changed {
		return nil, err
	}
	if w.Planning == nil || w.Planning.Repository == nil || w.Planning.Reviewer == nil || task.Status != "blocked" || r.State != "error" || !currentTaskAttempt(task, r.Attempt) || r.Contract != reviewContract(task) || r.CandidateSHA == "" || r.PreviousCandidate != w.Planning.Repository.Candidate || r.FragmentJournal.FinalJournal != "" || r.FragmentJournal.FinalJournalDigest != "" {
		return nil, fmt.Errorf("remplacement de méthode réservé à une revue interrompue du candidat courant, sans décision finale")
	}
	if err = s.managedBatchProviderIntact(w.Planning.Reviewer, *r); err != nil {
		return nil, err
	}
	modelRaw, _ := json.Marshal(w.Planning.Reviewer.ModelRoute)
	if hash(modelRaw) != r.FragmentJournal.ModelConfigDigest {
		return nil, fmt.Errorf("modèle modifié : utiliser son parcours de remplacement explicite")
	}
	_, j, err := s.readFragmentJournalAnchorVersion(*r, true)
	if err != nil {
		return nil, err
	}
	for _, entry := range j.Entries {
		if entry.State != "interrupted" && entry.State != "inspected" && entry.State != "unknown" {
			return nil, fmt.Errorf("réservation active ou refus sémantique : changement de méthode refusé")
		}
	}
	for path, digest := range map[string]string{r.Receipt: r.ReceiptDigest, r.Report: r.Digest} {
		name, err := safeReport(s.root, path)
		if err != nil {
			return nil, err
		}
		raw, err := os.ReadFile(name)
		if err != nil || digest == "" || hash(raw) != digest {
			return nil, fmt.Errorf("preuve originale de reprise modifiée")
		}
	}
	a, err := s.agent(r.Producer)
	if err != nil {
		return nil, err
	}
	if a.WorkID != w.ID || a.TaskID != task.ID || a.Attempt != r.Attempt || !recoveryProcessEnded(a, hostIdentity(), processStamp(a.Child)) {
		return nil, fmt.Errorf("producteur non attribué ou processus encore actif")
	}
	receipt, err := os.ReadFile(filepath.Join(s.root, r.Receipt))
	if err != nil {
		return nil, err
	}
	c, err := s.managedReviewContext(w, a, r.CandidateSHA, receipt)
	if err != nil {
		return nil, err
	}
	cfg := w.Planning.Reviewer
	timeout, err := reviewTimeoutSeconds(cfg)
	if err != nil {
		return nil, err
	}
	fresh, err := s.planCandidateFragments(w, a, c, cfg.MaxCalls-cfg.Calls)
	if err != nil {
		return nil, err
	}
	_, method, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		return nil, err
	}
	if err = preflightManagedFragmentCalls(managedReviewPrefix(method), c, fresh); err != nil {
		return nil, err
	}
	if err = s.providerCooldownGuard(cfg.Provider); err != nil {
		return nil, err
	}
	required := fragmentPaidInspections(fresh) + fresh.ReservedFinalCalls
	available := max(0, cfg.MaxCalls-cfg.Calls)
	if required > available {
		return nil, fmt.Errorf("budget insuffisant pour une revue complète sous la nouvelle méthode, sans remboursement")
	}
	v := ReviewRecovery{Revision: w.Revision, Task: task.ID, Review: r.ID, Candidate: r.CandidateSHA, State: "method_restart_ready", Cause: r.Reason, Actor: "operator", Inspections: fragmentPaidInspections(fresh), Required: required, Available: available, MinimumLimit: cfg.Calls + required, Timeout: timeout, Next: "Méthode ou contrat de citation modifié : nouvelle revue complète du même candidat ; anciens avis et appels conservés, aucune observation réutilisée."}
	return &preparedReviewMethodRestart{Revision: w.Revision, Review: r.ID, Producer: r.Producer, Workflow: digest, Provider: cfg.ProviderDigest, Model: hash(modelRaw), Preview: v}, nil
}

func (s *Store) applyReviewMethodRestart(w *Work, task *Task, prepared *preparedReviewMethodRestart, reason string) error {
	if prepared == nil || w.Revision != prepared.Revision || task.IndependentReview == nil || task.IndependentReview.ID != prepared.Review || task.IndependentReview.State != "error" {
		return fmt.Errorf("précontrôle de changement de méthode remplacé ; relire le travail")
	}
	changed, digest, err := s.fragmentReviewMethodChanged(task.IndependentReview)
	if err != nil {
		return err
	}
	modelRaw, _ := json.Marshal(w.Planning.Reviewer.ModelRoute)
	if !changed || digest != prepared.Workflow || w.Planning.Reviewer.ProviderDigest != prepared.Provider || hash(modelRaw) != prepared.Model {
		return fmt.Errorf("méthode ou fournisseur modifié depuis le précontrôle")
	}
	task.PreviousReviews = append(task.PreviousReviews, *task.IndependentReview)
	task.IndependentReview = nil
	task.PlanningRetry = false
	task.Next = reason
	w.Planning.Reviewer.Failure = ""
	return nil
}
