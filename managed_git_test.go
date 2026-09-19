//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func gitTest(t *testing.T, dir string, args ...string) string {
	t.Helper()
	v, e := managedGit(dir, args...)
	if e != nil {
		t.Fatal(e)
	}
	return v
}
func managedFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s := storeTest(t)
	source := filepath.Join(s.root, "project")
	os.MkdirAll(source, 0700)
	gitTest(t, source, "init")
	os.WriteFile(filepath.Join(source, "value.txt"), []byte("initial\n"), 0600)
	gitTest(t, source, "add", ".")
	gitTest(t, source, "commit", "-m", "base")
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 20, MaxActivations: 30, Repository: &ManagedRepositoryRequest{Path: source, CommittedOnly: true}, Checks: map[string][]ValidationControl{"req-1": automaticPolicy("git", "diff", "--exit-code").Controls}})
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("first"), planningTask("second")}
	var e error
	w, e = s.planningChange(w.ID, "decide", r)
	if e != nil {
		t.Fatal(e)
	}
	if e = organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 2); e != nil {
		t.Fatal(e)
	}
	if e = s.setMission(w.ID, true); e != nil {
		t.Fatal(e)
	}
	return s, w
}
func managedCompleted(t *testing.T, s *Store, w Work, id, value string) Agent {
	t.Helper()
	agent := Agent{ID: "agent-" + id, WorkID: w.ID, TaskID: id, Status: "completed", Role: "worker", Host: hostIdentity()}
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
	os.WriteFile(filepath.Join(path, "docs", id+".md"), []byte("Rapport nouveau "+id), 0600)
	os.WriteFile(filepath.Join(path, "value.txt"), []byte(value), 0600)
	raw, _ := json.Marshal(agent)
	_, e = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", agent.ID, w.ID, id, path, agent.Status, raw, []byte(`{}`))
	if e != nil {
		t.Fatal(e)
	}
	return agent
}
func TestManagedIntegrationAtomicAndConflict(t *testing.T) {
	s, w := managedFixture(t)
	base := w.Planning.Repository.Candidate
	first := managedCompleted(t, s, w, "first", "first\n")
	w, _ = s.get(w.ID)
	second := managedCompleted(t, s, w, "second", "second\n")
	if e := s.integrateManagedAttempt(first); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "accepted" || w.Planning.Repository.Candidate == base || !s.acceptedFresh(&w, &w.Tasks[0], map[string]bool{}) {
		t.Fatalf("unverified candidate %+v", w.Tasks[0])
	}
	accepted := w.Planning.Repository.Candidate
	if e := s.integrateManagedAttempt(first); e != nil {
		t.Fatal("replay", e)
	}
	if e := s.integrateManagedAttempt(second); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Planning.Repository.Candidate != accepted || w.Tasks[1].Status != "blocked" {
		t.Fatal("conflict published")
	}
	data, _ := os.ReadFile(filepath.Join(s.root, "project", "value.txt"))
	if string(data) != "initial\n" {
		t.Fatal("source modified")
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	after, _ := reopened.get(w.ID)
	if after.Planning.Repository.Candidate != accepted {
		t.Fatal("lost durable candidate")
	}
}
func TestManagedFailedControlNeverPublishes(t *testing.T) {
	s, w := managedFixture(t)
	// A check failure must leave the Git publication and source unchanged.
	w, e := s.mutate(w.ID, "test.policy", "policy", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].ValidationPolicy = automaticPolicy("git", "diff", "--exit-code", "--no-index", "value.txt", "missing.txt")
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	a := managedCompleted(t, s, w, "first", "failure\n")
	base := w.Planning.Repository.Candidate
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	if after.Planning.Repository.Candidate != base || after.Tasks[0].Status == "accepted" {
		t.Fatal("failed check published")
	}
}
func TestPlanningSharedBudgetAndInheritedChecks(t *testing.T) {
	s, w := managedFixture(t)
	if w.Tasks[0].ValidationPolicy == nil || w.Tasks[0].Criteria[0] != w.Criteria[0] {
		t.Fatal("controls not inherited")
	}
	raw, _ := json.Marshal(Budget{Limit: 2, Reserve: 1})
	s.db.Exec("INSERT INTO budgets(work_id,body) VALUES(?,?)", w.ID, raw)
	tx, e := s.db.Begin()
	if e != nil {
		t.Fatal(e)
	}
	if e = reservePlanningCall(tx, w.ID, "root", "paid"); e != nil {
		t.Fatal(e)
	}
	tx.Commit()
	v, e := s.budget(w.ID)
	if e != nil || v.Estimated != 1 || v.Remaining != 1 {
		t.Fatal(v, e)
	}
	tx, _ = s.db.Begin()
	if e = reservePlanningCall(tx, w.ID, "root", "second-paid"); e != nil {
		t.Fatal(e)
	}
	tx.Commit()
	tx, _ = s.db.Begin()
	defer tx.Rollback()
	if e = reservePlanningCall(tx, w.ID, "root", "excess"); e == nil {
		t.Fatal("planner exceeded shared budget")
	}
}

func TestManagedRetryRequiresNewDecisionAndStaysBounded(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "first\n")
	if e := s.managedFailure(a, "contrôle à corriger"); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "retry", ID: "first", Next: "Corriger la valeur constatée puis rejouer le contrôle"}}
	w, e := s.planningChange(w.ID, "decide", r)
	if e != nil {
		t.Fatal(e)
	}
	if !w.Tasks[0].PlanningRetry {
		t.Fatal("missing explicit retry")
	}
	// Another request cannot silently consume more attempts while the retry waits.
	w, r = planningClaim(t, s, planningDo(t, s, w, "resume", PlanningRequest{Reason: "autre retour"}), "root")
	r.Operations = []PlanningOperation{{Kind: "retry", ID: "first", Next: "encore"}}
	if _, e = s.planningChange(w.ID, "decide", r); e == nil {
		t.Fatal("duplicate retry accepted")
	}
}

