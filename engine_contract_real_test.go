//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// setPlanningProviderFixture supplies the planner-provider identity that the
// organization readiness check requires. A delegated mission has no single
// scope that owns every requirement, so the generic organizedFixtureStore
// helper (which resets scope 0's requirements from w.Criteria, assuming an
// undivided root) cannot be reused here without erasing the delegation.
func setPlanningProviderFixture(t *testing.T, s *Store, w Work, provider string) Work {
	t.Helper()
	w, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	w.Planning.Provider = provider
	data, e := json.Marshal(w)
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", data, w.ID); e != nil {
		t.Fatal(e)
	}
	return w
}

// managedDelegatedFixture sets up a managed-git mission whose single requirement
// is delegated by the root scope to a child scope, which owns the one worker
// task. It attaches a real, content-based verifiable control (not a trivial
// git diff) so a genuine defect can be injected and genuinely refused.
func managedDelegatedFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s := storeTest(t)
	source := filepath.Join(s.root, "project")
	os.MkdirAll(source, 0700)
	gitTest(t, source, "init")
	os.WriteFile(filepath.Join(source, "value.txt"), []byte("initial\n"), 0600)
	gitTest(t, source, "add", ".")
	gitTest(t, source, "commit", "-m", "base")
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{
		MaxTasks: 10, MaxDecisions: 20, MaxActivations: 30,
		Repository: &ManagedRepositoryRequest{Path: source, CommittedOnly: true},
		Checks:     map[string][]ValidationControl{"req-1": automaticPolicy("python3", "-c", "import sys; sys.exit(0 if open('value.txt').read()=='expected\\n' else 1)").Controls},
	})
	w, root := planningClaim(t, s, w, "root")
	root.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Sous-périmètre livraison", Requirements: []string{"req-1"}, Next: "Piloter la tâche déléguée jusqu’à preuve"}}
	w, e := s.planningChange(w.ID, "decide", root)
	if e != nil {
		t.Fatal(e)
	}
	w, child := planningClaim(t, s, w, "child")
	child.Operations = []PlanningOperation{planningTask("worker")}
	w, e = s.planningChange(w.ID, "decide", child)
	if e != nil {
		t.Fatal(e)
	}
	w = managedReviewFixture(t, s, w)
	w = setPlanningProviderFixture(t, s, w, "fixture-planner")
	if e = s.setAutonomy(w.ID, autonomyAuto, 2); e != nil {
		t.Fatal(e)
	}
	if e = s.setMission(w.ID, true); e != nil {
		t.Fatal(e)
	}
	return s, w
}

// managedCompletedAttempt is managedCompleted with an explicit agent ID, so a
// second (corrected) attempt on the same task does not collide with the
// managed copy already recorded for the first (defective) attempt.
func managedCompletedAttempt(t *testing.T, s *Store, w Work, id, agentID, value string) Agent {
	t.Helper()
	agent := Agent{ID: agentID, WorkID: w.ID, TaskID: id, Status: "completed", Role: "worker", Host: hostIdentity()}
	path, e := s.ensureManagedAttempt(w, Launch{EventID: agent.ID, TaskID: id})
	if e != nil {
		t.Fatal(e)
	}
	agent.CWD = path
	w = applyTest(t, s, w, "task.update", Request{ID: id, Status: "running"})
	task, _ := w.task(id)
	agent.Attempt = task.Attempts[len(task.Attempts)-1].ID
	w = applyTest(t, s, w, "task.update", Request{ID: id, Status: "blocked", Outcome: "completed", Blocker: "integration"})
	os.MkdirAll(filepath.Join(path, "docs"), 0700)
	os.WriteFile(filepath.Join(path, "docs", id+".md"), []byte("Rapport corrigé "+id), 0600)
	os.WriteFile(filepath.Join(path, "value.txt"), []byte(value), 0600)
	raw, e := json.Marshal(agent)
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", agent.ID, w.ID, id, path, agent.Status, raw, []byte(`{}`)); e != nil {
		t.Fatal(e)
	}
	return agent
}

