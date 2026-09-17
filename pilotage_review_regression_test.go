//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"testing"
	"time"
)

func TestReviewRejectsDuplicateDependency(t *testing.T) {
	if err := validateTaskGraph([]Task{{ID: "a"}, {ID: "b", Depends: []string{"a", "a"}}}); err == nil {
		t.Fatal("duplicate edge accepted")
	}
}

func TestReviewRetainsFinishedAttemptOutsideRecentWindow(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.finishAgent(a, "failed", "fixture", nil); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 205; i++ {
		x := Agent{ID: fmt.Sprintf("later-%d", i), WorkID: w.ID, TaskID: "other", Status: "completed"}
		raw, _ := json.Marshal(x)
		if _, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", x.ID, w.ID, x.TaskID, x.ID, x.Status, raw, []byte("{}")); err != nil {
			t.Fatal(err)
		}
	}
	agents, err := s.pilotAgents(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(r.TaskID)
	if !hasFinishedAttempt(task, agents) {
		t.Fatal("only restartable attempt omitted")
	}
	found := false
	for _, x := range agents {
		if x.ID == a.ID {
			found = true
		}
	}
	if !found {
		t.Fatal("attempt unavailable to retry selector")
	}
	if len(agents) != 201 {
		t.Fatalf("unexpected coverage %d", len(agents))
	}
	r.EventID, r.Revision, r.Previous = "retry-archived", w.Revision, a.ID
	next, created, err := s.prepare(w.ID, r)
	if err != nil || !created || next.Previous != a.ID {
		t.Fatal("archived attempt cannot actually be retried", next, err)
	}
}

func TestReviewActivityThresholdIsServerDated(t *testing.T) {
	instant := time.Date(2026, 9, 17, 12, 0, 0, 0, time.UTC)
	a := Agent{Status: "running", Host: "other-host", Limits: RunLimits{SilenceSeconds: 45}}
	for _, tc := range []struct {
		age  time.Duration
		want string
	}{{44 * time.Second, "recent"}, {45 * time.Second, "recent"}, {45*time.Second + time.Nanosecond, "old"}} {
		a.Progress.LastResult = instant.Add(-tc.age).Format(time.RFC3339Nano)
		if got := pilotAgentHealth(a, "", instant.Format(time.RFC3339Nano)); got["activity_state"] != tc.want {
			t.Fatal(got, tc)
		}
	}
}

func TestReviewAgentCommandReceiptAndTarget(t *testing.T) {
	for _, kind := range []string{"stop", "reconcile", "retry"} {
		t.Run(kind, func(t *testing.T) {
			s := storeTest(t)
			w, r := setupAgent(t, s)
			a, _, err := s.prepare(w.ID, r)
			if err != nil {
				t.Fatal(err)
			}
			if kind == "retry" {
				if err = s.finishAgent(a, "failed", "retry target fixture", nil); err != nil {
					t.Fatal(err)
				}
			}
			w, _ = s.get(w.ID)
			wrong := webRequest{Kind: kind, Work: w.ID, Task: "wrong-task", Agent: a.ID, Event: "wrong-command", Revision: w.Revision}
			if _, err = s.webAction(wrong); err == nil {
				t.Fatal("mismatched task accepted")
			}
			desired, _ := s.desired(a.ID)
			if desired != "" {
				t.Fatal("mismatched task mutated agent", desired)
			}
		})
	}
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	command := webRequest{Kind: "stop", Work: w.ID, Task: a.TaskID, Agent: a.ID, Event: "stop-once", Revision: w.Revision}
	if _, err = s.webAction(command); err != nil {
		t.Fatal(err)
	}
	if err = s.finishAgent(a, "interrupted", "fixture exit", nil); err != nil {
		t.Fatal(err)
	}
	// Finishing changes the work revision and makes an ordinary stop invalid.
	if _, err = s.webAction(command); err != nil {
		t.Fatal("lost response cannot replay after completion", err)
	}
	command.Agent = "different-agent"
	if _, err = s.webAction(command); err == nil {
		t.Fatal("same command ID accepted another target")
	}
	var count int
	s.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE kind='agent-command'").Scan(&count)
	if count != 1 {
		t.Fatal("duplicate command", count)
	}
}
