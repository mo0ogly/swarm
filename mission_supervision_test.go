package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestMissionLoopPublishesAndStopsItsLease(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	ctx, cancel := context.WithCancel(context.Background())
	done := make(chan struct{})
	go func() {
		s.missionLoop(ctx, w.ID)
		close(done)
	}()
	deadline := time.Now().Add(time.Second)
	for {
		d, err := s.missionSupervision(w.ID, time.Now())
		if err == nil && d.State == "active" && d.LastCheckAt != "" {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("conductor lease not published: %+v %v", d, err)
		}
		time.Sleep(10 * time.Millisecond)
	}
	cancel()
	select {
	case <-done:
	case <-time.After(time.Second):
		t.Fatal("mission loop did not stop")
	}
	d, err := s.missionSupervision(w.ID, time.Now())
	if err != nil || d.State != "absent" || d.NextCheckAt != "" {
		t.Fatalf("stopped conductor still reported active: %+v %v", d, err)
	}
}

func TestMissionSupervisionSeparatesAuthorizationHealthAndActivity(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	if err := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	request.Origin = originOperator
	if _, _, err := s.prepare(w.ID, request); err != nil {
		t.Fatal(err)
	}

	d, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if !d.Authorized || d.Enabled || d.Supervision.State != "absent" || d.ActiveAgents+d.UncertainAgents != 1 {
		t.Fatalf("authorization, conductor and agents conflated: %+v", d)
	}
	if d.Supervision.State != "absent" {
		t.Fatal(d.Next)
	}
}

func TestMissionSupervisionReportsErrorRetryAndStaleness(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	at := time.Now()
	if err := s.beginMissionSupervision(w.ID, "conductor-one", "serveur web", at); err != nil {
		t.Fatal(err)
	}
	if err := s.checkMissionSupervision(w.ID, "conductor-one", at, errors.New("lecture budget impossible")); err != nil {
		t.Fatal(err)
	}
	d, err := s.missionSupervision(w.ID, at.Add(time.Second))
	if err != nil || d.State != "error" || d.LastCheckAt == "" || d.NextCheckAt == "" || d.NextCheckRelative != "dans moins de 2 s" || !strings.Contains(d.LastError, "budget") {
		t.Fatalf("error not exposed: %+v %v", d, err)
	}
	d, err = s.missionSupervision(w.ID, at.Add(missionPollInterval+time.Second))
	if err != nil || d.State != "error" || !d.Late || !strings.Contains(d.NextCheckRelative, "en retard") {
		t.Fatalf("late retry not exposed: %+v %v", d, err)
	}
	d, err = s.missionSupervision(w.ID, at.Add(missionConductorStaleAfter+time.Second))
	if err != nil || d.State != "absent" || d.NextCheckAt != "" {
		t.Fatalf("stale lease reported active: %+v %v", d, err)
	}
}

func TestMissionSupervisionProvesLastConductorActionAndActor(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	at := time.Now().Add(-time.Minute)
	if _, err := s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", w.ID, at.Format(time.RFC3339Nano), "mission-policy", `{"enabled":true,"actor":"Codex"}`); err != nil {
		t.Fatal(err)
	}
	d, err := s.missionSupervision(w.ID, time.Now())
	if err != nil || d.LastAction != nil {
		t.Fatalf("authorization or external supervisor invented an action: %+v %v", d, err)
	}
	stamp := time.Now().Add(-2 * time.Minute).Format(time.RFC3339Nano)
	if _, err = s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", w.ID, stamp, "dispatch", "t1 : départ automatique · recette · rôle worker · espace /tmp/isolé"); err != nil {
		t.Fatal(err)
	}
	d, err = s.missionSupervision(w.ID, time.Now())
	if err != nil || d.LastAction == nil || d.LastAction.Actor != "Conducteur Swarm" || d.LastAction.Kind != "dispatch" || !strings.Contains(d.LastAction.Relative, "2 min") {
		t.Fatalf("persisted conductor action missing: %+v %v", d, err)
	}
	if _, err = s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", w.ID, time.Now().Format(time.RFC3339Nano), "dispatch", "aucune tâche prête"); err != nil {
		t.Fatal(err)
	}
	d, err = s.missionSupervision(w.ID, time.Now())
	if err != nil || d.LastAction == nil || d.LastAction.Summary == "aucune tâche prête" {
		t.Fatalf("waiting explanation promoted to action: %+v %v", d, err)
	}
	var plain bytes.Buffer
	if err = missionCLI(s, []string{"mission", "status", w.ID}, "", false, &plain); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(plain.String(), "Dernière action — Conducteur Swarm") || !strings.Contains(plain.String(), "départ automatique") {
		t.Fatalf("CLI does not expose proven actor and action: %s", plain.String())
	}
}

