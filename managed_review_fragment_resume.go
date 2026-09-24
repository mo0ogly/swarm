//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"path/filepath"
)

// Called inside the explicit retry transaction. Reservations already spent are
// retained; only interrupted inspections are authorized to receive a new call.
func (s *Store) queueManagedFragmentResume(w Work, task *Task, event string) error {
	r := *task.IndependentReview
	if r.FragmentJournal == nil || event == "" {
		return fmt.Errorf("reprise de fragments non attribuée")
	}
	if err := s.managedBatchPlanIntact(w, r); err != nil {
		return err
	}
	if err := s.managedReviewFilesIntact(r); err != nil {
		return err
	}
	p, j, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		return err
	}
	if r.FragmentJournal.FinalJournalDigest != "" {
		return s.queueManagedFragmentFinalResume(w, task, r, p, j)
	}
	for _, packet := range p.Packets {
		if !managedFragmentReplyFits(packet) {
			return s.queueManagedFragmentReplan(w, task, r, p, j)
		}
	}
	anchor := *r.FragmentJournal
	anchor.ResumeCalls = nil
	for i := range j.Entries {
		entry := &j.Entries[i]
		switch entry.State {
		case "reserved":
			entry.State = "interrupted"
			anchor.ResumeCalls = append(anchor.ResumeCalls, entry.CallID)
		case "interrupted":
			anchor.ResumeCalls = append(anchor.ResumeCalls, entry.CallID)
		case "inspected":
		default:
			return fmt.Errorf("avis de fragment défavorable ou inconnu : correction des preuves requise")
		}
	}
	reusable, err := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
	if err != nil {
		return err
	}
	required := len(p.Packets) - len(reusable) + p.ReservedFinalCalls
	cfg := w.Planning.Reviewer
	if cfg.MaxCalls-cfg.Calls < required {
		return fmt.Errorf("budget insuffisant : %d appels restants nécessaires sans remboursement", required)
	}
	raw, err := json.Marshal(j)
	if err != nil {
		return err
	}
	anchor.JournalDigest = hash(raw)
	anchor.Journal = filepath.ToSlash(filepath.Join(filepath.Dir(r.Context), "fragment-journal-"+hash(raw)+".json"))
	if err = atomicWrite(filepath.Join(s.root, anchor.Journal), raw); err != nil {
		return err
	}
	r.FragmentJournal = &anchor
	r.State = "queued"
	r.Finished = ""
	r.Reason = "Reprise explicite des inspections manquantes ; preuves et appels consommés conservés."
	task.IndependentReview = &r
	return nil
}

// Caller owns the attempt review lock; consume the queued authorization before
// touching a provider. A crash in running state needs a new explicit retry.
func (s *Store) activateManagedFragmentResume(work string, a Agent, reviewID string) (Work, IndependentReview, error) {
	w, err := s.get(work)
	if err != nil {
		return w, IndependentReview{}, err
	}
	raw, _ := json.Marshal(map[string]string{"review": reviewID})
	updated, err := s.mutate(work, "review.fragment.resume", reviewID+"-resume-"+fmt.Sprint(w.Revision), w.Revision, raw, func(current *Work) error {
		task, e := current.task(a.TaskID)
		if e != nil {
			return e
		}
		r := task.IndependentReview
		if r == nil || r.ID != reviewID || r.State != "queued" || r.FragmentJournal == nil || !currentTaskAttempt(task, a.Attempt) {
			return fmt.Errorf("reprise remplacée")
		}
		if e = s.managedReviewFilesIntact(*r); e != nil {
			return e
		}
		if e = s.managedBatchPlanIntact(*current, *r); e != nil {
			return e
		}
		r.State = "running"
		return nil
	})
	if err != nil {
		return w, IndependentReview{}, err
	}
	task, err := updated.task(a.TaskID)
	if err != nil {
		return w, IndependentReview{}, err
	}
	return updated, *task.IndependentReview, nil
}

func (s *Store) queueManagedFragmentFinalResume(w Work, task *Task, r IndependentReview, p managedReviewFragmentPlan, j managedFragmentJournal) error {
	f, c, prefix, err := s.readManagedFragmentFinal(r, p, j)
	if err != nil {
		return err
	}
	for i := range f.Calls {
		if f.Calls[i].State == "reserved" {
			f.Calls[i].State = "interrupted"
		}
	}
	state, _, err := validateManagedFragmentFinalJournal(f, j, c, p, prefix)
	if err != nil {
		return err
	}
	required := 0
	if state == "interrupted" {
		last := f.Calls[len(f.Calls)-1]
		found := false
		for _, id := range f.ResumeCalls {
			if id == last.CallID {
				found = true
			}
		}
		if !found {
			f.ResumeCalls = append(f.ResumeCalls, last.CallID)
		}
		required = 1
		if last.Phase == "selection" {
			required = 2
		}
	} else if state == "ready" {
		required = 1
	} else if state != "passed" {
		return fmt.Errorf("verdict final défavorable ou inconnu ; aucune reprise implicite")
	}
	if w.Planning.Reviewer.MaxCalls-w.Planning.Reviewer.Calls < required {
		return fmt.Errorf("budget final insuffisant sans remboursement")
	}
	if _, _, err = validateManagedFragmentFinalJournal(f, j, c, p, prefix); err != nil {
		return err
	}
	raw, err := json.Marshal(f)
	if err != nil {
		return err
	}
	anchor := *r.FragmentJournal
	anchor.FinalJournalDigest = hash(raw)
	anchor.FinalJournal = filepath.ToSlash(filepath.Join(filepath.Dir(r.Context), "fragment-final-"+hash(raw)+".json"))
	if err = atomicWrite(filepath.Join(s.root, anchor.FinalJournal), raw); err != nil {
		return err
	}
	r.FragmentJournal = &anchor
	r.State = "queued"
	r.Finished = ""
	r.Reason = "Reprise finale explicite ; inspections et dépenses conservées."
	task.IndependentReview = &r
	return nil
}

func (s *Store) managedFragmentVerdictDurable(r *IndependentReview) bool {
	if r == nil || r.FragmentJournal == nil {
		return false
	}
	p, j, err := s.readFragmentJournalAnchor(*r)
	if err != nil {
		return false
	}
	f, c, prefix, err := s.readManagedFragmentFinal(*r, p, j)
	if err != nil {
		return false
	}
	state, _, err := validateManagedFragmentFinalJournal(f, j, c, p, prefix)
	return err == nil && state == "passed"
}
