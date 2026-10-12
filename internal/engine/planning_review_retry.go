//go:build linux

package engine

import "fmt"

// Supervisors propose recovery; the engine verifies scope, event attribution,
// candidate integrity and the existing call budget. This queues no worker and
// makes no provider call. The conductor later spends its normal reservations.
func (s *Store) applyPlanningReviewRetry(w *Work, scope string, op PlanningOperation, request PlanningRequest) error {
	t, err := w.task(op.ID)
	if err != nil {
		return err
	}
	selection := request.selectionRestarts[op.ID]
	if t.ScopeID != scope || t.Status != "blocked" || t.LaunchHeld || (!currentReviewProtocolFailure(t) && selection == nil) || w.Planning.Repository == nil || w.Planning.Reviewer == nil {
		return fmt.Errorf("reprise du protocole de revue indisponible dans ce périmètre")
	}
	r := t.IndependentReview
	matched := false
	owner, ownerErr := w.Planning.scope(scope)
	expected := planningReviewFailureRouting(*w, w.Planning.Inbox)[t.ID]
	if selection != nil {
		expected = selection.Source
	}
	for _, event := range w.Planning.Inbox {
		if event.ID != reviewPlanningEventID(r) || event.Kind != "independent_review" || event.Scope != scope || event.Task != t.ID || event.Attempt != r.Attempt {
			continue
		}
		if event.Decision == "" {
			matched = containsString(request.Inputs, event.ID)
			if selection != nil {
				matched = matched && ownerErr == nil && owner.Delivery != nil && !owner.Delivery.HistoricalOnly && !owner.Delivery.ReviewRetryUnavailable && owner.Delivery.UnresolvedReviews[t.ID] == expected
			}
			continue
		}
		// Only the frozen receipt of a fresh activation can refer to an already
		// acknowledged incident. Verify its exact identity; never reopen it.
		if ownerErr != nil || owner.Delivery == nil || owner.Delivery.HistoricalOnly || owner.Delivery.ReviewRetryUnavailable || len(request.Inputs) == 0 {
			continue
		}
		fresh := true
		for _, input := range request.Inputs {
			fresh = fresh && containsString(owner.Delivery.Events, input)
		}
		received, exists := owner.Delivery.UnresolvedReviews[t.ID]
		matched = fresh && exists && received == expected
	}
	if !matched || r.Contract != reviewContract(t) || r.CandidateSHA == "" || r.PreviousCandidate != w.Planning.Repository.Candidate || r.FragmentJournal == nil {
		return fmt.Errorf("preuve, contrat ou candidat de reprise remplacé")
	}
	if len(op.Next) < 8 || len(op.Next) > 2000 || !nonempty(op.Next) || op.Next == t.Next {
		return fmt.Errorf("décrire une correction nouvelle du protocole avant reprise (8 à 2000 caractères)")
	}
	if op.Title != "" || len(op.Requirements) != 0 || op.Deliverable != "" || len(op.Criteria) != 0 || len(op.Depends) != 0 || op.MaxTasks != 0 || op.MaxActivations != 0 {
		return fmt.Errorf("review-retry accepte uniquement id et next ; aucune modification implicite du plan")
	}
	if err := s.providerCooldownGuard(w.Planning.Reviewer.Provider); err != nil {
		return err
	}
	if prepared := request.methodRestarts[op.ID]; prepared != nil {
		return s.applyReviewMethodRestart(w, t, prepared, op.Next)
	}
	if selection != nil {
		directive := &ManagedReviewRetryDirective{Decision: request.EventID, Scope: scope, ResultEvent: reviewPlanningEventID(r), Review: r.ID, Candidate: r.CandidateSHA, Attempt: r.Attempt, FailureCode: reviewSelectionCapacityChanged, Cause: r.Reason, Instruction: op.Next}
		if err := s.applySelectionCapacityRestart(w, t, selection, directive); err != nil {
			return err
		}
		t.Next = op.Next
		w.Planning.Reviewer.Failure = ""
		return nil
	}
	if err := s.queueManagedFragmentResume(*w, t, request.EventID); err != nil {
		return err
	}
	directive := &ManagedReviewRetryDirective{Decision: request.EventID, Scope: scope, ResultEvent: reviewPlanningEventID(r), Review: r.ID, Candidate: r.CandidateSHA, Attempt: r.Attempt, FailureCode: reviewFragmentProtocolError, Cause: r.Reason, Instruction: op.Next}
	if err := s.preflightReviewRetryDirective(*t.IndependentReview, directive); err != nil {
		return fmt.Errorf("consigne de reprise non transmissible avant réservation : %w", err)
	}
	t.IndependentReview.FragmentJournal.RetryDirective = directive
	t.PlanningRetry = false
	t.Next = op.Next
	w.Planning.Reviewer.Failure = ""
	return nil
}
