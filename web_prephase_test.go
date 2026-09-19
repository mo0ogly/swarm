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

func TestPreparationWebCLIParityAndSecurity(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "127.0.0.1:9876", "prep-test-session")
	request := func(method, path string, body []byte, cookie, csrf, origin bool) *httptest.ResponseRecorder {
		r := httptest.NewRequest(method, "http://127.0.0.1:9876"+path, bytes.NewReader(body))
		if cookie {
			r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "prep-test-session"})
		}
		if csrf {
			r.Header.Set("X-Swarm-CSRF", "prep-test-session")
		}
		if origin {
			r.Header.Set("Origin", "http://127.0.0.1:9876")
		}
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		return w
	}
	r := prepRequest(Preparation{}, "create")
	r.Title = "Reprise web et CLI"
	r.Text = "Besoin initial"
	raw, _ := json.Marshal(r)
	for _, auth := range [][3]bool{{false, true, true}, {true, false, true}, {true, true, false}} {
		w := request("POST", "/api/v1/preparations/command", raw, auth[0], auth[1], auth[2])
		if w.Code != 403 {
			t.Fatal("mutation without session protection", w.Code)
		}
	}
	w := request("GET", "/api/v1/preparations/command", nil, true, true, true)
	if w.Code != 405 {
		t.Fatal("GET mutated state")
	}
	ws, e := s.preparations()
	if e != nil || len(ws) != 0 {
		t.Fatal("unauthorized side effect", e)
	}
	w = request("POST", "/api/v1/preparations/command", raw, true, true, true)
	var p Preparation
	if e = json.Unmarshal(w.Body.Bytes(), &p); w.Code != 200 || e != nil {
		t.Fatal(w.Code, w.Body.String())
	}
	r = prepRequest(p, "save")
	r.Document = "brief"
	r.Text = "Rédigé depuis le CLI"
	raw, _ = json.Marshal(r)
	input := filepath.Join(t.TempDir(), "request.json")
	if e = os.WriteFile(input, raw, 0600); e != nil {
		t.Fatal(e)
	}
	var out, errout bytes.Buffer
	if code := run([]string{"--root", s.root, "prepare", "save", p.ID, "--input", input, "--json"}, &out, &errout); code != 0 {
		t.Fatal(code, errout.String())
	}
	w = request("GET", "/api/v1/preparations/show?id="+p.ID, nil, true, false, false)
	var current Preparation
	if e = json.Unmarshal(w.Body.Bytes(), &current); e != nil || current.Documents["brief"].Text != r.Text {
		t.Fatal("HTTP did not see CLI save", w.Body.String())
	}
	// Lost reply: replay the exact CLI request over HTTP, before revision checks.
	w = request("POST", "/api/v1/preparations/command", raw, true, true, true)
	if w.Code != 200 {
		t.Fatal("cross-client idempotent receipt failed", w.Body.String())
	}
	r.Event = newID("stale-")
	r.Text = "Écrasement interdit"
	raw, _ = json.Marshal(r)
	w = request("POST", "/api/v1/preparations/command", raw, true, true, true)
	if w.Code != 409 {
		t.Fatal("stale revision should be an actionable conflict", w.Code, w.Body.String())
	}
	w = request("GET", "/api/v1/preparations/show?id="+p.ID, nil, false, false, false)
	if w.Code != 403 {
		t.Fatal("document leaked without session")
	}
	w = request("GET", "/api/v1/preparations/show?id=missing", nil, true, false, false)
	if w.Code != 404 {
		t.Fatal(w.Code)
	}
	w = request("POST", "/api/v1/preparations/command", bytes.Repeat([]byte("x"), 65537), true, true, true)
	if w.Code == 200 {
		t.Fatal("unbounded request")
	}
	got, _ := s.preparation(p.ID)
	if got.Revision != current.Revision {
		t.Fatal("invalid request changed state")
	}
}

func TestPreparationMethodsRejectOutsideProject(t *testing.T) {
	s := storeTest(t)
	prepMethods(t, s)
	path := filepath.Join(s.root, ".claude/skills/apex/SKILL.md")
	outside := filepath.Join(t.TempDir(), "outside.md")
	if e := os.WriteFile(outside, []byte("outside"), 0600); e != nil {
		t.Fatal(e)
	}
	if e := os.Remove(path); e != nil {
		t.Fatal(e)
	}
	if e := os.Symlink(outside, path); e != nil {
		t.Fatal(e)
	}
	if _, e := s.preparationMethod("apex"); e == nil {
		t.Fatal("external method allowed")
	}
	if m, e := s.preparationMethod("audit_pdca"); e != nil || m.ID != "audit-pdca" {
		t.Fatal("alias", m, e)
	}
}

