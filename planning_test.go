//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func planningFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s := storeTest(t)
	w := createTest(t, s)
	return s, planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
}
func planningDo(t *testing.T, s *Store, w Work, action string, r PlanningRequest) Work {
	t.Helper()
	r.Schema = 1
	r.EventID = newID("test-plan-")
	r.Revision = w.Revision
	v, e := s.planningChange(w.ID, action, r)
	if e != nil {
		t.Fatal(action, e)
	}
	return v
}
func planningClaim(t *testing.T, s *Store, w Work, scope string) (Work, PlanningRequest) {
	t.Helper()
	owner, _ := w.Planning.scope(scope)
	w = planningDo(t, s, w, "claim", PlanningRequest{Scope: scope, ScopeRevision: owner.Revision, Holder: "test-owner", LeaseSeconds: 60})
	owner, _ = w.Planning.scope(scope)
	ids := []string{}
	for _, e := range w.Planning.Inbox {
		if e.Scope == scope && e.Decision == "" {
			ids = append(ids, e.ID)
		}
	}
	return w, PlanningRequest{Schema: 1, EventID: newID("decision-"), Revision: w.Revision, Scope: scope, ScopeRevision: owner.Revision, Generation: owner.Generation, Holder: owner.Holder, Inputs: ids, Reason: "Décision fondée sur les retours"}
}
func planningTask(id string) PlanningOperation {
	return PlanningOperation{Kind: "task", ID: id, Title: "Vérifier", Requirements: []string{"req-1"}, Deliverable: "proof.txt", Criteria: []string{"preuve courante"}, Next: "Vérifier les fichiers"}
}
func TestPlanningAtomicDecisionReplayAndRestart(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("first")}
	result, e := s.planningChange(w.ID, "decide", r)
	if e != nil {
		t.Fatal(e)
	}
	if len(result.Tasks) != 1 || result.Tasks[0].ScopeID != "root" || result.Planning.Inbox[0].Decision != r.EventID {
		t.Fatal(result)
	}
	second, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer second.db.Close()
	replay, e := second.planningChange(w.ID, "decide", r)
	if e != nil || replay.Revision != result.Revision || len(replay.Tasks) != 1 {
		t.Fatal(replay, e)
	}
	r.Reason = "différent"
	if _, e = second.planningChange(w.ID, "decide", r); e == nil {
		t.Fatal("conflicting replay accepted")
	}
}
func TestPlanningRollbackDoesNotConsumeInput(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	bad := planningTask("bad")
	bad.Depends = []string{"missing"}
	r.Operations = []PlanningOperation{planningTask("first"), bad}
	if _, e := s.planningChange(w.ID, "decide", r); e == nil {
		t.Fatal("invalid dependency accepted")
	}
	got, _ := s.get(w.ID)
	if len(got.Tasks) != 0 || got.Revision != w.Revision || got.Planning.Inbox[0].Decision != "" {
		t.Fatal("partial decision", got)
	}
}
func TestPlanningLeaseFencesOldOwnerAndPause(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	owner, _ := w.Planning.scope("root")
	err := s.applyPlanning(&w, "claim", PlanningRequest{Scope: "root", ScopeRevision: owner.Revision, Holder: "replacement", LeaseSeconds: 60}, time.Now().Add(61*time.Second))
	if err != nil {
		t.Fatal(err)
	}
	if err = s.applyPlanning(&w, "decide", r, time.Now().Add(62*time.Second)); err == nil {
		t.Fatal("old owner accepted")
	}
	current, _ := s.get(w.ID)
	paused := planningDo(t, s, current, "pause", PlanningRequest{})
	r.Revision = paused.Revision
	if _, err = s.planningChange(w.ID, "decide", r); err == nil {
		t.Fatal("paused decision accepted")
	}
	resumed := planningDo(t, s, paused, "resume", PlanningRequest{})
	r.Revision = resumed.Revision
	if _, err = s.planningChange(w.ID, "decide", r); err == nil {
		t.Fatal("old lease survived pause")
	}
}
func TestPlanningDelegationIsNotDependency(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Sous domaine", Requirements: []string{"req-1"}}}
	w, e := s.planningChange(w.ID, "decide", r)
	if e != nil {
		t.Fatal(e)
	}
	if len(w.Tasks) != 0 || len(w.Planning.Scopes) != 2 || len(w.Planning.Scopes[0].Requirements) != 0 {
		t.Fatal(w)
	}
	w, r = planningClaim(t, s, w, "child")
	r.Operations = []PlanningOperation{planningTask("worker")}
	w, e = s.planningChange(w.ID, "decide", r)
	if e != nil || w.Tasks[0].ScopeID != "child" || len(w.Tasks[0].Depends) != 0 {
		t.Fatal(w, e)
	}
}
func TestPlanningRejectsUnownedRequirementAndUnprovedClosure(t *testing.T) {
	for _, kind := range []string{"unowned", "close", "budget"} {
		t.Run(kind, func(t *testing.T) {
			s, w := planningFixture(t)
			w, r := planningClaim(t, s, w, "root")
			switch kind {
			case "unowned":
				op := planningTask("wrong")
				op.Requirements = []string{"foreign"}
				r.Operations = []PlanningOperation{op}
			case "close":
				r.Operations = []PlanningOperation{{Kind: "close"}}
			case "budget":
				for i := 0; i < 11; i++ {
					r.Operations = append(r.Operations, planningTask(newID("task-")))
				}
			}
			if _, e := s.planningChange(w.ID, "decide", r); e == nil {
				t.Fatal("unsafe decision accepted")
			}
		})
	}
}
func TestPlanningReturnWakesOwnerAndDeduplicates(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("worker")}
	w, e := s.planningChange(w.ID, "decide", r)
	if e != nil {
		t.Fatal(e)
	}
	a := Agent{ID: "a", TaskID: "worker", Attempt: "attempt-1", Activity: "Test échoué"}
	planningAttemptEnded(&w, a, "failed")
	planningAttemptEnded(&w, a, "failed")
	if len(w.Planning.Inbox) != 2 || w.Planning.Scopes[0].State != "ready" || w.Tasks[0].Status != "todo" {
		t.Fatal(w)
	}
}
func TestPlanningCLIAndAuthenticatedHTTPShareContract(t *testing.T) {
	s, w := planningFixture(t)
	var out bytes.Buffer
	if e := s.planningCLI([]string{"planning", "show", w.ID}, "", &out); e != nil || !strings.Contains(out.String(), "max_activations") {
		t.Fatal(e, out.String())
	}
	handler := newWebHandler(s, "localhost:18787", "secret")
	r := httptest.NewRequest("GET", "http://localhost:18787/api/v1/planning?work="+w.ID, nil)
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, r)
	if rec.Code != 403 {
		t.Fatal(rec.Code)
	}
	r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "secret"})
	rec = httptest.NewRecorder()
	handler.ServeHTTP(rec, r)
	if rec.Code != 200 {
		t.Fatal(rec.Code, rec.Body.String())
	}
	raw, _ := json.Marshal(PlanningRequest{Schema: 1, EventID: "http-pause", Revision: w.Revision})
	r = httptest.NewRequest("POST", "http://localhost:18787/api/v1/planning?work="+w.ID+"&action=pause", bytes.NewReader(raw))
	r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "secret"})
	r.Header.Set("Origin", "http://localhost:18787")
	r.Header.Set("X-Swarm-CSRF", "secret")
	rec = httptest.NewRecorder()
	handler.ServeHTTP(rec, r)
	if rec.Code != 200 {
		t.Fatal(rec.Code, rec.Body.String())
	}
	got, _ := s.get(w.ID)
	if !got.Planning.Paused {
		t.Fatal("HTTP did not pause")
	}
}
func TestPlanningProviderCreatesTaskWithoutHostDecision(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	script := filepath.Join(t.TempDir(), "claude")
	response := `{"input_events":["enable-test"],"reason":"Le brief demande une preuve","operations":[{"kind":"task","id":"generated","title":"Vérifier","requirements":["req-1"],"deliverable":"proof.txt","criteria":["preuve"],"next":"Vérifier"}]}`
	envelope, _ := json.Marshal(map[string]any{"type": "result", "result": response})
	if e := os.WriteFile(script, []byte("#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' '"+string(envelope)+"'\n"), 0700); e != nil {
		t.Fatal(e)
	}
	ps := Providers{Schema: 1, Providers: map[string]Provider{"test": {Command: script}}}
	raw, _ := json.Marshal(ps)
	os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600)
	w, e := s.planningChange(w.ID, "enable", PlanningRequest{Schema: 1, EventID: "enable-test", Revision: w.Revision, MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15, Provider: "test"})
	if e != nil {
		t.Fatal(e)
	}
	if e = s.planningStep(w.ID); e != nil {
		t.Fatal(e)
	}
	got, _ := s.get(w.ID)
	if len(got.Tasks) != 1 || got.Tasks[0].ID != "generated" || got.Planning.Activations != 1 || got.Planning.Decisions != 1 {
		t.Fatal(got)
	}
	if e = s.planningStep(w.ID); e != nil {
		t.Fatal(e)
	}
	again, _ := s.get(w.ID)
	if again.Revision != got.Revision {
		t.Fatal("idle loop produced writes")
	}
}

