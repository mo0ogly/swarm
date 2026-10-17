//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"reflect"
	"strings"
	"syscall"
	"testing"
	"time"
)

func identityContextHTTP(s *Store, token, body string) *httptest.ResponseRecorder {
	mux := http.NewServeMux()
	registerTrustActionsHTTP(s, mux, func(w http.ResponseWriter, v any) { json.NewEncoder(w).Encode(v) }, func(w http.ResponseWriter, e error) { http.Error(w, e.Error(), 400) })
	r := httptest.NewRequest("POST", "/api/v1/trust-actions?action=identity-context", strings.NewReader(body))
	r.Header.Set("Authorization", "Bearer "+token)
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, r)
	return w
}

func TestActionsConfianceT3IdentityContextPublicReadOnly(t *testing.T) {
	s, _ := trustFixture(t)
	p, _, _ := s.trustPaths()
	before, _ := os.ReadFile(p)
	w := identityContextHTTP(s, trustRequesterToken, `{}`)
	if w.Code != 200 {
		t.Fatalf("identity context expected 200, got %d: %s", w.Code, w.Body.String())
	}
	var d map[string]any
	if err := json.Unmarshal(w.Body.Bytes(), &d); err != nil {
		t.Fatal(err)
	}
	for _, k := range []string{"principal_id", "principal_kind", "issuer", "subject", "assurance", "attributes", "delegation_id", "authenticated_at", "expires_at", "token_digest"} {
		if _, ok := d[k]; !ok {
			t.Fatalf("missing context field %s", k)
		}
	}
	if d["principal_id"] != "alice" || d["principal_kind"] != "local" || d["issuer"] != "urn:swarm:local" || d["token_digest"] != trustTokenHash(trustRequesterToken) {
		t.Fatal("incorrect local identity context")
	}
	if d["expires_at"] != nil || d["delegation_id"] != nil || strings.Contains(w.Body.String(), trustRequesterToken) {
		t.Fatal("fabricated expiry/delegation or raw credential")
	}
	after, _ := os.ReadFile(p)
	if !bytes.Equal(before, after) {
		t.Fatal("context read mutated state")
	}
	for _, tc := range []struct{ name, token, body string }{{"missing", "", `{}`}, {"unknown", "unknown-token-012345", `{}`}, {"forged", trustRequesterToken, `{"actor":"admin"}`}, {"null", trustRequesterToken, `null`}, {"foreign", trustApproverToken, `{"mandate_id":"absent"}`}} {
		t.Run(tc.name, func(t *testing.T) {
			w := identityContextHTTP(s, tc.token, tc.body)
			if w.Code == 200 {
				t.Fatal("invalid context input accepted")
			}
			after, _ := os.ReadFile(p)
			if !bytes.Equal(before, after) {
				t.Fatal("denial mutated state")
			}
		})
	}
}

