package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

// The reviewer is a separate, tool-free process. Its opinion never replaces
// deterministic controls or an explicitly required human acceptance.
type ReviewerConfig struct {
	ModelSelection *RoleModel  `json:"model_selection,omitempty"`
	TimeoutSeconds int         `json:"timeout_seconds,omitempty"`
	Failure        string      `json:"failure,omitempty"`
	Provider       string      `json:"provider"`
	ProviderDigest string      `json:"provider_digest"`
	ModelRoute     *ModelRoute `json:"model_route,omitempty"`
	MaxCalls       int         `json:"max_calls"`
	Calls          int         `json:"calls"`
	Authorized     string      `json:"authorized"`
}
type IndependentReview struct {
	ReportArtifacts     map[string]string             `json:"report_artifacts,omitempty"`
	FragmentJournal     *ManagedFragmentJournalAnchor `json:"fragment_journal,omitempty"`
	ModelRoute          *ModelRoute                   `json:"model_route,omitempty"`
	BatchPlanDigest     string                        `json:"batch_plan_sha256,omitempty"`
	BatchProviderDigest string                        `json:"batch_provider_sha256,omitempty"`
	Batches             []ManagedReviewBatchVerdict   `json:"batches,omitempty"`
	ReplyPath           string                        `json:"reply_path,omitempty"`
	ReplyDigest         string                        `json:"reply_sha256,omitempty"`
	TimeoutSeconds      int                           `json:"timeout_seconds,omitempty"`
	Workflow            *AgentWorkflow                `json:"workflow,omitempty"`
	GitReport           string                        `json:"git_report,omitempty"`
	CandidateSHA        string                        `json:"candidate_commit,omitempty"`
	PreviousCandidate   string                        `json:"previous_candidate,omitempty"`
	Receipt             string                        `json:"receipt,omitempty"`
	ReceiptDigest       string                        `json:"receipt_sha256,omitempty"`
	Context             string                        `json:"context,omitempty"`
	ContextDigest       string                        `json:"context_sha256,omitempty"`
	ManagedTasks        []ManagedTaskReview           `json:"managed_tasks,omitempty"`
	ID                  string                        `json:"id"`
	Attempt             string                        `json:"attempt"`
	Producer            string                        `json:"producer"`
	Reviewer            string                        `json:"reviewer"`
	Report              string                        `json:"report"`
	Digest              string                        `json:"sha256"`
	Contract            string                        `json:"contract"`
	State               string                        `json:"state"`
	Reason              string                        `json:"reason"`
	Criteria            []ReviewCriterion             `json:"criteria,omitempty"`
	Started             string                        `json:"started"`
	Finished            string                        `json:"finished,omitempty"`
	Usage               *Usage                        `json:"usage,omitempty"`
}

type ManagedReviewBatchVerdict struct {
	RetryFeedback string   `json:"retry_feedback,omitempty"`
	ID            string   `json:"id"`
	Tasks         []string `json:"tasks"`
	Context       string   `json:"context"`
	ContextDigest string   `json:"context_sha256"`
	ReplyPath     string   `json:"reply_path,omitempty"`
	ReplyDigest   string   `json:"reply_sha256,omitempty"`
	State         string   `json:"state"`
	Started       string   `json:"started,omitempty"`
	Finished      string   `json:"finished,omitempty"`
}

// Zero keeps historical configurations at their original 90-second deadline.
func reviewTimeoutSeconds(cfg *ReviewerConfig) (int, error) {
	if cfg == nil {
		return 0, fmt.Errorf("vérificateur absent")
	}
	if cfg.TimeoutSeconds == 0 {
		return 90, nil
	}
	if cfg.TimeoutSeconds < 1 || cfg.TimeoutSeconds > 900 {
		return 0, fmt.Errorf("délai de revue : 1 à 900 secondes requis")
	}
	return cfg.TimeoutSeconds, nil
}

func (s *Store) setReviewTimeout(work string, r PlanningRequest) (Work, error) {
	if r.ReviewTimeoutSeconds < 1 || r.ReviewTimeoutSeconds > 900 || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return Work{}, fmt.Errorf("délai de revue : 1 à 900 secondes et motif explicite de 8 à 2000 caractères requis")
	}
	raw, _ := json.Marshal(r)
	return s.mutate(work, "review.timeout", r.EventID, r.Revision, raw, func(w *Work) error {
		if w.Planning == nil || w.Planning.Reviewer == nil {
			return fmt.Errorf("vérificateur absent")
		}
		for _, task := range w.Tasks {
			if task.IndependentReview != nil && task.IndependentReview.State == "running" {
				return fmt.Errorf("attendre la fin de la revue avant de modifier son délai")
			}
		}
		w.Planning.Reviewer.TimeoutSeconds = r.ReviewTimeoutSeconds
		return nil
	})
}

