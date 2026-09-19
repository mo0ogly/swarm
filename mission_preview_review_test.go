package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestMissionPreviewDoesNotPromiseUnaffordableDeparture(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	if err := s.setBudget(w.ID, Budget{Limit: 1, Reserve: 2, Source: "fixture", PriceDate: "2026-09-18"}); err != nil {
		t.Fatal(err)
	}
	p, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, 2)
	if err != nil {
		t.Fatal(err)
	}
	if p.Immediate != 0 {
		t.Fatalf("aperçu trompeur: %d départ annoncé malgré une réserve supérieure au budget", p.Immediate)
	}
	if len(p.Waiting) == 0 {
		t.Fatal("motif de budget absent")
	}
}

func TestMissionPreviewAggregatesFirstWaveReservations(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Deuxième départ", Deliverable: "rapport", Criteria: []string{"preuve"}})
	for _, name := range []string{"workspace-1", "workspace-2"} {
		if err := os.MkdirAll(filepath.Join(s.root, name), 0700); err != nil {
			t.Fatal(err)
		}
	}
	if err := s.setProfile(w.ID, "t1", LaunchProfile{Provider: "fixture", Workspace: "workspace-1", Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	if err := s.setProfile(w.ID, "t2", LaunchProfile{Provider: "fixture", Workspace: "workspace-2", Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	if err := s.setBudget(w.ID, Budget{Limit: 3, Reserve: 2, Source: "fixture", PriceDate: "2026-09-18"}); err != nil {
		t.Fatal(err)
	}
	preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, 2)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Immediate != 1 || len(preview.Waiting) != 1 || !strings.Contains(preview.Waiting[0].Reason, "Budget") {
		t.Fatalf("réservation de vague non agrégée : %+v", preview)
	}
}

func TestMissionPreviewPrevalidatesTaskProfileAndAttemptCeiling(t *testing.T) {
	t.Run("task provider unavailable", func(t *testing.T) {
		s := storeTest(t)
		w, _ := setupAgent(t, s)
		w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Tâche de repli", Deliverable: "rapport", Criteria: []string{"preuve"}})
		raw, err := os.ReadFile(filepath.Join(s.root, ".swarm/providers.json"))
		if err != nil {
			t.Fatal(err)
		}
		var providers Providers
		if err = json.Unmarshal(raw, &providers); err != nil {
			t.Fatal(err)
		}
		providers.Providers["indisponible"] = Provider{Command: filepath.Join(s.root, "absent")}
		raw, _ = json.Marshal(providers)
		if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
			t.Fatal(err)
		}
		if err = s.setProfile(w.ID, "t1", LaunchProfile{Provider: "indisponible", Workspace: s.root, Role: "worker"}, w.Revision); err != nil {
			t.Fatal(err)
		}
		preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, 1)
		if err != nil {
			t.Fatal(err)
		}
		if preview.Immediate != 1 || preview.Departures[0].ID != "t2" {
			t.Fatalf("la tâche valide devait remplacer le profil indisponible : %+v", preview)
		}
		if len(preview.Waiting) == 0 || !strings.Contains(preview.Waiting[0].Reason, "indisponible") {
			t.Fatalf("fournisseur indisponible non expliqué : %+v", preview.Waiting)
		}
	})

	t.Run("plan attempt ceiling", func(t *testing.T) {
		s := storeTest(t)
		w, launch := setupAgent(t, s)
		w = applyTest(t, s, w, "task.update", Request{ID: "t1", MaxAttempts: 1})
		launch.Revision = w.Revision
		agent, created, err := s.prepare(w.ID, launch)
		if err != nil || !created {
			t.Fatal(err)
		}
		if err = s.reconcile(agent.ID); err != nil {
			t.Fatal(err)
		}
		preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, 1)
		if err != nil {
			t.Fatal(err)
		}
		if preview.Immediate != 0 || len(preview.Waiting) != 1 || !strings.Contains(preview.Waiting[0].Reason, "Plafond du plan") {
			t.Fatalf("plafond de tentatives absent : %+v", preview)
		}
	})
}

func TestMissionStartRejectsStalePreviewAndReplaysSuccessfulRequest(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
	preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, profile, 1)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Token == "" {
		t.Fatal("jeton de verdict absent")
	}
	if err = s.setBudget(w.ID, Budget{Limit: 4, Reserve: 1, Source: "fixture", PriceDate: "2026-09-18"}); err != nil {
		t.Fatal(err)
	}
	request := webRequest{Kind: "mission-start", Work: w.ID, Event: "mission-preview-stale", Revision: w.Revision,
		Provider: "fixture", Workspace: s.root, Slots: 1, Level: "auto", PreviewToken: preview.Token}
	if _, err = s.webAction(request); err == nil || !strings.Contains(err.Error(), "aperçu") {
		t.Fatalf("aperçu périmé accepté : %v", err)
	}
	if policy, _ := s.missionPolicy(w.ID); policy.Enabled {
		t.Fatal("un aperçu périmé a autorisé la mission")
	}
	preview, err = organizedFixtureStore(t, s).missionLaunchPreview(w.ID, profile, 1)
	if err != nil {
		t.Fatal(err)
	}
	request.PreviewToken = preview.Token
	if _, err = s.webAction(request); err != nil {
		t.Fatal(err)
	}
	if err = s.setBudget(w.ID, Budget{Limit: 5, Reserve: 1, Source: "fixture modifiée", PriceDate: "2026-09-18"}); err != nil {
		t.Fatal(err)
	}
	if _, err = s.webAction(request); err != nil {
		t.Fatalf("rejeu idempotent refusé après changement d’état : %v", err)
	}
}

func TestMissionPreviewOriginCannotBypassPauseForRealLaunch(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	r.Origin = originMissionPreview
	if _, _, err := s.prepareLaunch(w.ID, r, false); err == nil {
		t.Fatal("real launch bypassed pause")
	}
}
