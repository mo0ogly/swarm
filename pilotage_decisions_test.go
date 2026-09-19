//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"strings"
	"testing"
	"time"
)

func TestPilotageDecisionsIncludeOldActiveSignalLost(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	old, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	old.Heartbeat = time.Now().Add(-time.Hour).UTC().Format(time.RFC3339Nano)
	if err := s.saveAgent(old); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 205; i++ {
		a := Agent{ID: fmt.Sprintf("later-%03d", i), WorkID: w.ID, TaskID: "unrelated", Status: "completed"}
		raw, _ := json.Marshal(a)
		if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, a.ID, a.Status, raw, []byte("{}")); err != nil {
			t.Fatal(err)
		}
	}
	decisions, err := s.decisions(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	for _, d := range decisions {
		if d.AgentID == old.ID && d.Kind == "silence" && d.ResolvedAt == "" {
			return
		}
	}
	t.Fatal("old active signal-loss decision omitted", decisions)
}

func TestPilotageStopPendingOracleKeepsReconcile(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	if err := s.stopAgent(a.ID); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	agents, _ := s.pilotAgents(w.ID)
	actions := s.taskActions(&w, &w.Tasks[0], agents)
	stopSeen, reconcileSeen := false, false
	for _, action := range actions {
		switch action.Kind {
		case "stop":
			stopSeen = true
			if action.Disponible || !strings.Contains(action.Raison, "déjà demandé") {
				t.Fatal(action)
			}
		case "reconcile":
			reconcileSeen = true
			if !action.Disponible || action.Raison != "" {
				t.Fatal(action)
			}
		}
	}
	if !stopSeen || !reconcileSeen {
		t.Fatal(actions)
	}
}
