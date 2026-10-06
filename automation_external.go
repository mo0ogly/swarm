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
