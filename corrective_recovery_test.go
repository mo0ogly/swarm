//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"sync"
	"testing"
)

func correctiveFixture(t *testing.T) (*Store, Work, PlanningRequest) {
	t.Helper()
	s, w := exhaustedTaskFixture(t)
	task := &w.Tasks[0]
	task.PlanMaxAttempts = 3
	task.Attempts = append(task.Attempts, Attempt{ID: "old-3", Status: "completed"})
	task.IndependentReview.Attempt = "old-3"
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	return s, w, PlanningRequest{Schema: 1, EventID: "corrective-grant", Revision: w.Revision, Task: task.ID, ReviewID: task.IndependentReview.ID, Attempt: "old-3", ConfirmRecovery: true, Reason: "Operator examined the rejected evidence", RecoveryInstruction: "Correct the missing evidence, rerun all required controls and submit the corrected candidate for independent review."}
}

func TestCorrectiveRecoveryCLIAndDurableSingleGrant(t *testing.T) {
	s, before, r := correctiveFixture(t)
	raw, _ := json.Marshal(r)
	input := filepath.Join(t.TempDir(), "recovery.json")
	if err := os.WriteFile(input, raw, 0600); err != nil {
		t.Fatal(err)
	}
	var out bytes.Buffer
	if err := s.planningCLI([]string{"planning", "authorize-recovery", before.ID}, input, &out); err != nil {
		t.Fatal(err)
	}
	after, err := s.get(before.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, old := after.Tasks[0], before.Tasks[0]
	if task.PlanMaxAttempts != 4 || !task.PlanningRetry || task.Next != r.RecoveryInstruction || task.Profile.Instruction != r.RecoveryInstruction || task.CorrectiveRecovery.Review != r.ReviewID {
		t.Fatalf("grant not ready: %+v", task)
	}
	if !reflect.DeepEqual(task.Attempts, old.Attempts) || !reflect.DeepEqual(task.IndependentReview, old.IndependentReview) || !reflect.DeepEqual(task.Criteria, old.Criteria) || !reflect.DeepEqual(after.Planning, before.Planning) || task.PlanToolLimit != old.PlanToolLimit || task.Status != "blocked" {
		t.Fatal("grant changed history, acceptance or budgets")
	}
	agents, _ := s.agents(before.ID)
	if len(agents) != 0 {
		t.Fatal("authorization invoked provider")
	}
	replay, err := s.planningChange(before.ID, "authorize-recovery", r)
	if err != nil || replay.Revision != after.Revision {
		t.Fatal("replay", err)
	}
	r.EventID, r.Revision = "second-grant", after.Revision
	if _, err := s.planningChange(before.ID, "authorize-recovery", r); err == nil {
		t.Fatal("second grant accepted")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	persisted, _ := reopened.get(before.ID)
	if !reflect.DeepEqual(after, persisted) {
		t.Fatal("grant lost on reopen")
	}
	events, _ := s.events(before.ID)
	last := events[len(events)-1]
	if last.Kind != "task.corrective-recovery" || !bytes.Contains(last.Payload, []byte(r.RecoveryInstruction)) {
		t.Fatal("missing audit")
	}
	// The existing conductor picks up the correction, and still respects pause.
	in := dispatchInputs{work: &after, profile: task.Profile, autonomy: autonomyAuto, slots: 1, depsReady: map[string]bool{task.ID: true}, agents: []Agent{{ID: "prior", TaskID: task.ID, Status: "completed"}}}
	decisions, _ := planDispatch(in)
	if len(decisions) != 1 || decisions[0].TaskID != task.ID || decisions[0].Profile.Instruction != r.RecoveryInstruction {
		t.Fatal("conductor did not queue the correction", decisions)
	}
	in.paused = true
	if decisions, _ := planDispatch(in); len(decisions) != 0 {
		t.Fatal("grant bypassed pause")
	}
	state, reason := missionDispatchState(in, task)
	if state != "waiting" || reason != "Mission en pause : reprendre la mission pour autoriser les départs" {
		t.Fatal("stale refusal hides authorized recovery", state, reason)
	}
}

func TestCorrectiveRecoveryRejectsUnsafeOrStaleDecisions(t *testing.T) {
	for _, mode := range []string{"confirmation", "review-id", "attempt-id", "revision", "same-instruction", "review-running", "review-pass", "reviewer-missing", "reviewer-budget", "not-exhausted", "already-used", "active", "reason"} {
		t.Run(mode, func(t *testing.T) {
			s, w, r := correctiveFixture(t)
			switch mode {
			case "confirmation":
				r.ConfirmRecovery = false
			case "review-id":
				r.ReviewID = "stale-review"
			case "attempt-id":
				r.Attempt = "old-2"
			case "revision":
				r.Revision--
			case "same-instruction":
				w.Tasks[0].Next = r.RecoveryInstruction
			case "review-running":
				w.Tasks[0].IndependentReview.State = "running"
			case "review-pass":
				w.Tasks[0].IndependentReview.State = "pass"
			case "reviewer-missing":
				w.Planning.Reviewer = nil
			case "reviewer-budget":
				w.Planning.Reviewer.Calls = w.Planning.Reviewer.MaxCalls
			case "not-exhausted":
				w.Tasks[0].Attempts = w.Tasks[0].Attempts[:2]
			case "already-used":
				w.Tasks[0].CorrectiveRecovery = &CorrectiveRecovery{Event: "previous"}
			case "reason":
				r.Reason = ""
			case "active":
				a := Agent{ID: "active", WorkID: w.ID, TaskID: r.Task, Status: "queued"}
				raw, _ := json.Marshal(a)
				if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, r.Task, s.root, a.Status, raw, []byte("{}")); err != nil {
					t.Fatal(err)
				}
			}
			raw, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			if _, err := s.planningChange(w.ID, "authorize-recovery", r); err == nil {
				t.Fatal("unsafe grant accepted")
			}
			after, _ := s.get(w.ID)
			if !reflect.DeepEqual(w, after) {
				t.Fatal("refusal changed work")
			}
		})
	}
}

func TestCorrectiveRecoveryHTTPConcurrentReplay(t *testing.T) {
	s, w, r := correctiveFixture(t)
	raw, _ := json.Marshal(r)
	h := newWebHandler(s, "local.test", "recovery-token")
	var wg sync.WaitGroup
	for i := 0; i < 2; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			req := httptest.NewRequest("POST", "http://local.test/api/v1/planning?work="+w.ID+"&action=authorize-recovery", bytes.NewReader(raw))
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "recovery-token"})
			req.Header.Set("Origin", "http://local.test")
			req.Header.Set("X-Swarm-CSRF", "recovery-token")
			rr := httptest.NewRecorder()
			h.ServeHTTP(rr, req)
			if rr.Code != 200 {
				t.Error(rr.Code, rr.Body.String())
			}
		}()
	}
	wg.Wait()
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision+1 || after.Tasks[0].PlanMaxAttempts != 4 {
		t.Fatal("duplicate grant", after.Revision)
	}
}

