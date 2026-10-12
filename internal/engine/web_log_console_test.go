//go:build linux

package engine

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestLogConsoleFrameCSPIsScopedAndAuthenticated(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "127.0.0.1:9876", "log-console-test-session")
	read := func(method, path string, auth bool) *httptest.ResponseRecorder {
		req := httptest.NewRequest(method, "http://127.0.0.1:9876"+path, nil)
		if auth {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "log-console-test-session"})
		}
		out := httptest.NewRecorder()
		h.ServeHTTP(out, req)
		return out
	}
	denied := read("GET", "/log-console.html", false)
	if denied.Code == 200 || strings.Contains(denied.Body.String(), "journal-editor") {
		t.Fatal("unauthenticated console served")
	}
	page := read("GET", "/log-console.html", true)
	policy := page.Header().Get("Content-Security-Policy")
	if page.Code != 200 || !strings.Contains(policy, "frame-ancestors 'self'") || !strings.Contains(policy, "style-src 'self' 'nonce-style-") || !strings.Contains(policy, "script-src 'self';") || strings.Contains(policy, "script-src 'unsafe-inline'") {
		t.Fatalf("console policy %d: %s", page.Code, policy)
	}
	if strings.Contains(page.Body.String(), "SWARM_STYLE_NONCE") || !strings.Contains(page.Body.String(), "swarm-style-nonce") {
		t.Fatal("console style nonce missing")
	}
	if read("GET", "/log-console.html", true).Header().Get("Content-Security-Policy") == policy {
		t.Fatal("console nonce reused")
	}
	root := read("GET", "/", true).Header().Get("Content-Security-Policy")
	if strings.Contains(root, "unsafe-inline") || strings.Contains(root, "nonce-") || !strings.Contains(root, "frame-ancestors 'none'") {
		t.Fatal("console exception spread to cockpit")
	}
	if head := read("HEAD", "/log-console.html", true); head.Code != 200 || head.Body.Len() != 0 {
		t.Fatal("HEAD did not remain read-only")
	}
	if post := read("POST", "/log-console.html", true); post.Code == 200 {
		t.Fatal("console accepted mutation")
	}
}
