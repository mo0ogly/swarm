//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestPlanningAcknowledgedProtocolIncidentRemainsRecoverableWithFreshInputAfterReopen(t *testing.T) {
	s, w, a := planningUnresolvedReviewFixture(t)
	w, ack := planningClaim(t, s, w, "root")
	before, _ := w.task(a.TaskID)
	original := *before.IndependentReview
	journalBytes, err := os.ReadFile(filepath.Join(s.root, original.FragmentJournal.Journal))
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.planningChange(w.ID, "decide", ack)
	if err != nil {
		t.Fatal(err)
	}
	originDecision := ""
	for _, event := range w.Planning.Inbox {
		if event.ID == original.ResultEvent {
			originDecision = event.Decision
		}
	}
	if originDecision != ack.EventID {
		t.Fatal("original receipt not acknowledged")
	}
	instruction := "For inspected use no needs; missing evidence requires unknown and a precise request. Preserve independent judgment."
	w, err = s.planningChange(w.ID, "resume", PlanningRequest{Schema: 1, EventID: "fresh-protocol-correction", Revision: w.Revision, Scope: "root", Reason: instruction})
	if err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	raw, _, err := reopened.planningDeliveryContext(w, "root", 64000)
	if err != nil {
		t.Fatal(err)
	}
	var received struct {
		Routing     map[string]reviewFailureRouting `json:"review_failure_routing"`
		Unavailable bool                            `json:"review_retry_unavailable"`
		Incidents   []PlanningEvent                 `json:"unresolved_review_incidents"`
	}
	if err = json.Unmarshal(raw, &received); err != nil {
		t.Fatal(err)
	}
	if received.Unavailable || received.Routing[a.TaskID].Review != original.ID || len(received.Incidents) != 1 || received.Incidents[0].Decision != originDecision {
		t.Fatal("acknowledgement hid unresolved current incident from fresh supervisor activation", received.Unavailable, received.Routing, len(received.Incidents))
	}
	w, request := planningClaim(t, reopened, w, "root")
	if containsString(request.Inputs, original.ResultEvent) || !containsString(request.Inputs, "fresh-protocol-correction") {
		t.Fatal("old acknowledged event was reopened or fresh input omitted")
	}
	request.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: instruction}}
	after, err := reopened.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	for _, event := range after.Planning.Inbox {
		if event.ID == original.ResultEvent && event.Decision != originDecision {
			t.Fatal("original decision overwritten")
		}
	}
	kept, err := os.ReadFile(filepath.Join(s.root, original.FragmentJournal.Journal))
	if err != nil || string(kept) != string(journalBytes) {
		t.Fatal("original journal overwritten", err)
	}
	provider := strings.Replace(fragmentRuntimeProviderFixture, "text=sys.stdin.read()", "text=sys.stdin.read()\nwith open(os.path.join(os.path.dirname(__file__),'received.jsonl'),'a') as f: f.write(json.dumps(text)+'\\n')\nif "+reviewRetryPythonLiteral(instruction)+" not in text: sys.exit(19)", 1)
	if err = os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(provider), 0700); err != nil {
		t.Fatal(err)
	}
	task, _ := after.task(a.TaskID)
	active, review, err := reopened.activateManagedFragmentResume(w.ID, a, task.IndependentReview.ID)
	if err != nil {
		t.Fatal(err)
	}
	state, _, runErr := reopened.runManagedFragmentReview(active, a, review)
	if runErr != nil || state != "passed" {
		t.Fatal("fresh recovery had no actual effect", state, runErr)
	}
	prompts, err := os.ReadFile(filepath.Join(s.root, "review-fixture/received.jsonl"))
	if err != nil {
		t.Fatal(err)
	}
	for _, line := range strings.Split(strings.TrimSpace(string(prompts)), "\n") {
		var prompt string
		if err = json.Unmarshal([]byte(line), &prompt); err != nil {
			t.Fatal(err)
		}
		for _, value := range []string{instruction, request.EventID, original.ID, original.ResultEvent, original.CandidateSHA} {
			if !strings.Contains(prompt, value) {
				t.Fatal("attribution not received", value)
			}
		}
	}
	if err = reopened.finishManagedFragmentReview(w.ID, a, review.ID, nil); err != nil {
		t.Fatal(err)
	}
	current, err := reopened.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = current.task(a.TaskID)
	if task.IndependentReview.State != "passed" || task.Status == "accepted" || task.PlanningRetry || len(task.Attempts) != len(before.Attempts) || current.Planning.Reviewer.Calls <= w.Planning.Reviewer.Calls {
		t.Fatal("recovery bypassed controls, created production or lost consumption")
	}
}

