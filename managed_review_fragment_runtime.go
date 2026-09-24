//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"time"
)

// Caller owns the attempt's managedReviewOwnershipLock. This internal runner
// consumes only individually committed reservations; it never publishes work.
func (s *Store) runManagedFragmentReview(w Work, a Agent, r IndependentReview) (string, []ManagedTaskReview, error) {
	reviewID := r.ID
	cfg := w.Planning.Reviewer
	ps, err := s.providers()
	if err != nil {
		return "", nil, err
	}
	provider, ok := ps.Providers[cfg.Provider]
	if !ok {
		return "", nil, fmt.Errorf("fournisseur absent")
	}
	level := "auto"
	if cfg.ModelRoute != nil {
		level = cfg.ModelRoute.Level
	}
	provider, route, err := resolveModel(provider, level, "planning")
	if err != nil {
		return "", nil, err
	}
	_, method, err := agentWorkflow("reviewer")
	if err != nil {
		return "", nil, err
	}
	prefix := managedReviewPrefix(method)
	refresh := func() error {
		current, e := s.get(w.ID)
		if e != nil {
			return e
		}
		task, e := current.task(a.TaskID)
		if e != nil {
			return e
		}
		if task.IndependentReview == nil || task.IndependentReview.ID != reviewID || task.IndependentReview.State != "running" || !currentTaskAttempt(task, a.Attempt) || current.Planning == nil || current.Planning.Repository == nil || current.Planning.Repository.Candidate != r.PreviousCandidate || managedReviewContract(current, a.TaskID) != managedReviewContract(w, a.TaskID) {
			return fmt.Errorf("revue active remplacée")
		}
		next := *task.IndependentReview
		if e = s.managedReviewFilesIntact(next); e != nil {
			return e
		}
		if e = s.managedBatchProviderIntact(current.Planning.Reviewer, next); e != nil {
			return e
		}
		model, _ := json.Marshal(current.Planning.Reviewer.ModelRoute)
		if next.FragmentJournal == nil || next.FragmentJournal.ModelConfigDigest != hash(model) {
			return fmt.Errorf("modèle de revue modifié")
		}
		r = next
		return nil
	}
	// Poll callbacks use a local copy, avoiding writes to r from the provider's
	// asynchronous cancellation monitor while the main goroutine records results.
	valid := func() bool {
		if s.paused(w.ID) || s.providerCooldownGuard(cfg.Provider) != nil {
			return false
		}
		current, e := s.get(w.ID)
		if e != nil || current.Planning == nil || current.Planning.Paused || current.Planning.Repository == nil || current.Planning.Repository.Candidate != w.Planning.Repository.Candidate || managedReviewContract(current, a.TaskID) != managedReviewContract(w, a.TaskID) {
			return false
		}
		task, e := current.task(a.TaskID)
		if e != nil || task.IndependentReview == nil || task.IndependentReview.ID != reviewID || task.IndependentReview.State != "running" || !currentTaskAttempt(task, a.Attempt) {
			return false
		}
		record := *task.IndependentReview
		model, _ := json.Marshal(current.Planning.Reviewer.ModelRoute)
		return record.FragmentJournal != nil && hash(model) == record.FragmentJournal.ModelConfigDigest && s.managedReviewFilesIntact(record) == nil && s.managedBatchProviderIntact(current.Planning.Reviewer, record) == nil
	}
	call := func(id, prompt, schema string) (string, error) {
		reply, e := runStructuredProvider(provider, route, prompt, schema, time.Duration(r.TimeoutSeconds)*time.Second, valid, func(u *Usage) { _ = s.savePlanningUsage(id, u) }, s.providerCooldownObserver(cfg.Provider, id))
		if reply != "" {
			// Preserve even malformed/provider-error replies for diagnosis. These files
			// are not review proof; only an anchored parsed response can be reused.
			path := filepath.Join(s.root, filepath.Dir(r.Context), id+"-raw-"+hash([]byte(reply))+".json")
			if writeErr := atomicWrite(path, []byte(reply)); writeErr != nil {
				return reply, writeErr
			}
		}
		return reply, e
	}
	if err = refresh(); err != nil {
		return "", nil, err
	}
	p, j, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		return "", nil, err
	}
	for i, packet := range p.Packets {
		reusable, e := validateManagedFragmentJournal(j, p, a.Attempt, r.BatchProviderDigest)
		if e != nil {
			return "", nil, e
		}
		if _, done := reusable[i]; done {
			continue
		}
		for _, entry := range j.Entries {
			authorized := false
			for _, id := range r.FragmentJournal.ResumeCalls {
				if id == entry.CallID {
					authorized = true
				}
			}
			if entry.State == "reserved" || (entry.State == "interrupted" && !authorized) || entry.State == "unknown" || entry.State == "changes_requested" {
				return entry.State, nil, fmt.Errorf("inspection inachevée ; reprise explicite requise")
			}
		}
		prompt, e := managedFragmentInspectionPrompt(prefix, packet)
		if e != nil {
			return "", nil, e
		}
		raw, _ := json.Marshal(packet)
		id := newID("fragment-call-")
		j.Entries = append(j.Entries, managedFragmentJournalEntry{Packet: i, PacketDigest: hash(raw), CallID: id, State: "reserved"})
		if e = s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, id+"-reserve", j); e != nil {
			return "", nil, e
		}
		if e = refresh(); e != nil {
			return "", nil, e
		}
		reply, callErr := call(id, prompt, managedFragmentInspectionSchema)
		state := "interrupted"
		if callErr == nil {
			state, _, callErr = parseManagedFragmentInspection(reply, packet)
		}
		entry := &j.Entries[len(j.Entries)-1]
		if callErr != nil {
			entry.State = "interrupted"
		} else {
			entry.State = state
			entry.Reply = reply
			entry.ReplyDigest = hash([]byte(reply))
		}
		if e = s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, id+"-result", j); e != nil {
			return "", nil, e
		}
		if e = refresh(); e != nil {
			return "", nil, e
		}
		if callErr != nil {
			return "error", nil, callErr
		}
		if state != "inspected" {
			return state, nil, fmt.Errorf("inspection %d : %s", i+1, state)
		}
	}
	f, c, prefix, err := s.readManagedFragmentFinal(r, p, j)
	if err != nil {
		return "", nil, err
	}
	reusable, err := validateManagedFragmentJournal(j, p, a.Attempt, r.BatchProviderDigest)
	if err != nil {
		return "", nil, err
	}
	replies := make([]string, len(p.Packets))
	for i := range replies {
		replies[i] = reusable[i]
	}
	for {
		state, records, e := validateManagedFragmentFinalJournal(f, j, c, p, prefix)
		if e != nil {
			return "", nil, e
		}
		if state == "passed" || state == "changes_requested" || state == "unknown" {
			return state, records, nil
		}
		if state == "interrupted" && len(f.Calls) > 0 {
			last := f.Calls[len(f.Calls)-1]
			for _, id := range f.ResumeCalls {
				if id == last.CallID {
					if last.Phase == "selection" {
						state = "pending"
					} else {
						state = "ready"
					}
				}
			}
		}
		if state != "pending" && state != "ready" {
			return state, nil, fmt.Errorf("appel final inachevé ; reprise explicite requise")
		}
		phase, schema := "selection", managedFragmentRequestSchema
		var prompt string
		var visible managedReviewContext
		if state == "pending" {
			prompt, e = managedFragmentRequestPrompt(prefix, c, p, replies)
		} else {
			phase, schema = "decision", managedReviewSchema
			selectionReply := ""
			for _, prior := range f.Calls {
				if prior.Phase == "selection" && prior.State == "ready" {
					selectionReply = prior.Reply
				}
			}
			selection, parseErr := parseManagedFragmentFinalRequest(selectionReply, prefix, c, p, replies)
			if parseErr != nil {
				return "", nil, parseErr
			}
			prompt, visible, e = managedFragmentDecisionPrompt(prefix, c, p, replies, selection.References)
		}
		if e != nil {
			return "", nil, e
		}
		id := newID("fragment-final-")
		f.Calls = append(f.Calls, managedFragmentFinalCall{Phase: phase, CallID: id, PromptDigest: hash([]byte(prompt)), State: "reserved"})
		if e = s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, r.FragmentJournal.FinalJournalDigest, id+"-reserve", f); e != nil {
			return "", nil, e
		}
		if e = refresh(); e != nil {
			return "", nil, e
		}
		reply, callErr := call(id, prompt, schema)
		returned := "interrupted"
		if callErr == nil && len(reply) > managedFragmentReplyLimit {
			callErr = fmt.Errorf("réponse finale trop grande")
		}
		if callErr == nil {
			if phase == "selection" {
				request, parseErr := parseManagedFragmentFinalRequest(reply, prefix, c, p, replies)
				callErr = parseErr
				returned = request.State
			} else {
				returned, _, callErr = parseManagedReview(reply, visible)
			}
		}
		entry := &f.Calls[len(f.Calls)-1]
		if callErr != nil {
			entry.State = "interrupted"
		} else {
			entry.State = returned
			entry.Reply = reply
			entry.ReplyDigest = hash([]byte(reply))
		}
		if e = s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, r.FragmentJournal.FinalJournalDigest, id+"-result", f); e != nil {
			return "", nil, e
		}
		if e = refresh(); e != nil {
			return "", nil, e
		}
		if callErr != nil {
			return "error", nil, callErr
		}
	}
}
