//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
)

type ManagedFragmentJournalAnchor struct {
	ReplannedFrom      *ManagedFragmentJournalAnchor `json:"replanned_from,omitempty"`
	ResumeCalls        []string                      `json:"resume_calls,omitempty"`
	InspectionTask     string                        `json:"inspection_task,omitempty"`
	InspectionAttempt  string                        `json:"inspection_attempt,omitempty"`
	InspectionProducer string                        `json:"inspection_producer,omitempty"`
	FinalJournal       string                        `json:"final_journal,omitempty"`
	FinalJournalDigest string                        `json:"final_journal_sha256,omitempty"`
	WorkflowDigest     string                        `json:"workflow_sha256"`
	ModelConfigDigest  string                        `json:"model_config_sha256"`
	Plan               string                        `json:"plan"`
	PlanDigest         string                        `json:"plan_sha256"`
	Journal            string                        `json:"journal"`
	JournalDigest      string                        `json:"journal_sha256"`
}

func (s *Store) readFragmentJournalAnchor(r IndependentReview) (managedReviewFragmentPlan, managedFragmentJournal, error) {
	var p managedReviewFragmentPlan
	var j managedFragmentJournal
	anchor := r.FragmentJournal
	if anchor == nil {
		return p, j, fmt.Errorf("journal de fragments absent")
	}
	if anchor.ReplannedFrom != nil {
		prior := *anchor.ReplannedFrom
		if prior.ReplannedFrom != nil || prior.FinalJournalDigest != "" {
			return p, j, fmt.Errorf("historique de redécoupage invalide")
		}
		old := r
		old.FragmentJournal = &prior
		_, oldJournal, err := s.readFragmentJournalAnchor(old)
		if err != nil {
			return p, j, err
		}
		for _, entry := range oldJournal.Entries {
			if entry.State != "interrupted" {
				return p, j, fmt.Errorf("redécoupage avec inspection durable ou active interdit")
			}
		}
	}
	workflow, prompt, err := agentWorkflow("reviewer")
	if err != nil {
		return p, j, err
	}
	methodRaw, _ := json.Marshal([]any{workflow, prompt})
	if anchor.WorkflowDigest != hash(methodRaw) {
		return p, j, fmt.Errorf("méthode de revue des fragments modifiée")
	}
	path, e := safeReport(s.root, anchor.Plan)
	if e != nil {
		return p, j, e
	}
	raw, e := os.ReadFile(path)
	if e != nil {
		return p, j, e
	}
	if anchor.PlanDigest == "" || hash(raw) != anchor.PlanDigest {
		return p, j, fmt.Errorf("plan de fragments modifié")
	}
	if e = strict(raw, &p); e != nil {
		return p, j, e
	}
	contextPath, e := safeReport(s.root, r.Context)
	if e != nil {
		return p, j, e
	}
	contextRaw, e := os.ReadFile(contextPath)
	if e != nil {
		return p, j, e
	}
	if hash(contextRaw) != r.ContextDigest || p.ContextDigest != r.ContextDigest || p.Candidate != r.CandidateSHA {
		return p, j, fmt.Errorf("contexte des fragments modifié")
	}
	var c managedReviewContext
	if e = strict(contextRaw, &c); e != nil {
		return p, j, e
	}
	if e = validateManagedReviewFragments(c, p); e != nil {
		return p, j, e
	}
	path, e = safeReport(s.root, anchor.Journal)
	if e != nil {
		return p, j, e
	}
	attempt := r.Attempt
	if anchor.InspectionAttempt != "" || anchor.InspectionTask != "" || anchor.InspectionProducer != "" {
		owner, copyBinding := false, false
		for _, tc := range c.Tasks {
			if tc.Task == anchor.InspectionTask && tc.Binding.Attempt == anchor.InspectionAttempt && tc.Binding.Producer == anchor.InspectionProducer {
				owner = true
			}
			if tc.Binding.Attempt == r.Attempt && tc.Binding.Producer == r.Producer && tc.Binding.Contract == r.Contract && tc.Binding.Report == r.GitReport && tc.Binding.ReportDigest == r.Digest {
				copyBinding = true
			}
		}
		if !owner || !copyBinding || anchor.InspectionAttempt == "" || anchor.InspectionProducer == "" {
			return p, j, fmt.Errorf("provenance de revue cumulative invalide")
		}
		attempt = anchor.InspectionAttempt
	}
	j, _, e = readManagedFragmentJournal(path, anchor.JournalDigest, p, attempt, r.BatchProviderDigest)
	if e != nil {
		return p, j, e
	}
	authorized := map[string]bool{}
	for _, id := range anchor.ResumeCalls {
		if id == "" || authorized[id] {
			return p, j, fmt.Errorf("autorisation de reprise dupliquée")
		}
		found := false
		for _, entry := range j.Entries {
			if entry.CallID == id && entry.State == "interrupted" {
				found = true
			}
		}
		if !found {
			return p, j, fmt.Errorf("autorisation sans appel interrompu")
		}
		authorized[id] = true
	}
	return p, j, nil
}

