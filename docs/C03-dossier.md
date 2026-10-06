# Dossier de revue — C03 événements externes authentifiés et bornés

## Identité, effet et limites

- Mission / tâche / tentative / producteur : `w-0e48458a2b6eb75532fa740d` / `C03` / `a-9dbf00d14ce73fe9df22f87f` / `auto-f2ad1082e74aabec9b07`.
- Exigence : `req-3`.
- Base observée : `cc3069dc7bb61b90168d21f945cb2eb5e27578ed`, branche `codex/clear-launch-recovery`, plus checkout sale partagé protégé.
- Effet : une requête HTTP loopback signée sur ses octets exacts produit au plus une demande C01 durable et bornée, sans commande, fournisseur, budget ou permission dérivé du payload.
- Autorité : tables SQLite v27 pour rejeu/journal et demandes C01 ; fichier privé local pour secrets et portées ; politique opérationnelle versionnée dans `config/automation.json`.
- Limites : la sandbox de cette tentative refuse l'ouverture d'un socket loopback ; les tests utilisent automatiquement un vrai serveur TCP quand permis et le même handler sans socket sinon. Le vrai TCP hôte et la revue indépendante restent requis.

## Matrice de revue

| Critère | Garde / scénario | Preuve worker | État |
| --- | --- | --- | --- |
| HMAC / timestamp / scope | corps exact, fenêtre ±300 s, clé/cible/action courantes | `TestAutomationC03Signature` | PASS handler ; TCP hôte dû |
| Rejeu / conflit | identité stable, réouverture Store, corps différent | `TestAutomationC03ReplayConflict` | PASS |
| Rotation / révocation | réception, claim et avant effet | `TestAutomationC03Revocation` | PASS |
| Taille / champs / cible | limite configurable, JSON strict, cible connue | `TestAutomationC03PayloadBounds` | PASS |
| Fréquence | clé empreintée, fenêtre et seuil configurables | `TestAutomationC03RateLimit` | PASS |
| Expurgation | réponse et tables sans secret/corps/marqueur | `TestAutomationC03Redaction` | PASS |
| Concurrence | mêmes six tests sous détecteur de course | `go test -race` ciblé | PASS |
| Revue indépendante | contexte séparé sur mêmes entrées | moteur / responsable | NOT TESTED |

## Service complet — automation_external.go

```go
package main

import (
        "crypto/hmac"
        "crypto/sha256"
        "database/sql"
        "encoding/hex"
        "encoding/json"
        "errors"
        "io"
        "net"
        "net/http"
        "os"
        "path/filepath"
        "strconv"
        "strings"
        "time"
)

const automationExternalPath = "/api/v1/automation/external"

const automationExternalMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS automation_external_events(
 event_id TEXT PRIMARY KEY,
 body_digest TEXT NOT NULL,
 request_id TEXT NOT NULL,
 key_fingerprint TEXT NOT NULL,
 received_at TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('received','accepted','rejected')),
 reason TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS automation_external_audit(
 audit_id TEXT PRIMARY KEY,
 event_fingerprint TEXT NOT NULL,
 key_fingerprint TEXT NOT NULL,
 request_id TEXT NOT NULL,
 received_at TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('admitted','accepted','rejected')),
 reason TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS automation_external_audit_rate ON automation_external_audit(key_fingerprint,received_at);
PRAGMA user_version=27;
COMMIT;`

type automationExternalEvent struct {
        Schema       int    `json:"schema_version"`
        EventID      string `json:"event_id"`
        TargetWorkID string `json:"target_work_id"`
        Action       string `json:"action"`
}

type automationExternalSecretFile struct {
        Schema int                        `json:"schema_version"`
        Keys   []automationExternalSecret `json:"keys"`
}

type automationExternalSecret struct {
        ID      string   `json:"id"`
        Secret  string   `json:"secret"`
        Active  bool     `json:"active"`
        Targets []string `json:"targets"`
        Actions []string `json:"actions"`
}

type automationExternalStored struct {
        BodyDigest string
        RequestID  string
        State      string
        Reason     string
}

