//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestBillingResumeUsesAttemptWorkspaceWithoutRootCopy(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	cwd := filepath.Join(s.root, "worker-space")
	scopedReport := filepath.ToSlash(filepath.Join("worker-space", report))
	if err := os.MkdirAll(filepath.Join(cwd, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.Rename(filepath.Join(s.root, report), filepath.Join(cwd, report)); err != nil {
		t.Fatal(err)
	}
	a.CWD = cwd
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	current, err = s.mutate(w.ID, "fixture", newID("fixture-"), current.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task("t1")
		task.Status = "submitted"
		task.IndependentReview.Report = scopedReport
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	resumed, err := s.resumeAutomaticValidations(w.ID)
	if err != nil || !resumed {
		t.Fatal(resumed, err)
	}
	current, _ = s.get(w.ID)
	task, _ := current.task("t1")
	if task.Status != "accepted" || task.AutoValidation.Artifacts[scopedReport] == "" {
		t.Fatalf("wrong report or no acceptance: %+v", task.AutoValidation)
	}
	if _, err = os.Stat(filepath.Join(s.root, report)); !os.IsNotExist(err) {
		t.Fatal("root copy unexpectedly exists", err)
	}
}

func TestBillingEnvironmentFailureDoesNotConsumeProducerAttempt(t *testing.T) {
	p := automaticPolicy("python3", "-c", "import sys; sys.exit(3)")
	p.Controls[0].EnvironmentExitCodes = []int{3}
	s, w, a, report := automaticValidationFixture(t, p, false)
	s.conduct(a, "completed")
	got, _ := s.get(w.ID)
	task, _ := got.task("t1")
	task.PlanMaxAttempts = 2
	if task.AutoValidation == nil || !task.AutoValidation.Controls[0].EnvironmentFailure {
		t.Fatalf("missing environmental classification: %+v", task.AutoValidation)
	}
	if _, _, retry := automaticCorrection(*task, a); retry {
		t.Fatal("environment launched new producer")
	}
	if len(task.Attempts) != 1 {
		t.Fatal("attempt budget changed")
	}
	// A real objective failure remains eligible under the same attempt budget.
	task.AutoValidation.Controls[0].EnvironmentFailure = false
	if _, _, retry := automaticCorrection(*task, a); !retry {
		t.Fatal("objective correction disabled")
	}
	_ = report
	p.Controls[0].EnvironmentExitCodes = []int{0}
	if _, err := normalizeValidationPolicy(*p); err == nil {
		t.Fatal("success classified as environment")
	}
}

func TestBillingStaleDependencyNotifiesOpenOwnerOnce(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), true)
	s.conduct(a, "completed")
	got, _ := s.get(w.ID)
	// Explicit hierarchy fixture: testing event attribution, not an AI verdict.
	got, err := s.mutate(w.ID, "fixture", newID("fixture-"), got.Revision, []byte(`{}`), func(w *Work) error {
		w.Planning = &PlanningState{Version: 1, MaxTasks: 10, MaxDecisions: 10, MaxActivations: 10, Reviewer: &ReviewerConfig{MaxCalls: 10}, Scopes: []PlanningScope{{ID: "root", State: "ready", Revision: 1, Requirements: []string{"req-1"}}}, Inbox: []PlanningEvent{}}
		for i := range w.Tasks {
			w.Tasks[i].ScopeID = "root"
			w.Tasks[i].Requirements = []string{"req-1"}
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if !s.acceptedFresh(&got, &got.Tasks[0], map[string]bool{}) {
		t.Fatalf("fixture stale: gate=%v review=%v record=%+v", s.validGate(&got.Tasks[0]), s.independentReviewGuard(&got, &got.Tasks[0]), got.Tasks[0].IndependentReview)
	}
	if err = os.WriteFile(filepath.Join(s.root, report), []byte("changed proof\n"), 0600); err != nil {
		t.Fatal(err)
	}
	got, err = s.signalDependencyProofDrift(got)
	if err != nil {
		t.Fatal(err)
	}
	if len(got.Planning.Inbox) != 1 || got.Planning.Inbox[0].Kind != "dependency_stale" || got.Planning.Inbox[0].Task != "t2" {
		t.Fatalf("owner not notified: %+v", got.Planning.Inbox)
	}
	again, err := s.signalDependencyProofDrift(got)
	if err != nil || again.Revision != got.Revision {
		t.Fatal("duplicate signal", err)
	}
	if got.Tasks[0].Status != "accepted" || len(got.Tasks[0].Attempts) != 1 {
		t.Fatal("history erased or producer relaunched")
	}
}

func TestBillingRequirementPrerequisitesCannotBeOmittedByPlanner(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w = applyTest(t, s, w, "work.update", Request{Criteria: []string{"lot approved", "settlement"}, RequirementPrerequisites: map[string][]string{"req-2": {"req-1"}}})
	w = applyTest(t, s, w, "task.add", Request{ID: "settle", Title: "Settlement", Deliverable: "settle.txt", Criteria: []string{"paid once"}})
	// A planner mapped the action to req-2 but omitted all task dependencies.
	w.Tasks[0].Requirements = []string{"req-2"}
	if err := s.requirementPrerequisiteGuard(&w, &w.Tasks[0]); err == nil {
		t.Fatal("missing prerequisite accepted")
	}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	r := Request{Schema: 1, EventID: newID("run-"), Revision: w.Revision, ID: "settle", Status: "running"}
	raw, _ = json.Marshal(r)
	if _, err := s.mutate(w.ID, "task.update", r.EventID, r.Revision, raw, func(w *Work) error { return s.apply(w, "task.update", r) }); err == nil || !strings.Contains(err.Error(), "prérequis") {
		t.Fatal("direct transition bypass", err)
	}
	got, _ := s.get(w.ID)
	if len(got.Tasks[0].Attempts) != 0 {
		t.Fatal("attempt reserved before prerequisite")
	}
	if err := updateWorkDefinition(&got, Request{RequirementPrerequisites: map[string][]string{}}); err == nil {
		t.Fatal("post-task rule removal allowed")
	}
	invalid := Work{Criteria: []string{"a", "b"}, RequirementPrerequisites: map[string][]string{"req-1": {"req-2"}, "req-2": {"req-1"}}}
	if validateRequirementPrerequisites(&invalid) == nil {
		t.Fatal("cyclic rule accepted")
	}
}

func TestBillingRequirementFreshCoverAllowsLaunchThenDriftBlocks(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), true)
	s.conduct(a, "completed")
	w, _ = s.get(w.ID)
	w.Criteria = []string{"lot", "payment"}
	w.RequirementPrerequisites = map[string][]string{"req-2": {"req-1"}}
	w.Tasks[0].Requirements = []string{"req-1"}
	w.Tasks[1].Requirements = []string{"req-2"}
	w.Tasks[1].Depends = nil
	if err := s.requirementPrerequisiteGuard(&w, &w.Tasks[1]); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("stale"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.requirementPrerequisiteGuard(&w, &w.Tasks[1]); err == nil {
		t.Fatal("stale approved lot permitted settlement")
	}
}

func TestBillingUnclassifiedBrainstormCannotBypassBusinessRules(t *testing.T) {
	s := storeTest(t)
	w := Work{Criteria: []string{"approve", "pay"}, RequirementPrerequisites: map[string][]string{"req-2": {"req-1"}}}
	task := Task{ID: "question", Brainstorm: true}
	if err := s.requirementPrerequisiteGuard(&w, &task); err == nil {
		t.Fatal("unclassified brainstorming bypassed business rules")
	}
	w.Tasks = []Task{{ID: "invalid", Requirements: []string{"req-999"}}}
	if validateRequirementPrerequisites(&w) == nil {
		t.Fatal("unknown task requirement accepted")
	}
}

func TestBillingBusyMutationRetriesWithoutDoubleEffect(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	config := []byte(`{"schema_version":1,"busy_retries":5,"busy_retry_delay_ms":30}`)
	if err := os.WriteFile(filepath.Join(s.root, ".swarm", "storage-retry.json"), config, 0600); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("PRAGMA busy_timeout=1; CREATE TABLE billing_test_counter(value INTEGER); INSERT INTO billing_test_counter VALUES(0)"); err != nil {
		t.Fatal(err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	tx, err := other.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", w.ID); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	event := newID("review-claim-")
	hook := func(tx *sql.Tx, w *Work) error {
		_, err := tx.Exec("UPDATE billing_test_counter SET value=value+1")
		return err
	}
	fn := func(w *Work) error { w.Summary = "durable decision"; return nil }
	go func() {
		_, err := s.mutateWithHook(w.ID, "review.claim", event, w.Revision, []byte(`{}`), fn, hook)
		done <- err
	}()
	select {
	case err := <-done:
		t.Fatal("did not wait for busy writer", err)
	case <-time.After(70 * time.Millisecond):
	}
	if err = tx.Commit(); err != nil {
		t.Fatal(err)
	}
	if err = <-done; err != nil {
		t.Fatal(err)
	}
	if _, err = s.mutateWithHook(w.ID, "review.claim", event, w.Revision, []byte(`{}`), fn, hook); err != nil {
		t.Fatal("idempotent replay failed", err)
	}
	var count int
	if err = s.db.QueryRow("SELECT value FROM billing_test_counter").Scan(&count); err != nil || count != 1 {
		t.Fatal("double reservation", count, err)
	}
}

func TestBillingPrerequisiteRefusesManualAgentBeforeReservation(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	w.Criteria = []string{"approve", "pay"}
	w.RequirementPrerequisites = map[string][]string{"req-2": {"req-1"}}
	w.Tasks[0].Requirements = []string{"req-2"}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	_, created, err := s.prepare(w.ID, r)
	if err == nil || created || !strings.Contains(err.Error(), "prérequis") {
		t.Fatal("manual launch bypassed prerequisites", created, err)
	}
	agents, err := s.agents(w.ID)
	if err != nil || len(agents) != 0 {
		t.Fatal("agent reserved before refusal", agents, err)
	}
	current, _ := s.get(w.ID)
	if len(current.Tasks[0].Attempts) != 0 {
		t.Fatal("attempt budget consumed")
	}
}

func TestBillingBusyPolicyIsBoundedAndRejectsInvalidConfig(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	path := filepath.Join(s.root, ".swarm", "storage-retry.json")
	if err := os.WriteFile(path, []byte(`{"schema_version":1,"busy_retries":0,"busy_retry_delay_ms":0}`), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("PRAGMA busy_timeout=1"); err != nil {
		t.Fatal(err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	tx, err := other.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", w.ID); err != nil {
		t.Fatal(err)
	}
	started := time.Now()
	_, err = s.mutate(w.ID, "checkpoint", newID("busy-"), w.Revision, []byte(`{}`), func(w *Work) error { t.Fatal("callback executed under unavailable writer"); return nil })
	if err == nil || time.Since(started) > time.Second {
		t.Fatal("zero retry policy not honored", err)
	}
	if err = tx.Rollback(); err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(path, []byte(`{"schema_version":1,"busy_retries":-1,"busy_retry_delay_ms":0}`), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err = s.storageRetryPolicy(); err == nil {
		t.Fatal("negative retry accepted")
	}
}

func TestBillingEnvironmentRecoveryRechecksExistingResultPublicly(t *testing.T) {
	policy := automaticPolicy("python3", "-c", "from pathlib import Path; import sys; sys.exit(0 if Path('service-ready').exists() else 3)")
	policy.Controls[0].EnvironmentExitCodes = []int{3}
	s, w, a, _ := automaticValidationFixture(t, policy, false)
	s.conduct(a, "completed")
	current, _ := s.get(w.ID)
	task, _ := current.task("t1")
	oldReceipt := task.AutoValidation.Receipt
	change := ValidationPolicyChange{Schema: 1, Revision: current.Revision, TaskID: "t1", Intent: "replace", Policy: task.ValidationPolicy, RecheckCompleted: true, EventID: newID("environment-recovery-")}
	if _, err := s.previewValidationPolicy(w.ID, change); err == nil {
		t.Fatal("identical retry without recovery reason")
	}
	// The precondition is actually restored before requesting revalidation.
	if err := os.WriteFile(filepath.Join(s.root, "service-ready"), []byte("restored"), 0600); err != nil {
		t.Fatal(err)
	}
	change.EnvironmentRecoveryReason = "Service local réparé et disponibilité vérifiée avant revalidation."
	preview, err := s.previewValidationPolicy(w.ID, change)
	if err != nil {
		t.Fatal(err)
	}
	change.PreviewToken = preview.Token
	got, err := s.applyValidationPolicy(w.ID, change)
	if err != nil {
		t.Fatal(err)
	}
	if got.Tasks[0].Status != "submitted" || len(got.Tasks[0].Attempts) != 1 {
		t.Fatal("new production consumed")
	}
	// The new control receipt still requires a fresh independent review, never an override.
	resumed, err := s.resumeAutomaticValidations(w.ID)
	if err != nil || !resumed {
		t.Fatal(resumed, err)
	}
	got, _ = s.get(w.ID)
	task, _ = got.task("t1")
	if task.AutoValidation == nil || !task.AutoValidation.Controls[0].Passed || task.AutoValidation.Attempt != a.Attempt {
		t.Fatal("existing attempt not successfully rechecked")
	}
	if _, err = os.Stat(filepath.Join(s.root, oldReceipt)); err != nil {
		t.Fatal("old failure receipt erased", err)
	}
}
