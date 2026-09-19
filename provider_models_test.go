//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func modelCatalogTest(t *testing.T) {
	t.Helper()
	root := t.TempDir()
	t.Setenv("CODEX_HOME", root)
	models := []map[string]any{}
	for _, id := range []string{"gpt-5.6-luna", "gpt-5.6-sol", "gpt-6-astra"} {
		models = append(models, map[string]any{"slug": id, "visibility": "list", "supported_reasoning_levels": []map[string]string{{"effort": "low"}, {"effort": "medium"}, {"effort": "high"}}})
	}
	b, _ := json.Marshal(map[string]any{"models": models})
	if err := os.WriteFile(filepath.Join(root, "models_cache.json"), b, 0600); err != nil {
		t.Fatal(err)
	}
}
func TestModelDefaultsNeverEscalate(t *testing.T) {
	modelCatalogTest(t)
	p := Provider{Command: "/fixture/codex", Args: []string{"exec", "--json", "--model", "gpt-6-astra", "-c", "model=\"gpt-6-astra\"", "--config=model_reasoning_effort=\"high\"", "-cmodel=\"gpt-6-astra\"", "-c", "sandbox_mode=\"read-only\"", "-"}}
	for _, tc := range []struct{ purpose, level, model, effort string }{{"page", "auto", "gpt-5.6-luna", "low"}, {"work", "auto", "gpt-5.6-sol", "medium"}, {"brainstorm", "auto", "gpt-5.6-sol", "medium"}, {"work", "exigeant", "gpt-6-astra", "high"}} {
		got, r, err := resolveModel(p, tc.level, tc.purpose)
		if err != nil {
			t.Fatal(err)
		}
		if r.Model != tc.model || r.Effort != tc.effort {
			t.Fatalf("unexpected route %+v", r)
		}
		args := strings.Join(got.Args, " ")
		if strings.Count(args, "--model") != 1 || strings.Count(args, "model_reasoning_effort") != 1 || strings.Contains(args, "model=\"gpt-6-astra\"") || !strings.Contains(args, "sandbox_mode=\"read-only\"") || got.Args[len(got.Args)-1] != "-" {
			t.Fatalf("ambiguous or altered CLI %s", args)
		}
	}
	p.ModelPolicy = effectiveModelPolicy(p)
	p.ModelPolicy.Levels["simple"] = ModelChoice{Model: "gpt-6-astra", Effort: "high"}
	if _, _, err := resolveModel(p, "simple", "page"); err == nil {
		t.Fatal("frontier at simple level")
	}
	p.ModelPolicy.Levels["simple"] = ModelChoice{Model: "missing"}
	if _, _, err := resolveModel(p, "auto", "page"); err == nil {
		t.Fatal("unknown model silently fell back")
	}
	p.ModelPolicy.Version = 9
	if _, _, err := resolveModel(p, "standard", "work"); err == nil {
		t.Fatal("malformed policy accepted")
	}
}
func TestOnPremiseRouteAndCustomAdapter(t *testing.T) {
	p := Provider{Command: "/fixture/skynet_harness", Args: []string{"--model", "glm-5.3-flash"}}
	for _, level := range modelLevels {
		_, r, err := resolveModel(p, level, "work")
		if err != nil || r.Model != "glm-5.3-flash" || r.Billing != "on_premise" {
			t.Fatalf("%+v %v", r, err)
		}
	}
	p = Provider{Command: "/fixture/unknown"}
	if _, r, e := resolveModel(p, "auto", "work"); e != nil || r != nil {
		t.Fatal("legacy adapter broken")
	}
	if _, _, e := resolveModel(p, "simple", "work"); e == nil {
		t.Fatal("unknown adapter guessed")
	}
}
func TestPolicyPreviewAtomicSaveConflictAndExport(t *testing.T) {
	s := storeTest(t)
	setupAgent(t, s)
	path := filepath.Join(s.root, ".swarm/providers.json")
	ps, _ := s.providers()
	ps.Providers["claude"] = Provider{Command: "/fixture/claude", Args: []string{"-p"}, Env: []string{"KEY_NAME_ONLY"}}
	raw, _ := json.Marshal(ps)
	os.WriteFile(path, raw, 0600)
	state, e := s.providerAdminState()
	if e != nil {
		t.Fatal(e)
	}
	exported, _ := json.Marshal(state["export"])
	if strings.Contains(string(exported), "KEY_NAME_ONLY") || strings.Contains(string(exported), "command") || strings.Contains(string(exported), "fixture") {
		t.Fatal("connection data exported", string(exported))
	}
	policy := effectiveModelPolicy(ps.Providers["claude"])
	policy.WorkLevel = "simple"
	change := PolicyChange{Version: 1, Expected: state["digest"].(string), Policies: map[string]*ModelPolicy{"claude": policy}, Preview: true}
	if _, e = s.changePolicies(change); e != nil {
		t.Fatal(e)
	}
	after, _ := os.ReadFile(path)
	if string(after) != string(raw) {
		t.Fatal("preview mutated config")
	}
	change.Preview = false
	if _, e = s.changePolicies(change); e != nil {
		t.Fatal(e)
	}
	got, _ := s.providers()
	if got.Providers["claude"].ModelPolicy.WorkLevel != "simple" || got.Providers["claude"].Command != "/fixture/claude" || got.Providers["claude"].Env[0] != "KEY_NAME_ONLY" {
		t.Fatal("save altered connection")
	}
	if _, e = s.changePolicies(change); e == nil {
		t.Fatal("stale admin overwrote config")
	}
	backup, e := os.ReadFile(filepath.Join(s.root, ".swarm/providers-before-"+hash(raw)+".json"))
	if e != nil || string(backup) != string(raw) {
		t.Fatal("prior configuration not recoverable", e)
	}
	var bad PolicyChange
	if strict([]byte(`{"version":1,"policies":{},"command":"evil"}`), &bad) == nil {
		t.Fatal("command import accepted")
	}
}
func TestTerminalModelSelection(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	ps, _ := s.providers()
	ps.Providers = map[string]Provider{"claude": {Command: "/fixture/claude"}}
	raw, _ := json.Marshal(ps)
	os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600)
	c := &consoleState{}
	s.openTaskDialog(w.ID, c)
	c.dialog.row = 0
	s.dialogKey(w.ID, c, "enter")
	s.refreshDialogModel(c.dialog)
	if c.dialog.modelDescription != "sonnet" {
		t.Fatal(c.dialog.modelDescription)
	}
	c.dialog.row = 4
	s.dialogKey(w.ID, c, "right")
	s.refreshDialogModel(c.dialog)
	if c.dialog.modelDescription != "haiku" || c.dialog.modelPolicyHash == "" {
		t.Fatal("choice invisible", c.dialog)
	}
	frame := s.renderDashboard(w.ID, c, 140, 40, "")
	if !strings.Contains(frame, "haiku") {
		t.Fatal("render hides model")
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 0 {
		t.Fatal("selecting model launched agent")
	}
}