func automationExternalDigest(value string) string {
        sum := sha256.Sum256([]byte(value))
        return hex.EncodeToString(sum[:])
}

func automationExternalBodyDigest(body []byte) string {
        sum := sha256.Sum256(body)
        return hex.EncodeToString(sum[:])
}

func automationExternalSignature(secret, timestamp string, body []byte) string {
        mac := hmac.New(sha256.New, []byte(secret))
        _, _ = mac.Write([]byte(timestamp))
        _, _ = mac.Write([]byte("\n"))
        _, _ = mac.Write(body)
        return "sha256=" + hex.EncodeToString(mac.Sum(nil))
}

func (s *Store) automationExternalSecrets() (automationExternalSecretFile, error) {
        var file automationExternalSecretFile
        path := filepath.Join(s.root, ".swarm", "automation-external-secrets.json")
        info, err := os.Lstat(path)
        if err != nil {
                return file, &CommandError{Code: "external_auth_unavailable", Message: "authentification externe indisponible"}
        }
        if !info.Mode().IsRegular() || info.Mode()&os.ModeSymlink != 0 || info.Mode().Perm()&0077 != 0 || info.Size() > 1048576 {
                return file, &CommandError{Code: "external_auth_unavailable", Message: "authentification externe indisponible"}
        }
        opened, err := os.Open(path)
        if err != nil {
                return file, &CommandError{Code: "external_auth_unavailable", Message: "authentification externe indisponible"}
        }
        defer opened.Close()
        openedInfo, err := opened.Stat()
        if err != nil || !os.SameFile(info, openedInfo) || !openedInfo.Mode().IsRegular() || openedInfo.Mode().Perm()&0077 != 0 || openedInfo.Size() > 1048576 {
                return file, &CommandError{Code: "external_auth_unavailable", Message: "authentification externe indisponible"}
        }
        raw, err := io.ReadAll(io.LimitReader(opened, 1048577))
        if err != nil || strict(raw, &file) != nil || file.Schema != 1 || len(file.Keys) == 0 || len(file.Keys) > 100 {
                return automationExternalSecretFile{}, &CommandError{Code: "external_auth_unavailable", Message: "authentification externe indisponible"}
        }
        seen := map[string]bool{}
        for _, key := range file.Keys {
                if !safeName(key.ID) || seen[key.ID] || len([]byte(key.Secret)) < 32 || len(key.Targets) == 0 || len(key.Targets) > 100 || len(key.Actions) == 0 || len(key.Actions) > 10 {
                        return automationExternalSecretFile{}, &CommandError{Code: "external_auth_unavailable", Message: "authentification externe indisponible"}
                }
                seen[key.ID] = true
                for _, target := range key.Targets {
                        if !safeName(target) {
                                return automationExternalSecretFile{}, &CommandError{Code: "external_auth_unavailable", Message: "authentification externe indisponible"}
                        }
                }
                for _, action := range key.Actions {
                        if action != "request_resume" {
                                return automationExternalSecretFile{}, &CommandError{Code: "external_auth_unavailable", Message: "authentification externe indisponible"}
                        }
                }
        }
        return file, nil
}

func externalContains(values []string, wanted string) bool {
        for _, value := range values {
                if value == wanted {
                        return true
                }
        }
        return false
}

func (s *Store) automationExternalKey(id string) (automationExternalSecret, error) {
        file, err := s.automationExternalSecrets()
        if err != nil {
                return automationExternalSecret{}, err
        }
        for _, key := range file.Keys {
                if key.ID == id && key.Active {
                        return key, nil
                }
        }
        return automationExternalSecret{}, &CommandError{Code: "external_auth_failed", Message: "authentification externe refusée"}
}

func (s *Store) automationExternalAuthorized(source, target, action string) (bool, string, error) {
        if !strings.HasPrefix(source, "external:") {
                return true, "", nil
        }
        key, err := s.automationExternalKey(strings.TrimPrefix(source, "external:"))
        if err != nil {
                return false, "external_key_revoked", nil
        }
        if !externalContains(key.Targets, target) || !externalContains(key.Actions, action) {
                return false, "external_scope_revoked", nil
        }
        return true, "", nil
}