type ReviewCriterion struct {
	Index    int    `json:"index"`
	Verdict  string `json:"verdict"`
	Evidence string `json:"evidence"`
}

func reviewContract(t *Task) string {
	b, _ := json.Marshal([]any{t.Title, t.Deliverable, t.Criteria, t.Depends, t.PlanBriefHash})
	return hash(b)
}
func (s *Store) independentReviewGuard(w *Work, t *Task) error {
	return s.independentReviewGuardVersion(w, t, false)
}

func (s *Store) independentReviewGuardVersion(w *Work, t *Task, historical bool) error {
	if w.Planning != nil && w.Planning.Reviewer == nil {
		return fmt.Errorf("vérificateur indépendant manquant")
	}
	if w.Planning == nil || w.Planning.Reviewer == nil {
		return nil
	}
	r := t.IndependentReview
	if r == nil || r.State != "passed" {
		return fmt.Errorf("vérification IA indépendante requise avant acceptation")
	}
	var project *ProjectContext
	if r.Workflow != nil {
		project = r.Workflow.Project
	}
	if !historical {
		if err := s.projectContextGuard(project, "reviewer"); err != nil {
			return err
		}
	}
	if len(t.Attempts) == 0 || r.Attempt != t.Attempts[len(t.Attempts)-1].ID || r.Contract != reviewContract(t) {
		return fmt.Errorf("vérification IA périmée : tentative ou consigne modifiée")
	}
	if w.Planning.Repository != nil {
		return s.managedIndependentReviewGuardVersion(w, t, historical)
	}
	if err := s.currentReportArtifacts(r.ReportArtifacts); err != nil {
		return err
	}
	p, e := safeReport(s.root, r.Report)
	if e != nil {
		return e
	}
	b, e := os.ReadFile(p)
	if e != nil || hash(b) != r.Digest {
		return fmt.Errorf("vérification IA périmée : rapport modifié")
	}
	return nil
}
func (s *Store) reviewerConfig(provider, level string, max int) (*ReviewerConfig, error) {
	if max < 1 || max > 100 {
		return nil, fmt.Errorf("vérification : budget de 1 à 100 appels requis")
	}
	ps, e := s.providers()
	if e != nil {
		return nil, e
	}
	p, ok := ps.Providers[provider]
	if !ok {
		return nil, fmt.Errorf("fournisseur de vérification inconnu")
	}
	if _, e = assistantProvider(p); e != nil {
		return nil, e
	}
	_, route, e := resolveModel(p, level, "planning")
	if e != nil {
		return nil, e
	}
	raw, _ := json.Marshal(p)
	return &ReviewerConfig{Provider: provider, ProviderDigest: hash(raw), ModelRoute: route, MaxCalls: max, Authorized: now()}, nil
}
func (s *Store) configureReviewer(work string, r PlanningRequest) (Work, error) {
	config, e := s.reviewerConfig(r.Provider, r.Level, r.MaxActivations)
	if e != nil {
		return Work{}, e
	}
	raw, _ := json.Marshal(r)
	return s.mutate(work, "planning.configure-reviewer", r.EventID, r.Revision, raw, func(w *Work) error {
		if w.Planning == nil {
			return fmt.Errorf("responsable de mission requis")
		}
		if w.Planning.Reviewer != nil {
			return fmt.Errorf("vérificateur déjà configuré ; sa configuration est conservée")
		}
		for _, t := range w.Tasks {
			if t.Status == "running" {
				return fmt.Errorf("attendre la fin des exécutants avant configuration")
			}
		}
		w.Planning.Reviewer = config
		w.Planning.ReviewerRequired = true
		return nil
	})
}

func reviewStateLabel(state string) string {
	switch state {
	case "running":
		return "Examen en cours"
	case "passed":
		return "Avis favorable"
	case "changes_requested":
		return "Corrections ou preuves demandées"
	case "error":
		return "Vérification interrompue"
	case "stale":
		return "Avis périmé"
	}
	return "Avis indisponible"
}

// refusedReviewEvidenceChanged permits a new explicit review only after a
// successfully read, previously bound input changed. Missing files and unrelated
// repository edits are not recovery evidence. It never authorizes acceptance.
func (s *Store) refusedReviewEvidenceChanged(w *Work, t *Task, audit ...[]Event) bool {
	return s.boundReviewEvidenceChanged(w, t, "changes_requested", audit...) || s.boundReviewEvidenceChanged(w, t, "error", audit...)
}

