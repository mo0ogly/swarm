//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Historical selection uncertainty, not a task verdict. All original partial
// replies were committed through real isolated reservation/reply transactions.
func selectionCapacityLegacyFixture(t *testing.T, scope string) (*Store, Work, Agent) {
	t.Helper()
	s, w, a, r, p, j, c, replies, prefix := finalStoreFixture(t)
	prompt, err := managedFragmentRequestPromptVersion(prefix, c, p, replies, 1)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(p)
	request := managedFragmentFinalRequest{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw), State: "unknown", Reason: "Necessary whole originals exceed the supplied selection capacity", References: []managedFragmentEvidenceRef{}}
	reply, _ := json.Marshal(request)
	raw, _ = json.Marshal(j)
	f := managedFragmentFinalJournal{Version: 1, InspectionDigest: hash(raw), Calls: []managedFragmentFinalCall{{Phase: "selection", CallID: "historical-selection", PromptDigest: hash([]byte(prompt)), State: "unknown", Reply: string(reply), ReplyDigest: hash(reply)}}}
	raw, _ = json.Marshal(f)
	name := filepath.ToSlash(filepath.Join(filepath.Dir(r.Context), "legacy-selection-"+hash(raw)+".json"))
	if err = os.WriteFile(filepath.Join(s.root, name), raw, 0600); err != nil {
		t.Fatal(err)
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "fixture.legacy-selection", "fixture-legacy-selection", w.Revision, []byte(`{}`), func(current *Work) error {
		if scope != "root" {
			task, _ := current.task(a.TaskID)
			owned := append([]string(nil), task.Requirements...)
			root, _ := current.Planning.scope("root")
			remaining := []string{}
			for _, requirement := range root.Requirements {
				if !containsString(owned, requirement) {
					remaining = append(remaining, requirement)
				}
			}
			root.Requirements = remaining
			current.Planning.Scopes = append(current.Planning.Scopes, PlanningScope{ID: scope, Parent: "root", State: "waiting", Revision: 1, Requirements: owned})
			for i := range current.Tasks {
				for _, requirement := range current.Tasks[i].Requirements {
					if containsString(owned, requirement) {
						current.Tasks[i].ScopeID = scope
					}
				}
			}
		}
		task, _ := current.task(a.TaskID)
		task.ScopeID = scope
		task.IndependentReview.FragmentJournal.FinalJournal = name
		task.IndependentReview.FragmentJournal.FinalJournalDigest = hash(raw)
		task.IndependentReview.FragmentJournal.InspectionTask = a.TaskID
		task.IndependentReview.FragmentJournal.InspectionAttempt = a.Attempt
		task.IndependentReview.FragmentJournal.InspectionProducer = a.ID
		task.IndependentReview.GitReport = c.Tasks[0].Binding.Report
		current.Planning.Reviewer.Calls++ // The immutable legacy selection was paid.
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.finishManagedFragmentReview(w.ID, a, r.ID, nil); err == nil {
		t.Fatal("historical uncertainty accepted")
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "fixture.blocked", "fixture-capacity-blocked", w.Revision, []byte(`{}`), func(current *Work) error {
		task, _ := current.task(a.TaskID)
		task.Status = "blocked"
		for _, event := range current.Planning.Inbox {
			if event.ID == task.IndependentReview.ResultEvent {
				current.Planning.Inbox = []PlanningEvent{event}
				return nil
			}
		}
		t.Fatal("actual original result absent")
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return s, w, a
}

func TestSelectionCapacityRecoverySupervisorReceivesDecidesAndHasActualEffect(t *testing.T) {
	for _, scope := range []string{"root", "child"} {
		t.Run(scope, func(t *testing.T) {
			s, w, a := selectionCapacityLegacyFixture(t, scope)
			task, _ := w.task(a.TaskID)
			original := *task.IndependentReview
			originalBytes, err := os.ReadFile(filepath.Join(s.root, original.FragmentJournal.FinalJournal))
			if err != nil {
				t.Fatal(err)
			}
			w, ack := planningClaim(t, s, w, scope)
			w, err = s.planningChange(w.ID, "decide", ack)
			if err != nil {
				t.Fatal(err)
			}
			w, err = s.planningChange(w.ID, "resume", PlanningRequest{Schema: 1, EventID: "fresh-capacity-correction", Revision: w.Revision, Scope: scope, Reason: "Installed verified final capacity calculation; whole originals now fit without changing candidate or original spending."})
			if err != nil {
				t.Fatal(err)
			}
			if task, _ := w.task(a.TaskID); task.IndependentReview.State != "unknown" {
				t.Fatal("fixture did not preserve actual historical uncertainty", task.IndependentReview.State, task.IndependentReview.Reason)
			}
			raw, _, err := s.planningDeliveryContext(w, scope, 64000)
			if err != nil {
				t.Fatal(err)
			}
			var projected struct {
				Routing  map[string]reviewFailureRouting `json:"review_failure_routing"`
				Recovery map[string]struct {
					Before   int    `json:"previous_selection_bytes"`
					After    int    `json:"current_selection_bytes"`
					Original string `json:"original_final_journal_sha256"`
				} `json:"selection_capacity_recovery"`
				Unavailable map[string]string `json:"selection_capacity_recovery_unavailable"`
			}
			if err = json.Unmarshal(raw, &projected); err != nil {
				t.Fatal(err)
			}
			info := projected.Recovery[a.TaskID]
			if projected.Routing[a.TaskID].Code != "review_selection_capacity_changed" || info.After <= info.Before || info.Original != original.FragmentJournal.FinalJournalDigest {
				t.Fatal("supervisor did not receive verified changed selection precondition", projected)
			}
			w, decision := planningClaim(t, s, w, scope)
			if containsString(decision.Inputs, original.ResultEvent) {
				t.Fatal("original acknowledgement reopened")
			}
			decision.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "Measured whole-artifact capacity is corrected on immutable received inspections. Select every necessary original, retain all adverse local opinions and an independent final decision."}}
			after, err := s.planningChange(w.ID, "decide", decision)
			if err != nil {
				t.Fatal(err)
			}
			queued, _ := after.task(a.TaskID)
			if queued.IndependentReview.State != "queued" || queued.IndependentReview.CandidateSHA != original.CandidateSHA || queued.PlanningRetry || len(queued.Attempts) != len(task.Attempts) || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || len(queued.PreviousReviews) == 0 {
				t.Fatal("replan spent/queued production or lost original", queued)
			}
			for _, event := range after.Planning.Inbox {
				if event.ID == original.ResultEvent && event.Decision != ack.EventID {
					t.Fatal("original decision changed")
				}
			}
			preserved, err := os.ReadFile(filepath.Join(s.root, original.FragmentJournal.FinalJournal))
			if err != nil || string(preserved) != string(originalBytes) {
				t.Fatal("original final journal rewritten", err)
			}
			replayed, err := s.planningChange(w.ID, "decide", decision)
			if err != nil || replayed.Revision != after.Revision {
				t.Fatal("decision replay spent or duplicated", err)
			}
			provider := strings.Replace(fragmentRuntimeProviderFixture, "text=sys.stdin.read()", "text=sys.stdin.read()\nwith open(os.path.join(os.path.dirname(__file__),'received.jsonl'),'a') as f: f.write(json.dumps(text)+'\\n')", 1)
			if err = os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(provider), 0700); err != nil {
				t.Fatal(err)
			}
			managedReviewMode(t, s, "decision-fail")
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			active, review, err := reopened.activateManagedFragmentResume(w.ID, a, original.ID)
			if err != nil {
				t.Fatal(err)
			}
			state, _, err := reopened.runManagedFragmentReview(active, a, review)
			if err != nil || state != "changes_requested" {
				t.Fatal("new selection did not reach independent actual refusal", state, err)
			}
			if err = reopened.finishManagedFragmentReview(w.ID, a, review.ID, nil); err == nil {
				t.Fatal("independent refusal accepted")
			}
			current, err := reopened.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			finished, _ := current.task(a.TaskID)
			if current.Planning.Reviewer.Calls != after.Planning.Reviewer.Calls+2 || finished.Status == "accepted" || finished.IndependentReview.State != "changes_requested" {
				t.Fatal("exact final spending or independent refusal lost", finished)
			}
			prompts, err := os.ReadFile(filepath.Join(s.root, "review-fixture/received.jsonl"))
			if err != nil {
				t.Fatal(err)
			}
			lines := strings.Split(strings.TrimSpace(string(prompts)), "\n")
			if len(lines) != 2 {
				t.Fatal("inspections repeated", len(lines))
			}
			for _, line := range lines {
				var prompt string
				if err = json.Unmarshal([]byte(line), &prompt); err != nil {
					t.Fatal(err)
				}
				for _, text := range []string{decision.EventID, decision.Operations[0].Next, original.CandidateSHA, original.ResultEvent, "review_selection_capacity_changed"} {
					if !strings.Contains(prompt, text) {
						t.Fatal("actual received directive unbound", text)
					}
				}
			}
		})
	}
}