func validateAutomationExternalEvent(event automationExternalEvent) error {
        if event.Schema != 1 || !safeName(event.EventID) || !safeName(event.TargetWorkID) || event.Action != "request_resume" {
                return &CommandError{Code: "invalid_external_event", Message: "événement externe invalide"}
        }
        return nil
}

func externalRemoteLoopback(remote string) bool {
        host, _, err := net.SplitHostPort(remote)
        if err != nil {
                return false
        }
        ip := net.ParseIP(host)
        return ip != nil && ip.IsLoopback()
}

func (s *Store) admitAutomationExternal(event automationExternalEvent, bodyDigest, keyID string, at time.Time, config AutomationConfigValues) (automationExternalStored, bool, error) {
        var stored automationExternalStored
        tx, err := s.db.Begin()
        if err != nil {
                return stored, false, err
        }
        defer tx.Rollback()
        err = tx.QueryRow("SELECT body_digest,request_id,state,reason FROM automation_external_events WHERE event_id=?", event.EventID).Scan(&stored.BodyDigest, &stored.RequestID, &stored.State, &stored.Reason)
        if err == nil {
                if stored.BodyDigest != bodyDigest {
                        return stored, false, &CommandError{Code: "idempotency_conflict", Message: "identité externe déjà utilisée"}
                }
                return stored, false, nil
        }
        if !errors.Is(err, sql.ErrNoRows) {
                return stored, false, err
        }
        keyFingerprint := automationExternalDigest(keyID)
        cutoff := at.Add(-time.Duration(config.ExternalRateWindowSeconds) * time.Second).UTC().Format(time.RFC3339Nano)
        var count int
        if err = tx.QueryRow("SELECT count(*) FROM automation_external_audit WHERE key_fingerprint=? AND received_at>=?", keyFingerprint, cutoff).Scan(&count); err != nil {
                return stored, false, err
        }
        if count >= config.ExternalRateLimit {
                return stored, false, &CommandError{Code: "external_rate_limited", Message: "fréquence externe dépassée"}
        }
        stamp := at.UTC().Format(time.RFC3339Nano)
        requestID := "request-" + hash([]byte("external-" + event.EventID))[:24]
        if _, err = tx.Exec(`INSERT INTO automation_external_audit(audit_id,event_fingerprint,key_fingerprint,request_id,received_at,state,reason)
 VALUES(?,?,?,?,?,'admitted','')`, newID("external-audit-"), automationExternalDigest(event.EventID), keyFingerprint, requestID, stamp); err != nil {
                return stored, false, err
        }
        if _, err = tx.Exec(`INSERT INTO automation_external_events(event_id,body_digest,request_id,key_fingerprint,received_at,state,reason)
 VALUES(?,?,?,?,?,'received','')`, event.EventID, bodyDigest, requestID, keyFingerprint, stamp); err != nil {
                return stored, false, err
        }
        if err = tx.Commit(); err != nil {
                return stored, false, err
        }
        return automationExternalStored{BodyDigest: bodyDigest, RequestID: requestID, State: "received"}, true, nil
}

func (s *Store) finishAutomationExternal(eventID, requestID, state, reason string) error {
        tx, err := s.db.Begin()
        if err != nil {
                return err
        }
        defer tx.Rollback()
        if _, err = tx.Exec("UPDATE automation_external_events SET request_id=?,state=?,reason=? WHERE event_id=?", requestID, state, reason, eventID); err != nil {
                return err
        }
        if _, err = tx.Exec("UPDATE automation_external_audit SET request_id=?,state=?,reason=? WHERE request_id=? AND state='admitted'", requestID, state, reason, requestID); err != nil {
                return err
        }
        return tx.Commit()
}

