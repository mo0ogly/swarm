//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func profileFixture(t *testing.T, s *Store) ProjectProfile {
	t.Helper()
	os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte("PROJECT_RULE: both themes and real evidence required.\n"), 0600)
	p := ProjectProfile{Version: 1, Name: "Wattson test", Instructions: []ProjectInstruction{{"AGENTS.md", append([]string{}, projectRoles...)}}}
	if err := s.applyProjectProfile(p, ""); err != nil {
		t.Fatal(err)
	}
	return p
}
func TestProjectProfileRoleIsolationAndFreshness(t *testing.T) {
	s := storeTest(t)
	p := profileFixture(t, s)
	os.WriteFile(filepath.Join(s.root, "worker.md"), []byte("WORKER_ONLY_MARKER"), 0600)
	p.Instructions = append(p.Instructions, ProjectInstruction{"worker.md", []string{"worker"}})
	old, _, _ := s.projectContext("worker")
	if err := s.applyProjectProfile(p, old.ProfileSHA256); err != nil {
		t.Fatal(err)
	}
	for _, role := range []string{"planner", "subplanner", "worker", "reviewer"} {
		wf, prompt, err := s.projectAgentWorkflow(role)
		if err != nil {
			t.Fatal(err)
		}
		if wf.Project == nil || !strings.Contains(prompt, "PROJECT_RULE") || strings.Contains(prompt, "WORKER_ONLY_MARKER") != (role == "worker") {
			t.Fatalf("incorrect role context %s", role)
		}
		if err = s.projectContextGuard(wf.Project, role); err != nil {
			t.Fatal(err)
		}
		if role == "reviewer" && !strings.Contains(prompt, "sans outils ni modification") {
			t.Fatal("reviewer permissions expanded")
		}
	}
	current, _, _ := s.projectContext("reviewer")
	os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte("CHANGED_RULE"), 0600)
	if err := s.projectContextGuard(current, "reviewer"); err == nil {
		t.Fatal("changed rules accepted")
	}
	if err := s.applyProjectProfile(p, "wrong"); err == nil {
		t.Fatal("stale profile overwrite accepted")
	}
}
func TestProjectProfileRefusesUnsafeOrOversizedSources(t *testing.T) {
	for _, kind := range []string{"missing", "outside", "settings", "binary", "secret", "oversized", "aggregate", "unknown-role"} {
		t.Run(kind, func(t *testing.T) {
			s := storeTest(t)
			p := profileFixture(t, s)
			raw, _ := os.ReadFile(filepath.Join(s.root, projectProfileFile))
			switch kind {
			case "missing":
				p.Instructions[0].Path = "missing.md"
			case "outside":
				outside := filepath.Join(t.TempDir(), "outside.md")
				os.WriteFile(outside, []byte("outside"), 0600)
				os.Symlink(outside, filepath.Join(s.root, "outside.md"))
				p.Instructions[0].Path = "outside.md"
			case "settings":
				p.Instructions[0].Path = ".claude/settings.json"
			case "binary":
				os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte{0, 255}, 0600)
			case "secret":
				os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte("sk-ant-"+strings.Repeat("a", 35)), 0600)
			case "oversized":
				os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte(strings.Repeat("x", 16001)), 0600)
			case "aggregate":
				os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte(strings.Repeat("x", 9000)), 0600)
				os.WriteFile(filepath.Join(s.root, "extra.md"), []byte(strings.Repeat("y", 9000)), 0600)
				p.Instructions = append(p.Instructions, ProjectInstruction{"extra.md", projectRoles})
			case "unknown-role":
				p.Instructions[0].Roles = []string{"admin"}
			}
			if err := s.applyProjectProfile(p, hash(raw)); err == nil {
				t.Fatal("unsafe profile accepted")
			}
			after, _ := os.ReadFile(filepath.Join(s.root, projectProfileFile))
			if string(after) != string(raw) {
				t.Fatal("failed configuration overwrote prior profile")
			}
		})
	}
}
func TestProjectProfilePreparationReceivesRulesNotSettings(t *testing.T) {
	s := storeTest(t)
	prepMethods(t, s)
	p := prepCreate(t, s)
	profileFixture(t, s)
	os.MkdirAll(filepath.Join(s.root, ".claude"), 0700)
	os.WriteFile(filepath.Join(s.root, ".claude/settings.json"), []byte(`{"env":{"API_KEY":"PRIVATE_MARKER_NOT_FOR_PROMPT"}}`), 0600)
	m, _ := s.preparationMethod(p.Method)
	prompt, err := s.preparationPromptForMode(p, m, nil, "Prepare the need", "brief", "")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(prompt, "PROJECT_RULE") || !strings.Contains(prompt, "Ne lance aucun outil") || strings.Contains(prompt, "PRIVATE_MARKER") {
		t.Fatal("preparation context leak or missing rules")
	}
	status, err := s.projectProfileStatus()
	if err != nil || len(status.ClaudeSettings) != 1 {
		t.Fatal(status, err)
	}
	raw, _ := json.Marshal(status)
	if strings.Contains(string(raw), "PRIVATE_MARKER") || strings.Contains(string(raw), "PROJECT_RULE") {
		t.Fatal("metadata response exposed source contents")
	}
	provider, err := assistantProvider(Provider{Command: "/bin/true"})
	_ = provider
	if err == nil {
		t.Fatal("unknown assistant adapter accepted")
	}
}
func TestProjectProfileWorkerSnapshotAndNoLaunchAfterChange(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	profileFixture(t, s)
	script := filepath.Join(t.TempDir(), "claude")
	os.WriteFile(script, []byte("#!/bin/sh\ncat >\"$0.prompt\"\n"), 0700)
	raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: script}}})
	os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if a.Workflow.Project == nil || !strings.Contains(a.Prompt, "PROJECT_RULE") {
		t.Fatal("worker did not record project context")
	}
	os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte("Changed before provider launch"), 0600)
	if err = s.supervise(a.ID); err != nil {
		t.Fatal(err)
	}
	if _, err = os.Stat(script + ".prompt"); !os.IsNotExist(err) {
		t.Fatal("provider called after project rule change")
	}
	a, err = s.agent(a.ID)
	if err != nil || a.Status != "failed" {
		t.Fatal(a, err)
	}
}

