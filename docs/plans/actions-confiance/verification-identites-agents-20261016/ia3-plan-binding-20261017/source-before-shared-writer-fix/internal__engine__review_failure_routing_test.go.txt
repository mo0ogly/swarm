//go:build linux

package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestReviewProtocolFailureRoutingIsCurrentAndScoped(t *testing.T) {
	for _, scope := range []string{"root", "child"} {
		t.Run(scope, func(t *testing.T) {
			s, w := managedFixture(t)
			w.Planning.Scopes = append(w.Planning.Scopes, PlanningScope{ID: "child", Parent: "root"})
			task := &w.Tasks[0]
			task.ScopeID, task.Status = scope, "blocked"
			task.Attempts = []Attempt{{ID: "attempt"}}
			task.IndependentReview = &IndependentReview{ID: "review-error", Attempt: "attempt", State: "error", Reason: "inspection de fragment : defect quote absent at original line", CandidateSHA: strings.Repeat("a", 40), FragmentJournal: &ManagedFragmentJournalAnchor{}}
			event := PlanningEvent{ID: reviewPlanningEventID(task.IndependentReview), Kind: "independent_review", Task: task.ID, Scope: scope, Attempt: "attempt"}
			w.Planning.Inbox = []PlanningEvent{event}
			raw, _, err := s.planningDeliveryContext(w, scope, 64000)
			if err != nil {
				t.Fatal(err)
			}
			var context struct {
				Routing map[string]reviewFailureRouting `json:"review_failure_routing"`
			}
			if err := json.Unmarshal(raw, &context); err != nil {
				t.Fatal(err)
			}
			route := context.Routing[task.ID]
			if route.Review != task.IndependentReview.ID || route.Attempt != event.Attempt || route.Candidate != task.IndependentReview.CandidateSHA || route.Owner != "reviewer_protocol" || route.Operation != "review-retry" || route.ProducerCorrection {
				t.Fatal(route)
			}
			for _, mode := range []string{"older review", "other attempt", "other scope", "worker handoff"} {
				changed := event
				switch mode {
				case "older review":
					changed.ID = "old"
				case "other attempt":
					changed.Attempt = "old"
				case "other scope":
					changed.Scope = "foreign"
				case "worker handoff":
					changed.Kind = "handoff"
				}
				if len(planningReviewFailureRouting(w, []PlanningEvent{changed})) != 0 {
					t.Fatal("misattributed routing", mode)
				}
			}
			task.IndependentReview.State = "changes_requested"
			if currentReviewProtocolFailure(task) {
				t.Fatal("semantic refusal treated as protocol failure")
			}
			task.IndependentReview.State, task.IndependentReview.FailureCode = "error", "other_error"
			if currentReviewProtocolFailure(task) {
				t.Fatal("legacy message overrides explicit error code")
			}
		})
	}
}

func planningProtocolReviewFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w, a, r, _, _ := fragmentStoreFixture(t)
	if err := s.finishManagedFragmentReview(w.ID, a, r.ID, &CommandError{Code: reviewFragmentProtocolError, Message: "inspection de fragment : defect quote absent at original line"}); err == nil {
		t.Fatal("protocol failure accepted")
	}
	w, _ = s.get(w.ID)
	var err error
	w, err = s.mutate(w.ID, "test.protocol-error", "protocol-error", w.Revision, []byte(`{}`), func(current *Work) error {
		task, _ := current.task(a.TaskID)
		task.Status = "blocked"
		for _, event := range current.Planning.Inbox {
			if event.ID == task.IndependentReview.ResultEvent {
				current.Planning.Inbox = []PlanningEvent{event}
				return nil
			}
		}
		t.Fatal("engine result event absent")
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return s, w, a
}

func TestPlanningProtocolFailureRejectsProducerRetry(t *testing.T) {
	s, w, a := planningProtocolReviewFixture(t)
	w, request := planningClaim(t, s, w, "root")
	request.Operations = []PlanningOperation{{Kind: "retry", ID: a.TaskID, Next: "Regenerate the producer report with different quotes."}}
	if _, err := s.planningChange(w.ID, "decide", request); err == nil || commandFailure(err).Code != "review_recovery_required" {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if after.Revision != w.Revision || task.PlanningRetry || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || after.Planning.Decisions != w.Planning.Decisions {
		t.Fatal("rejected decision changed state")
	}
	task.PlanningRetry = true
	in := dispatchTest([]Task{*task}, []Agent{{ID: a.ID, TaskID: task.ID, Status: "completed"}})
	if departures, reason := planDispatch(in); len(departures) != 0 || !strings.Contains(reason, "vérificateur") {
		t.Fatal("obsolete producer authorization bypassed protocol routing", departures, reason)
	}
	if state, reason := missionDispatchState(in, *task); state != "intervention" || !strings.Contains(reason, "vérificateur") {
		t.Fatal("console did not explain the actual dispatch guard", state, reason)
	}
}

func TestPlanningReviewRetryQueuesSameCandidateWithoutProductionOrSpend(t *testing.T) {
	s, w, a := planningProtocolReviewFixture(t)
	w, request := planningClaim(t, s, w, "root")
	before, _ := w.task(a.TaskID)
	before.PlanningRetry = true // Simulate the obsolete producer route in the existing mission.
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	request.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "Quote validation corrected to match exact bytes on the declared line; preserve the candidate."}}
	after, err := s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := after.task(a.TaskID)
	item, err := s.managedAttempt(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	if task.IndependentReview.State != "queued" || task.IndependentReview.FailureCode != "" || task.PlanningRetry || task.IndependentReview.CandidateSHA != before.IndependentReview.CandidateSHA || len(task.Attempts) != len(before.Attempts) || item.State != "integrating" || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
		t.Fatal("review route lost identity, spent a call or queued production", task, item)
	}
	if after.Planning.Inbox[0].Decision != request.EventID {
		t.Fatal("input event not acknowledged")
	}
	replayed, err := s.planningChange(w.ID, "decide", request)
	if err != nil || replayed.Revision != after.Revision {
		t.Fatal("retry decision replay was not idempotent", err)
	}
}

func TestPlanningReviewRetryRejectsInvalidRecoveryAtomically(t *testing.T) {
	for _, mode := range []string{"wrong scope", "semantic refusal", "changed candidate", "changed contract", "no budget", "implicit edit", "unchanged correction", "active producer"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := planningProtocolReviewFixture(t)
			task, _ := w.task(a.TaskID)
			op := PlanningOperation{Kind: "review-retry", ID: a.TaskID, Next: "The exact quote locator has been corrected before recovery."}
			switch mode {
			case "wrong scope":
				root, _ := w.Planning.scope("root")
				owned := append([]string(nil), root.Requirements...)
				root.Requirements = nil
				w.Planning.Scopes = append(w.Planning.Scopes, PlanningScope{ID: "foreign", Parent: "root", Objective: "Independent scope", Requirements: owned, Revision: 1, State: "waiting"})
				for i := range w.Tasks {
					w.Tasks[i].ScopeID = "foreign"
				}
			case "semantic refusal":
				task.IndependentReview.State = "changes_requested"
			case "changed candidate":
				w.Planning.Repository.Candidate = strings.Repeat("f", 40)
			case "changed contract":
				task.IndependentReview.Contract = "old"
			case "no budget":
				w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls
			case "implicit edit":
				op.Deliverable = "replacement.md"
			case "unchanged correction":
				op.Next = task.Next
			case "active producer":
				if _, err := s.db.Exec("UPDATE agents SET status='running' WHERE id=?", a.ID); err != nil {
					t.Fatal(err)
				}
			}
			raw, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			w, request := planningClaim(t, s, w, "root")
			request.Operations = []PlanningOperation{op}
			if _, err := s.planningChange(w.ID, "decide", request); err == nil {
				t.Fatal("unsafe recovery accepted")
			}
			after, _ := s.get(w.ID)
			task, _ = after.task(a.TaskID)
			if after.Revision != w.Revision || task.IndependentReview.State == "queued" || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
				t.Fatal("failed recovery mutated persisted state")
			}
		})
	}
}
