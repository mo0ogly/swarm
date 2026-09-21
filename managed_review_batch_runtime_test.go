//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func managedBatchRuntimeFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s := storeTest(t)
	source := filepath.Join(s.root, "project")
	os.MkdirAll(source, 0700)
	gitTest(t, source, "init")
	for name, content := range map[string]string{"value.txt": "initial\n", "one.go.txt": strings.Repeat("a", 60000), "two.go.txt": strings.Repeat("b", 60000)} {
		os.WriteFile(filepath.Join(source, name), []byte(content), 0600)
	}
	gitTest(t, source, "add", ".")
	gitTest(t, source, "commit", "-m", "base")
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 20, MaxActivations: 30, Repository: &ManagedRepositoryRequest{Path: source, CommittedOnly: true}, Checks: map[string][]ValidationControl{"req-1": automaticPolicy("git", "diff", "--exit-code").Controls}})
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("first"), planningTask("second")}
	var err error
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	if err = organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 2); err != nil {
		t.Fatal(err)
	}
	if err = s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	w = managedReviewFixture(t, s, w)
	fixture := strings.Replace(managedReviewerFixture, "for task in ctx['tasks']:", `selected=None
if 'LOT DE REVUE :' in text:
 import re
 selected=json.loads(re.search(r'retourne uniquement leurs verdicts : (\[[^\n]*\])\.',text).group(1))
for task in ctx['tasks']:
 if selected is not None and task['task'] not in selected: continue`, 1)
	// Digest pins the executable path/args, not its content, as in the existing
	// deterministic provider fixture. No external model is called by this test.
	if err = os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fixture), 0700); err != nil {
		t.Fatal(err)
	}
	first := managedCompleted(t, s, w, "first", "one\n")
	os.WriteFile(filepath.Join(first.CWD, "docs/first.md"), []byte(strings.Repeat("First report evidence.\n", 1800)), 0600)
	os.WriteFile(filepath.Join(first.CWD, "docs/first.review-context.json"), []byte(`{"version":1,"files":["one.go.txt"]}`), 0600)
	if err = s.integrateManagedAttempt(first); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "accepted" {
		t.Fatal(w.Tasks[0].Blocker)
	}
	second := managedCompleted(t, s, w, "second", "two\n")
	os.WriteFile(filepath.Join(second.CWD, "docs/second.md"), []byte(strings.Repeat("Second report evidence.\n", 1800)), 0600)
	os.WriteFile(filepath.Join(second.CWD, "docs/second.review-context.json"), []byte(`{"version":1,"files":["two.go.txt"]}`), 0600)
	w, _ = s.get(w.ID)
	return s, w, second
}

func TestManagedBatchRuntimePublishesOnlyCompleteCandidate(t *testing.T) {
	s, w, a := managedBatchRuntimeFixture(t)
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || task.IndependentReview == nil || len(task.IndependentReview.Batches) != 2 || managedReviewCalls(t, s) != 3 {
		t.Fatalf("not a complete two-batch review: %s %s calls=%d", task.Status, task.Blocker, managedReviewCalls(t, s))
	}
	for i := range after.Tasks {
		if !s.acceptedFresh(&after, &after.Tasks[i], map[string]bool{}) {
			t.Fatal("accepted evidence not fresh", after.Tasks[i].ID)
		}
	}
	if err := s.integrateManagedAttempt(a); err != nil || managedReviewCalls(t, s) != 3 {
		t.Fatal("duplicate calls", err)
	}
	r := task.IndependentReview
	os.WriteFile(filepath.Join(s.root, r.Batches[0].ReplyPath), []byte(`{}`), 0600)
	if err := s.managedIndependentReviewGuard(&after, task); err == nil {
		t.Fatal("changed batch reply stayed accepted")
	}
}

func TestManagedBatchRuntimeBudgetFailsBeforeCalls(t *testing.T) {
	s, w, a := managedBatchRuntimeFixture(t)
	w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls + 1
	raw, _ := json.Marshal(w)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	_ = s.integrateManagedAttempt(a)
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.Status == "accepted" || task.IndependentReview != nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("partial paid plan started", task.Status, managedReviewCalls(t, s))
	}
}

func TestManagedBatchRuntimeExplicitRetryReusesPaidPass(t *testing.T) {
	for _, tamper := range []bool{false, true} {
		t.Run(map[bool]string{false: "resume", true: "tampered-pass"}[tamper], func(t *testing.T) {
			s, w, a := managedBatchRuntimeFixture(t)
			path := filepath.Join(s.root, "review-fixture/claude")
			original, err := os.ReadFile(path)
			if err != nil {
				t.Fatal(err)
			}
			failing := strings.Replace(string(original), "if mode=='exit': sys.exit(9)", "if len(open(os.path.join(folder,'calls')).readlines())==3: mode='exit'\nif mode=='exit': sys.exit(9)", 1)
			os.WriteFile(path, []byte(failing), 0700)
			_ = s.integrateManagedAttempt(a)
			failed, _ := s.get(w.ID)
			task, _ := failed.task(a.TaskID)
			if task.IndependentReview == nil || task.IndependentReview.State != "error" || task.IndependentReview.Batches[0].State != "passed" || managedReviewCalls(t, s) != 3 || failed.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
				t.Fatal("partial result not retained safely", task.Status, task.Blocker)
			}
			passedID := task.IndependentReview.Batches[0].ID
			if tamper {
				os.WriteFile(filepath.Join(s.root, task.IndependentReview.Batches[0].ReplyPath), []byte(`{}`), 0600)
			}
			os.WriteFile(path, original, 0700)
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			_, err = reopened.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "resume-batch", Revision: failed.Revision, Task: a.TaskID, Reason: "Fixture provider failure corrected, preserve paid prior batch"})
			if err != nil {
				t.Fatal(err)
			}
			_ = reopened.reconcileKnownMissionResult(a, "fixture-conductor")
			after, _ := reopened.get(w.ID)
			task, _ = after.task(a.TaskID)
			if tamper {
				if task.Status == "accepted" || managedReviewCalls(t, s) != 3 {
					t.Fatal("tampered paid pass reused")
				}
				return
			}
			if task.Status != "accepted" || managedReviewCalls(t, s) != 4 || task.IndependentReview.Batches[0].ID != passedID || len(task.Attempts) != 1 {
				t.Fatal("resume duplicated calls or production", task.Status, task.Blocker, managedReviewCalls(t, s))
			}
		})
	}
}