func TestPlanningHandoffIdentityAndNewDiscovery(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("worker")}
	w, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "test.attempt", "start-worker", w.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task("worker")
		task.Status = "running"
		task.Attempts = []Attempt{{ID: "attempt-1", Status: "running"}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	a := Agent{ID: "worker-agent", WorkID: w.ID, TaskID: "worker", Attempt: "attempt-1", Role: "worker", Status: "running", CWD: s.root}
	body, _ := json.Marshal(a)
	_, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, a.CWD, a.Status, body, []byte(`{}`))
	if err != nil {
		t.Fatal(err)
	}
	os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("discovery"), 0600)
	submit := PlanningRequest{Schema: 1, EventID: "discovery", Revision: w.Revision, Agent: a.ID, Task: a.TaskID, Attempt: a.Attempt, Reason: "Le format existant ne couvre pas les accents", Artifacts: []ExchangeArtifact{{Path: "proof.txt", SHA256: hash([]byte("discovery"))}}}
	bad := submit
	bad.Agent = "unknown"
	if _, err = s.planningChange(w.ID, "handoff", bad); err == nil {
		t.Fatal("unknown agent admitted")
	}
	w, err = s.planningChange(w.ID, "handoff", submit)
	if err != nil {
		t.Fatal(err)
	}
	again, err := s.planningChange(w.ID, "handoff", submit)
	if err != nil || again.Revision != w.Revision {
		t.Fatal(err)
	}
	w, r = planningClaim(t, s, w, "root")
	op := planningTask("unicode-correction")
	op.Title = "Corriger la gestion des accents"
	r.Operations = []PlanningOperation{op}
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil || len(w.Tasks) != 2 {
		t.Fatal(w, err)
	}
	if w.Tasks[1].Status != "todo" {
		t.Fatal("proposal self validated")
	}
	submit.EventID = "second-handoff"
	submit.Revision = w.Revision
	if _, err = s.planningChange(w.ID, "handoff", submit); err == nil {
		t.Fatal("second handoff admitted")
	}
}
func TestPlanningProviderFailureDoesNotLoop(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	script := filepath.Join(t.TempDir(), "claude")
	os.WriteFile(script, []byte("#!/bin/sh\ncat >/dev/null\nexit 3\n"), 0700)
	ps := Providers{Schema: 1, Providers: map[string]Provider{"test": {Command: script}}}
	raw, _ := json.Marshal(ps)
	os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600)
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "test", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
	if err := s.planningStep(w.ID); err == nil {
		t.Fatal("failure hidden")
	}
	got, _ := s.get(w.ID)
	if got.Planning.Failure == "" || got.Planning.Activations != 1 || len(got.Tasks) != 0 {
		t.Fatal(got)
	}
	if err := s.planningStep(w.ID); err != nil {
		t.Fatal(err)
	}
	again, _ := s.get(w.ID)
	if again.Revision != got.Revision {
		t.Fatal("failure caused repeat inference")
	}
}
func TestPlanningMissionPauseRevokesEvenAfterResume(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	if err := s.pause(w.ID, false); err != nil {
		t.Fatal(err)
	}
	current, _ := s.get(w.ID)
	r.Revision = current.Revision
	if _, err := s.planningChange(w.ID, "decide", r); err == nil {
		t.Fatal("mission pause did not fence planner")
	}
}
func TestPlanningValidationReactivatesAfterResultConsumed(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("worker")}
	w, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "test.validation", "validation-signal", w.Revision, []byte(`{}`), func(w *Work) error { w.Tasks[0].Status = "accepted"; return nil })
	if err != nil {
		t.Fatal(err)
	}
	if w.Planning.Scopes[0].State != "ready" || w.Planning.Inbox[len(w.Planning.Inbox)-1].Kind != "validation_changed" {
		t.Fatal(w)
	}
	w, r = planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "close"}}
	if _, err = s.planningChange(w.ID, "decide", r); err == nil {
		t.Fatal("status without proof closed scope")
	}
}
func TestPlanningContextDeadlineAndRevocation(t *testing.T) {
	script := filepath.Join(t.TempDir(), "claude")
	os.WriteFile(script, []byte("#!/bin/sh\ncat >/dev/null\nsleep 30\n"), 0700)
	start := time.Now()
	if _, err := runPlanningProvider(Provider{Command: script}, "test", 100*time.Millisecond, func() bool { return true }); err == nil || time.Since(start) > 4*time.Second {
		t.Fatal(err, time.Since(start))
	}
}