func (s *Store) acceptedReviewRevalidationAvailable(w *Work, t *Task) bool {
	return t.Status == "accepted" && s.boundReviewEvidenceChanged(w, t, "passed")
}

func (s *Store) boundReviewEvidenceChanged(w *Work, t *Task, state string, audit ...[]Event) bool {
	r := t.IndependentReview
	// A validation-policy change archives the previous verdict. A refused,
	// completed result can still be explicitly reviewed after its bound proof
	// changes; archiving must not force another producer attempt.
	if r == nil && (state == "changes_requested" || state == "error") && t.Status == "blocked" && len(t.PreviousReviews) > 0 {
		r = &t.PreviousReviews[len(t.PreviousReviews)-1]
	}
	if w.Planning == nil || w.Planning.Repository != nil || r == nil || r.State != state || r.Finished == "" || len(t.Attempts) == 0 || r.Attempt != t.Attempts[len(t.Attempts)-1].ID || t.Attempts[len(t.Attempts)-1].Status != "completed" {
		return false
	}
	contractChanged := r.Contract != reviewContract(t)
	if contractChanged && !((state == "changes_requested" || state == "error") && t.Status == "blocked" && s.recordedContractRevisionMatches(w.ID, t, r.Contract, audit...)) {
		return false
	}
	inputs := make(map[string]string, len(r.ReportArtifacts)+1)
	for name, digest := range r.ReportArtifacts {
		inputs[name] = digest
	}
	if r.Report != "" && r.Digest != "" {
		inputs[r.Report] = r.Digest
	}
	changed := contractChanged
	for name, digest := range inputs {
		p, err := safeReport(s.root, name)
		if err != nil || digest == "" {
			return false
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return false
		}
		if hash(b) != digest {
			changed = true
		}
	}
	return changed
}

