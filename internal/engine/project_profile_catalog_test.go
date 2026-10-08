//go:build linux

package engine

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

func TestProjectProfileExplicitSelectionAndSnapshots(t *testing.T) {
	s := storeTest(t)
	profileFixture(t, s)
	os.MkdirAll(filepath.Join(s.root, ".claude"), 0700)
	os.WriteFile(filepath.Join(s.root, ".claude/CLAUDE.md"), []byte("CLAUDE_SELECTED_RULE"), 0600)
	os.WriteFile(filepath.Join(s.root, ".claude/settings.local.json"), []byte(`{"secret":"PRIVATE_SETTINGS_MARKER"}`), 0600)
	c, err := s.projectProfileCatalog()
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(c)
	if strings.Contains(string(raw), "PRIVATE_SETTINGS_MARKER") || strings.Contains(string(raw), "CLAUDE_SELECTED_RULE") {
		t.Fatal("catalog disclosed source contents")
	}
	var selected ProjectProfileChoice
	for _, choice := range c.Choices {
		if choice.ID == "claude-project" {
			selected = choice
		}
		if choice.ID == "gemini-root" && choice.Available {
			t.Fatal("missing Gemini document advertised")
		}
	}
	if !selected.Available {
		t.Fatal(selected.Reason)
	}
	previous, _, _ := s.projectContext("worker")
	v, err := s.selectProjectProfile("claude-project", c.ActiveSHA256)
	if err != nil || !v.Enabled {
		t.Fatal(v, err)
	}
	for _, role := range projectRoles {
		ctx, prompt, e := s.projectContext(role)
		if e != nil || ctx.Source != "claude-project" {
			t.Fatal(ctx, e)
		}
		if strings.Contains(prompt, "CLAUDE_SELECTED_RULE") != (role == "worker") {
			t.Fatal("incorrect role source allocation", role)
		}
	}
	if e := s.projectContextGuard(previous, "worker"); e == nil {
		t.Fatal("old snapshot incorrectly reused")
	}
	if _, e := s.selectProjectProfile("agents-root", c.ActiveSHA256); e == nil {
		t.Fatal("stale selector overwrote profile")
	}
	if _, e := s.selectProjectProfile("../../settings.json", ""); e == nil {
		t.Fatal("arbitrary profile selected")
	}
	var out bytes.Buffer
	if e := s.projectProfileCLI([]string{"list"}, "", true, &out); e != nil {
		t.Fatal(e)
	}
	var catalog ProjectProfileCatalog
	if e := json.Unmarshal(out.Bytes(), &catalog); e != nil || catalog.ActiveSource != "claude-project" {
		t.Fatal(catalog, e)
	}
}
func TestProjectProfileSelectionHTTPAuthorization(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "127.0.0.1:9876", "profile-session")
	os.WriteFile(filepath.Join(s.root, "AGENTS.md"), []byte("PROJECT_RULE"), 0600)
	request := func(body string, auth bool) *httptest.ResponseRecorder {
		r := httptest.NewRequest("POST", "http://127.0.0.1:9876/api/v1/project-profile/select", strings.NewReader(body))
		if auth {
			r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "profile-session"})
			r.Header.Set("Origin", "http://127.0.0.1:9876")
			r.Header.Set("X-Swarm-CSRF", "profile-session")
		}
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		return w
	}
	if w := request(`{"id":"agents-root","expected_sha256":""}`, false); w.Code == 200 {
		t.Fatal("unauthenticated selection")
	}
	if _, err := os.Stat(filepath.Join(s.root, projectProfileFile)); !os.IsNotExist(err) {
		t.Fatal("unauthorized write")
	}
	if w := request(`{"id":"agents-root","expected_sha256":""}`, true); w.Code != 200 {
		t.Fatal(w.Code, w.Body.String())
	}
	raw, _ := os.ReadFile(filepath.Join(s.root, projectProfileFile))
	for _, body := range []string{`{"id":"agents-root","expected_sha256":""}`, `{"id":"unknown","expected_sha256":""}`, `{"id":"agents-root","expected_sha256":"","extra":true}`} {
		if w := request(body, true); w.Code == 200 {
			t.Fatal("invalid selection succeeded", body)
		}
		after, _ := os.ReadFile(filepath.Join(s.root, projectProfileFile))
		if !bytes.Equal(raw, after) {
			t.Fatal("invalid request mutated profile")
		}
	}
}