func TestPreparationPageCSPAndSafeEntry(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "127.0.0.1:9876", "prep-test-session")
	read := func(path string) *httptest.ResponseRecorder {
		r := httptest.NewRequest("GET", "http://127.0.0.1:9876"+path, nil)
		r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "prep-test-session"})
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		return w
	}
	page := read("/prepare.html")
	if page.Code != 200 {
		t.Fatal(page.Code)
	}
	csp := page.Header().Get("Content-Security-Policy")
	if !strings.Contains(csp, "script-src 'self';") || !strings.Contains(csp, "style-src 'self' 'nonce-style-") || !strings.Contains(csp, "style-src-attr 'unsafe-inline'") {
		t.Fatal(csp)
	}
	if strings.Contains(page.Body.String(), "SWARM_STYLE_NONCE") {
		t.Fatal("nonce not rendered")
	}
	if strings.Contains(read("/").Header().Get("Content-Security-Policy"), "unsafe-inline") {
		t.Fatal("editor exception spread to cockpit")
	}
	if read("/prepare.html").Header().Get("Content-Security-Policy") == csp {
		t.Fatal("style nonce reused")
	}
	if read("/session/prep-test-session?view=prepare&id=prep-demo").Header().Get("Location") != "/prepare.html?id=prep-demo" {
		t.Fatal("missing preparation deep link")
	}
	if read("/session/prep-test-session?view=prepare&id=https://outside.invalid").Header().Get("Location") != "/prepare.html" {
		t.Fatal("unsafe preparation id")
	}
	link := read("/session/prep-test-session?view=prepare")
	if link.Header().Get("Location") != "/prepare.html" {
		t.Fatal("missing safe preparation entry")
	}
	if read("/session/prep-test-session?view=https://outside.invalid").Header().Get("Location") != "/" {
		t.Fatal("open redirect")
	}
}

func TestPreparationDialogueHTTPGuards(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "127.0.0.1:9876", "prep-test-session")
	for _, path := range []string{"/api/v1/preparations/send", "/api/v1/preparations/stop"} {
		for _, missing := range []string{"cookie", "csrf", "origin"} {
			r := httptest.NewRequest("POST", "http://127.0.0.1:9876"+path, strings.NewReader(`{}`))
			if missing != "cookie" {
				r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "prep-test-session"})
			}
			if missing != "csrf" {
				r.Header.Set("X-Swarm-CSRF", "prep-test-session")
			}
			if missing != "origin" {
				r.Header.Set("Origin", "http://127.0.0.1:9876")
			}
			w := httptest.NewRecorder()
			h.ServeHTTP(w, r)
			if w.Code != 403 {
				t.Fatal(path, missing, w.Code)
			}
		}
	}
	var count int
	s.db.QueryRow("SELECT count(*) FROM preparation_turns").Scan(&count)
	if count != 0 {
		t.Fatal("unauthorized turn")
	}
}

func TestPreparationFallbackAndStaticCache(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	h := newWebHandler(s, "127.0.0.1:9876", "test-session")
	get := func(path, etag string, authenticated bool) *httptest.ResponseRecorder {
		r := httptest.NewRequest("GET", "http://127.0.0.1:9876"+path, nil)
		if authenticated {
			r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "test-session"})
		}
		r.Header.Set("If-None-Match", etag)
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		return w
	}
	asset := get("/prephase.js", "", true)
	if asset.Code != 200 || asset.Header().Get("ETag") == "" {
		t.Fatal(asset.Code, asset.Header())
	}
	if w := get("/prephase.js", asset.Header().Get("ETag"), true); w.Code != 304 {
		t.Fatal(w.Code)
	}
	if w := get("/prephase.js", asset.Header().Get("ETag"), false); w.Code != 403 {
		t.Fatal("cache bypassed session", w.Code)
	}
	t.Setenv("SWARM_PREPARATION_DISABLED", "1")
	if w := get("/prepare.html", "", true); w.Code != 503 || !strings.Contains(w.Body.String(), "Revenir au pilotage") {
		t.Fatal(w.Code)
	}
	if w := get("/", "", true); w.Code != 200 {
		t.Fatal("fallback hid cockpit", w.Code)
	}
	if w := get("/api/v1/preparations/show?id="+p.ID, "", true); w.Code != 200 || w.Header().Get("Cache-Control") != "no-store" {
		t.Fatal("documents unavailable or cacheable", w.Code, w.Header())
	}
}