func TestActionsConfianceT3IdentityContextLegacyAndExpiry(t *testing.T) {
	f := newOIDCFixture(t)
	s := oidcConfiguredStore(t, f)
	now := time.Now().UTC()
	token := "legacy-session-0123456789"
	_, err := s.withTrustLock(true, func(st *trustState) (any, error) {
		st.OIDCSessions = map[string]trustOIDCSession{trustTokenHash(token): {Subject: "alice", Revision: st.Revision, ExpiresAt: now.Add(time.Minute)}}
		return nil, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	w := identityContextHTTP(s, token, `{}`)
	if w.Code != 200 {
		t.Fatal(w.Code, w.Body.String())
	}
	var d map[string]any
	json.Unmarshal(w.Body.Bytes(), &d)
	if d["principal_kind"] != "human" || d["authenticated_at"] != nil || d["issuer"] != nil || d["subject"] != nil {
		t.Fatal("legacy session invented provider metadata")
	}
	_, err = s.withTrustLock(true, func(st *trustState) (any, error) {
		before := st.OIDCSessions[trustTokenHash(token)]
		before.ExpiresAt = now.Add(-time.Second)
		st.OIDCSessions[trustTokenHash(token)] = before
		return nil, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if identityContextHTTP(s, token, `{}`).Code == 200 {
		t.Fatal("expired session resolved")
	}
	a, _ := s.trustView()
	b, _ := s.trustView()
	if !reflect.DeepEqual(a, b) {
		t.Fatal("view unstable")
	}
}

func TestActionsConfianceT3IdentityContextVerifiedSessions(t *testing.T) {
	for _, kind := range []string{"human", "service"} {
		t.Run(kind, func(t *testing.T) {
			var s *Store
			var token, issuer, external string
			if kind == "human" {
				f := newOIDCFixture(t)
				f.modify = func(c map[string]any) {
					c["auth_time"] = time.Now().Add(-time.Second).Unix()
					c["acr"] = "urn:test:assurance"
				}
				s = oidcConfiguredStore(t, f)
				v, err := s.trustOIDCFinish(context.Background(), oidcAuthorize(t, s))
				if err != nil {
					t.Fatal(err)
				}
				token = v.(map[string]any)["session_token"].(string)
				issuer = f.server.URL
				external = "employee-1"
			} else {
				f := newWorkloadFixture(t)
				s = workloadConfiguredStore(t, f)
				v, err := s.trustWorkloadLogin(context.Background(), workloadAssertion(t, f, nil))
				if err != nil {
					t.Fatal(err)
				}
				token = v.(map[string]any)["session_token"].(string)
				issuer = f.server.URL
				external = "service-principal"
			}
			v, err := s.trustIdentityContext([]byte(`{}`), token, time.Now().UTC())
			if err != nil {
				t.Fatal(err)
			}
			c := v.(*TrustIdentityContext)
			if c.PrincipalKind != kind || c.Issuer == nil || *c.Issuer != issuer || c.Subject == nil || *c.Subject != external || c.AuthenticatedAt == nil || c.ExpiresAt == nil || c.TokenDigest != trustTokenHash(token) {
				t.Fatal("verified session context incomplete")
			}
			if kind == "human" && (c.Assurance.ACR != "urn:test:assurance" || c.Assurance.ProviderAuthTime == nil) {
				t.Fatal("verified assurance lost")
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			v, err = reopened.trustIdentityContext([]byte(`{}`), token, time.Now().UTC())
			if err != nil || !reflect.DeepEqual(c, v) {
				t.Fatal("persisted context differs", err)
			}
			path, _, _ := s.trustPaths()
			raw, _ := os.ReadFile(path)
			if strings.Contains(string(raw), token) {
				t.Fatal("raw session persisted")
			}
			_, err = s.withTrustLock(true, func(st *trustState) (any, error) { st.Revision++; return nil, nil })
			if err != nil {
				t.Fatal(err)
			}
			if _, err = s.trustIdentityContext([]byte(`{}`), token, time.Now().UTC()); err == nil {
				t.Fatal("stale context accepted")
			}
		})
	}
}

func TestActionsConfianceT3IdentityContextEvidenceAndDelegation(t *testing.T) {
	s, now := trustFixture(t)
	m := createMandate(t, s, now, "identity-context", now.Add(time.Hour))
	v, err := s.trustIdentityContext([]byte(`{"mandate_id":"identity-context"}`), trustRequesterToken, now)
	if err != nil || v.(*TrustIdentityContext).DelegationID == nil || *v.(*TrustIdentityContext).DelegationID != m.ID {
		t.Fatal("own active mandate not bound", err)
	}
	if _, err = s.trustIdentityContext([]byte(`{"mandate_id":"identity-context"}`), trustApproverToken, now); err == nil {
		t.Fatal("foreign delegation accepted")
	}
	a := createAction(t, s, now, "identity-context-request", m.ID, "identity-context-key", 100)
	exp, err := s.trustExport()
	if err != nil {
		t.Fatal(err)
	}
	ev := exp.Evidence[len(exp.Evidence)-1]
	if ev.Actor != "alice" || ev.IdentityContext == nil || ev.IdentityContext.PrincipalID != ev.Actor || ev.IdentityContext.DelegationID == nil {
		t.Fatal("evidence lacks actual context")
	}
	anchor, err := s.trustAnchor()
	if err != nil {
		t.Fatal(err)
	}
	key, err := s.trustProofKey()
	if err != nil {
		t.Fatal(err)
	}
	if err = verifyTrustProof(exp, anchor, hex.EncodeToString(key)); err != nil {
		t.Fatal(err)
	}
	exp.Evidence[len(exp.Evidence)-1].IdentityContext.PrincipalKind = "forged"
	if verifyTrustProof(exp, anchor, hex.EncodeToString(key)) == nil {
		t.Fatal("context tampering undetected")
	}
	c := trustCmd("identity-context-request")
	c.MandateID = m.ID
	c.Resource = a.Resource
	c.Amount = a.Amount
	c.Currency = a.Currency
	c.Beneficiary = a.Beneficiary
	c.IdempotencyKey = a.IdempotencyKey
	before, _ := s.trustExport()
	if _, err = s.trustOperate("request", c, trustRequesterToken, now.Add(time.Second)); err != nil {
		t.Fatal(err)
	}
	after, _ := s.trustExport()
	if !reflect.DeepEqual(before, after) {
		t.Fatal("replay rewrote historical context")
	}
}

func TestActionsConfianceT3IdentityContextHistoricalHash(t *testing.T) {
	ev := TrustEvidence{Sequence: 1, Kind: "request", Actor: "legacy", Integrity: 1, ObjectID: "legacy-object", ContentHash: "digest", Result: "admitted", At: "2026-01-01T00:00:00Z"}
	_, legacy := trustCanonical(struct {
		Sequence                                                int `json:"sequence"`
		PreviousHash, Kind, Actor                               string
		Integrity                                               int
		ObjectID, ContentHash, OriginalHash, Result, Reason, At string
	}{ev.Sequence, ev.PreviousHash, ev.Kind, ev.Actor, ev.Integrity, ev.ObjectID, ev.ContentHash, ev.OriginalHash, ev.Result, ev.Reason, ev.At})
	if trustEvidenceHash(ev) != legacy {
		t.Fatal("historical hash changed")
	}
	c := &TrustIdentityContext{PrincipalID: "legacy"}
	ev.IdentityContext = c
	if trustEvidenceHash(ev) == legacy {
		t.Fatal("new context excluded from digest")
	}
}
func TestActionsConfianceT3IdentityContextCLIAndSuspension(t *testing.T) {
	s, _ := trustFixture(t)
	t.Setenv("SWARM_TRUST_TOKEN", trustRequesterToken)
	var out bytes.Buffer
	if err := s.trustActionsCLI([]string{"trust", "identity-context"}, "", &out); err != nil {
		t.Fatal(err)
	}
	var c TrustIdentityContext
	if json.Unmarshal(out.Bytes(), &c) != nil || c.PrincipalID != "alice" {
		t.Fatal("CLI context differs")
	}
	change := lifecycleChange(t, "alice", true, 1)
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	p, _, _ := s.trustPaths()
	before, _ := os.ReadFile(p)
	if identityContextHTTP(s, trustRequesterToken, `{}`).Code == 200 {
		t.Fatal("suspended local identity resolved")
	}
	after, _ := os.ReadFile(p)
	if !bytes.Equal(before, after) {
		t.Fatal("denial mutated state")
	}
}
func TestActionsConfianceT3IdentityContextInvalidProviderAuthTime(t *testing.T) {
	for _, n := range []int64{-1, time.Now().Add(time.Hour).Unix()} {
		t.Run(time.Unix(n, 0).UTC().Format(time.RFC3339), func(t *testing.T) {
			f := newOIDCFixture(t)
			f.modify = func(c map[string]any) { c["auth_time"] = n }
			s := oidcConfiguredStore(t, f)
			if _, err := s.trustOIDCFinish(context.Background(), oidcAuthorize(t, s)); err == nil {
				t.Fatal("invalid provider authentication time accepted")
			}
		})
	}
}

func TestActionsConfianceT3IdentityContextClockAfterLock(t *testing.T) {
	for _, operation := range []string{"identity-context", "request"} {
		t.Run(operation, func(t *testing.T) {
			f := newOIDCFixture(t)
			s := oidcConfiguredStore(t, f)
			token := "clock-expiring-session-012345"
			deadline := time.Now().Add(200 * time.Millisecond)
			_, err := s.withTrustLock(true, func(st *trustState) (any, error) {
				st.OIDCSessions = map[string]trustOIDCSession{trustTokenHash(token): {Subject: "alice", Revision: st.Revision, ExpiresAt: deadline}}
				return nil, nil
			})
			if err != nil {
				t.Fatal(err)
			}
			p, lockPath, _ := s.trustPaths()
			before, _ := os.ReadFile(p)
			lock, err := os.OpenFile(lockPath, os.O_RDWR, 0600)
			if err != nil {
				t.Fatal(err)
			}
			defer lock.Close()
			if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); err != nil {
				t.Fatal(err)
			}
			done := make(chan *httptest.ResponseRecorder, 1)
			started := make(chan struct{})
			go func() {
				mux := http.NewServeMux()
				registerTrustActionsHTTP(s, mux, func(w http.ResponseWriter, v any) { json.NewEncoder(w).Encode(v) }, func(w http.ResponseWriter, e error) { http.Error(w, e.Error(), 400) })
				body := `{}`
				if operation == "request" {
					body = `{"schema_version":1,"event_id":"clock-after-lock"}`
				}
				r := httptest.NewRequest("POST", "/api/v1/trust-actions?action="+operation, strings.NewReader(body))
				r.Header.Set("Authorization", "Bearer "+token)
				w := httptest.NewRecorder()
				close(started)
				mux.ServeHTTP(w, r)
				done <- w
			}()
			<-started
			time.Sleep(350 * time.Millisecond)
			select {
			case <-done:
				t.Fatal("operation did not wait for lock")
			default:
			}
			if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_UN); err != nil {
				t.Fatal(err)
			}
			select {
			case w := <-done:
				if w.Code == 200 {
					t.Fatal("expired credential accepted after waiting for lock")
				}
			case <-time.After(3 * time.Second):
				t.Fatal("operation did not finish after unlocking")
			}
			after, _ := os.ReadFile(p)
			if !bytes.Equal(before, after) {
				t.Fatal("expired operation changed state")
			}
		})
	}
}
