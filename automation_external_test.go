//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

const automationC03Secret = "fixture-secret-with-at-least-thirty-two-bytes"

type automationC03HTTP struct {
	store   *Store
	work    Work
	server  *httptest.Server
	handler http.Handler
	host    string
	keyID   string
	secret  string
}

func writeAutomationC03Secrets(t *testing.T, s *Store, keys []automationExternalSecret) {
	t.Helper()
	raw, err := json.Marshal(automationExternalSecretFile{Schema: 1, Keys: keys})
	if err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(s.root, ".swarm", "automation-external-secrets.json")
	if err = os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	if err = os.Chmod(path, 0600); err != nil {
		t.Fatal(err)
	}
}

func automationC03Fixture(t *testing.T) automationC03HTTP {
	t.Helper()
	s, w := automationC01Fixture(t)
	keyID := "fixture-key-one"
	writeAutomationC03Secrets(t, s, []automationExternalSecret{{ID: keyID, Secret: automationC03Secret, Active: true, Targets: []string{w.ID}, Actions: []string{"request_resume"}}})
	listener, err := net.Listen("tcp4", "127.0.0.1:0")
	if err != nil {
		if os.Getenv("SWARM_C03_REQUIRE_TCP") == "1" {
			t.Fatalf("C03 actual TCP loopback required: %v", err)
		}
		t.Log("C03 transport=handler-only; TCP loopback unavailable")
		host := "127.0.0.1"
		return automationC03HTTP{store: s, work: w, handler: newWebHandler(s, host, "fixture-browser-token"), host: host, keyID: keyID, secret: automationC03Secret}
	}
	host := listener.Addr().String()
	handler := newWebHandler(s, host, "fixture-browser-token")
	server := &httptest.Server{Listener: listener, Config: &http.Server{Handler: handler}}
	server.Start()
	t.Log("C03 transport=tcp-loopback")
	t.Cleanup(server.Close)
	return automationC03HTTP{store: s, work: w, server: server, handler: handler, host: host, keyID: keyID, secret: automationC03Secret}
}

func automationC03Body(eventID, target string) []byte {
	raw, _ := json.Marshal(automationExternalEvent{Schema: 1, EventID: eventID, TargetWorkID: target, Action: "request_resume"})
	return raw
}

func (f automationC03HTTP) request(t *testing.T, body []byte, keyID, secret string, signedAt time.Time) (int, []byte) {
	t.Helper()
	timestamp := fmt.Sprintf("%d", signedAt.Unix())
	url := "http://" + f.host + automationExternalPath
	if f.server != nil {
		url = f.server.URL + automationExternalPath
	}
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		t.Fatal(err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Swarm-Key-ID", keyID)
	req.Header.Set("X-Swarm-Timestamp", timestamp)
	req.Header.Set("X-Swarm-Signature", automationExternalSignature(secret, timestamp, body))
	if f.server != nil {
		response, requestErr := f.server.Client().Do(req)
		if requestErr != nil {
			t.Fatal(requestErr)
		}
		defer response.Body.Close()
		raw, readErr := io.ReadAll(response.Body)
		if readErr != nil {
			t.Fatal(readErr)
		}
		return response.StatusCode, raw
	}
	req.RemoteAddr = "127.0.0.1:12345"
	recorder := httptest.NewRecorder()
	f.handler.ServeHTTP(recorder, req)
	response := recorder.Result()
	defer response.Body.Close()
	raw, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}
	return response.StatusCode, raw
}

func automationC03Count(t *testing.T, s *Store, table string) int {
	t.Helper()
	allowed := map[string]bool{"automation_requests": true, "automation_occurrences": true, "automation_effects": true, "automation_external_events": true, "automation_external_audit": true}
	if !allowed[table] {
		t.Fatal("table de test non autorisée")
	}
	var count int
	if err := s.db.QueryRow("SELECT count(*) FROM " + table).Scan(&count); err != nil {
		t.Fatal(err)
	}
	return count
}