func automationExternalStatus(code string) int {
        switch code {
        case "external_auth_failed", "external_auth_unavailable", "external_timestamp_invalid", "external_timestamp_expired":
                return http.StatusUnauthorized
        case "external_scope_denied", "authorization_required":
                return http.StatusForbidden
        case "idempotency_conflict":
                return http.StatusConflict
        case "external_rate_limited":
                return http.StatusTooManyRequests
        case "unknown_target":
                return http.StatusNotFound
        default:
                return http.StatusBadRequest
        }
}

func automationExternalFailure(w http.ResponseWriter, err error) {
        failure := commandFailure(err)
        w.Header().Set("Content-Type", "application/json; charset=utf-8")
        w.WriteHeader(automationExternalStatus(failure.Code))
        _ = json.NewEncoder(w).Encode(map[string]any{"failure": failure})
}

func (s *Store) automationExternalHTTP(clock func() time.Time) http.HandlerFunc {
        return func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        w.Header().Set("Allow", http.MethodPost)
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                if mediaType := strings.TrimSpace(strings.Split(r.Header.Get("Content-Type"), ";")[0]); mediaType != "application/json" {
                        automationExternalFailure(w, &CommandError{Code: "invalid_external_event", Message: "événement externe invalide"})
                        return
                }
                if !externalRemoteLoopback(r.RemoteAddr) {
                        automationExternalFailure(w, &CommandError{Code: "external_loopback_required", Message: "réception externe locale uniquement"})
                        return
                }
                config, err := s.automationConfig()
                if err != nil {
                        automationExternalFailure(w, &CommandError{Code: "external_policy_unavailable", Message: "politique externe indisponible"})
                        return
                }
                limit := int64(config.Values.ExternalMaxPayloadBytes)
                if r.ContentLength > limit {
                        automationExternalFailure(w, &CommandError{Code: "external_payload_too_large", Message: "événement externe trop volumineux"})
                        return
                }
                body, err := io.ReadAll(io.LimitReader(r.Body, limit+1))
                if err != nil || int64(len(body)) > limit {
                        automationExternalFailure(w, &CommandError{Code: "external_payload_too_large", Message: "événement externe trop volumineux"})
                        return
                }
                keyID := r.Header.Get("X-Swarm-Key-ID")
                timestamp := r.Header.Get("X-Swarm-Timestamp")
                provided := r.Header.Get("X-Swarm-Signature")
                key, err := s.automationExternalKey(keyID)
                if err != nil {
                        automationExternalFailure(w, err)
                        return
                }
                seconds, err := strconv.ParseInt(timestamp, 10, 64)
                if err != nil {
                        automationExternalFailure(w, &CommandError{Code: "external_timestamp_invalid", Message: "horodatage externe invalide"})
                        return
                }
                at := clock().UTC()
                signedAt := time.Unix(seconds, 0).UTC()
                window := time.Duration(config.Values.ExternalTimestampWindowSeconds) * time.Second
                if signedAt.Before(at.Add(-window)) || signedAt.After(at.Add(window)) {
                        automationExternalFailure(w, &CommandError{Code: "external_timestamp_expired", Message: "horodatage externe hors fenêtre"})
                        return
                }
                expected := automationExternalSignature(key.Secret, timestamp, body)
                if !hmac.Equal([]byte(expected), []byte(provided)) {
                        automationExternalFailure(w, &CommandError{Code: "external_auth_failed", Message: "authentification externe refusée"})
                        return
                }
                var event automationExternalEvent
                if err = strict(body, &event); err != nil || validateAutomationExternalEvent(event) != nil {
                        automationExternalFailure(w, &CommandError{Code: "invalid_external_event", Message: "événement externe invalide"})
                        return
                }
                if !externalContains(key.Targets, event.TargetWorkID) || !externalContains(key.Actions, event.Action) {
                        automationExternalFailure(w, &CommandError{Code: "external_scope_denied", Message: "portée externe refusée"})
                        return
                }
                bodyDigest := automationExternalBodyDigest(body)
                stored, fresh, err := s.admitAutomationExternal(event, bodyDigest, key.ID, at, config.Values)
                if err != nil {
                        automationExternalFailure(w, err)
                        return
                }
                if !fresh {
                        if stored.State == "rejected" {
                                automationExternalFailure(w, &CommandError{Code: stored.Reason, Message: "événement externe précédemment refusé"})
                                return
                        }
                        if existing, lookupErr := s.automationRequest(stored.RequestID); lookupErr == nil {
                                w.Header().Set("Content-Type", "application/json; charset=utf-8")
                                _ = json.NewEncoder(w).Encode(map[string]any{"event_id": event.EventID, "request": existing, "replayed": true})
                                return
                        }
                }
                request := AutomationResumeRequest{Schema: 1, IdempotencyKey: "external-" + event.EventID, Source: "external:" + key.ID, TargetWorkID: event.TargetWorkID, Action: event.Action}
                request.ContentDigest = automationResumeDigest(request)
                record, _, err := s.submitAutomationResume(request)
                if err != nil {
                        failure := commandFailure(err)
                        _ = s.finishAutomationExternal(event.EventID, stored.RequestID, "rejected", failure.Code)
                        automationExternalFailure(w, &CommandError{Code: failure.Code, Message: "événement externe refusé"})
                        return
                }
                if err = s.finishAutomationExternal(event.EventID, record.RequestID, "accepted", ""); err != nil {
                        automationExternalFailure(w, &CommandError{Code: "storage_error", Message: "journal externe indisponible"})
                        return
                }
                w.Header().Set("Content-Type", "application/json; charset=utf-8")
                w.WriteHeader(http.StatusAccepted)
                _ = json.NewEncoder(w).Encode(map[string]any{"event_id": event.EventID, "request": record, "replayed": false})
        }
}

