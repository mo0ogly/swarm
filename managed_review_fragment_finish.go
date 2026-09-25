//go:build linux

package main

import (
	"fmt"
	"os"
)

// A proved negative inspection can stop early; it can never approve a task.
// Re-read the anchored original reply rather than trusting an error string.
func (s *Store) managedFragmentRefusal(r IndependentReview) (string, error) {
	if r.FragmentJournal == nil {
		return "", nil
	}
	p, j, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		return "", err
	}
	if len(j.Entries) == 0 {
		return "", nil
	}
	e := j.Entries[len(j.Entries)-1]
	if e.State != "changes_requested" {
		return "", nil
	}
	return fmt.Sprintf("Correction demandée par l’inspection %d (%d pièces) ; refus conservé dans le journal. Les autres pièces ne sont pas toutes examinées.", e.Packet+1, len(p.Packets[e.Packet].Artifacts)), nil
}

// Finalize only from the latest durable journal, never from the record captured
// before provider calls. The caller still owns the review lock. This records an
// opinion; candidate publication remains a separate, fully checked transaction.
func (s *Store) finishManagedFragmentReview(work string, a Agent, reviewID string, runErr error) error {
	w, err := s.get(work)
	if err != nil {
		return err
	}
	task, err := w.task(a.TaskID)
	if err != nil {
		return err
	}
	if task.IndependentReview == nil || task.IndependentReview.ID != reviewID || task.IndependentReview.State != "running" || task.IndependentReview.FragmentJournal == nil {
		return fmt.Errorf("revue de fragments remplacée ou déjà terminée")
	}
	record := *task.IndependentReview
	record.State = "error"
	record.Finished = now()
	record.ManagedTasks = nil
	record.Criteria = nil
	if runErr == nil {
		var p managedReviewFragmentPlan
		var j managedFragmentJournal
		p, j, err = s.readFragmentJournalAnchor(record)
		if err == nil {
			final, c, prefix, e := s.readManagedFragmentFinal(record, p, j)
			err = e
			if err == nil {
				record.State, record.ManagedTasks, err = validateManagedFragmentFinalJournal(final, j, c, p, prefix)
			}
		}
		if err == nil && record.State != "passed" && record.State != "changes_requested" && record.State != "unknown" {
			err = fmt.Errorf("verdict final incomplet ; aucune acceptation")
		}
		if err != nil {
			runErr = err
		}
	}
	if runErr != nil {
		record.State = "error"
		record.ManagedTasks = nil
		record.Reason = guardBlock(runErr.Error(), 4000)
		if reason, proofErr := s.managedFragmentRefusal(record); proofErr == nil && reason != "" {
			record.State = "changes_requested"
			record.Reason = reason
		}
	} else {
		record.Reason = "Verdict indépendant enregistré après inspection de toutes les pièces et examen final du même candidat."
		if record.State == "unknown" {
			record.Reason = "Preuves supplémentaires nécessaires ; la sélection ne permet pas de rendre un verdict final."
		}
		for _, entry := range record.ManagedTasks {
			if entry.Task == a.TaskID {
				record.Criteria = entry.Criteria
				record.Reason = entry.Reason
			}
		}
	}
	if err = s.saveManagedReview(work, a, record); err != nil {
		return err
	}
	// saveManagedReview may downgrade to stale when concurrent changes invalidate
	// evidence. Never return success from the pre-save local state.
	w, err = s.get(work)
	if err != nil {
		return err
	}
	task, err = w.task(a.TaskID)
	if err != nil {
		return err
	}
	if task.IndependentReview == nil || task.IndependentReview.ID != reviewID {
		return fmt.Errorf("revue remplacée pendant la finalisation")
	}
	if task.IndependentReview.State != "passed" {
		return fmt.Errorf("revue de fragments non favorable : %s", task.IndependentReview.Reason)
	}
	path, err := safeReport(s.root, record.Receipt)
	if err != nil {
		return err
	}
	receipt, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	_, err = s.managedReviewsForPublication(&w, a, record.CandidateSHA, record.Receipt, receipt)
	return err
}