func TestPlanningClosureRequiresFreshEvidenceAndReopens(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("t1")}
	w, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	raw := fixture(t, s.root)
	w = gateTest(t, s, w, raw)
	w, err = s.mutate(w.ID, "test.accept", "accept-proven", w.Revision, []byte(`{}`), func(w *Work) error { w.Tasks[0].Status = "accepted"; return nil })
	if err != nil {
		t.Fatal(err)
	}
	w, r = planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "close"}}
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	if w.Planning.Scopes[0].State != "closed" {
		t.Fatal(w)
	}
	os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("modified"), 0600)
	w, err = s.reconcilePlanningProofs(w)
	if err != nil {
		t.Fatal(err)
	}
	if w.Planning.Scopes[0].State != "ready" || w.Planning.Inbox[len(w.Planning.Inbox)-1].Kind != "proof_stale" {
		t.Fatal(w)
	}
	next, err := s.reconcilePlanningProofs(w)
	if err != nil || next.Revision != w.Revision {
		t.Fatal("proof drift loop", err)
	}
}
func TestPlanningArchiveAndMigration(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("t1")}
	w, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	file := filepath.Join(t.TempDir(), "planning.zip")
	if err = s.export(w.ID, file); err != nil {
		t.Fatal(err)
	}
	dest := storeTest(t)
	got, err := dest.importBundle(file)
	if err != nil || got.Planning == nil || got.Planning.Decisions != 1 {
		t.Fatal(got, err)
	}
	if _, err = s.db.Exec("PRAGMA user_version=17"); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	backups, _ := filepath.Glob(filepath.Join(s.root, ".swarm/state-pre-v18-*.db"))
	if len(backups) != 1 {
		t.Fatal(backups)
	}
	var version int
	reopened.db.QueryRow("PRAGMA user_version").Scan(&version)
	if version != schemaVersion {
		t.Fatal(version)
	}
}

func TestPlanningNeverBypassesMissionFinancialCap(t *testing.T) {
	s, w := planningFixture(t)
	raw, _ := json.Marshal(Budget{Limit: 0.5, Reserve: 1})
	if _, err := s.db.Exec("INSERT INTO budgets(work_id,body) VALUES(?,?)", w.ID, raw); err != nil {
		t.Fatal(err)
	}
	_, err := s.planningChange(w.ID, "claim", PlanningRequest{Schema: 1, EventID: "capped", Revision: w.Revision, Scope: "root", ScopeRevision: 1, Holder: "planner", LeaseSeconds: 30})
	if err == nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	if after.Planning.Activations != 0 {
		t.Fatal("blocked call consumed activation")
	}
}

func TestPlanningOldFailureCannotUndoAppliedDecision(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("t1")}
	done, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.planningFailure(w.ID, "root", r.Generation, "late error"); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	if after.Revision != done.Revision || after.Planning.Failure != "" {
		t.Fatal("old failure changed committed decision")
	}
}