func registerAutomationExternalHTTP(s *Store, mux *http.ServeMux, clock func() time.Time) {
        mux.Handle(automationExternalPath, s.automationExternalHTTP(clock))
}
```

## Tests critiques complets — automation_external_test.go

```go
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
```

## Raccord complet au claim C01

```go
func (s *Store) claimAutomationRequest(id, conductor string, at time.Time, config AutomationRequestConfig) (AutomationRequestRecord, bool, error) {
        var zero AutomationRequestRecord
        if !safeName(id) || !safeName(conductor) {
                return zero, false, &CommandError{Code: "invalid_input", Message: "identités de demande et conducteur invalides"}
        }
        if err := config.validate(); err != nil {
                return zero, false, err
        }
        tx, err := s.db.Begin()
        if err != nil {
                return zero, false, err
        }
        defer tx.Rollback()
        if _, err = tx.Exec("UPDATE automation_requests SET revision=revision WHERE request_id=?", id); err != nil {
                return zero, false, err
        }
        record, err := scanAutomationRequest(tx.QueryRow(automationRequestSelect+" WHERE r.request_id=?", id))
        if err != nil {
                return zero, false, err
        }
        if record.State == "executed" || record.State == "rejected" || record.State == "uncertain_effect" {
                return record, false, nil
        }
        if record.State == "claimed" && record.ClaimedBy != conductor {
                until, parseErr := time.Parse(time.RFC3339Nano, record.LeaseUntil)
                if parseErr == nil && at.Before(until) {
                        return record, false, nil
                }
        }
        if authorized, code, externalErr := s.automationExternalAuthorized(record.Source, record.TargetWorkID, record.Action); externalErr != nil {
                return zero, false, externalErr
        } else if !authorized {
                if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
                        return zero, false, err
                }
                if err = tx.Commit(); err != nil {
                        return zero, false, err
                }
                got, readErr := s.automationRequest(id)
                return got, false, readErr
        }
        authorized, code, err := automationPolicyTx(tx, record.TargetWorkID)
        if err != nil {
                return zero, false, err
        }
        if !authorized {
                if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
                        return zero, false, err
                }
                if err = tx.Commit(); err != nil {
                        return zero, false, err
                }
                got, readErr := s.automationRequest(id)
                return got, false, readErr
        }
        busy, err := s.automationWorkspaceBusyTx(tx, record.TargetWorkID)
        if err != nil {
                return zero, false, err
        }
        if busy {
                if err = updateAutomationStateTx(tx, id, "waiting", "workspace_wait", conductor, "retry_after_resource_release"); err != nil {
                        return zero, false, err
                }
                if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", id); err != nil {
                        return zero, false, err
                }
                if err = tx.Commit(); err != nil {
                        return zero, false, err
                }
                got, readErr := s.automationRequest(id)
                return got, false, readErr
        }
        lease := at.Add(time.Duration(config.ClaimLeaseSeconds) * time.Second).UTC().Format(time.RFC3339Nano)
        if err = updateAutomationStateTx(tx, id, "claimed", "", conductor, "apply_request_resume"); err != nil {
                return zero, false, err
        }
        result, err := tx.Exec(`UPDATE automation_occurrences SET claimed_by=?,lease_until=? WHERE request_id=? AND (claimed_by='' OR claimed_by=? OR lease_until<=?)`, conductor, lease, id, conductor, at.UTC().Format(time.RFC3339Nano))
        if err != nil {
                return zero, false, err
        }
        if n, _ := result.RowsAffected(); n != 1 {
                return record, false, nil
        }
        if err = tx.Commit(); err != nil {
                return zero, false, err
        }
        got, err := s.automationRequest(id)
        return got, err == nil, err
}
```

## Raccord complet immédiatement avant effet C01

```go
func (s *Store) applyAutomationResumeEffect(id, conductor string) (string, error) {
        tx, err := s.db.Begin()
        if err != nil {
                return "", err
        }
        defer tx.Rollback()
        if _, err = tx.Exec("UPDATE automation_requests SET revision=revision WHERE request_id=?", id); err != nil {
                return "", err
        }
        record, err := scanAutomationRequest(tx.QueryRow(automationRequestSelect+" WHERE r.request_id=?", id))
        if err != nil {
                return "", err
        }
        effectID := "effect-" + hash([]byte(id + "|request_resume"))[:24]
        var existing string
        if err = tx.QueryRow("SELECT effect_id FROM automation_effects WHERE request_id=?", id).Scan(&existing); err == nil {
                return existing, nil
        } else if !errors.Is(err, sql.ErrNoRows) {
                return "", err
        }
        if record.State != "claimed" || record.ClaimedBy != conductor {
                return "", &CommandError{Code: "stale_lease", Message: "bail de demande absent ou remplacé"}
        }
        until, parseErr := time.Parse(time.RFC3339Nano, record.LeaseUntil)
        if parseErr != nil || !time.Now().Before(until) {
                return "", &CommandError{Code: "stale_lease", Message: "bail de demande expiré"}
        }
        if authorized, code, externalErr := s.automationExternalAuthorized(record.Source, record.TargetWorkID, record.Action); externalErr != nil {
                return "", externalErr
        } else if !authorized {
                if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
                        return "", err
                }
                if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", id); err != nil {
                        return "", err
                }
                if err = tx.Commit(); err != nil {
                        return "", err
                }
                return "", &CommandError{Code: code, Message: "portée externe retirée avant effet"}
        }
        authorized, code, err := automationPolicyTx(tx, record.TargetWorkID)
        if err != nil {
                return "", err
        }
        if !authorized {
                if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
                        return "", err
                }
                if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", id); err != nil {
                        return "", err
                }
                if err = tx.Commit(); err != nil {
                        return "", err
                }
                return "", &CommandError{Code: code, Message: "autorisation ou condition retirée avant effet"}
        }
        if _, err = tx.Exec("INSERT INTO automation_effects(effect_id,request_id,target_work_id,action,created_at) VALUES(?,?,?,?,?)", effectID, id, record.TargetWorkID, record.Action, now()); err != nil {
                return "", err
        }
        message, _ := json.Marshal(map[string]string{"request_id": id, "effect_id": effectID, "action": record.Action, "source": record.Source})
        if _, err = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", record.TargetWorkID, now(), "automation-request", string(message)); err != nil {
                return "", err
        }
        if err = tx.Commit(); err != nil {
                return "", err
        }
        return effectID, nil
}
```

## Raccords migration et route

```go
                        return fail(e)
                }
        }
        if version < 26 {
                if version != 0 {
                        if _, e = db.Exec("VACUUM INTO ?", filepath.Join(dir, newID("state-pre-v26-")+".db")); e != nil {
                                return fail(e)
                        }
                }
                if _, e = db.Exec(automationScheduleMigration); e != nil {
                        return fail(e)
                }
        }
        if version < 27 {
                if version != 0 {
                        if _, e = db.Exec("VACUUM INTO ?", filepath.Join(dir, newID("state-pre-v27-")+".db")); e != nil {
                                return fail(e)
                        }
                }
                if _, e = db.Exec(automationExternalMigration); e != nil {
                        return fail(e)
                }
        }
        if e = os.Chmod(path, 0600); e != nil {
                return fail(e)
        }
        return s, nil
}

