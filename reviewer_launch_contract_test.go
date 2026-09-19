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

func TestReviewerLaunchContractRejectsLegacyOmission(t *testing.T) {
	for _, managed := range []bool{false, true} {
		s := storeTest(t)
		w, launch := setupAgent(t, s)
		organizedFixtureStore(t, s)
		w, _ = s.get(w.ID)
		w.Planning.Reviewer = nil
		w.Planning.ReviewerRequired = false // historical bug must not be an exemption
		if managed {
			w.Planning.Repository = &ManagedRepository{Source: s.root}
		}
		raw, _ := json.Marshal(w)
		if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
			t.Fatal(e)
		}
		if organization(w).Ready {
			t.Fatal("omitted reviewer reported ready")
		}
		launch.Revision = w.Revision
		if _, _, e := s.prepare(w.ID, launch); e == nil || !strings.Contains(e.Error(), "Vérificateur") {
			t.Fatalf("manual launch bypassed reviewer: %v", e)
		}
		profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
		if e := s.configureMission(w.ID, profile, 1, w.Revision); e == nil {
			t.Fatal("mission start bypassed reviewer")
		}
		preview, e := s.missionLaunchPreview(w.ID, profile, 1)
		if e != nil || preview.Organization.Ready || preview.Immediate != 0 || preview.Token != "" {
			t.Fatalf("preview promised an unreviewed launch: %+v %v", preview, e)
		}
		if _, e := s.webAction(webRequest{Kind: "mission-start", Work: w.ID, Event: "no-reviewer-web", Revision: w.Revision, Provider: "fixture", Workspace: s.root, Slots: 1, PreviewToken: preview.Token}); e == nil {
			t.Fatal("web action bypassed reviewer")
		}
		input := filepath.Join(t.TempDir(), "profile.json")
		raw, _ = json.Marshal(profile)
		if e := os.WriteFile(input, raw, 0600); e != nil {
			t.Fatal(e)
		}
		var out bytes.Buffer
		if e := missionCLI(s, []string{"mission", "start", w.ID}, input, true, &out); e == nil {
			t.Fatal("CLI start bypassed reviewer")
		}
		var count int
		if e := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&count); e != nil || count != 0 {
			t.Fatalf("refusal reserved an agent: %d %v", count, e)
		}
		after, _ := s.get(w.ID)
		if after.Revision != w.Revision {
			t.Fatal("refusal changed work")
		}
		policy, _ := s.missionPolicy(w.ID)
		if policy.Enabled {
			t.Fatal("refusal authorized mission")
		}
	}
}

func TestReviewerLaunchContractManagedEnableKeepsReviewer(t *testing.T) {
	s := storeTest(t)
	source := filepath.Join(s.root, "source")
	if e := os.MkdirAll(source, 0700); e != nil {
		t.Fatal(e)
	}
	gitTest(t, source, "init")
	if e := os.WriteFile(filepath.Join(source, "base.txt"), []byte("base\n"), 0600); e != nil {
		t.Fatal(e)
	}
	gitTest(t, source, "add", ".")
	gitTest(t, source, "commit", "-m", "base")
	script := filepath.Join(t.TempDir(), "claude")
	if e := os.WriteFile(script, []byte("#!/bin/sh\nexit 1\n"), 0700); e != nil {
		t.Fatal(e)
	}
	raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"review-fixture": {Command: script}}})
	if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); e != nil {
		t.Fatal(e)
	}
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "review-fixture", MaxTasks: 10, MaxDecisions: 20, MaxActivations: 20, Repository: &ManagedRepositoryRequest{Path: source, CommittedOnly: true}, Checks: map[string][]ValidationControl{"req-1": automaticPolicy("git", "diff", "--exit-code").Controls}})
	if w.Planning.Repository == nil || w.Planning.Reviewer == nil || !w.Planning.ReviewerRequired || w.Planning.Reviewer.Provider != "review-fixture" {
		t.Fatal("managed enable silently omitted review", w.Planning)
	}
	if w.Planning.Reviewer.Calls != 0 {
		t.Fatal("configuration started an inference")
	}
}

func TestReviewerLaunchContractUnavailableReviewer(t *testing.T) {
	for _, kind := range []string{"budget", "failure", "provider-removed", "provider-changed"} {
		t.Run(kind, func(t *testing.T) {
			s := storeTest(t)
			w, launch := setupAgent(t, s)
			organizedFixtureStore(t, s)
			w, _ = s.get(w.ID)
			switch kind {
			case "budget":
				w.Planning.Reviewer.Calls = w.Planning.Reviewer.MaxCalls
			case "failure":
				w.Planning.Reviewer.Failure = "quota indisponible"
			default:
				ps, e := s.providers()
				if e != nil {
					t.Fatal(e)
				}
				if kind == "provider-removed" {
					delete(ps.Providers, w.Planning.Reviewer.Provider)
				} else {
					p := ps.Providers[w.Planning.Reviewer.Provider]
					p.Args = []string{"--changed"}
					ps.Providers[w.Planning.Reviewer.Provider] = p
				}
				raw, _ := json.Marshal(ps)
				if e = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); e != nil {
					t.Fatal(e)
				}
			}
			raw, _ := json.Marshal(w)
			if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
				t.Fatal(e)
			}
			launch.Revision = w.Revision
			if _, _, e := s.prepare(w.ID, launch); e == nil {
				t.Fatal("unavailable reviewer allowed launch")
			}
			if e := s.configureMission(w.ID, LaunchProfile{Provider: "fixture", Role: "worker", Workspace: s.root}, 1, w.Revision); e == nil {
				t.Fatal("unavailable reviewer allowed mission")
			}
			var count int
			if e := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&count); e != nil || count != 0 {
				t.Fatal("agent created on refusal", count, e)
			}
		})
	}
}

func TestReviewerLaunchContractSpentBudgetDoesNotInvalidateOrganization(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	organizedFixtureStore(t, s)
	w, _ = s.get(w.ID)
	w.Planning.Reviewer.Calls = w.Planning.Reviewer.MaxCalls
	if e := organizationGuard(w); e != nil {
		t.Fatalf("spent review budget prevents conductor/planner completion: %v", e)
	}
	if e := reviewerLaunchGuard(w); e == nil {
		t.Fatal("spent review budget allows new producer")
	}
}