func TestSelectionCapacityRecoveryRefusalsPreserveOriginalAndSpending(t *testing.T) {
	for _, mode := range []string{"scope", "receipt", "history", "budget", "active", "live-terminal", "provider", "model", "context", "journal", "candidate", "contract", "attempt", "unchanged-next", "foreign-fields", "stale-revision"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := selectionCapacityLegacyFixture(t, "root")
			w, decision := planningClaim(t, s, w, "root")
			decision.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "Verified changed final transport capacity; retain original findings and request an independent final opinion."}}
			task, _ := w.task(a.TaskID)
			original := task.IndependentReview.FragmentJournal.FinalJournalDigest
			switch mode {
			case "scope":
				decision.Scope = "foreign"
			case "receipt":
				owner, _ := w.Planning.scope("root")
				route := owner.Delivery.UnresolvedReviews[a.TaskID]
				route.Candidate = "foreign"
				owner.Delivery.UnresolvedReviews[a.TaskID] = route
			case "history":
				owner, _ := w.Planning.scope("root")
				owner.Delivery.HistoricalOnly = true
			case "budget":
				w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls + 1
			case "active", "live-terminal":
				current, err := s.agent(a.ID)
				if err != nil {
					t.Fatal(err)
				}
				if mode == "active" {
					current.Status = "running"
					current.Ended = ""
				} else {
					current.Child = os.Getpid()
					current.ChildStamp = processStamp(current.Child)
					current.Host = hostIdentity()
				}
				raw, _ := json.Marshal(current)
				if _, err = s.db.Exec("UPDATE agents SET body=?,status=? WHERE id=?", raw, current.Status, a.ID); err != nil {
					t.Fatal(err)
				}
			case "provider":
				w.Planning.Reviewer.ProviderDigest = strings.Repeat("f", 64)
			case "model":
				task.IndependentReview.FragmentJournal.ModelConfigDigest = strings.Repeat("f", 64)
			case "context":
				if err := os.WriteFile(filepath.Join(s.root, task.IndependentReview.Context), []byte(`{}`), 0600); err != nil {
					t.Fatal(err)
				}
			case "journal":
				if err := os.WriteFile(filepath.Join(s.root, task.IndependentReview.FragmentJournal.Journal), []byte(`{}`), 0600); err != nil {
					t.Fatal(err)
				}
			case "candidate":
				task.IndependentReview.CandidateSHA = "foreign"
			case "contract":
				task.Criteria = append(task.Criteria, "new criterion")
			case "attempt":
				task.IndependentReview.Attempt = "foreign"
			case "unchanged-next":
				decision.Operations[0].Next = task.Next
			case "foreign-fields":
				decision.Operations[0].Criteria = []string{"invented"}
			case "stale-revision":
				decision.Revision--
			}
			raw, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			if _, err := s.planningChange(w.ID, "decide", decision); err == nil {
				t.Fatal("invalid recovery admitted", mode)
			}
			after, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			afterRaw, _ := json.Marshal(after)
			current, _ := after.task(a.TaskID)
			if string(afterRaw) != string(raw) || current.IndependentReview.FragmentJournal.FinalJournalDigest != original || current.Status != "blocked" {
				t.Fatal("refused transaction modified originals or spending")
			}
		})
	}
}