func TestMissionSupervisionPauseAndClockChangesDoNotInventPresence(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	at := time.Now()
	if err := s.beginMissionSupervision(w.ID, "clock-test", "mission watch", at); err != nil {
		t.Fatal(err)
	}
	if err := s.checkMissionSupervision(w.ID, "clock-test", at, nil); err != nil {
		t.Fatal(err)
	}
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	d, err := s.missionSupervision(w.ID, at.Add(time.Second))
	if err != nil || d.State != "active" || d.NextCheckRelative == "" {
		t.Fatalf("real conductor lost merely because mission paused: %+v %v", d, err)
	}
	d, err = s.missionSupervision(w.ID, at.Add(-10*time.Second))
	if err != nil || d.State != "absent" || d.NextCheckAt != "" || d.ClockIssue == "" {
		t.Fatalf("backward clock change invented active lease: %+v %v", d, err)
	}
}

func TestMissionSupervisionReplacementDoesNotDoubleStart(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	if err := s.setProfile(w.ID, "", LaunchProfile{Provider: request.Provider, Workspace: request.Workspace, Role: request.Role}, w.Revision); err != nil {
		t.Fatal(err)
	}
	if err := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	if err := s.beginMissionSupervision(w.ID, "old", "serveur web", time.Now().Add(-time.Hour)); err != nil {
		t.Fatal(err)
	}
	s.missionCycle(map[string]string{}, w.ID)
	before, err := s.agents(w.ID)
	if err != nil || len(before) != 1 {
		t.Fatalf("first departure: %v %v", before, err)
	}
	now := time.Now()
	if err = s.beginMissionSupervision(w.ID, "new", "mission watch", now); err != nil {
		t.Fatal(err)
	}
	s.missionCycle(map[string]string{}, w.ID)
	if err = s.checkMissionSupervision(w.ID, "new", now, nil); err != nil {
		t.Fatal(err)
	}
	after, err := s.agents(w.ID)
	if err != nil || len(after) != 1 {
		t.Fatalf("restart duplicated departure: %v %v", after, err)
	}
	d, err := s.missionSupervision(w.ID, now)
	if err != nil || d.State != "active" || d.Source != "mission watch" || d.ReconciledAt == "" {
		t.Fatalf("replacement not reconciled: %+v %v", d, err)
	}
}

func TestMissionCLIPlainAndJSONShareSupervisionVerdict(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	if err := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	var plain, structured bytes.Buffer
	if err := missionCLI(s, []string{"mission", "status", w.ID}, "", false, &plain); err != nil {
		t.Fatal(err)
	}
	if err := missionCLI(s, []string{"mission", "status", w.ID}, "", true, &structured); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(plain.String(), "conducteur : absent") || !strings.Contains(plain.String(), "supervision absente") || !strings.Contains(plain.String(), "aucune action enregistrée") {
		t.Fatal(plain.String())
	}
	if !strings.Contains(structured.String(), `"authorized": true`) || !strings.Contains(structured.String(), `"state": "absent"`) {
		t.Fatal(structured.String())
	}
}

func TestMissionCLIAndWebShareAbsentAndErrorVerdicts(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	if err := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	h := newWebHandler(s, "local.test", "mission-test-token")
	webStatus := func() MissionStatus {
		req := httptest.NewRequest(http.MethodGet, "http://local.test/api/v1/snapshot?work="+w.ID, nil)
		req.Host = "local.test"
		req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "mission-test-token"})
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, req)
		if rr.Code != http.StatusOK {
			t.Fatalf("snapshot web: %d %s", rr.Code, rr.Body.String())
		}
		var snapshot struct {
			Mission MissionStatus `json:"mission"`
		}
		if err := json.Unmarshal(rr.Body.Bytes(), &snapshot); err != nil {
			t.Fatal(err)
		}
		return snapshot.Mission
	}
	assertShared := func(state string) {
		t.Helper()
		want, err := s.missionStatus(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		got := webStatus()
		if got.Authorized != want.Authorized || got.Enabled != want.Enabled || got.Supervision.State != want.Supervision.State || got.Supervision.LastError != want.Supervision.LastError {
			t.Fatalf("verdict %s divergent : CLI=%+v web=%+v", state, want, got)
		}
		var plain bytes.Buffer
		if err := missionCLI(s, []string{"mission", "status", w.ID}, "", false, &plain); err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(plain.String(), "conducteur : "+state) {
			t.Fatalf("CLI sans état %s : %s", state, plain.String())
		}
		if state == "error" && (!strings.Contains(plain.String(), "conducteur en erreur") || strings.Contains(plain.String(), "supervision absente")) {
			t.Fatalf("diagnostic CLI contradictoire : %s", plain.String())
		}
	}
	assertShared("absent")
	at := time.Now()
	if err := s.beginMissionSupervision(w.ID, "test-error", "mission watch", at); err != nil {
		t.Fatal(err)
	}
	if err := s.checkMissionSupervision(w.ID, "test-error", at, errors.New("lecture isolée impossible")); err != nil {
		t.Fatal(err)
	}
	assertShared("error")
}
