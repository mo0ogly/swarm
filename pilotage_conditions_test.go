//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestPilotageHealthSeparatesProcessAndActivity(t *testing.T) {
	at := now()
	base := Agent{ID: "session", Attempt: "attempt", Status: "running", Host: hostIdentity(), Heartbeat: at, Supervisor: os.Getpid(), SupervisorStamp: processStamp(os.Getpid()), Limits: RunLimits{SilenceSeconds: 30}}
	cases := []struct {
		name, process, activity, desired string
		change                           func(*Agent)
	}{
		{"alive_without_result", "running", "unknown", "", func(a *Agent) {}},
		{"recent_result_lost_supervisor", "unknown/supervisor-lost", "recent", "", func(a *Agent) { a.SupervisorStamp = "different-process"; a.Progress.LastResult = at }},
		{"other_host", "unknown/other-host", "unknown", "", func(a *Agent) { a.Host = "other-host" }},
		{"no_heartbeat", "running/unconfirmed", "unknown", "", func(a *Agent) { a.Heartbeat = "" }},
		{"old_heartbeat", "unknown/no-heartbeat", "recent", "", func(a *Agent) {
			a.Heartbeat = time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano)
			a.Progress.LastResult = at
		}},
		{"stop_requested", "running", "unknown", "stop", func(a *Agent) {}},
		{"old_result_alive", "running", "old", "", func(a *Agent) { a.Progress.LastResult = time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano) }},
		{"degraded_recent_result", "running", "unknown", "", func(a *Agent) { a.Progress.LastResult = at; a.Progress.Degraded = "fixture missing events" }},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			a := base
			tc.change(&a)
			h := pilotAgentHealth(a, tc.desired, at)
			if h["process_state"] != tc.process || h["activity_state"] != tc.activity {
				t.Fatal(h)
			}
			if h["stop_requested"] != (tc.desired == "stop") {
				t.Fatal("stop intention not preserved", h)
			}
			if tc.desired == "stop" && !strings.Contains(h["process_label"].(string), "confirmation attendue") {
				t.Fatal("stop falsely confirmed", h)
			}
		})
	}
}

func TestPilotageExitZeroDoesNotValidateDelivery(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	zero := 0
	if err := s.finishAgent(a, "completed", "fixture exit zero", &zero); err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	w, _ = s.get(w.ID)
	v := s.validationState(&w)
	h := pilotAgentHealth(a, "", now())
	if h["process_state"] != "completed" || w.Tasks[0].Status == "accepted" || v.Validated != 0 {
		t.Fatal("process success confused with delivery validation", h, w.Tasks[0], v)
	}
}

func TestPilotageLaunchRechecksConditionsAfterPreview(t *testing.T) {
	for _, condition := range []string{"pause", "same_workspace", "child_workspace", "parent_workspace", "provider_removed", "budget"} {
		t.Run(condition, func(t *testing.T) {
			s := storeTest(t)
			w, launch := setupAgent(t, s)
			launch.Workspace = filepath.Join(s.root, "target")
			if err := os.MkdirAll(filepath.Join(launch.Workspace, "nested"), 0700); err != nil {
				t.Fatal(err)
			}
			r := webRequest{Kind: "launch-preview", Work: w.ID, Revision: w.Revision, Task: launch.TaskID, Provider: launch.Provider, Workspace: launch.Workspace}
			if result := s.launchEligibility(r); result["eligible"] != true {
				t.Fatal("initial preview refused", result)
			}
			want := ""
			switch condition {
			case "pause":
				want = "suspendus"
				if err := s.pause(w.ID, true); err != nil {
					t.Fatal(err)
				}
			case "provider_removed":
				want = "fournisseur"
				if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), []byte(`{"schema_version":1,"providers":{}}`), 0600); err != nil {
					t.Fatal(err)
				}
			case "budget":
				want = "budget"
				if err := s.setBudget(w.ID, Budget{Limit: 1, Reserve: 2, Source: "fixture", PriceDate: "2026-09-15"}); err != nil {
					t.Fatal(err)
				}
			default:
				want = "espace de travail"
				occupied := launch.Workspace
				if condition == "child_workspace" {
					occupied = filepath.Join(occupied, "nested")
				}
				if condition == "parent_workspace" {
					occupied = s.root
				}
				a := Agent{ID: "occupying-agent", WorkID: w.ID, TaskID: "another-task", CWD: occupied, Status: "running"}
				raw, _ := json.Marshal(a)
				if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, occupied, a.Status, raw, []byte("{}")); err != nil {
					t.Fatal(err)
				}
			}
			if result := s.launchEligibility(r); result["eligible"] != false || !strings.Contains(strings.ToLower(result["reason_label"].(string)), want) {
				t.Fatal("preview did not reflect changed condition", result)
			}
			if _, created, err := s.prepare(w.ID, launch); err == nil || created || !strings.Contains(strings.ToLower(err.Error()), want) {
				t.Fatal("actual launch bypassed changed condition", created, err)
			}
			current, _ := s.get(w.ID)
			var count int
			if err := s.db.QueryRow("SELECT count(*) FROM agents WHERE id=?", launch.EventID).Scan(&count); err != nil || count != 0 {
				t.Fatal("refused launch persisted agent", count, err)
			}
			if err := s.db.QueryRow("SELECT count(*) FROM reservations WHERE agent_id=?", launch.EventID).Scan(&count); err != nil || count != 0 {
				t.Fatal("refused launch reserved budget", count, err)
			}
			if current.Revision != w.Revision {
				t.Fatal("refused launch changed work revision", current.Revision, w.Revision)
			}
		})
	}
}
