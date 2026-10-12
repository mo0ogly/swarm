//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"path/filepath"
	"strings"
)

const reviewSelectionCapacityChanged = "review_selection_capacity_changed"

type selectionCapacityRecovery struct {
	Before      int    `json:"previous_selection_bytes"`
	After       int    `json:"current_selection_bytes"`
	FinalDigest string `json:"original_final_journal_sha256"`
	Reason      string `json:"original_selection_reason"`
	Required    int    `json:"calls_required"`
	Available   int    `json:"calls_available"`
}

type preparedSelectionCapacityRestart struct {
	Revision int
	Source   reviewFailureRouting
	Facts    selectionCapacityRecovery
	Context  string
	Journal  string
}

func managedSelectionCapacity(prefix string, c managedReviewContext, p managedReviewFragmentPlan, replies []string, version int) (int, error) {
	prompt, err := managedFragmentRequestPromptVersion(prefix, c, p, replies, version)
	if err != nil {
		return 0, err
	}
	_, raw, ok := strings.Cut(prompt, "SWARM_FRAGMENT_EVIDENCE_REQUEST\n")
	var supplied struct {
		Limit int `json:"selection_bytes_limit"`
	}
	if !ok || json.Unmarshal([]byte(raw), &supplied) != nil || supplied.Limit <= 0 {
		return 0, fmt.Errorf("capacité de sélection non attribuée")
	}
	return supplied.Limit, nil
}

// Only an independently verified transport-contract change can reopen a
// historical selection-stage uncertainty. A final task unknown/fail, v2
// selection, modified proof, active producer or unchanged capacity cannot.
// This is read-only and spends nothing; the supervisor must still decide.
func (s *Store) prepareSelectionCapacityRestart(w Work, task *Task) (*preparedSelectionCapacityRestart, error) {
	r := task.IndependentReview
	if r == nil || r.State != "unknown" || r.Finished == "" || r.FragmentJournal == nil || r.FragmentJournal.FinalJournalDigest == "" || len(r.ManagedTasks) != 0 {
		return nil, nil
	}
	if w.Planning == nil || w.Planning.Repository == nil || w.Planning.Reviewer == nil || task.Status != "blocked" || task.LaunchHeld || !currentTaskAttempt(task, r.Attempt) || r.Contract != reviewContract(task) || r.CandidateSHA == "" || r.PreviousCandidate != w.Planning.Repository.Candidate {
		return nil, fmt.Errorf("candidat, contrat ou tentative de sélection remplacé")
	}
	if err := s.managedBatchPlanIntact(w, *r); err != nil {
		return nil, err
	}
	if err := s.managedReviewFilesIntact(*r); err != nil {
		return nil, err
	}
	p, j, err := s.readFragmentJournalAnchor(*r)
	if err != nil {
		return nil, err
	}
	f, c, prefix, err := s.readManagedFragmentFinal(*r, p, j)
	if err != nil {
		return nil, err
	}
	if f.Version != 1 || len(f.Calls) == 0 || f.Calls[len(f.Calls)-1].Phase != "selection" {
		return nil, nil
	}
	state, records, err := validateManagedFragmentFinalJournal(f, j, c, p, prefix)
	if err != nil {
		return nil, err
	}
	if state != "unknown" || len(records) != 0 {
		return nil, nil
	}
	reusable, err := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
	if err != nil {
		return nil, err
	}
	replies := make([]string, len(p.Packets))
	for i := range replies {
		reply, ok := reusable[i]
		if !ok {
			return nil, fmt.Errorf("inspection originale incomplète")
		}
		replies[i] = reply
	}
	last := f.Calls[len(f.Calls)-1]
	callPrefix, err := managedReviewRecoveryPrefix(prefix, last.RetryDirective)
	if err != nil {
		return nil, err
	}
	before, err := managedSelectionCapacity(callPrefix, c, p, replies, 1)
	if err != nil {
		return nil, err
	}
	after, err := managedSelectionCapacity(callPrefix, c, p, replies, 2)
	if err != nil {
		return nil, err
	}
	if after <= before {
		return nil, nil
	}
	request, err := parseManagedFragmentFinalRequest(last.Reply, callPrefix, c, p, replies)
	if err != nil {
		return nil, err
	}
	source := reviewFailureRouting{ResultEvent: reviewPlanningEventID(r), Review: r.ID, Attempt: r.Attempt, Candidate: r.CandidateSHA, Code: reviewSelectionCapacityChanged, Owner: "reviewer_transport", Operation: "review-retry", ProducerCorrection: false}
	facts := selectionCapacityRecovery{Before: before, After: after, FinalDigest: r.FragmentJournal.FinalJournalDigest, Reason: request.Reason, Required: 2, Available: max(0, w.Planning.Reviewer.MaxCalls-w.Planning.Reviewer.Calls)}
	return &preparedSelectionCapacityRestart{Revision: w.Revision, Source: source, Facts: facts, Context: r.ContextDigest, Journal: r.FragmentJournal.JournalDigest}, nil
}

