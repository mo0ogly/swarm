//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestMissionLaunchIdentitySeparatesResolutionFromObservation(t *testing.T) {
	modelCatalogTest(t)
	t.Setenv("SWARM_LANG", "fr")
	s := storeTest(t)
	providers := Providers{Providers: map[string]Provider{"codex": {Command: "/fixture/codex"}, "legacy": {Command: "/fixture/unknown"}}}
	got, err := s.missionLaunchIdentity(LaunchProfile{Provider: "codex", Role: "worker", Level: "standard", Skills: []ActionSkillSelection{{Path: ".claude/skills/apex/SKILL.md"}}}, providers)
	if err != nil {
		t.Fatal(err)
	}
	if got.RequestedLevel != "standard" || got.ResolvedModel != "gpt-5.6-sol" || got.ActualModel != "" || len(got.Skills) != 1 || got.ProjectProfile != "" {
		t.Fatalf("resolution misrepresented: %+v", got)
	}
	var out bytes.Buffer
	printMissionLaunchIdentity(&out, got)
	if !strings.Contains(out.String(), "gpt-5.6-sol") || !strings.Contains(out.String(), "Inconnu avant l’exécution") || !strings.Contains(out.String(), ".claude/skills/apex/SKILL.md") {
		t.Fatal(out.String())
	}
	t.Setenv("SWARM_LANG", "en")
	out.Reset()
	printMissionLaunchIdentity(&out, got)
	if !strings.Contains(out.String(), "Requested level") || !strings.Contains(out.String(), "Unknown before execution") || !strings.Contains(out.String(), "Project profile") {
		t.Fatal(out.String())
	}
	t.Setenv("SWARM_LANG", "fr")
	legacy, err := s.missionLaunchIdentity(LaunchProfile{Provider: "legacy"}, providers)
	if err != nil || legacy.ResolvedModel != "" || legacy.ActualModel != "" || legacy.Role != "worker" || legacy.RequestedLevel != "auto" {
		t.Fatalf("legacy guessed: %+v %v", legacy, err)
	}
	out.Reset()
	printMissionLaunchIdentity(&out, legacy)
	if !strings.Contains(out.String(), "Modèle résolu par la configuration : Inconnu") || !strings.Contains(out.String(), "Aucun skill sélectionné") {
		t.Fatal(out.String())
	}
}

func TestMissionLaunchPreviewIdentityUsesTaskOverrides(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
	providers, _ := s.providers()
	providers.Providers["override"] = providers.Providers["fixture"]
	raw, _ := json.Marshal(providers)
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	taskProfile := profile
	taskProfile.Provider = "override"
	taskProfile.Instruction = "task specific instruction"
	if err := s.setProfile(w.ID, "t1", taskProfile, w.Revision); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, profile, 1)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Identity.Objective != w.Objective || preview.Departures[0].Identity.Objective != w.Objective {
		t.Fatal("objective lost from launch summary")
	}
	if len(preview.Departures) != 1 || preview.Departures[0].Identity == nil || preview.Departures[0].Identity.Provider != "override" || preview.Identity.Provider != "fixture" || preview.Departures[0].Identity.ActualModel != "" {
		t.Fatalf("missing departure identity %+v", preview)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision {
		t.Fatal("read-only preview mutated revision")
	}
}
