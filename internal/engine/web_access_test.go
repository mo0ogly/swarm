//go:build linux

package engine

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestWebStableAddressLoginAndRestart(t *testing.T) {
	s := storeTest(t)
	token, err := s.webSessionToken("127.0.0.1:18792")
	if err != nil {
		t.Fatal(err)
	}
	h := newWebHandler(s, "local.test", token)
	get := httptest.NewRecorder()
	h.ServeHTTP(get, httptest.NewRequest("GET", "http://local.test/?lang=en", nil))
	if get.Code != 200 || !strings.Contains(get.Body.String(), "Sign in to Swarm") || strings.Contains(get.Body.String(), token) {
		t.Fatal("missing private login page", get.Code)
	}
	for _, origin := range []string{"", "http://evil.test"} {
		r := httptest.NewRequest("POST", "http://local.test/login", strings.NewReader(url.Values{"token": {token}}.Encode()))
		r.Header.Set("Content-Type", "application/x-www-form-urlencoded")
		r.Header.Set("Origin", origin)
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, r)
		if rr.Code != 403 || len(rr.Result().Cookies()) != 0 {
			t.Fatal("cross-site login accepted")
		}
	}
	login := func(key, next string) *httptest.ResponseRecorder {
		r := httptest.NewRequest("POST", "http://local.test/login", strings.NewReader(url.Values{"token": {key}, "next": {next}}.Encode()))
		r.Header.Set("Content-Type", "application/x-www-form-urlencoded")
		r.Header.Set("Origin", "http://local.test")
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, r)
		return rr
	}
	if rr := login("wrong", "/"); rr.Code != 401 || len(rr.Result().Cookies()) != 0 {
		t.Fatal("wrong key accepted")
	}
	rr := login(token, "/?work=w-example&lang=en")
	if rr.Code != 303 || rr.Header().Get("Location") != "/?work=w-example&lang=en" {
		t.Fatal("login redirect", rr.Code)
	}
	cookie := rr.Result().Cookies()[0]
	if !cookie.HttpOnly || cookie.SameSite != http.SameSiteStrictMode || cookie.MaxAge <= 0 {
		t.Fatal("cookie not persistent/private")
	}
	again, err := s.webSessionToken("127.0.0.1:18792")
	if err != nil || again != token {
		t.Fatal("restart changed key")
	}
	restarted := newWebHandler(s, "local.test", again)
	req := httptest.NewRequest("GET", "http://local.test/api/v1/session", nil)
	req.AddCookie(cookie)
	after := httptest.NewRecorder()
	restarted.ServeHTTP(after, req)
	if after.Code != 200 || !strings.Contains(after.Body.String(), token) {
		t.Fatal("cookie lost on restart")
	}
	for _, next := range []string{"https://evil.test/", "//evil.test/", "/\\evil.test/", ""} {
		if login(token, next).Header().Get("Location") != "/" {
			t.Fatal("open redirect")
		}
	}
	api := httptest.NewRecorder()
	h.ServeHTTP(api, httptest.NewRequest("GET", "http://local.test/api/v1/works", nil))
	if api.Code != 403 {
		t.Fatal("unauthenticated API accepted")
	}
}

func TestWebSessionLifetimeConfiguration(t *testing.T) {
	s := storeTest(t)
	path := filepath.Join(s.root, ".swarm", "web-session.json")
	if err := os.WriteFile(path, []byte(`{"schema_version":1,"cookie_max_age_seconds":3600}`), 0600); err != nil {
		t.Fatal(err)
	}
	if age, err := s.webSessionLifetime(); err != nil || age != 3600 {
		t.Fatal("override not applied", age, err)
	}
	for _, raw := range []string{`{"schema_version":1,"cookie_max_age_seconds":0}`, `{"schema_version":1,"cookie_max_age_seconds":1,"unknown":true}`} {
		os.WriteFile(path, []byte(raw), 0600)
		if _, err := s.webSessionLifetime(); err == nil {
			t.Fatal("invalid configuration accepted")
		}
	}
	os.Remove(path)
	os.Symlink("missing", path)
	if _, err := s.webSessionLifetime(); err == nil {
		t.Fatal("symlink accepted")
	}
}

func TestWebLoginDesignAssetsRemainPublicAndAPIPrivate(t *testing.T) {
	h := newWebHandler(storeTest(t), "local.test", "fixture-secret")
	for _, asset := range []struct{ path, contentType string }{
		{"/swarm-design.css", "text/css"},
		{"/swarm-logo.png", "image/png"},
	} {
		for _, method := range []string{"GET", "HEAD"} {
			rr := httptest.NewRecorder()
			h.ServeHTTP(rr, httptest.NewRequest(method, "http://local.test"+asset.path, nil))
			if rr.Code != 200 || !strings.Contains(rr.Header().Get("Content-Type"), asset.contentType) {
				t.Fatalf("login asset %s %s unavailable: %d %s", method, asset.path, rr.Code, rr.Header().Get("Content-Type"))
			}
		}
		wrongHost := httptest.NewRecorder()
		h.ServeHTTP(wrongHost, httptest.NewRequest("GET", "http://evil.test"+asset.path, nil))
		if wrongHost.Code != 403 {
			t.Fatal("public asset bypassed host validation")
		}
	}
	for _, path := range []string{"/api/v1/works", "/cockpit.js", "/swarm-shell.js", "/terminal.html"} {
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, httptest.NewRequest("GET", "http://local.test"+path, nil))
		if rr.Code != 403 {
			t.Fatalf("private route %s opened without session: %d", path, rr.Code)
		}
	}
}