// Delivery can run inside a Work mutation. Never acquire the Store's single
// database connection again from that projection. Attribution and confirmed
// process termination are checked on the existing transaction at commit.
func validateSelectionProducerTx(tx *sql.Tx, work, task, attempt, producer string) error {
	var raw []byte
	if err := tx.QueryRow("SELECT body FROM agents WHERE id=? AND work_id=? AND task_id=?", producer, work, task).Scan(&raw); err != nil {
		return err
	}
	var a Agent
	if err := json.Unmarshal(raw, &a); err != nil {
		return err
	}
	if a.WorkID != work || a.TaskID != task || a.Attempt != attempt || !recoveryProcessEnded(a, hostIdentity(), processStamp(a.Child)) {
		return fmt.Errorf("producteur de sélection non attribué ou encore actif")
	}
	return nil
}

func (s *Store) planningSelectionCapacityRecovery(w Work, events []PlanningEvent) (map[string]reviewFailureRouting, map[string]selectionCapacityRecovery, map[string]string) {
	routes := map[string]reviewFailureRouting{}
	facts := map[string]selectionCapacityRecovery{}
	unavailable := map[string]string{}
	for _, event := range events {
		if event.Kind != "independent_review" {
			continue
		}
		task, err := w.task(event.Task)
		if err != nil || task.ScopeID != event.Scope || task.IndependentReview == nil || reviewPlanningEventID(task.IndependentReview) != event.ID || task.IndependentReview.Attempt != event.Attempt {
			continue
		}
		prepared, err := s.prepareSelectionCapacityRestart(w, task)
		if err != nil {
			unavailable[task.ID] = err.Error()
			continue
		}
		if prepared != nil {
			routes[task.ID] = prepared.Source
			facts[task.ID] = prepared.Facts
		}
	}
	return routes, facts, unavailable
}

// Apply only after a fresh, exact receipt and explicit authorized decision.
// Keep the original terminal unknown in history. Start a new v2 final journal,
// reuse unchanged inspected originals and reserve new calls only in the runner.
func (s *Store) applySelectionCapacityRestart(w *Work, task *Task, prepared *preparedSelectionCapacityRestart, directive *ManagedReviewRetryDirective) error {
	if prepared == nil || prepared.Revision != w.Revision {
		return fmt.Errorf("précontrôle de capacité remplacé")
	}
	current, err := s.prepareSelectionCapacityRestart(*w, task)
	if err != nil {
		return err
	}
	if current == nil || *current != *prepared {
		return fmt.Errorf("preuves ou capacité de sélection remplacées")
	}
	if prepared.Facts.Available < prepared.Facts.Required {
		return fmt.Errorf("budget final insuffisant : %d appels requis, %d disponibles, sans remboursement", prepared.Facts.Required, prepared.Facts.Available)
	}
	r := *task.IndependentReview
	p, j, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		return err
	}
	_, c, prefix, err := s.readManagedFragmentFinal(r, p, j)
	if err != nil {
		return err
	}
	prefix, err = managedReviewRecoveryPrefix(prefix, directive)
	if err != nil {
		return err
	}
	reusable, err := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
	if err != nil {
		return err
	}
	replies := make([]string, len(p.Packets))
	for i := range replies {
		replies[i] = reusable[i]
	}
	capacity, err := managedSelectionCapacity(prefix, c, p, replies, 2)
	if err != nil {
		return err
	}
	if capacity <= prepared.Facts.Before {
		return fmt.Errorf("nouvelle consigne annule la correction de capacité ; aucune reprise inchangée")
	}
	timeout, err := reviewTimeoutSeconds(w.Planning.Reviewer)
	if err != nil {
		return err
	}
	journalRaw, _ := json.Marshal(j)
	f := managedFragmentFinalJournal{Version: 2, InspectionDigest: hash(journalRaw), Calls: []managedFragmentFinalCall{}}
	raw, err := json.Marshal(f)
	if err != nil {
		return err
	}
	anchor := *r.FragmentJournal
	anchor.FinalJournalDigest = hash(raw)
	anchor.FinalJournal = filepath.ToSlash(filepath.Join(filepath.Dir(r.Context), "fragment-final-"+hash(raw)+".json"))
	anchor.RetryDirective = directive
	if err = atomicWrite(filepath.Join(s.root, anchor.FinalJournal), raw); err != nil {
		return err
	}
	task.PreviousReviews = append(task.PreviousReviews, *task.IndependentReview)
	r.FragmentJournal = &anchor
	r.State = "queued"
	r.FailureCode = ""
	r.Finished = ""
	r.TimeoutSeconds = timeout
	r.Reason = "Replan de sélection autorisé sous capacité vérifiée version2 ; avis original, inspections et dépenses conservés. Aucun verdict métier présumé."
	task.IndependentReview = &r
	task.PlanningRetry = false
	return nil
}
