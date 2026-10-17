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
	if r == nil {
		return false, "", nil
	}
	workflow, prompt, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		return false, "", err
	}
	raw, _ := json.Marshal([]any{workflow, prompt})
	digest := hash(raw)
	if r.State == "error" && r.FailureCode == reviewFragmentProtocolError && r.Context != "" {
		path, err := safeReport(s.root, r.Context)
		if err != nil {
			return false, digest, err
		}
		data, err := os.ReadFile(path)
		if err != nil || hash(data) != r.ContextDigest {
			return false, digest, fmt.Errorf("contexte original du protocole modifié")
		}
		var context managedReviewContext
		if err = strict(data, &context); err != nil {
			return false, digest, err
		}
		if context.CitationContractVersion == managedReviewCitationContractVersion {
			return false, digest, nil
		}
	}
	if r.FragmentJournal == nil {
		if r.State != "error" || r.FailureCode != reviewFragmentProtocolError || r.CandidateSHA == "" || r.Context == "" {
			return false, digest, nil
		}
		path, err := safeReport(s.root, r.Context)
		if err != nil {
			return false, digest, err
		}
		data, err := os.ReadFile(path)
		if err != nil || hash(data) != r.ContextDigest {
			return false, digest, fmt.Errorf("contexte original de méthode modifié")
		}
		var context managedReviewContext
		if err = strict(data, &context); err != nil {
			return false, digest, err
		}
		if context.CitationContractVersion < 0 || context.CitationContractVersion > managedReviewCitationContractVersion {
			return false, digest, fmt.Errorf("contrat de citation inconnu")
		}
		return context.CitationContractVersion != managedReviewCitationContractVersion || (r.Workflow != nil && r.Workflow.SHA256 != workflow.SHA256), digest, nil
	}
	if r.FragmentJournal.WorkflowDigest != digest {
		return true, digest, nil
	}
	// Completed historical opinions keep their own wire contract. Only an
	// interrupted review can adopt a corrected citation contract, by a fresh
	// explicit full review. Read immutable bytes only: this check also runs
	// inside a mutation and must not acquire another database connection.
	if r.State != "error" {
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
	contextPath, err := safeReport(s.root, r.Context)
	if err != nil {
		return false, digest, err
	}
	contextRaw, err := os.ReadFile(contextPath)
	if err != nil || hash(contextRaw) != r.ContextDigest {
		return false, digest, fmt.Errorf("contexte original de contrat modifié")
	}
	var context managedReviewContext
	if err = strict(contextRaw, &context); err != nil {
		return false, digest, err
	}
	if context.CitationContractVersion < 0 || context.CitationContractVersion > managedReviewCitationContractVersion {
		return false, digest, fmt.Errorf("contrat de citation inconnu")
	}
	changed := context.CitationContractVersion != managedReviewCitationContractVersion
	for _, packet := range plan.Packets {
		if !supportedFragmentInspectionContract(packet.InspectionContractVersion) {
			return false, digest, fmt.Errorf("contrat d’inspection inconnu")
		}
		changed = changed || packet.InspectionContractVersion != managedFragmentInspectionContractVersion
		if !supportedFragmentDefectCitationContract(packet.DefectCitationContractVersion) {
			return false, digest, fmt.Errorf("contrat de citation inconnu")
		}
		if !supportedFragmentFindingContract(packet.FindingContractVersion) {
			return false, digest, fmt.Errorf("contrat de constat inconnu")
		}
		changed = changed || (packet.Version >= 2 && packet.DefectCitationContractVersion != managedFragmentDefectCitationContractVersion)
		changed = changed || (packet.Version >= 2 && packet.FindingContractVersion != managedFragmentFindingContractVersion)
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
	if w.Planning == nil || w.Planning.Repository == nil || w.Planning.Reviewer == nil || task.Status != "blocked" || r.State != "error" || !currentTaskAttempt(task, r.Attempt) || r.Contract != reviewContract(task) || r.CandidateSHA == "" || r.PreviousCandidate != w.Planning.Repository.Candidate {
		return nil, fmt.Errorf("remplacement de méthode réservé à une revue interrompue du candidat courant, sans décision finale")
	}
	modelRaw, _ := json.Marshal(w.Planning.Reviewer.ModelRoute)
	if r.FragmentJournal != nil {
		if err = s.managedBatchProviderIntact(w.Planning.Reviewer, *r); err != nil {
			return nil, err
		}
		if hash(modelRaw) != r.FragmentJournal.ModelConfigDigest {
			return nil, fmt.Errorf("modèle modifié : utiliser son parcours de remplacement explicite")
		}
		p, j, err := s.readFragmentJournalAnchorVersion(*r, true)
		if err != nil {
			return nil, err
		}
		if err = s.validateManagedFinalRestartArchive(*r, p, j); err != nil {
			return nil, err
		}
		for _, entry := range j.Entries {
			if entry.State != "interrupted" && entry.State != "inspected" && entry.State != "unknown" {
				return nil, fmt.Errorf("réservation active ou refus sémantique : changement de méthode refusé")
			}
		}
	} else {
		if err = s.managedReviewFilesIntact(*r); err != nil {
			return nil, err
		}
		if r.ReplyPath != "" {
			path, err := safeReport(s.root, r.ReplyPath)
			if err != nil {
				return nil, err
			}
			raw, err := os.ReadFile(path)
			if err != nil || hash(raw) != r.ReplyDigest {
				return nil, fmt.Errorf("réponse originale de méthode modifiée")
			}
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
	_, method, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		return nil, err
	}
	prefix := managedReviewPrefixForContext(method, c)
	required, inspections := 0, 0
	if task.ReviewCoordination == nil {
		owners, err := managedReviewSourceOwners(w, c)
		if err != nil {
			return nil, err
		}
		batches, batchErr := planManagedReviewBatches(prefix, c, owners)
		if batchErr == nil {
			required = len(batches)
		}
	}
	if required == 0 {
		fresh, err := s.planCandidateFragments(w, a, c, cfg.MaxCalls)
		if err != nil {
			return nil, err
		}
		if err = preflightManagedFragmentCalls(prefix, c, fresh); err != nil {
			return nil, err
		}
		inspections = fragmentPaidInspections(fresh)
		required = inspections + fresh.ReservedFinalCalls
	}

	if err = s.providerCooldownGuard(cfg.Provider); err != nil {
		return nil, err
	}
	available := max(0, cfg.MaxCalls-cfg.Calls)

	v := ReviewRecovery{Missing: max(0, required-available), Revision: w.Revision, Task: task.ID, Review: r.ID, Candidate: r.CandidateSHA, State: "method_restart_ready", Cause: r.Reason, Actor: "operator", Inspections: inspections, Required: required, Available: available, MinimumLimit: cfg.Calls + required, Timeout: timeout, Next: "Méthode ou contrat de citation modifié : nouvelle revue complète du même candidat ; anciens avis et appels conservés, aucune observation réutilisée."}
	if v.Missing > 0 {
		v.State = "budget_authorization_required"
		v.Next = fmt.Sprintf("Revue complète sous le nouveau contrat : %d appels nécessaires, %d disponibles ; plafond minimal %d, sans remboursement ni appel lors de cet aperçu.", required, available, v.MinimumLimit)
	}
	return &preparedReviewMethodRestart{Revision: w.Revision, Review: r.ID, Producer: r.Producer, Workflow: digest, Provider: cfg.ProviderDigest, Model: hash(modelRaw), Preview: v}, nil
}

func (s *Store) applyReviewMethodRestart(w *Work, task *Task, prepared *preparedReviewMethodRestart, reason string) error {
	if prepared == nil || w.Revision != prepared.Revision || task.IndependentReview == nil || task.IndependentReview.ID != prepared.Review || task.IndependentReview.State != "error" {
		return fmt.Errorf("précontrôle de changement de méthode remplacé ; relire le travail")
	}
	if prepared.Preview.Missing > 0 {
		return fmt.Errorf("%s", prepared.Preview.Next)
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
	task.BatchReviewResume = nil
	task.PlanningRetry = false
	task.Next = reason
	w.Planning.Reviewer.Failure = ""
	return nil
}