// TestEngineContractRealNeedDelegationDefectCorrectionAcceptanceAndScopeClosure
// is a deterministic fixture test, despite the historical Real case name. It
// drives the P1 journey end to end through the engine's public planning,
// managed-integration and independent-review API (no direct DB writes for the
// journey itself; only the completed-agent process fixture, as elsewhere in
// this suite, stands in for the external provider that actually ran): a need,
// its delegation to an owned scope, a useful code change, its automatic
// control ("tests"), an injected verifiable defect that is genuinely refused,
// a bounded correction, a fresh independent review bound to the exact
// corrected SHA the controls tested, acceptance, then closure of both scopes.
//
// This deterministic behavioral test complements, and does not replace, the
// separate real-provider trial in tests/e6_real_trial_process.py, which runs
// this same journey through two genuine "claude" worker subprocesses and the
// engine's real (non-fixture) independent-review subprocess call.
func TestEngineContractRealNeedDelegationDefectCorrectionAcceptanceAndScopeClosure(t *testing.T) {
	s, w := managedDelegatedFixture(t)
	base := w.Planning.Repository.Candidate

	// 1) Injected verifiable defect: value.txt does not satisfy the content
	// control. This must be a real refusal, not an accepted candidate.
	defective := managedCompleted(t, s, w, "worker", "defective\n")
	if e := s.integrateManagedAttempt(defective); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task("worker")
	if task.Status == "accepted" || w.Planning.Repository.Candidate != base {
		t.Fatalf("injected defect was not refused: %+v", task)
	}
	if !strings.Contains(task.Blocker, "Contrôle") {
		t.Fatalf("refusal is not attributed to the verifiable control: %q", task.Blocker)
	}
	if task.IndependentReview != nil && task.IndependentReview.State == "passed" {
		t.Fatal("independent review must not run before the automatic control passes")
	}
	if managedReviewCalls(t, s) != 0 {
		t.Fatal("defective candidate must not spend a paid review call")
	}

	// 2) Bounded correction: an explicit new decision on the same task, within
	// PlanMaxAttempts, is required before another attempt can be produced.
	w, retry := planningClaim(t, s, w, "child")
	retry.Operations = []PlanningOperation{{Kind: "retry", ID: "worker", Next: "Corriger value.txt pour satisfaire le contrôle vérifiable"}}
	w, e := s.planningChange(w.ID, "decide", retry)
	if e != nil {
		t.Fatal(e)
	}
	task, _ = w.task("worker")
	if !task.PlanningRetry {
		t.Fatal("correction was not recorded as an explicit bounded retry")
	}

	// 3) Corrected attempt: the same task, satisfying the control this time.
	corrected := managedCompletedAttempt(t, s, w, "worker", "agent-worker-corrected", "expected\n")
	if e := s.integrateManagedAttempt(corrected); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	task, _ = w.task("worker")
	if task.Status != "accepted" {
		t.Fatalf("corrected candidate was not accepted: %+v", task)
	}
	correctedSHA := w.Planning.Repository.Candidate
	if correctedSHA == base {
		t.Fatal("candidate did not advance past the refused attempt")
	}
	review := task.IndependentReview
	if review == nil || review.State != "passed" {
		t.Fatalf("missing passing independent review on the corrected candidate: %+v", review)
	}
	if review.CandidateSHA != correctedSHA || review.CandidateSHA != task.AutoValidation.CandidateSHA {
		t.Fatalf("review is not bound to the exact SHA the controls tested: review=%s controls=%s candidate=%s", review.CandidateSHA, task.AutoValidation.CandidateSHA, correctedSHA)
	}
	if review.CandidateSHA == base {
		t.Fatal("review reused the pre-correction SHA")
	}
	if managedReviewCalls(t, s) != 1 {
		t.Fatalf("expected exactly one paid review call for the one reviewable candidate, got %d", managedReviewCalls(t, s))
	}
	if e := s.independentReviewGuard(&w, task); e != nil {
		t.Fatalf("accepted task fails its own freshness guard: %v", e)
	}

	// 4) Closure of both scopes: the child only after its one task is proven,
	// the root only after the child itself is closed ("fermeture des
	// périmètres" — a parent cannot close over an unfinished child).
	w, closeChild := planningClaim(t, s, w, "child")
	closeChild.Operations = []PlanningOperation{{Kind: "close"}}
	w, e = s.planningChange(w.ID, "decide", closeChild)
	if e != nil {
		t.Fatal(e)
	}
	childScope, e := w.Planning.scope("child")
	if e != nil || childScope.State != "closed" {
		t.Fatalf("child scope not closed: %+v %v", childScope, e)
	}

	w, closeRoot := planningClaim(t, s, w, "root")
	closeRoot.Operations = []PlanningOperation{{Kind: "close"}}
	w, e = s.planningChange(w.ID, "decide", closeRoot)
	if e != nil {
		t.Fatal(e)
	}
	rootScope, e := w.Planning.scope("root")
	if e != nil || rootScope.State != "closed" {
		t.Fatalf("root scope not closed despite a fully proven, closed child: %+v %v", rootScope, e)
	}
}

// TestEngineContractRealRootCannotCloseBeforeChildScope is the negative
// counterpart: a parent scope must refuse to close while its delegated child
// scope is still open, even when every task the parent can currently see is
// otherwise fine. Swarm's engine, not a prompt, must enforce this.
func TestEngineContractRealRootCannotCloseBeforeChildScope(t *testing.T) {
	s, w := managedDelegatedFixture(t)
	a := managedCompleted(t, s, w, "worker", "expected\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	// Root has nothing of its own pending yet (the accepted task belongs to
	// "child"); give it an explicit operator return so it has something to
	// decide, then attempt the close it must refuse.
	w = planningDo(t, s, w, "resume", PlanningRequest{Reason: "root vérifie l’état avant clôture"})
	w, closeRoot := planningClaim(t, s, w, "root")
	closeRoot.Operations = []PlanningOperation{{Kind: "close"}}
	if _, e := s.planningChange(w.ID, "decide", closeRoot); e == nil || !strings.Contains(e.Error(), "périmètre enfant non terminé") {
		t.Fatalf("root closed over an unfinished child scope: %v", e)
	}
}
