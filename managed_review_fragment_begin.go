//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"path/filepath"
)

// Internal preparation only. No public route calls this until execution,
// recovery and final publication are connected. Creating the review spends no
// call; every subsequent call still needs its own transactional reservation.
func (s *Store) beginManagedFragmentReview(w Work, a Agent, c managedReviewContext, receiptPath string, receipt []byte) (IndependentReview, error) {
	empty := IndependentReview{}
	if w.Planning == nil || w.Planning.Repository == nil || w.Planning.Reviewer == nil {
		return empty, fmt.Errorf("configuration de revue absente")
	}
	if _, err := safeReport(s.root, receiptPath); err != nil {
		return empty, err
	}
	if hash(c.Receipt) != hash(receipt) {
		return empty, fmt.Errorf("contexte et reçu de fragments différents")
	}
	canonical, err := s.managedReviewContext(w, a, c.Candidate, receipt)
	if err != nil {
		return empty, err
	}
	providedRaw, _ := json.Marshal(c)
	canonicalRaw, _ := json.Marshal(canonical)
	if hash(providedRaw) != hash(canonicalRaw) {
		return empty, fmt.Errorf("preuves de fragments différentes du candidat canonique")
	}
	cfg := w.Planning.Reviewer
	p, err := planManagedReviewFragments(c, cfg.MaxCalls-cfg.Calls, 2)
	if err != nil {
		return empty, err
	}
	workflow, method, err := agentWorkflow("reviewer")
	if err != nil {
		return empty, err
	}
	if err = preflightManagedFragmentCalls(managedReviewPrefix(method), c, p); err != nil {
		return empty, err
	}
	task, err := w.task(a.TaskID)
	if err != nil {
		return empty, err
	}
	if task.IndependentReview != nil || task.BatchReviewResume != nil {
		return empty, fmt.Errorf("revue existante à préserver")
	}
	timeout, err := reviewTimeoutSeconds(cfg)
	if err != nil {
		return empty, err
	}
	r := IndependentReview{ID: newID("review-"), Attempt: a.Attempt, Producer: a.ID, Reviewer: "reviewer://" + cfg.Provider, Contract: reviewContract(task), CandidateSHA: c.Candidate, PreviousCandidate: c.Previous, Receipt: receiptPath, ReceiptDigest: hash(receipt), State: "running", Reason: "Inspection indépendante des fragments en préparation.", Started: now(), Workflow: &workflow, TimeoutSeconds: timeout, BatchProviderDigest: cfg.ProviderDigest}
	providers, err := s.providers()
	if err != nil {
		return empty, err
	}
	provider, ok := providers.Providers[cfg.Provider]
	if !ok {
		return empty, fmt.Errorf("fournisseur absent")
	}
	level := "auto"
	if cfg.ModelRoute != nil {
		level = cfg.ModelRoute.Level
	}
	_, route, err := resolveModel(provider, level, "planning")
	if err != nil {
		return empty, err
	}
	r.ModelRoute = route
	// Unique paths preserve all previous files, including interrupted attempts.
	dir := filepath.Dir(receiptPath)
	write := func(name string, value any) (string, string, error) {
		raw, e := json.Marshal(value)
		if e != nil {
			return "", "", e
		}
		rel := filepath.ToSlash(filepath.Join(dir, r.ID+"-"+name+".json"))
		e = atomicWrite(filepath.Join(s.root, rel), raw)
		return rel, hash(raw), e
	}
	r.Context, r.ContextDigest, err = write("context", c)
	if err != nil {
		return empty, err
	}
	anchor := &ManagedFragmentJournalAnchor{}
	anchor.Plan, anchor.PlanDigest, err = write("plan", p)
	if err != nil {
		return empty, err
	}
	j := managedFragmentJournal{Version: 1, PlanDigest: anchor.PlanDigest, Attempt: a.Attempt, ProviderDigest: cfg.ProviderDigest}
	anchor.Journal, anchor.JournalDigest, err = write("journal", j)
	if err != nil {
		return empty, err
	}
	raw, _ := json.Marshal([]any{workflow, method})
	anchor.WorkflowDigest = hash(raw)
	raw, _ = json.Marshal(cfg.ModelRoute)
	anchor.ModelConfigDigest = hash(raw)
	r.FragmentJournal = anchor
	for _, tc := range c.Tasks {
		if tc.Task == a.TaskID {
			r.Report = filepath.ToSlash(filepath.Join(dir, r.ID+"-report.md"))
			r.GitReport = tc.Binding.Report
			r.Digest = hash([]byte(tc.Report))
			if r.Digest != tc.Binding.ReportDigest {
				return empty, fmt.Errorf("rapport du fragment différent du contrat")
			}
			if err = atomicWrite(filepath.Join(s.root, r.Report), []byte(tc.Report)); err != nil {
				return empty, err
			}
		}
	}
	raw, _ = json.Marshal(r)
	_, err = s.mutateWithHook(w.ID, "review.fragment.begin", r.ID+"-begin", w.Revision, raw, func(cw *Work) error {
		ct, e := cw.task(a.TaskID)
		if e != nil {
			return e
		}
		if ct.IndependentReview != nil || ct.BatchReviewResume != nil || !currentTaskAttempt(ct, a.Attempt) || cw.Planning == nil || cw.Planning.Repository == nil || cw.Planning.Reviewer == nil || cw.Planning.Paused || cw.Planning.Repository.Candidate != c.Previous || managedReviewContract(*cw, a.TaskID) != managedReviewContract(w, a.TaskID) {
			return fmt.Errorf("préparation des fragments périmée")
		}
		current := cw.Planning.Reviewer
		model, _ := json.Marshal(current.ModelRoute)
		if current.Failure != "" || current.MaxCalls-current.Calls < len(p.Packets)+2 || hash(model) != anchor.ModelConfigDigest {
			return fmt.Errorf("budget ou modèle des fragments indisponible")
		}
		if e = s.managedBatchProviderIntact(current, r); e != nil {
			return e
		}
		if e = s.managedReviewFilesIntact(r); e != nil {
			return e
		}
		ct.IndependentReview = &r
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
		var paused bool
		if e := tx.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", w.ID).Scan(&paused); e != nil && e != sql.ErrNoRows {
			return e
		}
		if paused {
			return fmt.Errorf("mission suspendue")
		}
		return s.providerCooldownGuard(cfg.Provider)
	})
	if err != nil {
		return empty, err
	}
	return r, nil
}