func TestManagedCleanupPreviewProtectsChangedCopyAndDelivery(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "first\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	request := ManagedCleanupRequest{Schema: 1, Revision: w.Revision}
	if _, e := s.managedCleanup(w.ID, request, false); e == nil {
		t.Fatal("active mission cleaned")
	}
	if e := s.setMission(w.ID, false); e != nil {
		t.Fatal(e)
	}
	preview, e := s.managedCleanup(w.ID, request, false)
	if e != nil {
		t.Fatal(e)
	}
	if len(preview.Paths) != 1 {
		t.Fatal(preview)
	}
	request.Token = preview.Token
	os.WriteFile(filepath.Join(a.CWD, "new.txt"), []byte("new edit"), 0600)
	if _, e = s.managedCleanup(w.ID, request, true); e == nil {
		t.Fatal("changed copy deleted")
	}
	preview, e = s.managedCleanup(w.ID, request, false)
	if e != nil {
		t.Fatal(e)
	}
	request.Token = preview.Token
	if _, e = s.managedCleanup(w.ID, request, true); e != nil {
		t.Fatal(e)
	}
	if _, e = os.Stat(a.CWD); !os.IsNotExist(e) {
		t.Fatal("copy remains")
	}
	if !s.acceptedFresh(&w, &w.Tasks[0], map[string]bool{}) {
		t.Fatal("cleanup damaged proof")
	}
	if e = s.managedBundle(w.ID, filepath.Join(s.root, "delivery.bundle")); e != nil {
		t.Fatal("delivery lost", e)
	}
}

func TestManagedBundleChecksOutDeliveredBranch(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "published\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	bundle := filepath.Join(s.root, "delivery.bundle")
	if e := s.managedBundle(w.ID, bundle); e != nil {
		t.Fatal(e)
	}
	dir := filepath.Join(s.root, "delivery")
	gitTest(t, s.root, "clone", bundle, dir)
	value, e := os.ReadFile(filepath.Join(dir, "value.txt"))
	if e != nil || string(value) != "published\n" {
		t.Fatal("bundle does not check out published tree", string(value), e)
	}
}