func TestProjectProfileRulesInvalidateIndependentAcceptance(t *testing.T) {
	s := storeTest(t)
	profileFixture(t, s)
	ctx, _, err := s.projectContext("reviewer")
	if err != nil {
		t.Fatal(err)
	}
	os.WriteFile(filepath.Join(s.root, "report.md"), []byte("observed evidence"), 0600)
	task := Task{ID: "task", Criteria: []string{"Evidence exists"}, Attempts: []Attempt{{ID: "attempt"}}}
	task.IndependentReview = &IndependentReview{State: "passed", Attempt: "attempt", Contract: reviewContract(&task), Report: "report.md", Digest: hash([]byte("observed evidence")), Workflow: &AgentWorkflow{Role: "reviewer", Project: ctx}}
	work := Work{Planning: &PlanningState{Reviewer: &ReviewerConfig{Provider: "fixture"}}}
	if err = s.independentReviewGuard(&work, &task); err != nil {
		t.Fatal(err)
	}
	os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte("New acceptance rules"), 0600)
	if err = s.independentReviewGuard(&work, &task); err == nil {
		t.Fatal("review remained acceptable after project rule change")
	}
}

func TestProjectProfileConcurrentUpdateAndRemoval(t *testing.T) {
	s := storeTest(t)
	p := profileFixture(t, s)
	original, _, err := s.projectContext("preparation")
	if err != nil {
		t.Fatal(err)
	}
	start := make(chan struct{})
	results := make(chan error, 2)
	for _, name := range []string{"first", "second"} {
		candidate := p
		candidate.Name = name
		go func() { <-start; results <- s.applyProjectProfile(candidate, original.ProfileSHA256) }()
	}
	close(start)
	success := 0
	for range 2 {
		if <-results == nil {
			success++
		}
	}
	if success != 1 {
		t.Fatalf("%d concurrent updates accepted; want exactly one", success)
	}
	current, _, err := s.projectContext("preparation")
	if err != nil {
		t.Fatal(err)
	}
	if err = s.projectContextGuard(nil, "preparation"); err == nil {
		t.Fatal("profile added after preparation went undetected")
	}
	if err = os.Remove(filepath.Join(s.root, projectProfileFile)); err != nil {
		t.Fatal(err)
	}
	if err = s.projectContextGuard(current, "preparation"); err == nil {
		t.Fatal("profile removal after preparation went undetected")
	}
}

func TestProjectProfilePreparationProviderReceivesSnapshot(t *testing.T) {
	capture := filepath.Join(t.TempDir(), "provider-prompt.txt")
	script := strings.Replace(prepReplyScript, "cat >/dev/null", "cat >'"+capture+"'", 1)
	s, _, r := prepDialogueFixture(t, script)
	profileFixture(t, s)
	turn, err := s.sendPreparation(r)
	if err != nil {
		t.Fatal(err)
	}
	if turn.Project == nil {
		t.Fatal("turn lost its project snapshot")
	}
	s.runPreparationTurn(turn)
	got, err := s.preparationTurn(turn.ID)
	if err != nil || got.Status != "answered" {
		t.Fatalf("provider did not finish: %+v %v", got, err)
	}
	transmitted, err := os.ReadFile(capture)
	if err != nil || !strings.Contains(string(transmitted), "PROJECT_RULE") || !strings.Contains(string(transmitted), turn.Project.SHA256) {
		t.Fatalf("provider did not receive frozen instructions: %v", err)
	}
	r.Event = "second-profile-turn"
	turn, err = s.sendPreparation(r)
	if err != nil {
		t.Fatal(err)
	}
	if err = os.Remove(capture); err != nil {
		t.Fatal(err)
	}
	if err = os.Remove(filepath.Join(s.root, projectProfileFile)); err != nil {
		t.Fatal(err)
	}
	s.runPreparationTurn(turn)
	if _, err = os.Stat(capture); !os.IsNotExist(err) {
		t.Fatal("provider was called after profile removal")
	}
	got, err = s.preparationTurn(turn.ID)
	if err != nil || got.Status == "answered" {
		t.Fatal("removed profile accepted")
	}
}
