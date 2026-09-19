//go:build linux

package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"golang.org/x/sys/unix"
)

func TestTerminalProviderHelper(t *testing.T) {
	if os.Getenv("SWARM_TEST_TERMINAL") != "1" {
		return
	}
	if _, e := unix.IoctlGetTermios(0, unix.TCGETS); e != nil {
		os.Exit(8)
	}
	for _, arg := range os.Args {
		if strings.Contains(arg, "TERMINAL_FLOOD") {
			_, _ = os.Stdout.Write(bytes.Repeat([]byte("x"), 5<<20))
			time.Sleep(time.Minute)
			os.Exit(0)
		}
	}
	fmt.Print("\x1b[32mPTY READY\x1b[0m\r\n")
	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		line := scanner.Text()
		if line == "/exit" {
			os.Exit(0)
		}
		ws, e := unix.IoctlGetWinsize(0, unix.TIOCGWINSZ)
		if e != nil {
			os.Exit(9)
		}
		fmt.Printf("REPLY:%s SIZE:%dx%d\r\n", line, ws.Col, ws.Row)
	}
	os.Exit(0)
}
func terminalFixture(t *testing.T, s *Store) (Work, Launch) {
	t.Helper()
	w, r := setupAgent(t, s)
	r.Mode = "terminal"
	p, _ := s.providers()
	provider := p.Providers["fixture"]
	provider.InteractiveArgs = []string{"-test.run=^TestTerminalProviderHelper$"}
	provider.Env = []string{"SWARM_TEST_TERMINAL"}
	p.Providers["fixture"] = provider
	t.Setenv("SWARM_TEST_TERMINAL", "1")
	b, _ := json.Marshal(p)
	if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), b, 0600); e != nil {
		t.Fatal(e)
	}
	return w, r
}
func terminalTranscript(t *testing.T, s *Store, id, want string) string {
	t.Helper()
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		var cursor int64
		var text strings.Builder
		for {
			events, e := s.terminalEvents(id, cursor)
			if e != nil {
				t.Fatal(e)
			}
			for _, v := range events {
				text.Write(v.Data)
				cursor = v.Seq
			}
			if len(events) < 16 {
				break
			}
		}
		if strings.Contains(text.String(), want) {
			return text.String()
		}
		time.Sleep(20 * time.Millisecond)
	}
	t.Fatalf("terminal output missing %q", want)
	return ""
}
func TestTerminalPTYLeaseResizeReplayAndFinish(t *testing.T) {
	s := storeTest(t)
	w, r := terminalFixture(t, s)
	a, created, e := s.prepare(w.ID, r)
	if e != nil || !created {
		t.Fatal(created, e)
	}
	duplicate, created, e := s.prepare(w.ID, r)
	if e != nil || created || duplicate.ID != a.ID {
		t.Fatal("launch replay", e)
	}
	done := make(chan error, 1)
	go func() { done <- s.supervise(a.ID) }()
	defer func() {
		_ = s.stopAgent(a.ID)
		select {
		case e := <-done:
			if e != nil {
				t.Error(e)
			}
		case <-time.After(7 * time.Second):
			t.Error("supervisor did not finish")
		}
	}()
	a = waitAgent(t, s, a.ID, func(a Agent) bool { return a.Status == "running" })
	terminalTranscript(t, s, a.ID, "\x1b[32mPTY READY")
	request := terminalRequest{Client: "view-a", Kind: "claim"}
	lease, e := s.terminalRPC(a, request)
	if e != nil || !lease.Writable {
		t.Fatal(lease, e)
	}
	other, e := s.terminalRPC(a, terminalRequest{Client: "view-b", Kind: "claim"})
	if e != nil || !other.Busy || other.Writable {
		t.Fatal("second writer", other, e)
	}
	request.Lease = lease.Lease
	request.Kind = "resize"
	request.Cols = 110
	request.Rows = 35
	if _, e = s.terminalRPC(a, request); e != nil {
		t.Fatal(e)
	}
	request.Kind = "input"
	request.Seq = 1
	request.Data = []byte("Bonjour é\n")
	if _, e = s.terminalRPC(a, request); e != nil {
		t.Fatal(e)
	}
	if _, e = s.terminalRPC(a, request); e != nil {
		t.Fatal("idempotent input", e)
	}
	transcript := terminalTranscript(t, s, a.ID, "REPLY:Bonjour é SIZE:110x35")
	if strings.Count(transcript, "REPLY:Bonjour é") != 1 {
		t.Fatal("duplicate write", transcript)
	}
	request.Data = []byte("wrong\n")
	if _, e = s.terminalRPC(a, request); e == nil {
		t.Fatal("same sequence different bytes accepted")
	}
	request.Kind = "release"
	if _, e = s.terminalRPC(a, request); e != nil {
		t.Fatal(e)
	}
	second, e := s.terminalRPC(a, terminalRequest{Client: "view-b", Kind: "claim"})
	if e != nil || !second.Writable {
		t.Fatal(e, second)
	}
	// Reopening a Store simulates another server process; output and supervisor survive.
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	terminalTranscript(t, reopened, a.ID, "REPLY:Bonjour é")
	_, e = reopened.terminalRPC(a, terminalRequest{Client: "view-b", Lease: second.Lease, Kind: "input", Seq: 1, Data: []byte("/exit\n")})
	if e != nil {
		t.Fatal(e)
	}
	ended := waitAgent(t, s, a.ID, func(a Agent) bool { return !activeAgent(a) })
	if ended.Status != "completed" || ended.Progress.Degraded == "" || ended.Usage != nil {
		t.Fatal(ended)
	}
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	task, _ := current.task(r.TaskID)
	if task.Status == "accepted" {
		t.Fatal("exit auto-accepted task")
	}
	if _, e = s.terminalRPC(ended, terminalRequest{Kind: "claim", Client: "view-c"}); e == nil {
		t.Fatal("ended terminal writable")
	}
}
func TestTerminalGuardAndCommandPolicy(t *testing.T) {
	s := storeTest(t)
	w, r := terminalFixture(t, s)
	r.Limits = &RunLimits{MaxToolCalls: 2}
	if _, _, e := s.prepare(w.ID, r); e == nil {
		t.Fatal("strict limit bypass")
	}
	r.Limits = nil
	current, _ := s.get(w.ID)
	current.Tasks[0].PlanToolLimit = 3
	b, _ := json.Marshal(current)
	_, _ = s.db.Exec("UPDATE works SET body=? WHERE id=?", b, w.ID)
	if _, _, e := s.prepare(w.ID, r); e == nil || !strings.Contains(e.Error(), "plafond") {
		t.Fatal("plan cap bypass", e)
	}
	if _, e := terminalProvider(Provider{Command: "/bin/claude", Args: []string{"-p", "--custom-policy"}}); e == nil {
		t.Fatal("unrecognized args removed")
	}
	p, e := terminalProvider(Provider{Command: "/bin/codex", Args: []string{"exec", "--json", "--sandbox", "workspace-write", "-"}})
	if e != nil || strings.Join(p.Args, " ") != "--sandbox workspace-write --no-alt-screen" {
		t.Fatal(p, e)
	}
	var count int
	_ = s.db.QueryRow("SELECT count(*) FROM agents").Scan(&count)
	if count != 0 {
		t.Fatal("refused launches persisted", count)
	}
}
func TestTerminalControlHTTPBoundary(t *testing.T) {
	s := storeTest(t)
	w, r := terminalFixture(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	handler := newWebHandler(s, "127.0.0.1:9876", "token")
	call := func(method, path, body, origin, csrf string, cookie bool) *httptest.ResponseRecorder {
		request := httptest.NewRequest(method, "http://127.0.0.1:9876"+path, strings.NewReader(body))
		if cookie {
			request.AddCookie(&http.Cookie{Name: "swarm_session", Value: "token"})
		}
		request.Header.Set("Origin", origin)
		request.Header.Set("X-Swarm-CSRF", csrf)
		response := httptest.NewRecorder()
		handler.ServeHTTP(response, request)
		return response
	}
	endpoint := "/api/v1/terminal/control"
	body := fmt.Sprintf(`{"agent":%q,"work":%q,"client":"view","kind":"claim"}`, a.ID, w.ID)
	for _, test := range []struct {
		origin, csrf string
		cookie       bool
	}{{"", "", false}, {"http://evil.invalid", "token", true}, {"http://127.0.0.1:9876", "wrong", true}} {
		if v := call("POST", endpoint, body, test.origin, test.csrf, test.cookie); v.Code != 403 {
			t.Fatal("boundary", v.Code, v.Body.String())
		}
	}
	read := call("GET", "/api/v1/terminal?work=wrong&agent="+a.ID+"&after=0", "", "", "", true)
	if read.Code == 200 {
		t.Fatal("cross work")
	}
	page := call("GET", "/terminal.html", "", "", "", true)
	csp := page.Header().Get("Content-Security-Policy")
	if page.Code != 200 || !strings.Contains(csp, "frame-ancestors 'self'") || strings.Contains(page.Body.String(), "SWARM_STYLE_NONCE") {
		t.Fatal(page.Code, csp)
	}
}

func TestTerminalDeadlineAndStop(t *testing.T) {
	s := storeTest(t)
	w, r := terminalFixture(t, s)
	r.Timeout = 1
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.supervise(a.ID); e != nil {
		t.Fatal(e)
	}
	a, e = s.agent(a.ID)
	if e != nil || a.Status != "interrupted" || a.StopKind != "delai" {
		t.Fatal(a.Status, a.StopKind, e)
	}
}

func TestTerminalOutputBound(t *testing.T) {
	s := storeTest(t)
	w, r := terminalFixture(t, s)
	r.Instruction = "TERMINAL_FLOOD"
	r.Timeout = 10
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.supervise(a.ID); e != nil {
		t.Fatal(e)
	}
	a, e = s.agent(a.ID)
	if e != nil || a.Status != "interrupted" || a.StopKind != "garde" {
		t.Fatal(a.Status, a.StopKind, e)
	}
	var size int
	if e = s.db.QueryRow("SELECT sum(length(data)) FROM terminal_events WHERE agent_id=?", a.ID).Scan(&size); e != nil || size > terminalOutputLimit {
		t.Fatal(size, e)
	}
}
