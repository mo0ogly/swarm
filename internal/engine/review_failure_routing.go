package engine

import (
	"encoding/json"
	"errors"
	"strings"
)

const reviewFragmentProtocolError = "review_fragment_protocol"
const reviewFragmentInterruptedError = "review_fragment_interrupted"

func reviewResponseProtocolError(err error) error {
	if err == nil {
		return nil
	}
	var existing *CommandError
	if errors.As(err, &existing) && existing.Code == reviewFragmentProtocolError {
		return err
	}
	return &CommandError{Code: reviewFragmentProtocolError, Message: err.Error()}
}

func reviewPlanningEventID(r *IndependentReview) string {
	if r.ResultEvent != "" {
		return r.ResultEvent
	}
	if r.FragmentJournal == nil {
		return r.ID
	}
	// Historical fragment events contain the digest of their exact terminal
	// record. A merely matching ID prefix is not enough to identify that result.
	raw, _ := json.Marshal(r)
	return r.ID + "-result-" + hash(raw)[:16]
}

// Only the current engine-recorded review can authorize this routing. A worker
// report, obsolete attempt or merely unfavorable semantic verdict cannot.
func currentReviewProtocolFailure(t *Task) bool {
	r := t.IndependentReview
	if r == nil || r.State != "error" || !currentTaskAttempt(t, r.Attempt) {
		return false
	}
	if r.FailureCode != "" {
		return r.FailureCode == reviewFragmentProtocolError || r.FailureCode == reviewFragmentInterruptedError
	}
	// Compatibility with immutable records written before failure_code existed.
	return r.FragmentJournal != nil && strings.HasPrefix(r.Reason, "inspection de fragment : ")
}

type reviewFailureRouting struct {
	ResultEvent        string `json:"result_event"`
	Review             string `json:"review_id"`
	Attempt            string `json:"attempt"`
	Candidate          string `json:"candidate_commit"`
	Code               string `json:"failure_code"`
	Owner              string `json:"owner"`
	Operation          string `json:"operation"`
	ProducerCorrection bool   `json:"producer_correction_required"`
}

func planningReviewFailureRouting(w Work, events []PlanningEvent) map[string]reviewFailureRouting {
	result := map[string]reviewFailureRouting{}
	for _, event := range events {
		if event.Kind != "independent_review" {
			continue
		}
		t, err := w.task(event.Task)
		if err != nil || t.ScopeID != event.Scope || !currentReviewProtocolFailure(t) || reviewPlanningEventID(t.IndependentReview) != event.ID || t.IndependentReview.Attempt != event.Attempt {
			continue
		}
		r := t.IndependentReview
		code, owner := reviewFragmentProtocolError, "reviewer_protocol"
		if r.FailureCode == reviewFragmentInterruptedError {
			code, owner = reviewFragmentInterruptedError, "reviewer_runtime"
		}
		result[t.ID] = reviewFailureRouting{ResultEvent: event.ID, Review: r.ID, Attempt: r.Attempt, Candidate: r.CandidateSHA, Code: code, Owner: owner, Operation: "review-retry", ProducerCorrection: false}
	}
	return result
}

// A receipt does not resolve a still-current incident. Return acknowledged
// originals separately from pending inputs; their prior decision is immutable.
// This cannot wake a scope by itself or authorize an unchanged automatic retry.
func planningUnresolvedReviewIncidents(w Work, scope string) []PlanningEvent {
	incidents := []PlanningEvent{}
	if w.Planning == nil {
		return incidents
	}
	for _, event := range w.Planning.Inbox {
		if event.Scope != scope || event.Decision == "" || event.Kind != "independent_review" {
			continue
		}
		task, err := w.task(event.Task)
		if err != nil || task.ScopeID != scope || task.Status != "blocked" || task.LaunchHeld || !currentReviewProtocolFailure(task) || reviewPlanningEventID(task.IndependentReview) != event.ID || task.IndependentReview.Attempt != event.Attempt {
			continue
		}
		incidents = append(incidents, event)
	}
	return incidents
}
