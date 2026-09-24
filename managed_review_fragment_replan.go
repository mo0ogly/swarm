//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

// Repartition only an interrupted plan with no reusable finding. The public
// retry transaction owns authorization; this helper never calls a provider.
func (s *Store) managedFragmentReplan(w Work, r IndependentReview, old managedReviewFragmentPlan, j managedFragmentJournal) (managedReviewFragmentPlan, error) {
	if r.State != "error" || r.FragmentJournal.ReplannedFrom != nil || r.FragmentJournal.FinalJournalDigest != "" {
		return managedReviewFragmentPlan{}, fmt.Errorf("redécoupage réservé à une inspection interrompue sans décision finale")
	}
	for _, e := range j.Entries {
		if e.State != "interrupted" {
			return managedReviewFragmentPlan{}, fmt.Errorf("inspection active ou résultat durable : conserver le plan existant")
		}
	}
	path, err := safeReport(s.root, r.Context)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	var c managedReviewContext
	if err = strict(raw, &c); err != nil {
		return managedReviewFragmentPlan{}, err
	}
	cfg := w.Planning.Reviewer
	return planManagedReviewFragments(c, cfg.MaxCalls-cfg.Calls, old.ReservedFinalCalls)
}

func (s *Store) queueManagedFragmentReplan(w Work, task *Task, r IndependentReview, old managedReviewFragmentPlan, journal managedFragmentJournal) error {
	p, err := s.managedFragmentReplan(w, r, old, journal)
	if err != nil {
		return err
	}
	raw, err := json.Marshal(p)
	if err != nil {
		return err
	}
	prior := *r.FragmentJournal
	next := prior
	next.ReplannedFrom = &prior
	next.ResumeCalls = nil
	next.PlanDigest = hash(raw)
	next.Plan = filepath.ToSlash(filepath.Join(filepath.Dir(prior.Plan), "fragment-plan-"+hash(raw)+".json"))
	j := managedFragmentJournal{Version: 1, PlanDigest: next.PlanDigest, Attempt: journal.Attempt, ProviderDigest: journal.ProviderDigest}
	jr, err := json.Marshal(j)
	if err != nil {
		return err
	}
	next.JournalDigest = hash(jr)
	next.Journal = filepath.ToSlash(filepath.Join(filepath.Dir(prior.Journal), "fragment-journal-"+hash(jr)+".json"))
	if err = atomicWrite(filepath.Join(s.root, next.Plan), raw); err != nil {
		return err
	}
	if err = atomicWrite(filepath.Join(s.root, next.Journal), jr); err != nil {
		return err
	}
	r.FragmentJournal = &next
	r.State = "queued"
	r.Finished = ""
	r.Reason = "Redécoupage explicite pour capacité de réponse ; ancien plan, journal et dépenses conservés."
	task.IndependentReview = &r
	return nil
}
