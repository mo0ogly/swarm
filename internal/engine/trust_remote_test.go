//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

const trustRemoteFixtureSession = "session-0123456789abcdef01234567"

func trustRemoteFixture(t *testing.T) (*Store, *httptest.Server) {
	t.Helper()
	s, _ := trustFixture(t)
	server := httptest.NewUnstartedServer(nil)
	handler := newWebHandler(s, server.Listener.Addr().String(), trustRemoteFixtureSession)
	server.Config.Handler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodGet && r.ContentLength != 0 {
			http.Error(w, "GET body refused", http.StatusBadRequest)
			return
		}
		handler.ServeHTTP(w, r)
	})
	server.Start()
	t.Cleanup(server.Close)
	t.Setenv("SWARM_WEB_SESSION", trustRemoteFixtureSession)
	t.Setenv("SWARM_TRUST_TOKEN", trustRequesterToken)
	return s, server
}

func TestActionsConfianceT4RemoteDestination(t *testing.T) {
	_, server := trustRemoteFixture(t)
	root := filepath.Join(t.TempDir(), "must-not-exist")
	var out, errs bytes.Buffer
	code := run([]string{"--root", root, "--server", server.URL, "trust", "show", "--json"}, &out, &errs)
	var view map[string]any
	if code != 0 || json.Unmarshal(out.Bytes(), &view) != nil || view["revision"] != float64(1) {
		t.Fatalf("server destination not used: exit=%d, output=%s error=%s", code, out.Bytes(), errs.Bytes())
	}
	if _, err := os.Stat(root); !os.IsNotExist(err) {
		t.Fatalf("local root accessed: %v", err)
	}
}

func TestActionsConfianceT4RemoteAuthenticatedPost(t *testing.T) {
	s, server := trustRemoteFixture(t)
	root := filepath.Join(t.TempDir(), "absent")
	var out, errs bytes.Buffer
	if code := run([]string{"--root", root, "--server", server.URL, "trust", "identity-context", "--json"}, &out, &errs); code != 0 {
		t.Fatalf("authenticated context refused: %d %s", code, errs.Bytes())
	}
	var identity map[string]any
	if json.Unmarshal(out.Bytes(), &identity) != nil || identity["principal_id"] != "alice" {
		t.Fatalf("wrong context: %s", out.Bytes())
	}
	request := `{"schema_version":1,"event_id":"remote-mandate","mandate_id":"remote-mandate","subject":"alice","actions":["settle"],"resource":"invoice/acme","max_amount_minor":50000,"currency":"EUR","beneficiary":"ACME"}`
	path := filepath.Join(t.TempDir(), "input.json")
	if err := os.WriteFile(path, []byte(request), 0600); err != nil {
		t.Fatal(err)
	}
	t.Setenv("SWARM_TRUST_TOKEN", trustAdminToken)
	out.Reset()
	errs.Reset()
	if code := run([]string{"--root", root, "--server", server.URL, "trust", "mandate-create", "--input", path, "--json"}, &out, &errs); code != 0 {
		t.Fatalf("remote mandate failed: %d %s", code, errs.Bytes())
	}
	view, err := s.trustView()
	if err != nil || len(view.(map[string]any)["mandates"].(map[string]TrustMandate)) != 1 {
		t.Fatalf("remote mutation missing: %v", err)
	}
	if _, err := os.Stat(root); !os.IsNotExist(err) {
		t.Fatalf("local root accessed: %v", err)
	}
}

func TestActionsConfianceT4RemoteInvalidOrigins(t *testing.T) {
	for _, origin := range []string{"https://127.0.0.1:1234", "http://localhost:1234", "http://192.0.2.1:1234", "http://127.0.0.1", "http://127.0.0.1:0", "http://127.0.0.1:65536", "http://user:password@127.0.0.1:1234", "http://127.0.0.1:1234/session/private", "http://127.0.0.1:1234/?token=private", "http://127.0.0.1:1234/#private", "http://127.0.0.1:1234/%2f"} {
		t.Run(origin, func(t *testing.T) {
			var out bytes.Buffer
			err := trustRemoteCLI(context.Background(), []string{"trust", "show"}, "", origin, trustRemoteFixtureSession, "", &out)
			if err == nil || out.Len() != 0 || strings.Contains(err.Error(), "password") || strings.Contains(err.Error(), "private") {
				t.Fatalf("unsafe origin admitted or disclosed: %v", err)
			}
		})
	}
	for _, origin := range []string{"http://127.0.0.1:1234/", "http://[::1]:1234"} {
		if _, err := trustRemoteOrigin(origin); err != nil {
			t.Fatal(err)
		}
	}
}

