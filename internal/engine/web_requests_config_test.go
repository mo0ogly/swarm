//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

func webRequestChange(id string, revision int, general, snapshot, connection int64) map[string]any {
	return map[string]any{"schema_version": 1, "event_id": id, "expected_revision": revision,
		"reason": "Explicit browser waiting policy for isolated recipe",
		"values": map[string]int64{"general_ms": general, "snapshot_ms": snapshot, "connection_test_ms": connection}}
}

func TestWebRequestsPublicPersistencePreviewReplayAndConflict(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "local.test", "requests-token")
	read := func() map[string]any {
		t.Helper()
		r := runLimitsWebRequest(h, "GET", "/api/v1/web-requests", "requests-token", nil)
		if r.Code != 200 {
			t.Fatal("browser policy absent", r.Code, r.Body.String())
		}
		var c map[string]any
		if e := json.Unmarshal(r.Body.Bytes(), &c); e != nil {
			t.Fatal(e)
		}
		return c
	}
	before := read()
	raw, _ := json.Marshal(webRequestChange("settings-1", 0, 0, 123456, 70000))
	for _, action := range []string{"preview", "apply", "apply"} {
		r := runLimitsWebRequest(h, "POST", "/api/v1/web-requests/"+action, "requests-token", raw)
		if r.Code != 200 {
			t.Fatal(action, r.Code, r.Body.String())
		}
		if action == "preview" && read()["revision"] != before["revision"] {
			t.Fatal("preview mutated")
		}
	}
	c := read()
	if c["revision"] != float64(1) || len(c["history"].([]any)) != 1 {
		t.Fatal("replay changed history", c)
	}
	if c["values"].(map[string]any)["general_ms"] != float64(0) || c["values"].(map[string]any)["snapshot_ms"] != float64(123456) {
		t.Fatal(c)
	}
	input := filepath.Join(t.TempDir(), "config.json")
	var output bytes.Buffer
	if e := s.runLimitsCLI([]string{"run-limits", "web-requests", "show"}, "", &output); e != nil {
		t.Fatal(e)
	}
	var cli map[string]any
	if e := json.Unmarshal(output.Bytes(), &cli); e != nil || cli["revision"] != c["revision"] {
		t.Fatal(output.String(), e)
	}
	if e := os.WriteFile(input, raw, 0600); e != nil {
		t.Fatal(e)
	}
	output.Reset()
	if e := s.runLimitsCLI([]string{"run-limits", "web-requests", "apply"}, input, &output); e != nil {
		t.Fatal(e)
	}
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	h2 := newWebHandler(other, "local.test", "requests-token")
	r := runLimitsWebRequest(h2, "GET", "/api/v1/web-requests/script.js", "requests-token", nil)
	if r.Code != 200 || !strings.Contains(r.Body.String(), `"snapshot_ms":123456`) || !strings.Contains(r.Header().Get("Content-Type"), "javascript") || r.Header().Get("Cache-Control") != "no-store" {
		t.Fatal("effective bootstrap", r.Code, r.Header(), r.Body.String())
	}
	for _, id := range []string{"settings-1", "stale"} {
		raw, _ = json.Marshal(webRequestChange(id, 0, 100, 100, 100))
		if r := runLimitsWebRequest(h2, "POST", "/api/v1/web-requests/apply", "requests-token", raw); r.Code != 409 {
			t.Fatal("conflict not refused", id, r.Code, r.Body.String())
		}
	}
}

