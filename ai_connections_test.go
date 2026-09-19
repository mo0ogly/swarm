//go:build linux

package main

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestAIConnectionsLifecycle(t *testing.T) {
	s := storeTest(t)
	if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), []byte(`{"schema_version":1,"providers":{}}`), 0600); e != nil {
		t.Fatal(e)
	}
	_, d, e := s.readAIConnections()
	if e != nil {
		t.Fatal(e)
	}
	c := AIConnection{ID: "local", Label: "Mon modèle", BaseURL: "http://localhost:11434/v1", Model: "custom/model", Key: "private-secret"}
	change := AIConnectionChange{Version: 1, Expected: d, Connection: c, ReplaceKey: true}
	if e = s.saveAIConnection(change); e != nil {
		t.Fatal(e)
	}
	public, e := s.aiConnectionsPublic()
	if e != nil {
		t.Fatal(e)
	}
	raw, _ := json.Marshal(public)
	if strings.Contains(string(raw), c.Key) {
		t.Fatal("secret exposed")
	}
	st, _ := os.Stat(filepath.Join(s.root, ".swarm/ai-connections.json"))
	if st.Mode().Perm() != 0600 {
		t.Fatal("secret file permissions")
	}
	ps, e := s.providers()
	if e != nil {
		t.Fatal(e)
	}
	p := ps.Providers["api-local"]
	if p.APIConnectionID != "local" || !preparationCapability("api-local", p).Available {
		t.Fatal("not usable in preparation")
	}
	if _, e = assistantProvider(p); e != nil {
		t.Fatal(e)
	}
	if e = s.saveAIConnection(change); e == nil {
		t.Fatal("stale update accepted")
	}
	_, d, _ = s.readAIConnections()
	change.Expected = d
	change.ReplaceKey = false
	change.Connection.Key = ""
	change.Connection.BaseURL = "http://elsewhere.test/v1"
	if e = s.saveAIConnection(change); e == nil {
		t.Fatal("secret reused on changed destination")
	}
	change.Connection.BaseURL = c.BaseURL
	change.Connection.Disabled = true
	if e = s.saveAIConnection(change); e != nil {
		t.Fatal(e)
	}
	ps, e = s.providers()
	if e != nil {
		t.Fatal(e)
	}
	if _, ok := ps.Providers["api-local"]; ok {
		t.Fatal("disabled connection available")
	}
}
func TestAIConnectionWire(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/chat/completions" || r.Header.Get("Authorization") != "Bearer secret" {
			t.Error("incorrect request")
		}
		var body map[string]any
		_ = json.NewDecoder(r.Body).Decode(&body)
		if body["model"] != "custom/model" || body["stream"] != false || body["tools"] != nil {
			t.Error("incorrect payload")
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"choices":[{"message":{"content":"{\"message\":\"OK\"}"}}]}`))
	}))
	defer server.Close()
	c := AIConnection{ID: "test", Label: "Test", BaseURL: server.URL + "/v1", Model: "custom/model", Key: "secret"}
	text, e := callAIConnection(context.Background(), c, "Test", "{}")
	if e != nil || text != `{"message":"OK"}` {
		t.Fatal(text, e)
	}
	denied := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { http.Error(w, "secret-provider-error", 401) }))
	defer denied.Close()
	c.BaseURL = denied.URL
	_, e = callAIConnection(context.Background(), c, "Test", "")
	if e == nil || strings.Contains(e.Error(), "secret-provider-error") || !strings.Contains(e.Error(), "401") {
		t.Fatal("unsafe error", e)
	}
	redirect := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, server.URL+"/v1/chat/completions", 307)
	}))
	defer redirect.Close()
	c.BaseURL = redirect.URL
	if _, e = callAIConnection(context.Background(), c, "Test", ""); e == nil {
		t.Fatal("redirect followed")
	}
}

func TestPreparedAPIPlannerRequiresSeparateWorker(t *testing.T) {
	s, _, _ := prepDialogueFixture(t, prepReplyScript)
	_, digest, _ := s.readAIConnections()
	c := AIConnection{ID: "manager", Label: "Manager API", Model: "local-model", BaseURL: "http://localhost:11434/v1"}
	if e := s.saveAIConnection(AIConnectionChange{Version: 1, Expected: digest, Connection: c}); e != nil {
		t.Fatal(e)
	}
	p := readyPreparation(t, s, "")
	r := conversionRequest(t, s, p, "create-missions")
	r.Organization = &PreparationOrganization{Provider: "api-manager", Level: "standard", Workspace: s.root, Validation: "human", MaxTasks: 20, MaxCalls: 40}
	if _, e := s.preparationCommand(r); e == nil {
		t.Fatal("API accepted as file worker")
	}
	r.Organization.WorkerProvider = "test"
	p, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	w, e := s.get(p.WorkID)
	if e != nil {
		t.Fatal(e)
	}
	if w.Planning.Provider != "api-manager" || w.Planning.Reviewer.Provider != "api-manager" || w.Profile.Provider != "test" {
		t.Fatal("roles incorrectly assigned")
	}
}