func TestActionsConfianceT4RemoteRefusals(t *testing.T) {
	s, server := trustRemoteFixture(t)
	before, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	beforeJSON, _ := json.Marshal(before)
	for _, tc := range []struct{ name, session, token, action string }{
		{"missing-session", "", trustRequesterToken, "identity-context"},
		{"wrong-session", "session-111111111111111111111111", trustRequesterToken, "identity-context"},
		{"missing-bearer", trustRemoteFixtureSession, "", "identity-context"},
		{"wrong-bearer", trustRemoteFixtureSession, "invalid", "identity-context"},
		{"header-injection", trustRemoteFixtureSession, "bad\r\nheader", "identity-context"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			var out bytes.Buffer
			err := trustRemoteCLI(context.Background(), []string{"trust", tc.action}, "", server.URL, tc.session, tc.token, &out)
			if err == nil || commandFailure(err).Code != "authorization_required" || out.Len() != 0 {
				t.Fatalf("refusal lost: %v", err)
			}
		})
	}
	after, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	afterJSON, _ := json.Marshal(after)
	if !bytes.Equal(beforeJSON, afterJSON) {
		t.Fatal("refusal mutated state")
	}
}

func TestActionsConfianceT4RemoteResponseBoundaries(t *testing.T) {
	for _, tc := range []struct {
		name, body string
		status     int
	}{
		{"html", "<html>private</html>", 200}, {"empty", "", 200}, {"null", "null", 200},
		{"empty-object", "{}", 200}, {"array", "[]", 200}, {"trailing", "{\"ok\":true} {}", 200},
		{"unstructured-error", "private", 500}, {"structured-error", `{"failure":{"code":"revision_conflict","message":"stale revision"}}`, 409},
	} {
		t.Run(tc.name, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				w.WriteHeader(tc.status)
				_, _ = w.Write([]byte(tc.body))
			}))
			defer server.Close()
			var out bytes.Buffer
			err := trustRemoteCLI(context.Background(), []string{"trust", "show"}, "", server.URL, trustRemoteFixtureSession, "", &out)
			if err == nil || out.Len() != 0 || strings.Contains(err.Error(), "private") {
				t.Fatalf("invalid response admitted/disclosed: %v", err)
			}
			if tc.name == "structured-error" && commandFailure(err).Code != "revision_conflict" {
				t.Fatalf("code lost: %v", err)
			}
		})
	}
}

func TestActionsConfianceT4RemoteNoRedirect(t *testing.T) {
	reached := make(chan bool, 1)
	target := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { reached <- true; _, _ = w.Write([]byte(`{"ok":true}`)) }))
	defer target.Close()
	redirect := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, target.URL, http.StatusTemporaryRedirect)
	}))
	defer redirect.Close()
	var out bytes.Buffer
	err := trustRemoteCLI(context.Background(), []string{"trust", "show"}, "", redirect.URL, trustRemoteFixtureSession, trustRequesterToken, &out)
	if err == nil || out.Len() != 0 {
		t.Fatalf("redirect admitted: %v", err)
	}
	select {
	case <-reached:
		t.Fatal("redirect received credentials")
	default:
	}
}

func TestActionsConfianceT4RemoteCancellation(t *testing.T) {
	started, stopped := make(chan bool, 1), make(chan bool, 1)
	release := make(chan struct{})
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		started <- true
		select {
		case <-r.Context().Done():
			stopped <- true
		case <-release:
		}
	}))
	defer server.Close()
	defer close(release)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	done := make(chan error, 1)
	go func() {
		var out bytes.Buffer
		done <- trustRemoteCLI(ctx, []string{"trust", "show"}, "", server.URL, trustRemoteFixtureSession, "", &out)
	}()
	select {
	case <-started:
	case <-time.After(3 * time.Second):
		t.Fatal("request not started")
	}
	cancel()
	select {
	case err := <-done:
		if commandFailure(err).Code != "interrupted" {
			t.Fatalf("cancellation lost: %v", err)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("transport not cancelled")
	}
	select {
	case <-stopped:
	case <-time.After(3 * time.Second):
		t.Fatal("server did not observe cancellation")
	}
}

func TestActionsConfianceT4RemoteUnavailableAndLocal(t *testing.T) {
	s, _ := trustFixture(t)
	server := httptest.NewServer(http.NotFoundHandler())
	origin := server.URL
	server.Close()
	for _, destination := range []string{origin, ""} {
		var out, errs bytes.Buffer
		code := run([]string{"--root", s.root, "--server", destination, "trust", "show", "--json"}, &out, &errs)
		if code == 0 || out.Len() != 0 {
			t.Fatal("unavailable/empty server fell back to local data")
		}
	}
	var out, errs bytes.Buffer
	if code := run([]string{"--root", s.root, "trust", "show", "--json"}, &out, &errs); code != 0 || !strings.Contains(out.String(), `"revision": 1`) {
		t.Fatalf("legacy local changed: %d %s", code, errs.Bytes())
	}
}