func TestAutomationC03Signature(t *testing.T) {
	f := automationC03Fixture(t)
	nowAt := time.Now().UTC()
	valid := automationC03Body("signed-event", f.work.ID)
	status, raw := f.request(t, valid, f.keyID, f.secret, nowAt)
	if status != http.StatusAccepted || !bytes.Contains(raw, []byte(`"state":"received"`)) {
		t.Fatalf("événement signé non accepté : status=%d body=%s", status, raw)
	}
	bad := automationC03Body("bad-signature", f.work.ID)
	status, raw = f.request(t, bad, f.keyID, "wrong-fixture-secret-with-thirty-two-bytes", nowAt)
	if status != http.StatusUnauthorized || bytes.Contains(raw, bad) {
		t.Fatalf("mauvaise signature non refusée ou reflétée : status=%d body=%s", status, raw)
	}
	status, _ = f.request(t, automationC03Body("expired-event", f.work.ID), f.keyID, f.secret, nowAt.Add(-10*time.Minute))
	if status != http.StatusUnauthorized {
		t.Fatalf("timestamp expiré accepté : %d", status)
	}
	status, _ = f.request(t, automationC03Body("outside-scope", "other-target"), f.keyID, f.secret, nowAt)
	if status != http.StatusForbidden {
		t.Fatalf("cible hors portée acceptée : %d", status)
	}
	secretPath := filepath.Join(f.store.root, ".swarm", "automation-external-secrets.json")
	if err := os.Chmod(secretPath, 0644); err != nil {
		t.Fatal(err)
	}
	status, _ = f.request(t, automationC03Body("public-secret-file", f.work.ID), f.keyID, f.secret, nowAt)
	if status != http.StatusUnauthorized {
		t.Fatalf("fichier secret public accepté : %d", status)
	}
	if err := os.Chmod(secretPath, 0600); err != nil {
		t.Fatal(err)
	}
	if got := automationC03Count(t, f.store, "automation_requests"); got != 1 {
		t.Fatalf("rejets ont produit une demande : %d", got)
	}
	nonLoopbackBody := automationC03Body("remote-event", f.work.ID)
	timestamp := fmt.Sprintf("%d", nowAt.Unix())
	request := httptest.NewRequest(http.MethodPost, "http://"+f.host+automationExternalPath, bytes.NewReader(nonLoopbackBody))
	request.RemoteAddr = "192.0.2.10:12345"
	request.Header.Set("Content-Type", "application/json")
	request.Header.Set("X-Swarm-Key-ID", f.keyID)
	request.Header.Set("X-Swarm-Timestamp", timestamp)
	request.Header.Set("X-Swarm-Signature", automationExternalSignature(f.secret, timestamp, nonLoopbackBody))
	recorder := httptest.NewRecorder()
	f.handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusBadRequest {
		t.Fatalf("pair non-loopback accepté : %d", recorder.Code)
	}
	if got := automationC03Count(t, f.store, "automation_requests"); got != 1 {
		t.Fatalf("pair non-loopback a produit une demande : %d", got)
	}
	legacyRoot := t.TempDir()
	legacy, err := openStore(legacyRoot, true)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = legacy.db.Exec("DROP TABLE automation_external_audit; DROP TABLE automation_external_events; PRAGMA user_version=26"); err != nil {
		t.Fatal(err)
	}
	if err = legacy.db.Close(); err != nil {
		t.Fatal(err)
	}
	migrated, err := openStore(legacyRoot, false)
	if err != nil {
		t.Fatal(err)
	}
	defer migrated.db.Close()
	var version int
	if err = migrated.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != schemaVersion {
		t.Fatalf("migration C03 incomplète : version=%d err=%v", version, err)
	}
	if got := automationC03Count(t, migrated, "automation_external_events"); got != 0 {
		t.Fatalf("table C03 migrée non vide : %d", got)
	}
}

