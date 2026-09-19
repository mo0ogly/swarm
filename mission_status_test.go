package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
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
			in.agents = []Agent{{TaskID: "t1", Status: "failed", Origin: originConductor, Activity: "temporary failure", Recovery: RecoveryState{AutomaticUsed: 2, AutomaticMax: 2, BudgetRemaining: 0}}, {TaskID: "t1", Status: "interrupted", Origin: originConductor}}
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
		{"external workspace", "waiting", "occupé", func(in *dispatchInputs) { in.occupiedWorkspaces = []string{"/tmp/ws/child"} }},
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

func TestMissionStatusPresentsMissingProfileAsPreparationWithoutLaunch(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	d, e := s.missionStatus(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if len(d.Tasks) != 1 || d.Tasks[0].State != "configure" || d.Tasks[0].Action != "configure" || d.Tasks[0].Label != "Préparer le lancement" {
		t.Fatalf("%+v", d)
	}
	if !strings.Contains(d.Tasks[0].Reason, "préparer") || !strings.Contains(d.Next, "Préparer une mission hiérarchique") {
		t.Fatal(d.Next)
	}
	agents, e := s.agents(w.ID)
	if e != nil || len(agents) != 0 {
		t.Fatal(agents, e)
	}
}

func TestMissionLaunchPreviewUsesDispatcherAndDoesNotMutate(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	w, err := s.executeRequest(w.ID, "task.add", Request{Schema: 1, EventID: "preview-second", Revision: w.Revision, ID: "t2", Title: "Deuxième tâche", Deliverable: "rapport", Criteria: []string{"preuve"}, Owner: "fixture", Next: "lancer"})
	if err != nil {
		t.Fatal(err)
	}
	profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
	preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, profile, 2)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Revision != w.Revision || preview.Immediate != 1 || preview.EffectiveConcurrency != 1 || len(preview.Departures) != 1 || len(preview.Waiting) != 1 {
		t.Fatalf("shared workspace preview: %+v", preview)
	}
	if !strings.Contains(preview.Waiting[0].Reason, "espace partagé") || !preview.SharedWorkspace || preview.ConcurrencyMode != "sequential_shared" || !strings.Contains(preview.ConcurrencyDetail, "un écrivain") {
		t.Fatalf("waiting not explained: %+v", preview)
	}
	current, _ := s.get(w.ID)
	if current.Profile != nil || current.Revision != w.Revision {
		t.Fatalf("preview mutated work: %+v", current)
	}
}

