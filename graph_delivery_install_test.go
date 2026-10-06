package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

type d04InstalledVersion struct {
	Schema int `json:"schema_version"`
	Binary struct {
		Version    string  `json:"version"`
		Commit     *string `json:"commit"`
		Modified   *bool   `json:"modified"`
		BuildDate  *string `json:"build_date"`
		Provenance string  `json:"provenance"`
	} `json:"binary"`
}

func d04Environment(overrides map[string]string) []string {
	blocked := make(map[string]bool, len(overrides))
	for key := range overrides {
		blocked[key] = true
	}
	env := make([]string, 0, len(os.Environ())+len(overrides))
	for _, entry := range os.Environ() {
		key, _, _ := strings.Cut(entry, "=")
		if !blocked[key] {
			env = append(env, entry)
		}
	}
	for key, value := range overrides {
		env = append(env, key+"="+value)
	}
	return env
}

func installD04Binary(t *testing.T, binDir, version, commit string) string {
	t.Helper()
	cmd := exec.Command("./install.sh", "--mode", "native", "--bin-dir", binDir)
	cmd.Env = d04Environment(map[string]string{
		"SWARM_VERSION":    version,
		"SWARM_COMMIT":     commit,
		"SWARM_MODIFIED":   "false",
		"SWARM_BUILD_DATE": "2026-10-06T12:00:00Z",
	})
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("native install %s failed: %v\n%s", version, err, output)
	}
	path := filepath.Join(binDir, "swarm")
	info, err := os.Stat(path)
	if err != nil || info.Mode().Perm() != 0755 {
		t.Fatalf("installed binary permissions: info=%v err=%v", info, err)
	}
	if !bytes.Contains(output, []byte("Binaire installé")) || bytes.Contains(output, []byte("session/")) {
		t.Fatalf("native install did not stay build-only: %s", output)
	}
	return path
}

func readD04Version(t *testing.T, binary string) d04InstalledVersion {
	t.Helper()
	cmd := exec.Command(binary, "--json", "version")
	cmd.Dir = t.TempDir()
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("installed version command failed: %v\n%s", err, output)
	}
	var got d04InstalledVersion
	if err := json.Unmarshal(output, &got); err != nil {
		t.Fatalf("installed version is not JSON: %v\n%s", err, output)
	}
	return got
}

func assertD04Version(t *testing.T, got d04InstalledVersion, version, commit string) {
	t.Helper()
	if got.Schema != 1 || got.Binary.Version != version || got.Binary.Commit == nil || *got.Binary.Commit != commit || got.Binary.Modified == nil || *got.Binary.Modified || got.Binary.BuildDate == nil || *got.Binary.BuildDate != "2026-10-06T12:00:00Z" || got.Binary.Provenance != "injected" {
		t.Fatalf("installed identity mismatch: %+v", got)
	}
}

func TestGraphDeliveryD04InstallVersion(t *testing.T) {
	binDir := filepath.Join(t.TempDir(), "bin with spaces")
	commit := strings.Repeat("4", 40)
	binary := installD04Binary(t, binDir, "v0.0.0-d04", commit)
	assertD04Version(t, readD04Version(t, binary), "v0.0.0-d04", commit)

	// Exercise the authenticated static-file boundary used by an installed
	// binary. The ETag must follow the embedded bytes so replacing the binary
	// cannot leave a fixed JS entry point silently stale.
	const host, token = "d04.local", "d04-session"
	handler := newWebHandler(nil, host, token)
	asset, err := cockpitWeb.ReadFile("web/graph.js")
	if err != nil || len(asset) == 0 {
		t.Fatalf("embedded graph asset unavailable: bytes=%d err=%v", len(asset), err)
	}
	req := httptest.NewRequest(http.MethodGet, "http://"+host+"/graph.js", nil)
	req.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token})
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, req)
	if response.Code != http.StatusOK || !bytes.Equal(response.Body.Bytes(), asset) {
		t.Fatalf("installed asset response: status=%d bytes=%d want=%d", response.Code, response.Body.Len(), len(asset))
	}
	etag := response.Header().Get("ETag")
	if etag != fmt.Sprintf("%q", hash(asset)) || response.Header().Get("Cache-Control") != "private, no-cache" {
		t.Fatalf("asset freshness headers: etag=%q cache=%q", etag, response.Header().Get("Cache-Control"))
	}
	req = httptest.NewRequest(http.MethodGet, "http://"+host+"/graph.js", nil)
	req.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token})
	req.Header.Set("If-None-Match", etag)
	response = httptest.NewRecorder()
	handler.ServeHTTP(response, req)
	if response.Code != http.StatusNotModified || response.Body.Len() != 0 {
		t.Fatalf("asset revalidation: status=%d body=%q", response.Code, response.Body.String())
	}
}

func TestGraphDeliveryD04RollbackSafety(t *testing.T) {
	root := t.TempDir()
	binDir := filepath.Join(root, "bin")
	sentinel := filepath.Join(root, "project-state.must-not-change")
	if err := os.WriteFile(sentinel, []byte("preserved"), 0600); err != nil {
		t.Fatal(err)
	}

	oldCommit := strings.Repeat("1", 40)
	binary := installD04Binary(t, binDir, "v0.0.0-d04-old", oldCommit)
	oldBytes, err := os.ReadFile(binary)
	if err != nil {
		t.Fatal(err)
	}
	oldDigest := sha256.Sum256(oldBytes)
	backup := filepath.Join(root, "swarm.before-update")
	if err := os.WriteFile(backup, oldBytes, 0500); err != nil {
		t.Fatal(err)
	}

	failedUpdate := exec.Command("./install.sh", "--mode", "native", "--bin-dir", binDir)
	failedUpdate.Env = d04Environment(map[string]string{
		"SWARM_VERSION":    "not-a-release",
		"SWARM_COMMIT":     strings.Repeat("f", 40),
		"SWARM_MODIFIED":   "false",
		"SWARM_BUILD_DATE": "2026-10-06T12:00:00Z",
	})
	failedOutput, failedErr := failedUpdate.CombinedOutput()
	if failedErr == nil || !bytes.Contains(failedOutput, []byte("invalid SWARM_VERSION")) {
		t.Fatalf("invalid update was not rejected: err=%v output=%s", failedErr, failedOutput)
	}
	assertD04Version(t, readD04Version(t, binary), "v0.0.0-d04-old", oldCommit)

	newCommit := strings.Repeat("2", 40)
	installD04Binary(t, binDir, "v0.0.0-d04-new", newCommit)
	assertD04Version(t, readD04Version(t, binary), "v0.0.0-d04-new", newCommit)

	// Restore through a staged file and rename, mirroring the installer's
	// same-filesystem replacement. No project root or database is opened.
	staged := filepath.Join(binDir, ".swarm-rollback-stage")
	backupBytes, err := os.ReadFile(backup)
	if err != nil {
		t.Fatal(err)
	}
	if got := sha256.Sum256(backupBytes); got != oldDigest {
		t.Fatalf("backup digest changed: got=%x want=%x", got, oldDigest)
	}
	if err := os.WriteFile(staged, backupBytes, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.Rename(staged, binary); err != nil {
		t.Fatal(err)
	}
	assertD04Version(t, readD04Version(t, binary), "v0.0.0-d04-old", oldCommit)
	if raw, err := os.ReadFile(sentinel); err != nil || string(raw) != "preserved" {
		t.Fatalf("rollback touched unrelated state: %q err=%v", raw, err)
	}
}