func TestSelectionCapacityRecoveryRequiresHistoricalSelectionAndExplicitOperator(t *testing.T) {
	for _, mode := range []string{"current-selection", "business-verdict", "operator-confirmation", "no-fresh-input"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := selectionCapacityLegacyFixture(t, "root")
			task, _ := w.task(a.TaskID)
			switch mode {
			case "operator-confirmation":
				_, err := s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "unconfirmed-retry", Revision: w.Revision, Task: a.TaskID, Reason: "Capacity changed under an independently verified contract."})
				if err == nil {
					t.Fatal("operator confirmation omitted")
				}
				return
			case "no-fresh-input":
				w, ack := planningClaim(t, s, w, "root")
				w, err := s.planningChange(w.ID, "decide", ack)
				if err != nil {
					t.Fatal(err)
				}
				raw, receipt, err := s.planningDeliveryContext(w, "root", 64000)
				if err != nil {
					t.Fatal(err)
				}
				var projected map[string]json.RawMessage
				json.Unmarshal(raw, &projected)
				if string(projected["selection_capacity_recovery"]) != "{}" || len(receipt.UnresolvedReviews) != 0 {
					t.Fatal("acknowledgement alone authorized another call")
				}
				return
			case "business-verdict":
				task.IndependentReview.ManagedTasks = []ManagedTaskReview{{Task: a.TaskID}}
			case "current-selection":
				p, j, err := s.readFragmentJournalAnchor(*task.IndependentReview)
				if err != nil {
					t.Fatal(err)
				}
				f, c, prefix, err := s.readManagedFragmentFinal(*task.IndependentReview, p, j)
				if err != nil {
					t.Fatal(err)
				}
				reusable, err := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
				if err != nil {
					t.Fatal(err)
				}
				replies := make([]string, len(p.Packets))
				for i := range replies {
					replies[i] = reusable[i]
				}
				prompt, err := managedFragmentRequestPromptVersion(prefix, c, p, replies, 2)
				if err != nil {
					t.Fatal(err)
				}
				f.Version = 2
				f.Calls[0].PromptDigest = hash([]byte(prompt))
				raw, _ := json.Marshal(f)
				name := filepath.ToSlash(filepath.Join(filepath.Dir(task.IndependentReview.Context), "current-selection-"+hash(raw)+".json"))
				if err := os.WriteFile(filepath.Join(s.root, name), raw, 0600); err != nil {
					t.Fatal(err)
				}
				task.IndependentReview.FragmentJournal.FinalJournal = name
				task.IndependentReview.FragmentJournal.FinalJournalDigest = hash(raw)
			}
			prepared, err := s.prepareSelectionCapacityRestart(w, task)
			if err != nil || prepared != nil {
				t.Fatal("business/current unknown gained retry authority", mode, prepared, err)
			}
		})
	}
}
