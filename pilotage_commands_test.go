//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestPilotageTaskActionsRetainOldActiveAgent(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	old, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 205; i++ {
		a := Agent{ID: fmt.Sprintf("finished-%03d", i), WorkID: w.ID, TaskID: "other", Status: "completed"}
		raw, _ := json.Marshal(a)
		if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, a.ID, a.Status, raw, []byte("{}")); err != nil {
			t.Fatal(err)
		}
	}
	h := newWebHandler(s, "local.test", "test-session")
	req := httptest.NewRequest("GET", "http://local.test/api/v1/task?work="+w.ID+"&task="+old.TaskID, nil)
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "test-session"})
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, req)
	if rr.Code != http.StatusOK {
		t.Fatal(rr.Code, rr.Body.String())
	}
	var result struct {
		Actions []TaskAction `json:"actions"`
	}
	if err := json.Unmarshal(rr.Body.Bytes(), &result); err != nil {
		t.Fatal(err)
	}
	for _, action := range result.Actions {
		if action.Kind == "stop" {
			if !action.Disponible {
				t.Fatal("active attempt missing from modal oracle", action)
			}
			return
		}
	}
	t.Fatal("stop action absent")
}

func TestPilotageRetryReplayBeforeRevisionGuard(t *testing.T) {
	s := storeTest(t)
	w, first := setupAgent(t, s)
	previous, _, err := s.prepare(w.ID, first)
	if err != nil {
		t.Fatal(err)
	}
	if err := s.finishAgent(previous, "failed", "fixture finished", nil); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	r := webRequest{Kind: "retry", Work: w.ID, Agent: previous.ID, Event: "retry-replay", Revision: w.Revision, Instruction: "explicit retry"}
	level := ""
	if previous.ModelRoute != nil {
		level = previous.ModelRoute.Level
	}
	// Persist the first request without starting any external process, then
	// simulate the same HTTP request after its response was lost.
	next, created, err := s.prepare(w.ID, Launch{Schema: 1, EventID: r.Event, Revision: r.Revision, TaskID: previous.TaskID, Provider: previous.Provider, Role: previous.Role, Workspace: previous.CWD, Instruction: r.Instruction, Previous: previous.ID, Parent: previous.Parent, Level: level})
	if err != nil || !created {
		t.Fatal(created, err)
	}
	result, err := s.webAction(r)
	if err != nil {
		t.Fatal("same persisted retry refused after revision changed", err)
	}
	if result.(Agent).ID != next.ID {
		t.Fatal("replay changed session identity", result)
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 2 {
		t.Fatal("replay created extra attempt", len(agents))
	}
	r.Instruction = "changed command"
	if _, err := s.webAction(r); err == nil {
		t.Fatal("same event ID accepted different request")
	}
	r.Event = "different-event"
	if _, err := s.webAction(r); err == nil {
		t.Fatal("new retry accepted stale revision")
	}
}

func TestPilotageStopRepeatedIntentLogsOnce(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 3; i++ {
		if err := s.stopAgent(a.ID); err != nil {
			t.Fatal(err)
		}
	}
	var count int
	if err := s.db.QueryRow("SELECT count(*) FROM agent_logs WHERE agent_id=? AND message=?", a.ID, "Arrêt demandé ; attendre confirmation du superviseur").Scan(&count); err != nil || count != 1 {
		t.Fatal("repeated stop logged more than once", count, err)
	}
	if desired, err := s.desired(a.ID); err != nil || desired != "stop" {
		t.Fatal(desired, err)
	}
}