func TestWebRequestsInvalidAuthenticationAndConcurrentRevision(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "local.test", "requests-token")
	for _, path := range []string{"/api/v1/web-requests", "/api/v1/web-requests/script.js"} {
		if r := runLimitsWebRequest(h, "GET", path, "", nil); r.Code == 200 {
			t.Fatal("unauthenticated", path)
		}
	}
	for _, values := range []map[string]any{
		{"general_ms": -1, "snapshot_ms": 0, "connection_test_ms": 0},
		{"general_ms": 1.5, "snapshot_ms": 0, "connection_test_ms": 0},
		{"general_ms": int64(9007199254740992), "snapshot_ms": 0, "connection_test_ms": 0},
		{"general_ms": 1, "snapshot_ms": 2},
		{"general_ms": 1, "snapshot_ms": 2, "connection_test_ms": 3, "unknown": 4},
	} {
		req := webRequestChange("invalid", 0, 0, 0, 0)
		req["values"] = values
		raw, _ := json.Marshal(req)
		if r := runLimitsWebRequest(h, "POST", "/api/v1/web-requests/apply", "requests-token", raw); r.Code != 400 {
			t.Fatal("invalid accepted", values, r.Code, r.Body.String())
		}
	}
	var wg sync.WaitGroup
	replies := make(chan int, 2)
	for _, id := range []string{"first", "second"} {
		wg.Add(1)
		go func(id string) {
			defer wg.Done()
			raw, _ := json.Marshal(webRequestChange(id, 0, 100, 200, 300))
			replies <- runLimitsWebRequest(h, "POST", "/api/v1/web-requests/apply", "requests-token", raw).Code
		}(id)
	}
	wg.Wait()
	close(replies)
	successes, conflicts := 0, 0
	for code := range replies {
		if code == 200 {
			successes++
		}
		if code == 409 {
			conflicts++
		}
	}
	if successes != 1 || conflicts != 1 {
		t.Fatal("concurrent revisions", successes, conflicts)
	}
}

func TestWebRequestsConfigRefusesSymlinkAndAllPagesLoadPolicy(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "local.test", "requests-token")
	for _, page := range []string{"/", "/prepare.html", "/projects.html", "/terminal.html", "/log-console.html"} {
		r := runLimitsWebRequest(h, "GET", page, "requests-token", nil)
		if r.Code != 200 || !strings.Contains(r.Body.String(), `src="/api/v1/web-requests/script.js"`) || !strings.Contains(r.Body.String(), `src="/web-requests.js"`) {
			t.Fatal("policy omitted", page, r.Code)
		}
	}
	outside := filepath.Join(t.TempDir(), "outside.json")
	if e := os.WriteFile(outside, []byte(`{}`), 0600); e != nil {
		t.Fatal(e)
	}
	if e := os.Symlink(outside, filepath.Join(s.root, ".swarm", "web-requests.json")); e != nil {
		t.Fatal(e)
	}
	if r := runLimitsWebRequest(h, "GET", "/api/v1/web-requests", "requests-token", nil); r.Code == 200 {
		t.Fatal("config symlink accepted")
	}
}

func TestWebRequestsFullHistoryPreviewAndApplyAgree(t *testing.T) {
	s := storeTest(t)
	c, err := s.webRequestsConfig()
	if err != nil {
		t.Fatal(err)
	}
	request := WebRequestsChange{Schema: 1, EventID: "next-history", Values: c.Values, Reason: strings.Repeat("R", 2000)}
	for {
		next := c
		next.History = append(append([]WebRequestsRevision(nil), c.History...), WebRequestsRevision{Revision: c.Revision + 1, Values: c.Values, Actor: "fixture", At: "2026-10-13T07:00:00.000000000Z", Change: WebRequestsChange{Schema: 1, EventID: fmt.Sprintf("history-%d", c.Revision+1), Revision: c.Revision, Values: c.Values, Reason: request.Reason}})
		next.Revision++
		raw, e := json.MarshalIndent(next, "", "  ")
		if e != nil {
			t.Fatal(e)
		}
		if len(raw)+1 > 1048576 {
			break
		}
		c = next
	}
	path := filepath.Join(s.root, ".swarm", "web-requests.json")
	before, _ := json.MarshalIndent(c, "", "  ")
	before = append(before, '\n')
	if err := os.WriteFile(path, before, 0600); err != nil {
		t.Fatal(err)
	}
	request.Revision = c.Revision
	for _, preview := range []bool{true, false} {
		if _, err := s.changeWebRequests(request, preview); err == nil {
			t.Fatalf("history overflow approved; preview=%v", preview)
		}
		after, err := os.ReadFile(path)
		if err != nil || !bytes.Equal(before, after) {
			t.Fatal("rejection changed persisted history", err)
		}
		read, err := s.webRequestsConfig()
		if err != nil || read.Revision != c.Revision {
			t.Fatal("history lost after refusal", err)
		}
	}
}