func TestLaunchBindsPolicyAndPersistsSelectedModel(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	dir := t.TempDir()
	command := filepath.Join(dir, "claude")
	os.WriteFile(command, []byte("#!/bin/sh\nexit 0\n"), 0700)
	ps := Providers{Schema: 1, Providers: map[string]Provider{"claude": {Command: command}}}
	path := filepath.Join(s.root, ".swarm/providers.json")
	raw, _ := json.Marshal(ps)
	os.WriteFile(path, raw, 0600)
	r.Provider = "claude"
	r.Level = "simple"
	_, route, e := resolveModel(ps.Providers["claude"], r.Level, "work")
	if e != nil {
		t.Fatal(e)
	}
	r.ModelPolicyHash = route.PolicyHash
	p := ps.Providers["claude"]
	p.ModelPolicy = effectiveModelPolicy(p)
	p.ModelPolicy.Levels["simple"] = ModelChoice{Model: "sonnet"}
	ps.Providers["claude"] = p
	raw, _ = json.Marshal(ps)
	os.WriteFile(path, raw, 0600)
	if _, _, e = s.prepare(w.ID, r); e == nil {
		t.Fatal("changed preview policy accepted")
	}
	agents, _ := s.agents(w.ID)
	if len(agents) > 0 {
		t.Fatal("stale choice queued process")
	}
	_, route, e = resolveModel(p, r.Level, "work")
	if e != nil {
		t.Fatal(e)
	}
	r.ModelPolicyHash = route.PolicyHash
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	stored, _ := s.agent(a.ID)
	if stored.ModelRoute == nil || stored.ModelRoute.Model != "sonnet" || !strings.Contains(strings.Join(stored.Args, " "), "--model sonnet") {
		t.Fatal("route not persisted", stored)
	}
}

func TestProviderAdminAuthenticationAndStrictImport(t *testing.T) {
	s := storeTest(t)
	setupAgent(t, s)
	h := newWebHandler(s, "local.test", "test-token")
	for _, tc := range []struct {
		auth, csrf bool
		body       string
		want       int
	}{{false, false, `{}`, 403}, {true, false, `{}`, 403}, {true, true, `{"version":1,"policies":{},"command":"evil"}`, 400}} {
		req := httptest.NewRequest("POST", "http://local.test/api/v1/providers/policies", bytes.NewBufferString(tc.body))
		req.Host = "local.test"
		req.Header.Set("Origin", "http://local.test")
		if tc.auth {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "test-token"})
		}
		if tc.csrf {
			req.Header.Set("X-Swarm-CSRF", "test-token")
		}
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, req)
		if rr.Code != tc.want {
			t.Fatalf("got %d: %s", rr.Code, rr.Body.String())
		}
	}
}