// Use runtime initialization, including actual producer/task/attempt anchors.
// The older journal-only fixture intentionally cannot prove final publication.
func planningUnresolvedReviewFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	if s.finishManagedFragmentReview(w.ID, a, r.ID, &CommandError{Code: reviewFragmentProtocolError, Message: "inspection de fragment : supplied needs on inspected artifact"}) == nil {
		t.Fatal("initial protocol error accepted")
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "fixture.blocked", "fixture-blocked", w.Revision, []byte(`{}`), func(current *Work) error {
		task, _ := current.task(a.TaskID)
		task.Status = "blocked"
		for _, event := range current.Planning.Inbox {
			if event.ID == task.IndependentReview.ResultEvent {
				current.Planning.Inbox = []PlanningEvent{event}
				return nil
			}
		}
		t.Fatal("actual result missing")
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return s, w, a
}

func TestPlanningUnresolvedReviewIncidentRemainsExactScopedAndReadOnly(t *testing.T) {
	for _, mode := range []string{"current", "old-review", "other-attempt", "foreign-scope", "semantic-refusal", "held", "missing-origin", "no-fresh-input"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := planningUnresolvedReviewFixture(t)
			task, _ := w.task(a.TaskID)
			w.Planning.Inbox[0].Decision = "prior-supervisor-receipt"
			switch mode {
			case "old-review":
				w.Planning.Inbox[0].ID = "other-review-result"
			case "other-attempt":
				w.Planning.Inbox[0].Attempt = "other-attempt"
			case "foreign-scope":
				w.Planning.Inbox[0].Scope = "child"
			case "semantic-refusal":
				task.IndependentReview.State = "changes_requested"
			case "held":
				task.LaunchHeld = true
			case "missing-origin":
				w.Planning.Inbox = nil
			}
			if mode != "no-fresh-input" {
				w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "fresh-replan", Scope: "root", Kind: "operator_resume", Message: "New method installed; examine current protocol incident."})
			}
			before, _ := json.Marshal(w)
			raw, delivery, err := s.planningDeliveryContext(w, "root", 64000)
			if err != nil {
				t.Fatal(err)
			}
			var received struct {
				Routing   map[string]reviewFailureRouting `json:"review_failure_routing"`
				Incidents []PlanningEvent                 `json:"unresolved_review_incidents"`
			}
			if err = json.Unmarshal(raw, &received); err != nil {
				t.Fatal(err)
			}
			want := mode == "current"
			if (len(received.Routing) > 0) != want || (len(delivery.UnresolvedReviews) > 0) != want || delivery.ReviewRetryUnavailable == want {
				t.Fatal("recovery scope/identity boundary", mode, delivery, received)
			}
			if want {
				encoded, _ := json.Marshal(delivery)
				var persisted PlanningDelivery
				if err = json.Unmarshal(encoded, &persisted); err != nil || !reflect.DeepEqual(persisted.UnresolvedReviews, delivery.UnresolvedReviews) {
					t.Fatal("received incident binding lost on round trip", err)
				}
			}
			after, _ := json.Marshal(w)
			if string(before) != string(after) {
				t.Fatal("projection modified source acknowledgements or spending")
			}
		})
	}
}

func TestPlanningAcknowledgedIncidentDifferentReceivedCandidateCannotQueueOrSpend(t *testing.T) {
	s, w, a := planningUnresolvedReviewFixture(t)
	w, ack := planningClaim(t, s, w, "root")
	var err error
	w, err = s.planningChange(w.ID, "decide", ack)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.planningChange(w.ID, "resume", PlanningRequest{Schema: 1, EventID: "changed-precondition", Revision: w.Revision, Scope: "root", Reason: "A corrected review method is available; inspect the original error."})
	if err != nil {
		t.Fatal(err)
	}
	w, req := planningClaim(t, s, w, "root")
	// Deliberately corrupt only this isolated receipt to exercise the public guard.
	owner, _ := w.Planning.scope("root")
	received := owner.Delivery.UnresolvedReviews[a.TaskID]
	received.Candidate = "unreceived-candidate"
	owner.Delivery.UnresolvedReviews[a.TaskID] = received
	raw, _ := json.Marshal(w)
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	req.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "Inspected findings must have an empty needs list; missing evidence requires unknown."}}
	if _, err = s.planningChange(w.ID, "decide", req); err == nil {
		t.Fatal("unreceived candidate authorized recovery")
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := current.task(a.TaskID)
	if current.Revision != w.Revision || current.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || task.IndependentReview.State != "error" || task.IndependentReview.FragmentJournal.RetryDirective != nil {
		t.Fatal("invalid recovery changed state or budget")
	}
}

func TestUnresolvedReviewStorageUpgradePreservesReceivedIncidentWithoutRewriting(t *testing.T) {
	s, w, _ := planningUnresolvedReviewFixture(t)
	w, ack := planningClaim(t, s, w, "root")
	var err error
	w, err = s.planningChange(w.ID, "decide", ack)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.planningChange(w.ID, "resume", PlanningRequest{Schema: 1, EventID: "new-precondition-before-migration", Revision: w.Revision, Scope: "root", Reason: "A corrected review method is available; diagnose the current protocol error."})
	if err != nil {
		t.Fatal(err)
	}
	w, _ = planningClaim(t, s, w, "root")
	owner, _ := w.Planning.scope("root")
	if len(owner.Delivery.UnresolvedReviews) != 1 {
		t.Fatal("missing actual incident receipt")
	}
	var before []byte
	if err = s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&before); err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("PRAGMA user_version=28"); err != nil {
		t.Fatal(err)
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reader, err := openStoreWithMigration(s.root, false, false)
	if err == nil {
		reader.db.Close()
		t.Fatal("v28 storage can lose new receipt identities")
	}
	if commandFailure(err).Code != "storage_upgrade_required" {
		t.Fatal(err)
	}
	upgraded, err := openStoreWithMigration(s.root, false, true)
	if err != nil {
		t.Fatal(err)
	}
	defer upgraded.db.Close()
	var after []byte
	if err = upgraded.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&after); err != nil {
		t.Fatal(err)
	}
	if string(before) != string(after) {
		t.Fatal("upgrade rewrote work or original receipt")
	}
	restored, err := upgraded.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	received, _ := restored.Planning.scope("root")
	if !reflect.DeepEqual(received.Delivery.UnresolvedReviews, owner.Delivery.UnresolvedReviews) || restored.Revision != w.Revision || restored.Planning.Activations != w.Planning.Activations || restored.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
		t.Fatal("upgrade lost received incident binding or consumed history")
	}
	backups, err := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v29-*.db"))
	if err != nil || len(backups) != 1 {
		t.Fatal("private original backup missing", err)
	}
	st, err := os.Stat(backups[0])
	if err != nil || st.Mode().Perm() != 0600 {
		t.Fatal("original backup permissions", err)
	}
}