// Inject a durable-storage failure in an isolated fixture, then reopen the
// store. This exercises recovery after paid calls without pretending to kill
// the live mission or to measure a real provider's behavior.
func TestManagedBatchRuntimeInterruptedPersistence(t *testing.T) {
	for _, phase := range []string{"all-passed", "reply-not-durable"} {
		t.Run(phase, func(t *testing.T) {
			s, w, a := managedBatchRuntimeFixture(t)
			kind := "review.managed.result"
			if phase == "reply-not-durable" {
				kind = "review.managed.batch.result"
			}
			_, err := s.db.Exec("CREATE TRIGGER fail_batch_persistence BEFORE INSERT ON events WHEN NEW.kind='" + kind + "' BEGIN SELECT RAISE(ABORT,'fixture persistence failure'); END")
			if err != nil {
				t.Fatal(err)
			}
			_ = s.integrateManagedAttempt(a)
			interrupted, _ := s.get(w.ID)
			task, _ := interrupted.task(a.TaskID)
			if task.IndependentReview == nil || task.IndependentReview.State != "running" || interrupted.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
				t.Fatal("interrupted parent lost or published", task.Blocker)
			}
			r := *task.IndependentReview
			calls := managedReviewCalls(t, s)
			if phase == "all-passed" && (!managedBatchesAllPassed(&r) || calls != 3) {
				t.Fatal("completed children not durable")
			}
			if phase == "reply-not-durable" && (r.Batches[0].State != "running" || calls != 2) {
				t.Fatal("unfinished child treated as complete")
			}
			if _, err = s.db.Exec("DROP TRIGGER fail_batch_persistence"); err != nil {
				t.Fatal(err)
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			receipt, err := os.ReadFile(filepath.Join(s.root, r.Receipt))
			if err != nil {
				t.Fatal(err)
			}
			if err = reopened.reviewManagedCandidate(interrupted, a, r.CandidateSHA, r.Receipt, receipt); err == nil {
				t.Fatal("running parent implicitly resumed")
			}
			failed, _ := reopened.get(w.ID)
			task, _ = failed.task(a.TaskID)
			if task.IndependentReview.State != "error" || managedReviewCalls(t, s) != calls {
				t.Fatal("interruption consumed a new call")
			}
			if phase == "all-passed" {
				// Exhaust the fixture's authorized budget at exactly the paid calls.
				failed.Planning.Reviewer.MaxCalls = failed.Planning.Reviewer.Calls
				raw, _ := json.Marshal(failed)
				if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
					t.Fatal(err)
				}
			}
			_, err = reopened.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "resume-persistence", Revision: failed.Revision, Task: a.TaskID, Reason: "Fixture storage restored; reuse only durable passed batches"})
			if err != nil {
				t.Fatal(err)
			}
			_ = reopened.reconcileKnownMissionResult(a, "fixture-conductor")
			after, _ := reopened.get(w.ID)
			task, _ = after.task(a.TaskID)
			expected := 3
			if phase == "reply-not-durable" {
				expected = 4
			}
			if task.Status != "accepted" || managedReviewCalls(t, s) != expected || len(task.Attempts) != 1 {
				t.Fatal("incorrect durable recovery", task.Status, task.Blocker, managedReviewCalls(t, s))
			}
		})
	}
}

func TestManagedBatchRuntimeNegativeVerdicts(t *testing.T) {
	for _, mode := range []string{"fail", "unknown", "wrong-sha", "missing-task"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := managedBatchRuntimeFixture(t)
			managedReviewMode(t, s, mode)
			_ = s.integrateManagedAttempt(a)
			after, _ := s.get(w.ID)
			task, _ := after.task(a.TaskID)
			if task.Status == "accepted" || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate || managedReviewCalls(t, s) != 2 {
				t.Fatal("bad batch published or further calls made", task.Status, managedReviewCalls(t, s))
			}
			expected := "error"
			if mode == "fail" || mode == "unknown" {
				expected = "changes_requested"
			}
			if task.IndependentReview == nil || task.IndependentReview.State != expected {
				t.Fatal("wrong refusal state", task.IndependentReview)
			}
		})
	}
}

func TestManagedBatchRuntimeProviderChangeInvalidatesPublication(t *testing.T) {
	s, w, a := managedBatchRuntimeFixture(t)
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" {
		t.Fatal(task.Blocker)
	}
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	p := ps.Providers[after.Planning.Reviewer.Provider]
	p.Command += "-changed"
	ps.Providers[after.Planning.Reviewer.Provider] = p
	raw, _ := json.Marshal(ps)
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	if err = s.managedIndependentReviewGuard(&after, task); err == nil {
		t.Fatal("changed provider stayed valid")
	}
	if managedReviewCalls(t, s) != 3 {
		t.Fatal("guard made a paid call")
	}
}
