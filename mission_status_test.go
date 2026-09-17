package main

import (
	"strings"
	"testing"
)

func TestMissionDispatchExplainsAutomaticRetentions(t *testing.T) {
	for _, tc := range []struct {
		name, state, reason string
		change              func(*dispatchInputs)
	}{
		{"ready", "ready", "automatique", func(in *dispatchInputs) {}},
		{"operator stop", "intervention", "opérateur", func(in *dispatchInputs) {
			in.agents = []Agent{{TaskID: "t1", Status: "interrupted", StopKind: originOperator}}
		}},
		{"retry ceiling", "intervention", "2 tentatives", func(in *dispatchInputs) {
			in.agents = []Agent{{TaskID: "t1", Status: "failed", Origin: originConductor}, {TaskID: "t1", Status: "interrupted", Origin: originConductor}}
		}},
		{"missing handoff", "intervention", "rapport", func(in *dispatchInputs) {
			in.work.Tasks[0].Status = "blocked"
			in.agents = []Agent{{TaskID: "t1", Status: "completed"}}
		}},
		{"cost threshold", "intervention", "coût", func(in *dispatchInputs) { in.reserve = 1; in.taskCost = map[string]CostTotal{"t1": {Reported: 3}} }},
		{"no profile", "intervention", "profil", func(in *dispatchInputs) { in.profile = nil }},
		{"dependency", "waiting", "Dépendance", func(in *dispatchInputs) { in.depsReady["t1"] = false }},
		{"pause", "waiting", "pause", func(in *dispatchInputs) { in.paused = true }},
		{"blocked reason while paused", "intervention", "Observation utilisateur", func(in *dispatchInputs) {
			in.paused = true
			in.work.Tasks[0].Status = "blocked"
			in.work.Tasks[0].Blocker = "Observation utilisateur nécessaire ; aucun agent à relancer"
		}},
		{"manual", "manual", "désactivés", func(in *dispatchInputs) { in.autonomy = autonomyManual }},
		{"slots", "waiting", "occupé", func(in *dispatchInputs) {
			in.slots = 1
			in.agents = []Agent{{TaskID: "other", Status: "running", CWD: "/tmp/other"}}
		}},
		{"workspace", "waiting", "occupé", func(in *dispatchInputs) { in.agents = []Agent{{TaskID: "other", Status: "running", CWD: "/tmp/ws"}} }},
	} {
		t.Run(tc.name, func(t *testing.T) {
			in := dispatchTest([]Task{{ID: "t1", Status: "todo"}}, nil)
			tc.change(&in)
			state, reason := missionDispatchState(in, in.work.Tasks[0])
			if state != tc.state || !strings.Contains(reason, tc.reason) {
				t.Fatalf("got %q %q, want %q containing %q", state, reason, tc.state, tc.reason)
			}
			plan, _ := planDispatch(in)
			if state == "ready" && len(plan) != 1 {
				t.Fatal("ready without dispatch candidate")
			}
		})
	}
}

func TestMissionStatusSurfacesMissingProfileWithoutLaunch(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	d, e := s.missionStatus(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if len(d.Tasks) != 1 || d.Tasks[0].State != "intervention" || !strings.Contains(d.Tasks[0].Reason, "profil") {
		t.Fatalf("%+v", d)
	}
	if !strings.Contains(d.Next, "intervention") {
		t.Fatal(d.Next)
	}
	agents, e := s.agents(w.ID)
	if e != nil || len(agents) != 0 {
		t.Fatal(agents, e)
	}
}

func TestMissionStatusRequiresConsentForReadyLabel(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	if e := s.setProfile(w.ID, "", LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, w.Revision); e != nil {
		t.Fatal(e)
	}
	d, e := s.missionStatus(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if d.Tasks[0].State != "manual" {
		t.Fatalf("without consent: %+v", d.Tasks[0])
	}
	if e = s.setAutonomy(w.ID, autonomyAuto, 2); e != nil {
		t.Fatal(e)
	}
	if e = s.setMission(w.ID, true); e != nil {
		t.Fatal(e)
	}
	d, e = s.missionStatus(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if d.Tasks[0].State != "ready" {
		t.Fatalf("with consent: %+v", d.Tasks[0])
	}
}
