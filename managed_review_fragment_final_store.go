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

func finalFragmentTransition(old, next managedFragmentFinalJournal) (string, error) {
	if old.Version != next.Version || old.InspectionDigest != next.InspectionDigest {
		return "", fmt.Errorf("identité finale modifiée")
	}
	n := len(old.Calls)
	if len(next.Calls) == n+1 && (n == 0 || reflect.DeepEqual(next.Calls[:n], old.Calls)) {
		entry := next.Calls[n]
		if entry.State != "reserved" || entry.Reply != "" || entry.ReplyDigest != "" {
			return "", fmt.Errorf("réservation finale requise")
		}
		return entry.CallID, nil
	}
	if n == 0 || len(next.Calls) != n || !reflect.DeepEqual(next.Calls[:n-1], old.Calls[:n-1]) {
		return "", fmt.Errorf("historique final modifié")
	}
	a, b := old.Calls[n-1], next.Calls[n-1]
	if a.State != "reserved" || b.State == "reserved" || a.CallID != b.CallID || a.Phase != b.Phase || a.PromptDigest != b.PromptDigest {
		return "", fmt.Errorf("retour final non attribué")
	}
	return "", nil
}

func (s *Store) readManagedFragmentFinal(r IndependentReview, p managedReviewFragmentPlan, j managedFragmentJournal) (managedFragmentFinalJournal, managedReviewContext, string, error) {
	var c managedReviewContext
	empty := managedFragmentFinalJournal{}
	path, err := safeReport(s.root, r.Context)
	if err != nil {
		return empty, c, "", err
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		return empty, c, "", err
	}
	if hash(raw) != r.ContextDigest {
		return empty, c, "", fmt.Errorf("contexte final modifié")
	}
	if err = strict(raw, &c); err != nil {
		return empty, c, "", err
	}
	_, method, err := agentWorkflow("reviewer")
	if err != nil {
		return empty, c, "", err
	}
	prefix := managedReviewPrefix(method)
	anchor := r.FragmentJournal
	if anchor == nil {
		return empty, c, "", fmt.Errorf("ancrage final absent")
	}
	journalRaw, _ := json.Marshal(j)
	f := managedFragmentFinalJournal{Version: 1, InspectionDigest: hash(journalRaw)}
	if anchor.FinalJournalDigest != "" {
		path, err = safeReport(s.root, anchor.FinalJournal)
		if err != nil {
			return empty, c, "", err
		}
		raw, err = os.ReadFile(path)
		if err != nil {
			return empty, c, "", err
		}
		if hash(raw) != anchor.FinalJournalDigest {
			return empty, c, "", fmt.Errorf("journal final modifié")
		}
		if err = strict(raw, &f); err != nil {
			return empty, c, "", err
		}
	} else if anchor.FinalJournal != "" {
		return empty, c, "", fmt.Errorf("empreinte finale absente")
	}
	_, _, err = validateManagedFragmentFinalJournal(f, j, c, p, prefix)
	return f, c, prefix, err
}

func (s *Store) commitManagedFragmentFinal(work string, a Agent, reviewID, inspectionDigest, expectedFinalDigest, event string, next managedFragmentFinalJournal) error {
	w, err := s.get(work)
	if err != nil {
		return err
	}
	t, err := w.task(a.TaskID)
	if err != nil {
		return err
	}
	r := t.IndependentReview
	if r == nil || r.ID != reviewID || r.State != "running" || r.FragmentJournal == nil || r.FragmentJournal.JournalDigest != inspectionDigest || r.FragmentJournal.FinalJournalDigest != expectedFinalDigest {
		return fmt.Errorf("revue finale remplacée")
	}
	p, j, err := s.readFragmentJournalAnchor(*r)
	if err != nil {
		return err
	}
	old, c, prefix, err := s.readManagedFragmentFinal(*r, p, j)
	if err != nil {
		return err
	}
	if _, _, err = validateManagedFragmentFinalJournal(next, j, c, p, prefix); err != nil {
		return err
	}
	call, err := finalFragmentTransition(old, next)
	if err != nil {
		return err
	}
	raw, err := json.Marshal(next)
	if err != nil {
		return err
	}
	parent, err := safeReport(s.root, r.Context)
	if err != nil {
		return err
	}
	name := "fragment-final-" + hash(raw) + ".json"
	relative := filepath.ToSlash(filepath.Join(filepath.Dir(r.Context), name))
	if err = atomicWrite(filepath.Join(filepath.Dir(parent), name), raw); err != nil {
		return err
	}
	_, err = s.mutateWithHook(work, "review.fragment.final", event, w.Revision, raw, func(cw *Work) error {
		ct, e := cw.task(a.TaskID)
		if e != nil {
			return e
		}
		cr := ct.IndependentReview
		if cr == nil || cr.ID != reviewID || cr.State != "running" || cr.FragmentJournal == nil || cr.FragmentJournal.JournalDigest != inspectionDigest || cr.FragmentJournal.FinalJournalDigest != expectedFinalDigest || !currentTaskAttempt(ct, a.Attempt) || cr.Attempt != a.Attempt || cr.Producer != a.ID || cw.Planning == nil || cw.Planning.Repository == nil || cw.Planning.Repository.Candidate != cr.PreviousCandidate || managedReviewContract(*cw, a.TaskID) != managedReviewContract(w, a.TaskID) {
			return fmt.Errorf("réservation finale périmée")
		}
		if e = s.managedReviewFilesIntact(*cr); e != nil {
			return e
		}
		if e = s.managedBatchProviderIntact(cw.Planning.Reviewer, *cr); e != nil {
			return e
		}
		cfg := cw.Planning.Reviewer
		model, _ := json.Marshal(cfg.ModelRoute)
		if hash(model) != cr.FragmentJournal.ModelConfigDigest {
			return fmt.Errorf("modèle final modifié")
		}
		if call != "" {
			required := 1
			if next.Calls[len(next.Calls)-1].Phase == "selection" {
				required = 2
			}
			if cw.Planning.Paused || cfg.Failure != "" || cfg.Calls+required > cfg.MaxCalls {
				return fmt.Errorf("budget ou disponibilité finale insuffisante")
			}
			cfg.Calls++
		}
		anchor := *cr.FragmentJournal
		anchor.FinalJournal = relative
		anchor.FinalJournalDigest = hash(raw)
		cr.FragmentJournal = &anchor
		return nil
	}, func(tx *sql.Tx, cw *Work) error {
		if call == "" {
			return nil
		}
		var paused bool
		if e := tx.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", work).Scan(&paused); e != nil && e != sql.ErrNoRows {
			return e
		}
		if paused {
			return fmt.Errorf("mission suspendue")
		}
		if e := s.providerCooldownGuard(cw.Planning.Reviewer.Provider); e != nil {
			return e
		}
		return reservePlanningCall(tx, work, "reviewer", call)
	})
	return err
}
