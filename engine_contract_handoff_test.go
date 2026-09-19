//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// No provider is invoked. Only fixture work/agent records are populated; every
// routing, lease, artifact check and Git copy operation uses the actual engine.
func engineOwnershipFixture(t *testing.T, nested ...bool) (*Store, Work, Agent, Agent) {
	t.Helper()
	s := storeTest(t)
	top := filepath.Join(s.root, "project")
	source := top
	if len(nested) > 0 && nested[0] {
		source = filepath.Join(top, "nested")
	}
	os.MkdirAll(source, 0700)
	gitTest(t, top, "init")
	os.WriteFile(filepath.Join(source, "value.txt"), []byte("base\n"), 0600)
	gitTest(t, top, "add", ".")
	gitTest(t, top, "commit", "-m", "base")
	w := createTest(t, s)
	w = applyTest(t, s, w, "work.update", Request{Criteria: []string{"Alpha", "Beta"}})
	controls := automaticPolicy("git", "diff", "--exit-code").Controls
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 8, MaxDecisions: 20, MaxActivations: 30, Repository: &ManagedRepositoryRequest{Path: source, CommittedOnly: true}, Checks: map[string][]ValidationControl{"req-1": controls, "req-2": controls}})
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "delegate", ID: "alpha", Title: "Périmètre Alpha", Requirements: []string{"req-1"}, Next: "Examiner Alpha et remettre les preuves et limites"}, {Kind: "delegate", ID: "beta", Title: "Périmètre Beta", Requirements: []string{"req-2"}, Next: "Examiner Beta et remettre les preuves et limites"}}
	var e error
	w, e = s.planningChange(w.ID, "decide", r)
	if e != nil {
		t.Fatal(e)
	}
	for i, scope := range []string{"alpha", "beta"} {
		w, r = planningClaim(t, s, w, scope)
		op := planningTask(scope + "-worker")
		op.Requirements = []string{[]string{"req-1", "req-2"}[i]}
		r.Operations = []PlanningOperation{op}
		w, e = s.planningChange(w.ID, "decide", r)
		if e != nil {
			t.Fatal(e)
		}
	}
	first := managedCompleted(t, s, w, "alpha-worker", "alpha result\n")
	w, _ = s.get(w.ID)
	second := managedCompleted(t, s, w, "beta-worker", "beta result\n")
	w, _ = s.get(w.ID)
	return s, w, first, second
}
func engineHandoffRequest(t *testing.T, s *Store, w Work, a Agent, id string) PlanningRequest {
	t.Helper()
	report := filepath.Join(a.CWD, "docs", a.TaskID+".md")
	data, e := os.ReadFile(report)
	if e != nil {
		t.Fatal(e)
	}
	rel, e := filepath.Rel(s.root, report)
	if e != nil {
		t.Fatal(e)
	}
	return PlanningRequest{Schema: 1, EventID: id, Revision: w.Revision, Agent: a.ID, Task: a.TaskID, Attempt: a.Attempt, Scope: "alpha", Reason: "Changement produit ; preuve dans le rapport ; limites restantes à examiner par le responsable.", Artifacts: []ExchangeArtifact{{Path: filepath.ToSlash(rel), SHA256: hash(data)}}}
}
func TestEngineContractOwnershipHandoffCopiesAndReferencesAreDistinct(t *testing.T) {
	s, w, a, b := engineOwnershipFixture(t)
	if a.CWD == b.CWD || a.CWD == w.Planning.Repository.Source || b.CWD == w.Planning.Repository.Source {
		t.Fatal("executors share a workspace")
	}
	for _, agent := range []Agent{a, b} {
		metadata, e := os.Lstat(filepath.Join(agent.CWD, ".git"))
		if e != nil || !metadata.IsDir() || metadata.Mode()&os.ModeSymlink != 0 {
			t.Fatal("Git metadata not isolated", e)
		}
		if got := gitTest(t, agent.CWD, "rev-parse", "--show-toplevel"); got != agent.CWD {
			t.Fatal("foreign Git root", got)
		}
		if _, e := os.Stat(filepath.Join(agent.CWD, ".git", "objects", "info", "alternates")); !os.IsNotExist(e) {
			t.Fatal("Git objects delegated to a shared object store", e)
		}
	}
	os.WriteFile(filepath.Join(a.CWD, "value.txt"), []byte("alpha-only\n"), 0600)
	beta, _ := os.ReadFile(filepath.Join(b.CWD, "value.txt"))
	source, _ := os.ReadFile(filepath.Join(w.Planning.Repository.Source, "value.txt"))
	if string(beta) != "beta result\n" || string(source) != "base\n" {
		t.Fatal("one workspace changed another")
	}
	gitTest(t, a.CWD, "update-ref", "refs/heads/alpha-private", w.Planning.Repository.Base)
	for _, path := range []string{b.CWD, filepath.Join(w.Planning.Repository.Storage, "repository.git")} {
		if _, e := managedGit(path, "show-ref", "--verify", "refs/heads/alpha-private"); e == nil {
			t.Fatal("executor reference leaked into another repository")
		}
	}
	if _, e := s.ensureManagedAttempt(w, Launch{EventID: a.ID, TaskID: b.TaskID}); e == nil {
		t.Fatal("copy identity reassigned to another task")
	}
}
func TestEngineContractOwnershipHandoffRejectsForeignScopeAndArtifact(t *testing.T) {
	for _, kind := range []string{"foreign-scope", "foreign-artifact", "traversal-artifact", "symlink-artifact", "foreign-agent", "stale-attempt"} {
		t.Run(kind, func(t *testing.T) {
			s, w, a, b := engineOwnershipFixture(t)
			r := engineHandoffRequest(t, s, w, a, "invalid-"+kind)
			switch kind {
			case "foreign-scope":
				r.Scope = "beta"
			case "foreign-artifact":
				other := engineHandoffRequest(t, s, w, b, "other")
				r.Artifacts = other.Artifacts
			case "traversal-artifact":
				other := engineHandoffRequest(t, s, w, b, "other")
				rel, _ := filepath.Rel(s.root, a.CWD)
				r.Artifacts = other.Artifacts
				r.Artifacts[0].Path = filepath.ToSlash(rel) + "/../" + filepath.Base(b.CWD) + "/docs/" + b.TaskID + ".md"
			case "symlink-artifact":
				os.Symlink(filepath.Join(b.CWD, "docs", b.TaskID+".md"), filepath.Join(a.CWD, "foreign.md"))
				other := engineHandoffRequest(t, s, w, b, "other")
				rel, _ := filepath.Rel(s.root, filepath.Join(a.CWD, "foreign.md"))
				r.Artifacts = []ExchangeArtifact{{Path: rel, SHA256: other.Artifacts[0].SHA256}}
			case "foreign-agent":
				r.Agent = b.ID
			case "stale-attempt":
				r.Attempt = "previous-attempt"
			}
			if _, e := s.planningChange(w.ID, "handoff", r); e == nil {
				t.Fatal("invalid handoff accepted", kind)
			}
			after, _ := s.get(w.ID)
			if after.Revision != w.Revision || len(after.Planning.Inbox) != len(w.Planning.Inbox) {
				t.Fatal("rejected handoff partially woke manager or changed work")
			}
		})
	}
}
func TestEngineContractOwnershipHandoffCrashRecoveryRefusesRedirectedCopy(t *testing.T) {
	s, w, a, b := engineOwnershipFixture(t)
	item, e := s.managedAttempt(a.ID)
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.db.Exec("DELETE FROM managed_attempts WHERE agent_id=?", a.ID); e != nil {
		t.Fatal(e)
	}
	// Recover the specific rename-before-database window before a task attempt
	// was recorded, as ensureManagedAttempt's retained manifest describes.
	task, _ := w.task(a.TaskID)
	task.Attempts = nil
	backup := item.Path + "-preserved"
	if e = os.Rename(item.Path, backup); e != nil {
		t.Fatal(e)
	}
	if e = os.Symlink(b.CWD, item.Path); e != nil {
		t.Fatal(e)
	}
	if _, e = s.ensureManagedAttempt(w, Launch{EventID: a.ID, TaskID: a.TaskID}); e == nil {
		t.Fatal("recovery attributed another executor's checkout")
	}
	if _, e = s.managedAttempt(a.ID); e == nil {
		t.Fatal("redirected copy was durably attributed")
	}
}

