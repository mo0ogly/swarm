package main

import (
	"strings"
	"testing"
)

func TestOrganizationRejectsHistoricalAutonomy(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	before := w.Revision
	if err := s.setAutonomy(w.ID, autonomyAuto, 1); err == nil || !strings.Contains(err.Error(), "Organisation autonome") {
		t.Fatalf("autonomy accepted: %v", err)
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	if err := automaticLaunchGuard(tx, w.ID); err == nil {
		t.Fatal("transaction accepted missing organization")
	}
	tx.Rollback()
	after, _ := s.get(w.ID)
	if after.Revision != before {
		t.Fatal("refusal mutated mission")
	}
	status, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if status.Organization.Ready || status.Enabled || !strings.Contains(status.Understanding.What, "Organisation autonome") {
		t.Fatalf("dishonest status: %+v", status)
	}
}
func TestOrganizationChecksRolesAndValidation(t *testing.T) {
	w := Work{Criteria: []string{"test"}, Planning: &PlanningState{Version: 1, MaxTasks: 10, MaxDecisions: 10, MaxActivations: 10, Provider: "fixture", Scopes: []PlanningScope{{ID: "root", Revision: 1, Requirements: []string{"req-1"}}}, Checks: map[string][]ValidationControl{"req-1": automaticPolicy("git", "diff", "--exit-code").Controls}}}
	w.Planning.Reviewer = &ReviewerConfig{Provider: "fixture-reviewer", Authorized: now(), MaxCalls: 10}
	if !organization(w).Ready {
		t.Fatal(organization(w))
	}
	w.Tasks = []Task{{ID: "task", ScopeID: "root", Requirements: []string{"req-1"}, Criteria: []string{"test"}}}
	if organization(w).Ready {
		t.Fatal("missing validation accepted")
	}
	policy := automaticPolicy("git", "diff", "--exit-code")
	policy.Authorized = now()
	w.Tasks[0].ValidationPolicy = policy
	if !organization(w).Ready {
		t.Fatal(organization(w))
	}
	w.Tasks[0].ScopeID = "missing"
	if organization(w).Ready {
		t.Fatal("missing owner accepted")
	}
	w.Tasks = nil
	delete(w.Planning.Checks, "req-1")
	if organization(w).Ready {
		t.Fatal("missing requirement check accepted")
	}
}

func TestOrganizationRefusesMissionStartAndPreviewDoesNotPromiseLaunch(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
	preview, err := s.missionLaunchPreview(w.ID, profile, 1)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Organization.Ready || preview.Immediate != 0 || preview.Token != "" {
		t.Fatalf("invalid preview: %+v", preview)
	}
	if err = s.configureMission(w.ID, profile, 1, w.Revision); err == nil || !strings.Contains(err.Error(), "Organisation autonome") {
		t.Fatalf("start allowed: %v", err)
	}
	policy, _ := s.missionPolicy(w.ID)
	if policy.Enabled {
		t.Fatal("authorization persisted on refusal")
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision {
		t.Fatal("refusal mutated work")
	}
}
func TestOrganizationRevocationBlocksReservation(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	organizedFixtureStore(t, s)
	if err := s.setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	// Loss of the contract after authorization must be checked at reservation.
	if _, err := s.db.Exec("UPDATE works SET body=json_remove(body,'$.planning') WHERE id=?", w.ID); err != nil {
		t.Fatal(err)
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	if err = automaticLaunchGuard(tx, w.ID); err == nil {
		t.Fatal("old authorization bypassed missing organization")
	}
}