// Allowed transitions: append exactly one reservation, or finish the last
// reservation. No earlier result or consumed call can be rewritten.
func fragmentJournalTransition(old, next managedFragmentJournal) (string, error) {
	if old.Version != next.Version || old.PlanDigest != next.PlanDigest || old.Attempt != next.Attempt || old.ProviderDigest != next.ProviderDigest {
		return "", fmt.Errorf("identité du journal modifiée")
	}
	if len(next.Entries) == len(old.Entries)+1 && (len(old.Entries) == 0 || reflect.DeepEqual(next.Entries[:len(old.Entries)], old.Entries)) {
		for _, entry := range old.Entries {
			if entry.State == "reserved" || entry.State == "changes_requested" {
				return "", fmt.Errorf("inspection précédente non résolue")
			}
		}
		entry := next.Entries[len(old.Entries)]
		if entry.State != "reserved" || entry.Reply != "" || entry.ReplyDigest != "" {
			return "", fmt.Errorf("réservation initiale requise")
		}
		return entry.CallID, nil
	}
	n := len(old.Entries)
	if n == 0 || len(next.Entries) != n || !reflect.DeepEqual(next.Entries[:n-1], old.Entries[:n-1]) {
		return "", fmt.Errorf("historique des appels modifié")
	}
	a, b := old.Entries[n-1], next.Entries[n-1]
	if a.State != "reserved" || b.State == "reserved" || a.Packet != b.Packet || a.PacketDigest != b.PacketDigest || a.CallID != b.CallID {
		return "", fmt.Errorf("fin de réservation incohérente")
	}
	return "", nil
}

// No public route invokes this until final review and restart orchestration are
// complete. Atomic reservation and journal anchoring share the Store transaction.
func (s *Store) commitFragmentJournal(work string, a Agent, reviewID, expectedDigest, event string, next managedFragmentJournal) error {
	w, e := s.get(work)
	if e != nil {
		return e
	}
	t, e := w.task(a.TaskID)
	if e != nil {
		return e
	}
	r := t.IndependentReview
	if r == nil || r.ID != reviewID || r.State != "running" || r.FragmentJournal == nil || r.FragmentJournal.JournalDigest != expectedDigest || r.FragmentJournal.FinalJournalDigest != "" {
		return fmt.Errorf("revue des fragments remplacée")
	}
	p, old, e := s.readFragmentJournalAnchor(*r)
	if e != nil {
		return e
	}
	if _, e = validateManagedFragmentJournal(next, p, r.Attempt, r.BatchProviderDigest); e != nil {
		return e
	}
	call, e := fragmentJournalTransition(old, next)
	if e != nil {
		return e
	}
	raw, e := json.Marshal(next)
	if e != nil {
		return e
	}
	// Content-addressed writes cannot overwrite the previously anchored journal.
	relative := filepath.ToSlash(filepath.Join(filepath.Dir(r.FragmentJournal.Journal), "fragment-journal-"+hash(raw)+".json"))
	oldPath, e := safeReport(s.root, r.FragmentJournal.Journal)
	if e != nil {
		return e
	}
	path := filepath.Join(filepath.Dir(oldPath), filepath.Base(relative))
	if e = atomicWrite(path, raw); e != nil {
		return e
	}
	_, e = s.mutateWithHook(work, "review.fragment.journal", event, w.Revision, raw, func(cw *Work) error {
		ct, err := cw.task(a.TaskID)
		if err != nil {
			return err
		}
		cr := ct.IndependentReview
		if cr == nil || cr.ID != reviewID || cr.State != "running" || cr.FragmentJournal == nil || cr.FragmentJournal.JournalDigest != expectedDigest || cr.FragmentJournal.FinalJournalDigest != "" || !currentTaskAttempt(ct, a.Attempt) || cr.Attempt != a.Attempt || cr.Producer != a.ID || cw.Planning == nil || cw.Planning.Repository == nil || cw.Planning.Repository.Candidate != cr.PreviousCandidate || managedReviewContract(*cw, a.TaskID) != managedReviewContract(w, a.TaskID) {
			return fmt.Errorf("réservation des fragments périmée")
		}
		if err = s.managedReviewFilesIntact(*cr); err != nil {
			return err
		}
		if err = s.managedBatchProviderIntact(cw.Planning.Reviewer, *cr); err != nil {
			return err
		}
		modelRaw, _ := json.Marshal(cw.Planning.Reviewer.ModelRoute)
		if cr.FragmentJournal.ModelConfigDigest != hash(modelRaw) {
			return fmt.Errorf("modèle des fragments modifié")
		}
		if call != "" {
			if cw.Planning.Paused {
				return fmt.Errorf("planification suspendue")
			}
			cfg := cw.Planning.Reviewer
			reusable, err := validateManagedFragmentJournal(old, p, r.Attempt, r.BatchProviderDigest)
			if err != nil {
				return err
			}
			required := len(p.Packets) - len(reusable) + p.ReservedFinalCalls
			if cfg.Failure != "" || cfg.Calls+required > cfg.MaxCalls {
				return fmt.Errorf("budget ou fournisseur des fragments indisponible")
			}
			cfg.Calls++
		}
		updated := *cr.FragmentJournal
		updated.Journal = relative
		updated.JournalDigest = hash(raw)
		cr.FragmentJournal = &updated
		return nil
	}, func(tx *sql.Tx, cw *Work) error {
		if call == "" {
			return nil
		}
		var paused bool
		if err := tx.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", work).Scan(&paused); err != nil && err != sql.ErrNoRows {
			return err
		}
		if paused {
			return fmt.Errorf("mission suspendue")
		}
		if err := s.providerCooldownGuard(cw.Planning.Reviewer.Provider); err != nil {
			return err
		}
		return reservePlanningCall(tx, work, "reviewer", call)
	})
	return e
}