func TestMissionLaunchPreviewNamesPreconfiguredParallelismAndIntegrationLimit(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Deuxième tâche", Deliverable: "rapport", Criteria: []string{"preuve"}, Next: "lancer"})
	for _, name := range []string{"isole-a", "isole-b"} {
		if err := os.Mkdir(filepath.Join(s.root, name), 0700); err != nil {
			t.Fatal(err)
		}
	}
	if err := s.setProfile(w.ID, "t1", LaunchProfile{Provider: "fixture", Workspace: "isole-a", Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	if err := s.setProfile(w.ID, "t2", LaunchProfile{Provider: "fixture", Workspace: "isole-b", Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, 2)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Immediate != 2 || preview.EffectiveConcurrency != 2 || preview.SharedWorkspace || preview.ConcurrencyMode != "parallel_isolated_unintegrated" {
		t.Fatalf("isolated preview: %+v", preview)
	}
	if !strings.Contains(preview.ConcurrencyDetail, "n’intègre") || !strings.Contains(preview.ConcurrencyDetail, "contrôlés") {
		t.Fatalf("integration limit hidden: %+v", preview)
	}
	// A third task sharing a workspace but waiting for t1 must not hide the
	// two disjoint departures or their external integration requirement.
	w, _ = s.get(w.ID)
	w = applyTest(t, s, w, "task.add", Request{ID: "t3", Title: "Suite partagée", Deliverable: "rapport", Criteria: []string{"preuve"}, Next: "attendre", Depends: []string{"t1"}})
	if err := s.setProfile(w.ID, "t3", LaunchProfile{Provider: "fixture", Workspace: "isole-a", Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	mixed, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, 2)
	if err != nil || mixed.Immediate != 2 || !mixed.SharedWorkspace || mixed.ConcurrencyMode != "parallel_isolated_unintegrated" {
		t.Fatalf("mixed preview: %+v %v", mixed, err)
	}

}

func TestMissionLaunchPreviewTreatsNestedWorkspacesAsShared(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Deuxième tâche", Deliverable: "rapport", Criteria: []string{"preuve"}, Next: "lancer"})
	if err := os.MkdirAll(filepath.Join(s.root, "parent", "enfant"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := s.setProfile(w.ID, "t1", LaunchProfile{Provider: "fixture", Workspace: "parent", Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	if err := s.setProfile(w.ID, "t2", LaunchProfile{Provider: "fixture", Workspace: "parent/enfant", Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, 2)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Immediate != 1 || !preview.SharedWorkspace || preview.ConcurrencyMode != "sequential_shared" {
		t.Fatalf("nested workspaces not serialized: %+v", preview)
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
	if e = organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 2); e != nil {
		t.Fatal(e)
	}
	if e = s.setMission(w.ID, true); e != nil {
		t.Fatal(e)
	}
	d, e = s.missionStatus(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if d.Tasks[0].State != "waiting" || d.Tasks[0].Action != "supervise" || d.Enabled || !d.Authorized {
		t.Fatalf("consent without conductor: %+v", d)
	}
	now := time.Now()
	if e = s.beginMissionSupervision(w.ID, "test-conductor", "mission watch", now); e != nil {
		t.Fatal(e)
	}
	if e = s.checkMissionSupervision(w.ID, "test-conductor", now, nil); e != nil {
		t.Fatal(e)
	}
	d, e = s.missionStatus(w.ID)
	if e != nil || d.Tasks[0].State != "ready" || !d.Enabled || d.Supervision.State != "active" {
		t.Fatalf("consent with conductor: %+v %v", d, e)
	}
}

func TestMissionUnderstandingDistinguishesWaitingBlockingAndHumanDecision(t *testing.T) {
	mission := MissionStatus{Enabled: true}
	cases := []struct {
		name      string
		task      MissionTask
		situation string
		actor     string
	}{
		{"normal wait", MissionTask{State: "waiting", Reason: "Attend un prérequis", Label: "Examiner le prérequis"}, "attente_normale", "Le superviseur"},
		{"block", MissionTask{State: "intervention", Reason: "Le contrôle a échoué", Label: "Comprendre et résoudre"}, "blocage", "Vous"},
		{"human decision", MissionTask{State: "review", Reason: "Résultat à examiner", Label: "Examiner le résultat"}, "decision_humaine", "Vous"},
		{"missing agent", MissionTask{ID: "t1", State: "running"}, "information_manquante", "Le superviseur"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := taskUnderstanding(tc.task, mission, nil)
			if got.Situation != tc.situation || got.Actor != tc.actor || got.What == "" || got.NextStep == "" {
				t.Fatalf("understanding: %+v", got)
			}
		})
	}
	running := taskUnderstanding(MissionTask{ID: "t1", State: "running"}, mission, []Agent{{TaskID: "t1", Status: "running", Host: hostIdentity(), Heartbeat: now()}})
	if running.ActorKind != "agent" || running.Situation != "en_cours" || !strings.Contains(running.What, "travaille") {
		t.Fatalf("observed agent: %+v", running)
	}
	missingSupervisor := taskUnderstanding(MissionTask{State: "waiting", Action: "supervise", Reason: "Aucun conducteur actif"}, MissionStatus{Authorized: true}, nil)
	if missingSupervisor.Situation != "information_manquante" || missingSupervisor.ActorKind != "user" {
		t.Fatalf("missing supervisor: %+v", missingSupervisor)
	}
	manualWait := taskUnderstanding(MissionTask{State: "waiting", Reason: "Attend un prérequis"}, MissionStatus{}, nil)
	if manualWait.Situation != "attente_normale" || manualWait.ActorKind != "user" {
		t.Fatalf("manual wait: %+v", manualWait)
	}
}

func TestMissionStatusCLIPrintsSameUnderstanding(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	d, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	var out bytes.Buffer
	if err = missionCLI(s, []string{"mission", "status", w.ID}, "", false, &out); err != nil {
		t.Fatal(err)
	}
	text := out.String()
	for _, expected := range []string{
		"Mission\n  Ce qui se passe : " + d.Understanding.What,
		"  Prochaine étape : " + d.Understanding.NextStep,
		"  Qui agit : " + d.Understanding.Actor,
		"Tâche — " + d.Tasks[0].Title,
		"  Ce qui se passe : " + d.Tasks[0].Understanding.What,
		"  Prochaine étape : " + d.Tasks[0].Understanding.NextStep,
		"  Qui agit : " + d.Tasks[0].Understanding.Actor,
	} {
		if !strings.Contains(text, expected) {
			t.Fatalf("sortie CLI sans %q :\n%s", expected, text)
		}
	}
}

func TestMissionUnderstandingAutomaticReviewActor(t *testing.T) {
	task := MissionTask{State: "review", ValidationMode: "automatic"}
	for _, tc := range []struct {
		enabled, paused, authorized bool
		actor                       string
	}{
		{true, false, true, "supervisor"}, {false, false, true, "user"}, {false, true, true, "user"}, {false, false, false, "user"},
	} {
		d := MissionStatus{Total: 1, Review: 1, Enabled: tc.enabled, Paused: tc.paused, Authorized: tc.authorized, Tasks: []MissionTask{task}}
		for _, got := range []MissionUnderstanding{taskUnderstanding(task, d, nil), missionUnderstanding(d)} {
			if got.ActorKind != tc.actor || got.Situation == "decision_humaine" {
				t.Fatalf("automatic review: %+v", got)
			}
		}
	}
}