func TestAutomationC03ReplayConflict(t *testing.T) {
	f := automationC03Fixture(t)
	body := automationC03Body("stable-event", f.work.ID)
	status, _ := f.request(t, body, f.keyID, f.secret, time.Now())
	if status != http.StatusAccepted {
		t.Fatalf("première réception : %d", status)
	}
	reopened, err := openStore(f.store.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	if f.server != nil {
		listener, listenErr := net.Listen("tcp4", "127.0.0.1:0")
		if listenErr != nil {
			t.Fatal(listenErr)
		}
		host := listener.Addr().String()
		handler := newWebHandler(reopened, host, "other-browser-token")
		otherServer := &httptest.Server{Listener: listener, Config: &http.Server{Handler: handler}}
		otherServer.Start()
		defer otherServer.Close()
		f.server, f.handler, f.host = otherServer, handler, host
	} else {
		f.handler = newWebHandler(reopened, f.host, "other-browser-token")
	}
	status, raw := f.request(t, body, f.keyID, f.secret, time.Now())
	if status != http.StatusOK || !bytes.Contains(raw, []byte(`"replayed":true`)) {
		t.Fatalf("rejeu exact après réouverture : status=%d body=%s", status, raw)
	}
	changed := append(append([]byte{}, body...), ' ')
	status, _ = f.request(t, changed, f.keyID, f.secret, time.Now())
	if status != http.StatusConflict {
		t.Fatalf("même identité/corps exact différent non conflictuel : %d", status)
	}
	if got := automationC03Count(t, reopened, "automation_requests"); got != 1 {
		t.Fatalf("rejeu/conflit a dupliqué la demande : %d", got)
	}
}

func TestAutomationC03Revocation(t *testing.T) {
	f := automationC03Fixture(t)
	body := automationC03Body("revoked-before-claim", f.work.ID)
	status, _ := f.request(t, body, f.keyID, f.secret, time.Now())
	if status != http.StatusAccepted {
		t.Fatal(status)
	}
	var requestID string
	if err := f.store.db.QueryRow("SELECT request_id FROM automation_external_events WHERE event_id='revoked-before-claim'").Scan(&requestID); err != nil {
		t.Fatal(err)
	}
	writeAutomationC03Secrets(t, f.store, []automationExternalSecret{{ID: f.keyID, Secret: f.secret, Active: false, Targets: []string{f.work.ID}, Actions: []string{"request_resume"}}})
	status, _ = f.request(t, automationC03Body("revoked-at-reception", f.work.ID), f.keyID, f.secret, time.Now())
	if status != http.StatusUnauthorized {
		t.Fatalf("clé révoquée acceptée à la réception : %d", status)
	}
	record, err := f.store.processAutomationRequest(requestID, "revocation-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || record.State != "rejected" || record.Reason != "external_key_revoked" {
		t.Fatalf("révocation non revalidée au claim : %+v err=%v", record, err)
	}
	if got := automationC03Count(t, f.store, "automation_effects"); got != 0 {
		t.Fatalf("révocation a produit un effet : %d", got)
	}

	newSecret := "rotated-fixture-secret-with-thirty-two-bytes"
	writeAutomationC03Secrets(t, f.store, []automationExternalSecret{
		{ID: f.keyID, Secret: f.secret, Active: false, Targets: []string{f.work.ID}, Actions: []string{"request_resume"}},
		{ID: "fixture-key-two", Secret: newSecret, Active: true, Targets: []string{f.work.ID}, Actions: []string{"request_resume"}},
	})
	status, _ = f.request(t, automationC03Body("rotated-key-event", f.work.ID), "fixture-key-two", newSecret, time.Now())
	if status != http.StatusAccepted {
		t.Fatalf("clé tournée active refusée : %d", status)
	}
	if err := f.store.db.QueryRow("SELECT request_id FROM automation_external_events WHERE event_id='rotated-key-event'").Scan(&requestID); err != nil {
		t.Fatal(err)
	}
	claimed, owned, err := f.store.claimAutomationRequest(requestID, "effect-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || !owned || claimed.State != "claimed" {
		t.Fatalf("claim préalable impossible : %+v owned=%v err=%v", claimed, owned, err)
	}
	writeAutomationC03Secrets(t, f.store, []automationExternalSecret{{ID: "fixture-key-two", Secret: newSecret, Active: false, Targets: []string{f.work.ID}, Actions: []string{"request_resume"}}})
	if _, err = f.store.applyAutomationResumeEffect(requestID, "effect-driver"); err == nil || commandFailure(err).Code != "external_key_revoked" {
		t.Fatalf("révocation entre claim et effet non appliquée : %v", err)
	}
	got, err := f.store.automationRequest(requestID)
	if err != nil || got.State != "rejected" || got.Reason != "external_key_revoked" {
		t.Fatalf("rejet avant effet non durable : %+v err=%v", got, err)
	}
	if effects := automationC03Count(t, f.store, "automation_effects"); effects != 0 {
		t.Fatalf("révocation tardive a produit un effet : %d", effects)
	}
}

func TestAutomationC03PayloadBounds(t *testing.T) {
	f := automationC03Fixture(t)
	config, err := f.store.automationConfig()
	if err != nil {
		t.Fatal(err)
	}
	config.Values.ExternalMaxPayloadBytes = 256
	rawConfig, _ := json.Marshal(config)
	if err = os.WriteFile(filepath.Join(f.store.root, ".swarm", "automation.json"), rawConfig, 0600); err != nil {
		t.Fatal(err)
	}
	large := []byte(`{"schema_version":1,"event_id":"large-event","target_work_id":"` + f.work.ID + `","action":"request_resume","padding":"` + strings.Repeat("x", 300) + `"}`)
	status, _ := f.request(t, large, f.keyID, f.secret, time.Now())
	if status != http.StatusBadRequest {
		t.Fatalf("payload surdimensionné accepté : %d", status)
	}
	foreign := []byte(`{"schema_version":1,"event_id":"foreign-field","target_work_id":"` + f.work.ID + `","action":"request_resume","command":"do-not-log-this-marker"}`)
	status, _ = f.request(t, foreign, f.keyID, f.secret, time.Now())
	if status != http.StatusBadRequest {
		t.Fatalf("champ commande étranger accepté : %d", status)
	}
	writeAutomationC03Secrets(t, f.store, []automationExternalSecret{{ID: f.keyID, Secret: f.secret, Active: true, Targets: []string{f.work.ID, "unknown-target"}, Actions: []string{"request_resume"}}})
	status, _ = f.request(t, automationC03Body("unknown-event", "unknown-target"), f.keyID, f.secret, time.Now())
	if status != http.StatusNotFound {
		t.Fatalf("cible inconnue non refusée : %d", status)
	}
	if got := automationC03Count(t, f.store, "automation_requests"); got != 0 {
		t.Fatalf("entrées invalides ont produit une demande : %d", got)
	}
}

func TestAutomationC03RateLimit(t *testing.T) {
	f := automationC03Fixture(t)
	config, err := f.store.automationConfig()
	if err != nil {
		t.Fatal(err)
	}
	config.Values.ExternalRateLimit = 2
	config.Values.ExternalRateWindowSeconds = 60
	rawConfig, _ := json.Marshal(config)
	if err = os.WriteFile(filepath.Join(f.store.root, ".swarm", "automation.json"), rawConfig, 0600); err != nil {
		t.Fatal(err)
	}
	for index := 0; index < 3; index++ {
		status, _ := f.request(t, automationC03Body(fmt.Sprintf("rate-event-%d", index), f.work.ID), f.keyID, f.secret, time.Now())
		want := http.StatusAccepted
		if index == 2 {
			want = http.StatusTooManyRequests
		}
		if status != want {
			t.Fatalf("requête %d : status=%d want=%d", index, status, want)
		}
	}
	if got := automationC03Count(t, f.store, "automation_requests"); got != 2 {
		t.Fatalf("limite de fréquence a produit %d demandes", got)
	}
}

func TestAutomationC03Redaction(t *testing.T) {
	f := automationC03Fixture(t)
	marker := "payload-sensitive-marker"
	body := []byte(`{"schema_version":1,"event_id":"redaction-event","target_work_id":"` + f.work.ID + `","action":"request_resume","unexpected":"` + marker + `"}`)
	status, response := f.request(t, body, f.keyID, f.secret, time.Now())
	if status != http.StatusBadRequest || bytes.Contains(response, []byte(marker)) || bytes.Contains(response, []byte(f.secret)) {
		t.Fatalf("erreur non expurgée : status=%d body=%s", status, response)
	}
	rows, err := f.store.db.Query(`SELECT event_fingerprint,key_fingerprint,request_id,state,reason FROM automation_external_audit
 UNION ALL SELECT event_id,body_digest,request_id,state,reason FROM automation_external_events`)
	if err != nil {
		t.Fatal(err)
	}
	defer rows.Close()
	for rows.Next() {
		var values [5]string
		if err = rows.Scan(&values[0], &values[1], &values[2], &values[3], &values[4]); err != nil {
			t.Fatal(err)
		}
		joined := strings.Join(values[:], "|")
		if strings.Contains(joined, marker) || strings.Contains(joined, f.secret) {
			t.Fatalf("journal contient une donnée sensible : %q", joined)
		}
	}
	if err = rows.Err(); err != nil {
		t.Fatal(err)
	}
}
