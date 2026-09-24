//go:build linux

package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

type managedPreflightRetry struct {
	Revision int
	Agent    Agent
	Item     ManagedAttempt
	Work     Work
	Context  managedReviewContext
}

// Read-only preparation outside the write transaction. A no-call size refusal
// has no IndependentReview record: it still needs a public, bounded recovery.
// Never create a producer, rerun checks, replace a candidate or synthesize an
// opinion here. The conductor revalidates the retained candidate afterward.
func (s *Store) readManagedPreflightEvidence(work, task string) (*managedPreflightRetry, error) {
	w, e := s.get(work)
	if e != nil {
		return nil, e
	}
	t, e := w.task(task)
	if e != nil {
		return nil, e
	}
	if w.Planning == nil || w.Planning.Repository == nil || w.Planning.Reviewer == nil || t.Status != "blocked" || t.IndependentReview != nil || t.BatchReviewResume != nil || len(t.Attempts) == 0 {
		return nil, fmt.Errorf("aucun précontrôle de revue impayé à reprendre")
	}
	agents, e := s.agents(work)
	if e != nil {
		return nil, e
	}
	var a Agent
	for _, candidate := range agents {
		if candidate.TaskID == task && candidate.Attempt == t.Attempts[len(t.Attempts)-1].ID {
			if a.ID != "" {
				return nil, fmt.Errorf("producteur de la tentative ambigu")
			}
			a = candidate
		}
	}
	if a.ID == "" || !recoveryProcessEnded(a, hostIdentity(), processStamp(a.Child)) {
		return nil, fmt.Errorf("production terminée et fin du processus confirmée requises")
	}
	item, e := s.managedAttempt(a.ID)
	if e != nil {
		return nil, e
	}
	if a.Status != "completed" && !recoveredResultMatches(t, a, item) {
		return nil, fmt.Errorf("résultat réparé attribuable requis pour une tentative interrompue")
	}
	sizeRefusal := item.Detail == managedReviewContextTooLarge || strings.HasPrefix(item.Detail, managedReviewContextTooLarge+" : tâche ")
	if item.Work != work || item.Task != task || item.State != "conflict" || item.Result == "" || !(sizeRefusal || item.Detail == "revue retenue : modification binaire non examinable par ce vérificateur") || t.Blocker != item.Detail {
		return nil, fmt.Errorf("aucun refus de taille attribuable au résultat courant")
	}
	repo := w.Planning.Repository
	dir := filepath.Join(repo.Storage, "proofs", managedProofKey(t, a))
	data, e := os.ReadFile(filepath.Join(dir, "candidate.json"))
	if e != nil {
		return nil, e
	}
	var cp managedReviewCheckpoint
	if e = json.Unmarshal(data, &cp); e != nil {
		return nil, e
	}
	if cp.Previous != repo.Candidate || cp.Result != item.Result || cp.Contract != managedReviewContract(w, task) || cp.Candidate == "" {
		return nil, fmt.Errorf("candidat de précontrôle périmé")
	}
	receipt, e := os.ReadFile(filepath.Join(dir, "receipt.json"))
	if e != nil {
		return nil, e
	}
	var saved struct {
		Agent     string                               `json:"agent_id"`
		Attempt   string                               `json:"attempt_id"`
		Base      string                               `json:"base_commit"`
		Previous  string                               `json:"previous_candidate"`
		Candidate string                               `json:"candidate_commit"`
		Controls  map[string][]ValidationControlResult `json:"controls"`
	}
	if e = json.Unmarshal(receipt, &saved); e != nil {
		return nil, e
	}
	actual, _ := json.Marshal(saved.Controls)
	expected, _ := json.Marshal(cp.Results)
	if saved.Agent != a.ID || saved.Attempt != a.Attempt || saved.Base != item.Base || saved.Previous != cp.Previous || saved.Candidate != cp.Candidate || string(actual) != string(expected) {
		return nil, fmt.Errorf("reçu de précontrôle différent du candidat conservé")
	}
	c, e := s.managedReviewContext(w, a, cp.Candidate, receipt)
	if e != nil {
		return nil, e
	}
	return &managedPreflightRetry{Revision: w.Revision, Agent: a, Item: item, Work: w, Context: c}, nil
}

func (s *Store) prepareManagedPreflightRetry(work, task string) (*managedPreflightRetry, error) {
	prepared, e := s.readManagedPreflightEvidence(work, task)
	if e != nil {
		return nil, e
	}
	w, c := prepared.Work, prepared.Context
	owners, e := managedReviewSourceOwners(w, c)
	if e != nil {
		return nil, e
	}
	_, workflow, e := agentWorkflow("reviewer")
	if e != nil {
		return nil, e
	}
	batches, e := planManagedReviewBatches(managedReviewPrefix(workflow), c, owners)
	calls := len(batches)
	if errors.Is(e, errManagedReviewBatchSize) {
		cfg := w.Planning.Reviewer
		plan, err := planManagedReviewFragments(c, cfg.MaxCalls-cfg.Calls, 2)
		if err != nil {
			return nil, err
		}
		if err = preflightManagedFragmentCalls(managedReviewPrefix(workflow), c, plan); err != nil {
			return nil, err
		}
		calls = len(plan.Packets) + plan.ReservedFinalCalls
	} else if e != nil {
		return nil, e
	}
	if w.Planning.Reviewer.Failure != "" {
		return nil, fmt.Errorf("vérificateur indisponible : %s", w.Planning.Reviewer.Failure)
	}
	if e = s.providerCooldownGuard(w.Planning.Reviewer.Provider); e != nil {
		return nil, e
	}
	if w.Planning.Reviewer.Calls+calls > w.Planning.Reviewer.MaxCalls {
		return nil, fmt.Errorf("budget insuffisant pour les %d lots de revue", calls)
	}
	return prepared, nil
}

// A read-only preview of complete evidence transport; never an execution permit.
func (s *Store) previewManagedFragments(work, task string) (managedReviewFragmentPlan, error) {
	w, err := s.get(work)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	t, err := w.task(task)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	if r := t.IndependentReview; r != nil && r.FragmentJournal != nil {
		if err = s.managedBatchPlanIntact(w, *r); err != nil {
			return managedReviewFragmentPlan{}, err
		}
		if err = s.managedReviewFilesIntact(*r); err != nil {
			return managedReviewFragmentPlan{}, err
		}
		old, j, err := s.readFragmentJournalAnchor(*r)
		if err != nil {
			return managedReviewFragmentPlan{}, err
		}
		return s.managedFragmentReplan(w, *r, old, j)
	}

	p, e := s.readManagedPreflightEvidence(work, task)
	if e != nil {
		return managedReviewFragmentPlan{}, e
	}
	cfg := p.Work.Planning.Reviewer
	return planManagedReviewFragments(p.Context, cfg.MaxCalls-cfg.Calls, 2)
}
