package main

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"
)

func TestMissionCLIPreviewUsesSharedVerdictWithoutMutation(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	if err := s.setProfile(w.ID, "", LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	organizedFixtureStore(t, s)
	w, _ = s.get(w.ID)
	var out bytes.Buffer
	if err := missionCLI(s, []string{"mission", "preview", w.ID}, "", true, &out); err != nil {
		t.Fatal(err)
	}
	var preview MissionLaunchPreview
	if err := json.Unmarshal(out.Bytes(), &preview); err != nil {
		t.Fatal(err)
	}
	if preview.Revision != w.Revision || preview.Immediate != 1 || preview.EffectiveConcurrency != 1 {
		t.Fatalf("unexpected preview: %+v", preview)
	}
	out.Reset()
	if err := missionCLI(s, []string{"mission", "preview", w.ID}, "", false, &out); err != nil {
		t.Fatal(err)
	}
	if text := out.String(); !strings.Contains(text, "concurrence réelle 1/2") || !strings.Contains(text, preview.ConcurrencyDetail) {
		t.Fatalf("plain CLI diverges from shared verdict: %q", text)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision || s.autonomy(w.ID) == autonomyAuto {
		t.Fatalf("preview mutated mission: %+v", current)
	}
}

func TestMissionRequiresExplicitLocalAuthorization(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	if p, e := s.missionPolicy(w.ID); e != nil || p.Enabled {
		t.Fatal(p, e)
	}
	if e := s.setMission(w.ID, true); e != nil {
		t.Fatal(e)
	}
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	if p, e := other.missionPolicy(w.ID); e != nil || !p.Enabled {
		t.Fatal(p, e)
	}
	if e = s.setMission(w.ID, false); e != nil {
		t.Fatal(e)
	}
	if p, e := other.missionPolicy(w.ID); e != nil || p.Enabled {
		t.Fatal(p, e)
	}
}
func TestMissionDiagnosticActionAndResult(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	d, e := s.missionStatus(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if d.Validated != 0 || d.Review != 0 || len(d.Tasks) != 1 || d.Tasks[0].Action == "" {
		t.Fatalf("%+v", d)
	}
	if strings.Contains(d.Summary, "terminée") {
		t.Fatal(d.Summary)
	}
}
func TestConductorTransactionHonorsPause(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Origin = originConductor
	if e := s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	if _, _, e := s.prepare(w.ID, r); e == nil {
		t.Fatal("paused automatic launch admitted")
	}
	a, _ := s.agents(w.ID)
	if len(a) != 0 {
		t.Fatal(a)
	}
}

func TestConductorTransactionHonorsOccupiedSlots(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if e := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); e != nil {
		t.Fatal(e)
	}
	r.Origin = originConductor
	if _, _, e := s.prepare(w.ID, r); e != nil {
		t.Fatal(e)
	}
	tx, e := s.db.Begin()
	if e != nil {
		t.Fatal(e)
	}
	defer tx.Rollback()
	if e = automaticLaunchGuard(tx, w.ID); e == nil {
		t.Fatal("occupied slot admitted")
	}
}

func TestMissionConfigurationAtomicReplayAndConflict(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	p := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
	if e := organizedFixtureStore(t, s).configureMission(w.ID, p, 1, w.Revision, "mission-config"); e != nil {
		t.Fatal(e)
	}
	current, _ := s.get(w.ID)
	if e := organizedFixtureStore(t, s).configureMission(w.ID, p, 1, w.Revision, "mission-config"); e != nil {
		t.Fatal("replay", e)
	}
	again, _ := s.get(w.ID)
	if again.Revision != current.Revision {
		t.Fatal("replay mutated")
	}
	if e := s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	if e := organizedFixtureStore(t, s).configureMission(w.ID, p, 2, w.Revision, "mission-stale"); e == nil {
		t.Fatal("stale configuration accepted")
	}
	if !s.paused(w.ID) || s.slots(w.ID) != 1 {
		t.Fatal("failed config unpaused or changed slots")
	}
	if e := s.stopMission(w.ID); e != nil {
		t.Fatal(e)
	}
	if e := s.pause(w.ID, false); e != nil {
		t.Fatal(e)
	}
	tx, e := s.db.Begin()
	if e != nil {
		t.Fatal(e)
	}
	defer tx.Rollback()
	if automaticLaunchGuard(tx, w.ID) == nil {
		t.Fatal("revoked mission relaunched")
	}
}

func TestMissionWebPreservesExistingLimits(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	p := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker", Instruction: "Contraintes communes", Timeout: 123, Capture: true, Limits: &RunLimits{MaxToolCalls: 7}}
	if e := s.setProfile(w.ID, "", p, w.Revision); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	preview, e := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, p, 1)
	if e != nil {
		t.Fatal(e)
	}
	r := webRequest{Kind: "mission-start", Work: w.ID, Event: "web-mission-limits", Revision: w.Revision, Provider: "fixture", Workspace: s.root, Slots: 1, Level: "auto", PreviewToken: preview.Token}
	if _, e := s.webAction(r); e != nil {
		t.Fatal(e)
	}
	if _, e := s.webAction(r); e != nil {
		t.Fatal("replay", e)
	}
	w, _ = s.get(w.ID)
	if w.Profile.Timeout != 123 || !w.Profile.Capture || w.Profile.Limits.MaxToolCalls != 7 || w.Profile.Instruction != "Contraintes communes" {
		t.Fatalf("lost profile %+v", w.Profile)
	}
}

func TestMissionWatchScopeAndWaitingLogDedup(t *testing.T) {
	s := storeTest(t)
	a, _ := setupAgent(t, s)
	b, _ := setupAgent(t, s)
	for _, w := range []Work{a, b} {
		if e := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, slotsDefault); e != nil {
			t.Fatal(e)
		}
		if e := s.setMission(w.ID, true); e != nil {
			t.Fatal(e)
		}
	}
	previous := map[string]string{}
	s.missionCycle(previous, a.ID)
	// Force another evaluation as an external recheck would do.
	s.missionCycle(map[string]string{}, a.ID)
	var ca, cb int
	s.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='dispatch'", a.ID).Scan(&ca)
	s.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='dispatch'", b.ID).Scan(&cb)
	if ca != 1 || cb != 0 {
		t.Fatalf("scope/dedup : %d %d", ca, cb)
	}
}

func TestMissionActivityDoesNotExposePolicyJSON(t *testing.T) {
	if got := missionActivityMessage("mission-policy", `{"enabled":true,"actor":"local","at":"now"}`); strings.Contains(got, "{") || !strings.Contains(got, "autorisée") {
		t.Fatal(got)
	}
}

func TestMissionConcurrentConductorDoesNotDoubleReserve(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if e := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); e != nil {
		t.Fatal(e)
	}
	r.Origin = originConductor
	results := make(chan error, 2)
	start := make(chan struct{})
	for _, id := range []string{"concurrent-one", "concurrent-two"} {
		go func(id string) { <-start; req := r; req.EventID = id; _, _, e := s.prepare(w.ID, req); results <- e }(id)
	}
	close(start)
	success := 0
	for i := 0; i < 2; i++ {
		if <-results == nil {
			success++
		}
	}
	if success != 1 {
		t.Fatalf("%d successful reservations", success)
	}
	agents, e := s.agents(w.ID)
	if e != nil || len(agents) != 1 {
		t.Fatal(agents, e)
	}
}