func (s *Store) retryIndependentReview(work string, r PlanningRequest) (Work, error) {
	if len(r.Reason) < 8 || len(r.Reason) > 2000 {
		return Work{}, fmt.Errorf("décrire la correction avant une nouvelle vérification (8 à 2000 caractères)")
	}
	audit, auditErr := s.events(work)
	if auditErr != nil {
		return Work{}, auditErr
	}
	raw, _ := json.Marshal(r)
	managedProducer := ""
	preflight, preflightErr := s.prepareManagedPreflightRetry(work, r.Task)
	replacement, replacementErr := s.reviewRecoveryPreview(work, r.Task)
	unpaid := false
	return s.mutateWithHook(work, "review.retry", r.EventID, r.Revision, raw, func(w *Work) error {
		if w.Planning == nil || w.Planning.Reviewer == nil {
			return fmt.Errorf("vérificateur absent")
		}
		cfg := w.Planning.Reviewer
		t, e := w.task(r.Task)
		if e != nil {
			return e
		}
		v := t.IndependentReview
		if v == nil && cfg.Failure != "" && r.ConfirmReviewErrorRepair {
			if err := s.repairedImageReviewPreflight(w, t); err != nil {
				return err
			}
			cfg.Failure = ""
			return nil
		}
		if v == nil && s.refusedReviewEvidenceChanged(w, t, audit) {
			v = &t.PreviousReviews[len(t.PreviousReviews)-1]
		}
		if cfg.Calls >= cfg.MaxCalls && !managedBatchesAllPassed(v) && !s.managedFragmentVerdictDurable(v) {
			return fmt.Errorf("budget du vérificateur atteint ; aucun appel supplémentaire autorisé")
		}
		managed := w.Planning.Repository != nil
		if managed && v == nil && t.Status == "blocked" {
			if preflightErr != nil {
				return preflightErr
			}
			if preflight == nil || preflight.Revision != w.Revision || !currentTaskAttempt(t, preflight.Agent.Attempt) {
				return fmt.Errorf("précontrôle remplacé ; relire le travail")
			}
			managedProducer = preflight.Agent.ID
			unpaid = true
			t.Next = "Dossier de revue prêt ; reprise du résultat existant sans nouvelle production."
			return nil
		}
		// Use the same freshness check as the console. A completed favorable
		// record can become stale without its persisted state changing.
		derivedStale := (v != nil && v.State == "passed" && v.Finished != "" && s.independentReviewGuard(w, t) != nil) || s.refusedReviewEvidenceChanged(w, t, audit)
		revalidating := s.acceptedReviewRevalidationAvailable(w, t)
		if (t.Status != "submitted" && !revalidating && !(t.Status == "blocked" && (managed || s.refusedReviewEvidenceChanged(w, t, audit)))) || v == nil || (v.State != "error" && v.State != "stale" && !derivedStale) {
			return fmt.Errorf("seule une vérification interrompue ou périmée d’un résultat soumis peut être reprise")
		}
		if managed {
			if len(t.Attempts) == 0 || v.Attempt != t.Attempts[len(t.Attempts)-1].ID || v.CandidateSHA == "" {
				return fmt.Errorf("tentative gérée remplacée ou candidat absent")
			}
			managedProducer = v.Producer
		}
		if derivedStale {
			if !managed && (t.Status == "blocked" || revalidating) {
				t.Status, t.Blocker = "submitted", ""
				t.Next = "Livrable corrigé remis au vérificateur indépendant ; acceptation toujours requise."
				if revalidating {
					// Old gates and receipts remain in mutation history and on disk.
					// Current acceptance must require fresh checks and a new decision.
					t.Gate, t.AutoValidation, t.Override = nil, nil, nil
				}
			}
			// Changed evidence requires a fresh review, not continuation of the
			// completed journal. Preserve the old verdict and spent calls.
			if t.IndependentReview != nil {
				t.PreviousReviews = append(t.PreviousReviews, *v)
			}
			t.IndependentReview = nil
			t.BatchReviewResume = nil
			cfg.Failure = ""
			return nil
		}
		if v.FragmentJournal != nil && fragmentReviewerChanged(*w, *v) {
			if replacementErr != nil {
				return replacementErr
			}
			preview := replacement
			if preview.Revision != w.Revision || preview.Review != v.ID {
				return fmt.Errorf("préparation du remplacement périmée")
			}
			if preview.Missing > 0 {
				return fmt.Errorf("%s", preview.Next)
			}
			// Retain the previous review and immutable evidence in the task history.
			// No old inspection is attributed to the newly selected reviewer.
			t.PreviousReviews = append(t.PreviousReviews, *v)
			t.IndependentReview = nil
			cfg.Failure = ""
			t.Next = preview.Next
			return nil
		}
		if v.FragmentJournal != nil {
			if err := s.queueManagedFragmentResume(*w, t, r.EventID); err != nil {
				return err
			}
			cfg.Failure = ""
			return nil
		}
		// The former record remains in the event history. Call reservations are never refunded.
		if len(v.Batches) > 0 {
			t.BatchReviewResume = v
		}
		t.IndependentReview = nil
		cfg.Failure = ""
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
		if managedProducer == "" {
			return nil
		}
		var result sql.Result
		var e error
		if unpaid {
			result, e = tx.Exec("UPDATE managed_attempts SET state='integrating',detail='' WHERE agent_id=? AND work_id=? AND task_id=? AND state='conflict' AND result_commit=? AND detail=?", managedProducer, work, r.Task, preflight.Item.Result, preflight.Item.Detail)
		} else {
			result, e = tx.Exec("UPDATE managed_attempts SET state='integrating',detail='' WHERE agent_id=? AND state IN ('conflict','integrating') AND result_commit!=''", managedProducer)
		}
		if e != nil {
			return e
		}
		n, e := result.RowsAffected()
		if e != nil {
			return e
		}
		if n != 1 {
			return fmt.Errorf("candidat de reprise introuvable")
		}
		return nil
	})
}

func managedBatchesAllPassed(r *IndependentReview) bool {
	if r == nil || len(r.Batches) == 0 {
		return false
	}
	for _, b := range r.Batches {
		if b.State != "passed" {
			return false
		}
	}
	return true
}

// archiveIndependentReview invalidates evidence without refunding a review or
// changing the production attempt. It is used when a task must be re-evaluated.
func archiveIndependentReview(t *Task) {
	if t.IndependentReview != nil {
		t.PreviousReviews = append(t.PreviousReviews, *t.IndependentReview)
		t.IndependentReview = nil
	}
}

// A contract amendment may only resume a refused completed result. The durable
// operator event proves the explicit old-to-new contract transition; it never
// makes an old favorable verdict valid under a different contract.
func (s *Store) recordedContractRevisionMatches(work string, t *Task, prior string, audit ...[]Event) bool {
	var events []Event
	if len(audit) > 0 {
		events = audit[0]
	} else {
		var err error
		events, err = s.events(work)
		if err != nil {
			return false
		}
	}
	for i := len(events) - 1; i >= 0; i-- {
		if events[i].Kind != "task.update" {
			continue
		}
		var request Request
		if json.Unmarshal(events[i].Payload, &request) != nil || request.ID != t.ID || !request.ConfirmContractRevision || request.ExpectedContract != prior || len(strings.TrimSpace(request.ContractRevisionReason)) < 16 {
			continue
		}
		reconstructed := *t
		reconstructed.Criteria = request.Criteria
		if reviewContract(&reconstructed) == reviewContract(t) {
			return true
		}
	}
	return false
}