// Les réglages d'autonomie s'ajoutent à une table existante : selon la version
// d'origine, les colonnes peuvent déjà être présentes. Vérifier avant d'ajouter,
                        status = http.StatusNotFound
                }
                if code == "storage_unavailable" {
                        status = http.StatusInsufficientStorage
                }
                w.WriteHeader(status)
                send(w, map[string]any{"error": commandFailure(e).Message, "failure": commandFailure(e)})
        }
        s.registerPlanning(mux, send, fail)
        s.registerProviderAdmin(mux, send, fail)
        s.registerRunLimitsAdmin(mux, send, fail)
        registerGraphDraftHTTP(s, mux, send, fail)
        registerAutomationExternalHTTP(s, mux, time.Now)
        s.registerPreparations(mux)
        s.registerTerminals(mux, send, fail)
        mux.HandleFunc("/api/v1/runtime-health", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != "GET" {
                        http.Error(w, "GET requis", 405)
                        return
                }
                send(w, s.runtimeHealth())
        })
        mux.HandleFunc("/api/v1/works", func(w http.ResponseWriter, r *http.Request) {
                v, e := s.list()
                if e != nil {
                        fail(w, e)
                w.Header().Set("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'")
                if r.Host != host {
                        http.Error(w, "Hôte refusé", 403)
                        return
                }
                if r.URL.Path == automationExternalPath {
                        mux.ServeHTTP(w, r)
                        return
                }
                if strings.HasPrefix(r.URL.Path, "/session/") {
                        if r.Method != "GET" || !same(strings.TrimPrefix(r.URL.Path, "/session/"), token) {
                                http.Error(w, "Session refusée", 403)
                                return
                        }
                        http.SetCookie(w, &http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token, Path: "/", HttpOnly: true, SameSite: http.SameSiteStrictMode})
                        target := "/"
                        if r.URL.Query().Get("view") == "prepare" {
package main

import (
        "crypto/rand"
        "encoding/hex"
        "encoding/json"
        "fmt"
        "os"
        "path/filepath"
        "strings"
        "time"
)

const schemaVersion = 27

type ManualOverride struct {
        Reason         string `json:"reason"`
        Actor          string `json:"actor"`
```

## Politique versionnée réelle

```json
{
  "schema_version": 1,
  "revision": 0,
  "values": {
    "claim_lease_seconds": 30,
    "due_grace_seconds": 60,
    "max_preview_occurrences": 20,
    "max_schedule_occurrences": 366,
    "external_max_payload_bytes": 65536,
    "external_rate_limit": 60,
    "external_rate_window_seconds": 60,
    "external_timestamp_window_seconds": 300
  },
  "history": []
}
```

## Guide opérateur français réel

## Événements externes C03

La route `POST /api/v1/automation/external` est uniquement joignable par le
serveur web loopback existant. Elle reçoit un document JSON borné contenant
exactement `schema_version`, `event_id`, `target_work_id` et l'action unique
`request_resume`. Les en-têtes `X-Swarm-Key-ID`, `X-Swarm-Timestamp` et
`X-Swarm-Signature` sont obligatoires. La signature vaut
`sha256=HMAC-SHA256(secret, timestamp + "\n" + corps_exact)` : changer les
espaces du JSON change donc le contenu signé et provoque un conflit si
`event_id` est réutilisé.

Les clés sont lues depuis `.swarm/automation-external-secrets.json`, qui doit
être un fichier local régulier sans permission groupe/monde. Chaque clé active
énumère ses missions et actions autorisées. Rotation et révocation sont relues
à la réception, au claim et juste avant l'effet ; une ancienne signature ne
crée aucune autorisation durable. Ne placez jamais ce fichier, une vraie clé ou
un corps reçu dans un rapport, une commande, un journal public ou un dépôt.

La politique versionnée définit par défaut
`external_max_payload_bytes=65536`, `external_rate_limit=60`,
`external_rate_window_seconds=60` et
`external_timestamp_window_seconds=300`. Les bornes admissibles sont
respectivement 256..1048576 octets, 1..10000 événements, 1..3600 secondes et
1..3600 secondes. Le journal conserve seulement identités/digests, résultat et
code de cause ; ni corps ni secret. Un événement accepté signifie seulement
qu'une demande durable existe. Son traitement, le résultat de la mission et
son acceptation sont trois états distincts.

## Actual English operator guide

## C03 external events

The `POST /api/v1/automation/external` route is reachable only through the
existing loopback web server. It accepts a bounded JSON document containing
exactly `schema_version`, `event_id`, `target_work_id`, and the sole
`request_resume` action. `X-Swarm-Key-ID`, `X-Swarm-Timestamp`, and
`X-Swarm-Signature` are required. The signature is
`sha256=HMAC-SHA256(secret, timestamp + "\n" + exact_body)`: changing JSON
whitespace changes the signed content and conflicts when an `event_id` is
reused.

Keys are read from `.swarm/automation-external-secrets.json`, which must be a
regular local file with no group or world permissions. Each active key lists
its authorized missions and actions. Rotation and revocation are reread at
reception, claim time, and immediately before the effect; an old signature
never creates lasting authority. Never place this file, a real key, or a
received body in a report, command, public log, or repository.

The versioned policy defaults to `external_max_payload_bytes=65536`,
`external_rate_limit=60`, `external_rate_window_seconds=60`, and
`external_timestamp_window_seconds=300`. Their accepted ranges are 256..1048576
bytes, 1..10000 events, 1..3600 seconds, and 1..3600 seconds. The journal keeps
only identities/digests, outcome, and reason code—never the body or secret. An
accepted event means only that a durable request exists. Request processing,
mission outcome, and acceptance remain three separate states.

## Commandes et résultats observés

- `go test ./... -run '^TestAutomationC03(Signature|ReplayConflict|Revocation|PayloadBounds|RateLimit|Redaction)$' -count=1 -timeout=150s` : exit 0, `ok ... 0.293s` après renforcement.
- `go test -race ./... -run '^TestAutomationC03(Signature|ReplayConflict|Revocation|PayloadBounds|RateLimit|Redaction)$' -count=1 -timeout=150s` : exit 0, `ok ... 6.068s` après dernier raccord.
- `go test ./... -run '^TestAutomationC0[12](Persistence|Idempotency|ClaimConcurrency|CrashRecovery|Authorization|WorkspaceWait|SchedulePreview|DST|RestartSkip|PauseArchive|Coalescing|CausalRecovery|Lifecycle)$' -count=1 -timeout=150s` : exit 0, `ok ... 1.000s`.
- Première tentative TCP sandbox : exit 1, `listen tcp6 [::1]:0: socket: operation not permitted`. La précondition n'a pas changé ; aucun rejeu socket identique.
- Aucun test n'a créé d'agent ni contacté de fournisseur. Les roots proviennent de `t.TempDir()`.

Ce dossier permet la lecture indépendante mais ne constitue ni verdict indépendant ni acceptation moteur.