func TestManagedRepositorySupportsProjectBelowGitRoot(t *testing.T) {
	s := storeTest(t)
	gitTest(t, s.root, "init")
	source := filepath.Join(s.root, "nested")
	os.MkdirAll(source, 0700)
	os.WriteFile(filepath.Join(source, "value.txt"), []byte("initial\n"), 0600)
	gitTest(t, s.root, "add", "nested")
	gitTest(t, s.root, "commit", "-m", "base")
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 3, MaxDecisions: 8, MaxActivations: 10, Repository: &ManagedRepositoryRequest{Path: source, CommittedOnly: true}, Checks: map[string][]ValidationControl{"req-1": automaticPolicy("git", "diff", "--exit-code").Controls}})
	if w.Planning.Repository.Subdir != "nested" {
		t.Fatal("lost project scope")
	}
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("first")}
	w, e := s.planningChange(w.ID, "decide", r)
	if e != nil {
		t.Fatal(e)
	}
	organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1)
	s.setMission(w.ID, true)
	a := managedCompleted(t, s, w, "first", "nested result\n")
	if filepath.Base(a.CWD) != "nested" {
		t.Fatal(a.CWD)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "accepted" {
		t.Fatal(w.Tasks[0].Blocker)
	}
}

func TestManagedPreparationRecoversRenameBeforeDatabase(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{EventID: "recover-copy", TaskID: "first"}
	path, e := s.ensureManagedAttempt(w, r)
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.db.Exec("DELETE FROM managed_attempts WHERE agent_id=?", r.EventID); e != nil {
		t.Fatal(e)
	}
	recovered, e := s.ensureManagedAttempt(w, r)
	if e != nil || recovered != path {
		t.Fatal(recovered, e)
	}
	manifest := filepath.Join(w.Planning.Repository.Storage, "repository.json")
	if e = os.Remove(manifest); e != nil {
		t.Fatal(e)
	}
	restored, e := s.configureManagedRepository(w.ID, ManagedRepositoryRequest{Path: w.Planning.Repository.Source, CommittedOnly: true})
	if e != nil || restored.Base != w.Planning.Repository.Base {
		t.Fatal(restored, e)
	}
}
func TestManagedLifecycleRestoresLedgers(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "result\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	s.setMission(w.ID, false)
	w, _ = s.get(w.ID)
	p, e := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "delete"})
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.lifecycleApply(w.ID, lifecycleRequest(p, "delete-managed")); e != nil {
		t.Fatal(e)
	}
	p, e = s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "restore"})
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.lifecycleApply(w.ID, lifecycleRequest(p, "restore-managed")); e != nil {
		t.Fatal(e)
	}
	row, e := s.managedAttempt(a.ID)
	if e != nil || row.State != "integrated" {
		t.Fatal(row, e)
	}
	var calls int
	s.db.QueryRow("SELECT count(*) FROM planning_calls WHERE work_id=?", w.ID).Scan(&calls)
	if calls != 1 {
		t.Fatal("financial ledger lost", calls)
	}
}

func TestManagedPlannerWaitsForControllerVerdict(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "result\n")
	w, _ = s.get(w.ID)
	for _, state := range []string{"running", "completed"} {
		if _, e := s.db.Exec("UPDATE agents SET status=? WHERE id=?", state, a.ID); e != nil {
			t.Fatal(e)
		}
		pending, e := s.managedIntegrationPending(w, "root")
		if e != nil || !pending {
			t.Fatal("planner must wait", state, pending, e)
		}
	}
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	pending, e := s.managedIntegrationPending(w, "root")
	if e != nil || pending {
		t.Fatal("verdict did not wake planner", e)
	}
}