func TestEngineContractOwnershipHandoffWakesOnlyOwnerAndReplaysAfterRestart(t *testing.T) {
	s, w, a, b := engineOwnershipFixture(t)
	beforeRoot, _ := w.Planning.scope("root")
	rootState := beforeRoot.State
	beforeBeta, _ := w.Planning.scope("beta")
	betaState := beforeBeta.State
	r := engineHandoffRequest(t, s, w, a, "owner-handoff")
	delivered, e := s.planningChange(w.ID, "handoff", r)
	if e != nil {
		t.Fatal(e)
	}
	alpha, _ := delivered.Planning.scope("alpha")
	root, _ := delivered.Planning.scope("root")
	beta, _ := delivered.Planning.scope("beta")
	if alpha.State != "ready" || root.State != rootState || beta.State != betaState {
		t.Fatal("handoff woke an unrelated manager")
	}
	var handoffs int
	for _, event := range delivered.Planning.Inbox {
		if event.Kind == "handoff" {
			handoffs++
			if event.Scope != "alpha" || event.Task != a.TaskID || event.Attempt != a.Attempt || len(event.Artifacts) != 1 || event.Artifacts[0] != r.Artifacts[0] {
				t.Fatal("handoff attribution lost", event)
			}
		}
	}
	task, _ := delivered.task(a.TaskID)
	if handoffs != 1 || task.Status == "accepted" || task.Gate != nil {
		t.Fatal("handoff duplicated or promoted result")
	}
	next, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer next.db.Close()
	replay, e := next.planningChange(w.ID, "handoff", r)
	if e != nil || replay.Revision != delivered.Revision || len(replay.Planning.Inbox) != len(delivered.Planning.Inbox) {
		t.Fatal("restart duplicated handoff", e)
	}
	context, e := planningContext(replay, "alpha")
	if e != nil {
		t.Fatal(e)
	}
	var projection struct {
		Tasks []struct {
			ID string `json:"id"`
		} `json:"tasks"`
		Events []PlanningEvent `json:"events"`
	}
	if e = json.Unmarshal(context, &projection); e != nil {
		t.Fatal(e)
	}
	if len(projection.Tasks) != 1 || projection.Tasks[0].ID != a.TaskID {
		t.Fatal("manager received another scope's tasks")
	}
	for _, event := range projection.Events {
		if event.Scope != "alpha" {
			t.Fatal("foreign event leaked to manager", event)
		}
	}
	// A sibling manager cannot acknowledge or act on this owner's event.
	before, _ := next.get(w.ID)
	betaHandoff := engineHandoffRequest(t, next, before, b, "beta-own-handoff")
	betaHandoff.Scope = "beta"
	before, e = next.planningChange(w.ID, "handoff", betaHandoff)
	if e != nil {
		t.Fatal(e)
	}
	claimed, decision := planningClaim(t, next, before, "beta")
	decision.Inputs = []string{r.EventID}
	decision.Operations = nil
	if _, e = next.planningChange(w.ID, "decide", decision); e == nil {
		t.Fatal("sibling acknowledged owner's handoff")
	}
	unchanged, _ := next.get(w.ID)
	if unchanged.Revision != claimed.Revision {
		t.Fatal("foreign acknowledgement partially mutated work")
	}
}
func TestEngineContractOwnershipHandoffCorrectionRetainsTaskAndRejectsOldAttempt(t *testing.T) {
	s, w, a, _ := engineOwnershipFixture(t)
	handoff := engineHandoffRequest(t, s, w, a, "handoff-before-correction")
	var e error
	w, e = s.planningChange(w.ID, "handoff", handoff)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.managedFailure(a, "Échec contrôlé du livrable fixture : corriger le résultat puis refaire les contrôles"); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	priorTasks := len(w.Tasks)
	priorAttempts := len(w.Tasks[0].Attempts)
	_, decision := planningClaim(t, s, w, "alpha")
	correction := "Corriger le résultat signalé, conserver les limites et fournir de nouvelles preuves vérifiables."
	decision.Operations = []PlanningOperation{{Kind: "retry", ID: a.TaskID, Next: correction}}
	updated, e := s.planningChange(w.ID, "decide", decision)
	if e != nil {
		t.Fatal(e)
	}
	task, _ := updated.task(a.TaskID)
	if len(updated.Tasks) != priorTasks || len(task.Attempts) != priorAttempts || !task.PlanningRetry || task.Next != correction || task.Status == "accepted" {
		t.Fatal("correction duplicated task, consumed an attempt or accepted it", task)
	}
	replay, e := s.planningChange(w.ID, "decide", decision)
	if e != nil || replay.Revision != updated.Revision {
		t.Fatal("decision replay duplicated correction", e)
	}
	// Starting the replacement attempt changes the identity. The prior handoff
	// remains historical; a new command cannot turn it into a current result.
	updated = applyTest(t, s, updated, "task.update", Request{ID: a.TaskID, Status: "running"})
	second, _ := updated.task(a.TaskID)
	if len(second.Attempts) != priorAttempts+1 || second.Attempts[len(second.Attempts)-1].ID == a.Attempt {
		t.Fatal("replacement attempt identity missing")
	}
	delayed := handoff
	delayed.EventID = "late-old-attempt"
	delayed.Revision = updated.Revision
	if _, e = s.planningChange(w.ID, "handoff", delayed); e == nil {
		t.Fatal("late handoff applied to replacement attempt")
	}
	again, e := s.planningChange(w.ID, "handoff", handoff)
	if e != nil || again.Revision != updated.Revision || len(again.Tasks) != priorTasks {
		t.Fatal("exact old command replay should be inert", e)
	}
}
func TestEngineContractOwnershipHandoffRejectsLateralExchangeAndParent(t *testing.T) {
	s, w, a, b := engineOwnershipFixture(t)
	report := engineHandoffRequest(t, s, w, a, "report")
	for _, kind := range []string{"handoff", "help_request", "help_answer"} {
		request := ExchangeSend{Schema: 1, EventID: "lateral-" + kind, Kind: kind, AgentID: a.ID, TaskID: a.TaskID, AttemptID: a.Attempt, RecipientTask: b.TaskID, RecipientRole: "worker", Need: "Aide directe interdite entre exécutants", ResultState: "completed", Artifacts: report.Artifacts, Timeout: 60}
		if _, _, e := s.sendExchange(w.ID, request); e == nil {
			t.Fatal("lateral exchange admitted", kind)
		}
	}
	var exchanges int
	s.db.QueryRow("SELECT count(*) FROM agent_exchanges WHERE work_id=?", w.ID).Scan(&exchanges)
	if exchanges != 0 {
		t.Fatal("rejected exchange left an inbox record")
	}
	// The launch role/Parent gate is earlier than process creation. Test it with
	// an otherwise valid public launch fixture, no provider process runs.
	w = managedReviewFixture(t, s, w)
	request := Launch{Schema: 1, EventID: "hierarchical-parent-denied", Revision: w.Revision, TaskID: a.TaskID, Provider: w.Planning.Reviewer.Provider, Workspace: a.CWD, Role: "worker", Parent: b.ID}
	_, _, e := s.prepare(w.ID, request)
	if e == nil || !strings.Contains(e.Error(), "hiérarchique") {
		t.Fatal("cross-agent Parent was not refused by hierarchy contract", e)
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision {
		t.Fatal("Parent refusal mutated work")
	}
	task, _ := w.task(a.TaskID)
	copyPath := managedCopyRoot(w.Planning.Repository, a.TaskID, len(task.Attempts)+1)
	manifest := filepath.Join(w.Planning.Repository.Storage, "copy-"+request.EventID+".json")
	for _, path := range []string{copyPath, manifest} {
		if _, e := os.Lstat(path); !os.IsNotExist(e) {
			t.Fatal("rejected Parent left a copy or attribution blocking a future launch", path, e)
		}
	}
}

func TestEngineContractOwnershipHandoffEffectiveWorkspaceCannotRedirect(t *testing.T) {
	for _, recovery := range []bool{false, true} {
		t.Run(map[bool]string{false: "replay", true: "crash-recovery"}[recovery], func(t *testing.T) {
			s, w, a, b := engineOwnershipFixture(t, true)
			if w.Planning.Repository.Subdir != "nested" {
				t.Fatal("fixture did not scope a nested project")
			}
			if cwd, e := s.ensureManagedAttempt(w, Launch{EventID: a.ID, TaskID: a.TaskID}); e != nil || cwd != a.CWD {
				t.Fatal("legitimate nested workspace cannot replay", cwd, e)
			}
			handoff := engineHandoffRequest(t, s, w, a, "nested-valid-handoff")
			var e error
			w, e = s.planningChange(w.ID, "handoff", handoff)
			if e != nil {
				t.Fatal("legitimate nested report cannot reach its owner", e)
			}
			if recovery {
				if _, e := s.db.Exec("DELETE FROM managed_attempts WHERE agent_id=?", a.ID); e != nil {
					t.Fatal(e)
				}
				task, _ := w.task(a.TaskID)
				task.Attempts = nil
			}
			if e := os.Rename(a.CWD, a.CWD+"-preserved"); e != nil {
				t.Fatal(e)
			}
			if e := os.Symlink(b.CWD, a.CWD); e != nil {
				t.Fatal(e)
			}
			if _, e := s.ensureManagedAttempt(w, Launch{EventID: a.ID, TaskID: a.TaskID}); e == nil {
				t.Fatal("effective workspace redirected to another worker despite isolated Git root")
			}
			if recovery {
				if _, e := s.managedAttempt(a.ID); e == nil {
					t.Fatal("redirected workspace durably attributed")
				}
			}
		})
	}
}

func TestEngineContractOwnershipHandoffNewCopyRejectsRedirectedWorkspace(t *testing.T) {
	s, w, a, b := engineOwnershipFixture(t, true)
	repo := w.Planning.Repository
	top := filepath.Dir(repo.Source)
	gitTest(t, top, "rm", "-r", "nested")
	if e := os.Symlink(b.CWD, repo.Source); e != nil {
		t.Fatal(e)
	}
	gitTest(t, top, "add", "nested")
	gitTest(t, top, "commit", "-m", "fixture redirected candidate")
	repo.Candidate = gitTest(t, top, "rev-parse", "HEAD")
	gitTest(t, filepath.Join(repo.Storage, "repository.git"), "fetch", top, repo.Candidate)
	request := Launch{EventID: "new-redirected-copy", TaskID: a.TaskID}
	if _, e := s.ensureManagedAttempt(w, request); e == nil {
		t.Fatal("new candidate with redirected effective workspace admitted")
	}
	if _, e := s.managedAttempt(request.EventID); e == nil {
		t.Fatal("redirected new copy attributed")
	}
	task, _ := w.task(a.TaskID)
	for _, path := range []string{managedCopyRoot(repo, a.TaskID, len(task.Attempts)+1), filepath.Join(repo.Storage, "copy-"+request.EventID+".json")} {
		if _, e := os.Lstat(path); !os.IsNotExist(e) {
			t.Fatal("redirected new copy left an attribution or copy", path, e)
		}
	}
}
