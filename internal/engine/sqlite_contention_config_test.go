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

func TestSQLiteConfigCLI(t *testing.T) {
	s := storeTest(t)
	input := filepath.Join(t.TempDir(), "storage-retry.json")
	if err := os.WriteFile(input, []byte(`{"schema_version":1,"busy_retries":2,"busy_retry_delay_ms":7,"busy_timeout_ms":123}`), 0600); err != nil {
		t.Fatal(err)
	}
	var stdout, stderr bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "storage-retry", "apply", "--input", input}, &stdout, &stderr); code != 0 {
		t.Fatalf("apply exit=%d stderr=%s", code, stderr.String())
	}
	var applied StorageRetryStatus
	if err := json.Unmarshal(stdout.Bytes(), &applied); err != nil {
		t.Fatal(err)
	}
	if !applied.Persisted || applied.Source != ".swarm/storage-retry.json" || applied.Effective.BusyTimeoutMS != 123 || applied.MaximumTotalWaitMS != 383 || applied.StorageFailureCause != "sqlite_busy" {
		t.Fatalf("unexpected applied status: %+v", applied)
	}
	lowerCLI := strings.ToLower(stdout.String())
	if strings.Contains(lowerCLI, "api_key") || strings.Contains(lowerCLI, "password") || strings.Contains(stdout.String(), "PRIVATE_MARKER") {
		t.Fatalf("CLI response contains a sensitive field or marker: %s", stdout.String())
	}

	stdout.Reset()
	stderr.Reset()
	if code := run([]string{"--root", s.root, "--json", "storage-retry", "show"}, &stdout, &stderr); code != 0 {
		t.Fatalf("show exit=%d stderr=%s", code, stderr.String())
	}
	var shown StorageRetryStatus
	if err := json.Unmarshal(stdout.Bytes(), &shown); err != nil || shown.Configured != applied.Configured || shown.Effective != applied.Effective {
		t.Fatalf("persisted readback mismatch: %+v err=%v", shown, err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	var busyTimeout int
	if err = reopened.db.QueryRow("PRAGMA busy_timeout").Scan(&busyTimeout); err != nil || busyTimeout != 123 {
		t.Fatalf("effective busy_timeout=%d err=%v", busyTimeout, err)
	}

	before, err := os.ReadFile(filepath.Join(s.root, ".swarm", "storage-retry.json"))
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(input, []byte(`{"schema_version":1,"busy_retries":11,"busy_retry_delay_ms":7,"busy_timeout_ms":123}`), 0600); err != nil {
		t.Fatal(err)
	}
	stdout.Reset()
	stderr.Reset()
	if code := run([]string{"--root", s.root, "--json", "storage-retry", "apply", "--input", input}, &stdout, &stderr); code != 2 {
		t.Fatalf("invalid config exit=%d stdout=%s stderr=%s", code, stdout.String(), stderr.String())
	}
	after, err := os.ReadFile(filepath.Join(s.root, ".swarm", "storage-retry.json"))
	if err != nil || !bytes.Equal(before, after) {
		t.Fatalf("invalid configuration changed persisted values: err=%v before=%s after=%s", err, before, after)
	}
}

func sqliteConfigRequest(h http.Handler, method string, body []byte) *httptest.ResponseRecorder {
	req := httptest.NewRequest(method, "http://local.test/api/v1/storage-retry", bytes.NewReader(body))
	req.Host = "local.test"
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "sqlite-config-token"})
	if method == http.MethodPost {
		req.Header.Set("Origin", "http://local.test")
		req.Header.Set("X-Swarm-CSRF", "sqlite-config-token")
	}
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, req)
	return rr
}

func TestSQLiteConfigHTTP(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "local.test", "sqlite-config-token")
	rr := sqliteConfigRequest(h, http.MethodGet, nil)
	if rr.Code != http.StatusOK {
		t.Fatalf("GET status=%d body=%s", rr.Code, rr.Body.String())
	}
	var initial StorageRetryStatus
	if err := json.Unmarshal(rr.Body.Bytes(), &initial); err != nil || initial.Source != "config/storage-retry.json" || initial.Effective.BusyTimeoutMS != legacyStorageBusyTimeoutMS {
		t.Fatalf("unexpected defaults: %+v err=%v", initial, err)
	}

	invalid := []byte(`{"schema_version":1,"busy_retries":1,"busy_retry_delay_ms":5,"busy_timeout_ms":0}`)
	rr = sqliteConfigRequest(h, http.MethodPost, invalid)
	if rr.Code != http.StatusBadRequest || !strings.Contains(rr.Body.String(), "busy_timeout_ms=1..60000") {
		t.Fatalf("invalid status=%d body=%s", rr.Code, rr.Body.String())
	}
	if _, err := os.Stat(filepath.Join(s.root, ".swarm", "storage-retry.json")); !os.IsNotExist(err) {
		t.Fatalf("invalid HTTP request persisted a file: %v", err)
	}

	valid := []byte(`{"schema_version":1,"busy_retries":4,"busy_retry_delay_ms":10,"busy_timeout_ms":250}`)
	rr = sqliteConfigRequest(h, http.MethodPost, valid)
	if rr.Code != http.StatusOK {
		t.Fatalf("POST status=%d body=%s", rr.Code, rr.Body.String())
	}
	var saved StorageRetryStatus
	if err := json.Unmarshal(rr.Body.Bytes(), &saved); err != nil || !saved.Persisted || saved.MaximumTotalWaitMS != 1290 || saved.Effective.BusyTimeoutMS != 250 {
		t.Fatalf("unexpected saved status: %+v err=%v", saved, err)
	}
	rr = sqliteConfigRequest(h, http.MethodGet, nil)
	var readback StorageRetryStatus
	if err := json.Unmarshal(rr.Body.Bytes(), &readback); err != nil || readback.Configured != saved.Configured || readback.Source != ".swarm/storage-retry.json" {
		t.Fatalf("HTTP persistence mismatch: %+v err=%v", readback, err)
	}
	if strings.Contains(strings.ToLower(rr.Body.String()), "password") || strings.Contains(strings.ToLower(rr.Body.String()), "api_key") {
		t.Fatalf("HTTP response exposed secret-shaped fields: %s", rr.Body.String())
	}
}
