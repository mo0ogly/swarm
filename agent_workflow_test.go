//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func assertWorkflowDelivery(t *testing.T, prompt, role string, recorded *AgentWorkflow) {
	t.Helper()
	want, prefix, err := agentWorkflow(role)
	if err != nil || recorded == nil || recorded.SHA256 != want.SHA256 || recorded.Role != role {
		t.Fatalf("missing or incorrect workflow receipt: %+v (%v)", recorded, err)
	}
	if !strings.HasPrefix(prompt, prefix) {
		t.Fatal("actual provider input is missing the frozen role-specific methods")
	}
	if !containsString(recorded.Methods, "audit-pdca") {
		t.Fatal("PDCA absent from role framing")
	}
}

func TestAgentWorkflowRolesAreBounded(t *testing.T) {
	for _, role := range []string{"planner", "subplanner", "worker", "reviewer"} {
		t.Run(role, func(t *testing.T) {
			w, prompt, err := agentWorkflow(role)
			if err != nil {
				t.Fatal(err)
			}
			assertWorkflowDelivery(t, prompt, role, &w)
			if (role == "worker") != strings.Contains(prompt, "SOURCE tools/agent-workflows/templates/HANDOFF.md") {
				t.Fatal("file-writing template assigned to the wrong role")
			}
			if role == "reviewer" && (!containsString(w.Methods, "code-reviewer") || containsString(w.Methods, "apex")) {
				t.Fatal("reviewer received an implementation workflow")
			}
			t.Logf("%s workflow: %d bytes", role, len(prompt))
		})
	}
	if _, _, err := agentWorkflow("admin"); err == nil {
		t.Fatal("unknown role accepted")
	}
}

func TestAgentWorkflowReachesWorkerAndSurvivesReopen(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	// A writable project file is not the engine's authoritative method pack.
	local := filepath.Join(s.root, ".claude/skills/apex/SKILL.md")
	if err := os.MkdirAll(filepath.Dir(local), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(local, []byte("UNTRUSTED_METHOD_OVERRIDE"), 0600); err != nil {
		t.Fatal(err)
	}
	script := filepath.Join(t.TempDir(), "codex")
	if err := os.WriteFile(script, []byte("#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' '{\"type\":\"result\",\"result\":\"fixture completed\"}'\n"), 0700); err != nil {
		t.Fatal(err)
	}
	providers := Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: script}}}
	raw, _ := json.Marshal(providers)
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	assertWorkflowDelivery(t, a.Prompt, "worker", a.Workflow)
	if strings.Contains(a.Prompt, "UNTRUSTED_METHOD_OVERRIDE") || a.Context.SHA256 != hash([]byte(a.Prompt)) {
		t.Fatal("local override replaced methods or methods escaped the context fingerprint")
	}
	if err = s.supervise(a.ID); err != nil {
		t.Fatal(err)
	}
	observed, err := os.ReadFile(script + ".prompt")
	if err != nil {
		t.Fatal(err)
	}
	assertWorkflowDelivery(t, string(observed), "worker", a.Workflow)
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	stored, err := reopened.agent(a.ID)
	if err != nil || stored.Status != "completed" {
		t.Fatalf("worker did not finish: %+v %v", stored, err)
	}
	assertWorkflowDelivery(t, stored.Prompt, "worker", stored.Workflow)
	current, err := reopened.get(w.ID)
	if err != nil || current.Tasks[0].Status == "accepted" {
		t.Fatal("method framing implicitly accepted a task", err)
	}
}