func TestCorrectiveRecoveryLaunchStillEnforcesLimitAndReviewer(t *testing.T) {
	s, w, r := correctiveFixture(t)
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	ps.Providers["fixture"] = Provider{Command: "/bin/true"}
	raw, _ := json.Marshal(ps)
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	for _, attempt := range w.Tasks[0].Attempts {
		a := Agent{ID: "agent-" + attempt.ID, WorkID: w.ID, TaskID: r.Task, Attempt: attempt.ID, Status: "completed"}
		raw, _ := json.Marshal(a)
		if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, r.Task, s.root, a.Status, raw, []byte("{}")); err != nil {
			t.Fatal(err)
		}
	}
	launch := Launch{Schema: 1, EventID: "fourth-launch", Revision: w.Revision, TaskID: r.Task, Provider: "fixture", Role: "worker", Instruction: r.RecoveryInstruction}
	if _, _, err := s.prepareLaunch(w.ID, launch, true); err == nil || !strings.Contains(err.Error(), "Plafond du plan") {
		t.Fatal("unapproved fourth launch", err)
	}
	w, err = s.planningChange(w.ID, "authorize-recovery", r)
	if err != nil {
		t.Fatal(err)
	}
	launch.Revision = w.Revision
	if _, _, err := s.prepareLaunch(w.ID, launch, true); err != nil {
		t.Fatal("authorized fourth preview", err)
	}
	// Revoking reviewer configuration still blocks the authorized correction.
	ps.Providers = map[string]Provider{"fixture": ps.Providers["fixture"]}
	raw, _ = json.Marshal(ps)
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	if _, _, err := s.prepareLaunch(w.ID, launch, true); err == nil || !strings.Contains(err.Error(), "vérificateur") {
		t.Fatal("reviewer bypassed", err)
	}
	// Restore the independent fixture, then consume the final allowance in isolation.
	w = managedReviewFixture(t, s, w)
	a := Agent{ID: "agent-fourth", WorkID: w.ID, TaskID: r.Task, Attempt: "fourth", Status: "completed"}
	raw, _ = json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, r.Task, s.root, a.Status, raw, []byte("{}")); err != nil {
		t.Fatal(err)
	}
	w.Tasks[0].Attempts = append(w.Tasks[0].Attempts, Attempt{ID: "fourth", Status: "completed"})
	raw, _ = json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	launch.EventID = "fifth-launch"
	if _, _, err := s.prepareLaunch(w.ID, launch, true); err == nil || !strings.Contains(err.Error(), "Plafond du plan") {
		t.Fatal("fifth launch allowed", err)
	}
}

func TestCorrectiveRecoveryBrowserRecipe(t *testing.T) {
	binary, out := os.Getenv("SWARM_CORRECTIVE_UI_BINARY"), os.Getenv("SWARM_CORRECTIVE_UI_OUT")
	if binary == "" {
		t.Skip("set SWARM_CORRECTIVE_UI_BINARY and SWARM_CORRECTIVE_UI_OUT")
	}
	if out == "" {
		t.Fatal("output directory required")
	}
	s, w, _ := correctiveFixture(t)
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command("node", "tests/corrective_recovery_ui.cjs", binary, out, s.root, w.ID)
	b, err := cmd.CombinedOutput()
	t.Log(string(b))
	if err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	if after.Tasks[0].PlanMaxAttempts != 4 || after.Tasks[0].CorrectiveRecovery == nil || len(after.Tasks[0].Attempts) != 3 || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || !s.paused(w.ID) {
		t.Fatal("browser authorization changed history, reviews or pause")
	}
}
