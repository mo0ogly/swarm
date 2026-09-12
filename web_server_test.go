//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestWebAuthenticationOriginAndSharedGuard(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	_, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	h := newWebHandler(s, "local.test", "secret-test-capability")
	request := func(method, path, origin, csrf, host string, auth bool, body []byte) *httptest.ResponseRecorder {
		req := httptest.NewRequest(method, "http://local.test"+path, bytes.NewReader(body))
		req.Host = host
		req.Header.Set("Origin", origin)
		req.Header.Set("X-Swarm-CSRF", csrf)
		if auth {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "secret-test-capability"})
		}
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, req)
		return rr
	}
	if r := request("GET", "/api/v1/works", "", "", "local.test", false, nil); r.Code != 403 {
		t.Fatal("unauthenticated access", r.Code)
	}
	if r := request("GET", "/api/v1/works", "", "", "evil.test", true, nil); r.Code != 403 {
		t.Fatal("DNS rebinding permitted")
	}
	current, _ := s.get(w.ID)
	raw, _ := json.Marshal(webRequest{Kind: "task", Work: w.ID, Task: "t1", Event: newID("test-"), Revision: current.Revision, Request: Request{Status: "todo"}})
	for _, origin := range []string{"", "http://evil.test"} {
		if r := request("POST", "/api/v1/action", origin, "secret-test-capability", "local.test", true, raw); r.Code != 403 {
			t.Fatal("cross-site mutation", r.Code)
		}
	}
	if r := request("POST", "/api/v1/action", "http://local.test", "wrong", "local.test", true, raw); r.Code != 403 {
		t.Fatal("missing CSRF rejected")
	}
	rr := request("POST", "/api/v1/action", "http://local.test", "secret-test-capability", "local.test", true, raw)
	if rr.Code != 400 || !strings.Contains(rr.Body.String(), "active_agent") {
		t.Fatal("web bypasses shared guard", rr.Code, rr.Body.String())
	}
	if rr.Header().Get("Content-Security-Policy") == "" {
		t.Fatal("missing CSP")
	}
}
func TestWebReportCannotEscapeDocumentation(t *testing.T) {
	s := storeTest(t)
	os.Mkdir(filepath.Join(s.root, "docs"), 0700)
	os.WriteFile(filepath.Join(s.root, "private.txt"), []byte("private"), 0600)
	os.Symlink("../private.txt", filepath.Join(s.root, "docs/link.md"))
	h := newWebHandler(s, "local.test", "token")
	r := httptest.NewRequest("GET", "http://local.test/api/v1/report?path=docs/link.md", nil)
	r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "token"})
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, r)
	if rr.Code != 403 || strings.Contains(rr.Body.String(), "private") {
		t.Fatal("report leaked outside docs", rr.Code)
	}
}
func TestWebEventCursorReconnect(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	// Exercise the actual HTTP stream and Last-Event-ID, without issuing mutations.
	var host string
	server := httptest.NewUnstartedServer(http.HandlerFunc(func(rw http.ResponseWriter, r *http.Request) { newWebHandler(s, host, "token").ServeHTTP(rw, r) }))
	server.Start()
	defer server.Close()
	host = strings.TrimPrefix(server.URL, "http://")
	req, _ := http.NewRequest("GET", server.URL+"/api/v1/events?work="+w.ID, nil)
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "token"})
	req.Header.Set("Last-Event-ID", "1")
	resp, e := server.Client().Do(req)
	if e != nil {
		t.Fatal(e)
	}
	defer resp.Body.Close()
	buf := make([]byte, 4096)
	n, e := resp.Body.Read(buf)
	if e != nil && e != io.EOF {
		t.Fatal(e)
	}
	if strings.Contains(string(buf[:n]), "id: 1\n") || !strings.Contains(string(buf[:n]), "id: 2\n") {
		t.Fatal("cursor replay", string(buf[:n]))
	}
}
