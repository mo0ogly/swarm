//go:build linux

package engine

import (
	"bytes"
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"encoding/json"
	jose "github.com/go-jose/go-jose/v4"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestActionsConfianceT3OIDCConfigurationRejectsUnsafeInputs(t *testing.T) {
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.OIDC = &TrustOIDCConfig{Enabled: true, Issuer: "http://example.com", ClientID: "swarm", RedirectURI: "http://127.0.0.1/callback", RequestTimeoutSeconds: 5, LoginTTLSeconds: 300, SessionTTLSeconds: 600, Mappings: []TrustOIDCMapping{{ExternalSubject: "employee-1", Subject: "alice"}}}
	if _, err := s.trustConfig(c, true); err == nil {
		t.Fatal("plaintext remote issuer accepted")
	}
}

type oidcFixture struct {
	server       *httptest.Server
	key          *ecdsa.PrivateKey
	mu           sync.Mutex
	codes        map[string]url.Values
	modify       func(map[string]any)
	badSignature bool
	kid          string
	exchanges    int
}

func newOIDCFixture(t *testing.T) *oidcFixture {
	t.Helper()
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	f := &oidcFixture{key: key, codes: map[string]url.Values{}, kid: "key-1"}
	f.server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		f.mu.Lock()
		defer f.mu.Unlock()
		w.Header().Set("Content-Type", "application/json")
		issuer := f.server.URL
		switch r.URL.Path {
		case "/.well-known/openid-configuration":
			json.NewEncoder(w).Encode(map[string]any{"issuer": issuer, "authorization_endpoint": issuer + "/authorize", "token_endpoint": issuer + "/token", "jwks_uri": issuer + "/keys", "response_types_supported": []string{"code"}, "subject_types_supported": []string{"public"}, "id_token_signing_alg_values_supported": []string{"ES256"}})
		case "/keys":
			json.NewEncoder(w).Encode(jose.JSONWebKeySet{Keys: []jose.JSONWebKey{{Key: &f.key.PublicKey, KeyID: f.kid, Algorithm: "ES256", Use: "sig"}}})
		case "/authorize":
			q := r.URL.Query()
			if q.Get("code_challenge_method") != "S256" || q.Get("nonce") == "" || q.Get("code_challenge") == "" || q.Get("response_type") != "code" {
				http.Error(w, "invalid PKCE authorization", 400)
				return
			}
			code := newID("code-")
			f.codes[code] = q
			target, _ := url.Parse(q.Get("redirect_uri"))
			v := target.Query()
			v.Set("code", code)
			v.Set("state", q.Get("state"))
			target.RawQuery = v.Encode()
			http.Redirect(w, r, target.String(), 302)
		case "/token":
			r.ParseForm()
			q, ok := f.codes[r.Form.Get("code")]
			delete(f.codes, r.Form.Get("code"))
			f.exchanges++
			if !ok || q.Get("code_challenge") != trustOIDCChallenge(r.Form.Get("code_verifier")) || q.Get("client_id") != r.Form.Get("client_id") || q.Get("redirect_uri") != r.Form.Get("redirect_uri") {
				http.Error(w, "invalid authorization code or verifier", 400)
				return
			}
			now := time.Now().Unix()
			claims := map[string]any{"iss": issuer, "aud": "swarm", "sub": "employee-1", "iat": now, "exp": now + 600, "nonce": q.Get("nonce"), "roles": []string{"admin"}, "integrity": 3}
			if f.modify != nil {
				f.modify(claims)
			}
			signingKey := f.key
			if f.badSignature {
				signingKey, _ = ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
			}
			signer, err := jose.NewSigner(jose.SigningKey{Algorithm: jose.ES256, Key: signingKey}, (&jose.SignerOptions{}).WithHeader("kid", f.kid))
			if err != nil {
				t.Error(err)
				http.Error(w, "signer", 500)
				return
			}
			raw, _ := json.Marshal(claims)
			signed, err := signer.Sign(raw)
			if err != nil {
				t.Error(err)
				return
			}
			token, _ := signed.CompactSerialize()
			json.NewEncoder(w).Encode(map[string]any{"access_token": "test-access", "token_type": "Bearer", "id_token": token, "expires_in": 600})
		default:
			http.NotFound(w, r)
		}
	}))
	t.Cleanup(f.server.Close)
	return f
}

func oidcConfiguredStore(t *testing.T, f *oidcFixture) *Store {
	t.Helper()
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.OIDC = &TrustOIDCConfig{Enabled: true, Issuer: f.server.URL, ClientID: "swarm", RedirectURI: "http://127.0.0.1/trust-oidc-callback.html", RequestTimeoutSeconds: 5, LoginTTLSeconds: 300, SessionTTLSeconds: 600, Mappings: []TrustOIDCMapping{{ExternalSubject: "employee-1", Subject: "alice"}}}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	return s
}

func oidcAuthorize(t *testing.T, s *Store) trustOIDCCommand {
	t.Helper()
	v, err := s.trustOIDCStart(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	login := v.(trustOIDCLogin)
	client := http.Client{CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }}
	r, err := client.Get(login.AuthorizationURL)
	if err != nil {
		t.Fatal(err)
	}
	r.Body.Close()
	redirect, err := url.Parse(r.Header.Get("Location"))
	if err != nil || redirect.Query().Get("state") != login.State {
		t.Fatal("authorization state missing", err)
	}
	return trustOIDCCommand{State: login.State, Binding: login.Binding, Code: redirect.Query().Get("code")}
}

func TestActionsConfianceT3OIDCLoginBusinessPersistenceAndRevocation(t *testing.T) {
	f := newOIDCFixture(t)
	s := oidcConfiguredStore(t, f)
	cmd := oidcAuthorize(t, s)
	v, err := s.trustOIDCFinish(context.Background(), cmd)
	if err != nil {
		t.Fatal(err)
	}
	result := v.(map[string]any)
	token := result["session_token"].(string)
	if result["subject"] != "alice" || result["integrity"] != 1 {
		t.Fatal("external claims elevated identity", result["subject"], result["integrity"])
	}
	now := time.Now().UTC()
	createMandate(t, s, now, "oidc-mandate", now.Add(time.Hour))
	request := trustCmd("oidc-request")
	request.MandateID = "oidc-mandate"
	request.Resource = "invoice/acme"
	request.Amount = 100
	request.Currency = "EUR"
	request.Beneficiary = "ACME"
	request.IdempotencyKey = "oidc-key"
	value, err := s.trustOperate("request", request, token, now)
	if err != nil || value.(TrustAction).Requester != "alice" {
		t.Fatal("OIDC business request failed", err)
	}
	st, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	if _, err = findTrustIdentityAt(&st, token, now.Add(time.Hour)); err == nil {
		t.Fatal("expired session authenticated")
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	if _, err = reopened.trustOperate("request", request, token, now); err != nil {
		t.Fatal("session/replay did not survive reopen", err)
	}
	if _, err = reopened.trustOIDCFinish(context.Background(), cmd); err == nil {
		t.Fatal("login replay accepted")
	}
	raw, _ := json.Marshal(st)
	if strings.Contains(string(raw), token) || strings.Contains(string(raw), "test-access") {
		t.Fatal("raw session/provider token persisted")
	}
	if _, err = reopened.trustConfig(lifecycleChange(t, "alice", true, 2), false); err != nil {
		t.Fatal(err)
	}
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	if _, err = reopened.trustOperate("request", request, token, now); commandFailure(err).Code != "authorization_required" {
		t.Fatal("suspension/config did not revoke session", err)
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("revoked session mutated state")
	}
}

func TestActionsConfianceT3OIDCRejectsInvalidClaimsAndSignatures(t *testing.T) {
	cases := map[string]func(map[string]any){
		"issuer":                         func(c map[string]any) { c["iss"] = "https://wrong.example" },
		"audience":                       func(c map[string]any) { c["aud"] = "another-app" },
		"nonce":                          func(c map[string]any) { c["nonce"] = "wrong-nonce" },
		"expired":                        func(c map[string]any) { c["exp"] = time.Now().Add(-time.Minute).Unix() },
		"future-iat":                     func(c map[string]any) { c["iat"] = time.Now().Add(time.Hour).Unix() },
		"missing-iat":                    func(c map[string]any) { delete(c, "iat") },
		"future-nbf":                     func(c map[string]any) { c["nbf"] = time.Now().Add(time.Hour).Unix() },
		"wrong-azp":                      func(c map[string]any) { c["azp"] = "another-app" },
		"multiple-audiences-without-azp": func(c map[string]any) { c["aud"] = []string{"swarm", "other"} },
		"unmapped-subject":               func(c map[string]any) { c["sub"] = "unmapped" },
		"signature":                      nil,
	}
	for name, modify := range cases {
		t.Run(name, func(t *testing.T) {
			f := newOIDCFixture(t)
			f.modify = modify
			f.badSignature = name == "signature"
			s := oidcConfiguredStore(t, f)
			cmd := oidcAuthorize(t, s)
			before, _ := s.trustView()
			beforeRaw, _ := json.Marshal(before)
			if _, err := s.trustOIDCFinish(context.Background(), cmd); commandFailure(err).Code != "authorization_required" {
				t.Fatal("invalid OIDC accepted", err)
			}
			after, _ := s.trustView()
			afterRaw, _ := json.Marshal(after)
			if !bytes.Equal(beforeRaw, afterRaw) {
				t.Fatal("refused OIDC changed business state")
			}
			st, _ := s.loadTrustState()
			if len(st.OIDCSessions) != 0 {
				t.Fatal("refusal created a session")
			}
		})
	}
}

func TestActionsConfianceT3OIDCBindingRevisionRotationAndConcurrency(t *testing.T) {
	t.Run("browser-binding", func(t *testing.T) {
		f := newOIDCFixture(t)
		s := oidcConfiguredStore(t, f)
		cmd := oidcAuthorize(t, s)
		bad := cmd
		bad.Binding = "wrong-browser-binding"
		path, _, _ := s.trustPaths()
		before, _ := os.ReadFile(path)
		if _, err := s.trustOIDCFinish(context.Background(), bad); err == nil {
			t.Fatal("wrong browser binding accepted")
		}
		after, _ := os.ReadFile(path)
		if !bytes.Equal(before, after) {
			t.Fatal("invalid binding consumed login")
		}
		if _, err := s.trustOIDCFinish(context.Background(), cmd); err != nil {
			t.Fatal("valid binding lost", err)
		}
	})
	t.Run("configuration-revokes-pending", func(t *testing.T) {
		f := newOIDCFixture(t)
		s := oidcConfiguredStore(t, f)
		cmd := oidcAuthorize(t, s)
		if _, err := s.trustConfig(lifecycleChange(t, "alice", false, 2), false); err != nil {
			t.Fatal(err)
		}
		if _, err := s.trustOIDCFinish(context.Background(), cmd); err == nil {
			t.Fatal("stale pending login accepted")
		}
		if f.exchanges != 0 {
			t.Fatal("stale login reached provider")
		}
	})
	t.Run("key-rotation", func(t *testing.T) {
		f := newOIDCFixture(t)
		s := oidcConfiguredStore(t, f)
		for _, kid := range []string{"key-1", "key-2"} {
			cmd := oidcAuthorize(t, s)
			f.mu.Lock()
			f.key, _ = ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
			f.kid = kid
			f.mu.Unlock()
			if _, err := s.trustOIDCFinish(context.Background(), cmd); err != nil {
				t.Fatal("rotated key rejected", err)
			}
		}
	})
	t.Run("one-consumer", func(t *testing.T) {
		f := newOIDCFixture(t)
		s := oidcConfiguredStore(t, f)
		cmd := oidcAuthorize(t, s)
		results := make(chan error, 2)
		for i := 0; i < 2; i++ {
			go func() { _, err := s.trustOIDCFinish(context.Background(), cmd); results <- err }()
		}
		successes := 0
		for i := 0; i < 2; i++ {
			if <-results == nil {
				successes++
			}
		}
		if successes != 1 || f.exchanges != 1 {
			t.Fatalf("successes=%d exchanges=%d", successes, f.exchanges)
		}
	})
	t.Run("unavailable-provider", func(t *testing.T) {
		f := newOIDCFixture(t)
		s := oidcConfiguredStore(t, f)
		cmd := oidcAuthorize(t, s)
		f.server.Close()
		start := time.Now()
		if _, err := s.trustOIDCFinish(context.Background(), cmd); err == nil {
			t.Fatal("unavailable provider accepted")
		}
		if time.Since(start) > 5*time.Second {
			t.Fatal("network failure exceeded configured budget")
		}
		st, _ := s.loadTrustState()
		if len(st.OIDCSessions) != 0 || len(st.OIDCPending) != 0 {
			t.Fatal("failed exchange reusable or authenticated")
		}
	})
	t.Run("logout", func(t *testing.T) {
		f := newOIDCFixture(t)
		s := oidcConfiguredStore(t, f)
		v, err := s.trustOIDCFinish(context.Background(), oidcAuthorize(t, s))
		if err != nil {
			t.Fatal(err)
		}
		token := v.(map[string]any)["session_token"].(string)
		raw, _ := json.Marshal(map[string]string{"session_token": token})
		if _, err = s.trustOIDCOperation(context.Background(), "oidc-logout", raw); err != nil {
			t.Fatal(err)
		}
		st, _ := s.loadTrustState()
		if _, err = findTrustIdentity(&st, token); err == nil {
			t.Fatal("logged-out session authenticated")
		}
	})
}

func TestActionsConfianceT3OIDCPublicAPIAndCLI(t *testing.T) {
	f := newOIDCFixture(t)
	s := oidcConfiguredStore(t, f)
	h := newWebHandler(s, "localhost:18787", "oidc-operator-session")
	call := func(action string, body []byte, authorized bool) *httptest.ResponseRecorder {
		r := httptest.NewRequest("POST", "http://localhost:18787/api/v1/trust-actions?action="+action, bytes.NewReader(body))
		r.Header.Set("Origin", "http://localhost:18787")
		r.Header.Set("X-Swarm-CSRF", "oidc-operator-session")
		if authorized {
			r.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte("localhost:18787"))[:12], Value: "oidc-operator-session"})
		}
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		return w
	}
	if w := call("oidc-start", []byte(`{}`), false); w.Code != 403 {
		t.Fatal("operator boundary bypassed", w.Code)
	}
	w := call("oidc-start", []byte(`{}`), true)
	if w.Code != 200 || w.Header().Get("Cache-Control") != "no-store" {
		t.Fatal("API login start failed", w.Code)
	}
	var login trustOIDCLogin
	if json.Unmarshal(w.Body.Bytes(), &login) != nil || login.AuthorizationURL == "" {
		t.Fatal("wrong API login shape")
	}
	client := http.Client{CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }}
	response, err := client.Get(login.AuthorizationURL)
	if err != nil {
		t.Fatal(err)
	}
	response.Body.Close()
	u, _ := url.Parse(response.Header.Get("Location"))
	raw, _ := json.Marshal(trustOIDCCommand{State: login.State, Binding: login.Binding, Code: u.Query().Get("code")})
	w = call("oidc-finish", raw, true)
	if w.Code != 200 || !strings.Contains(w.Body.String(), `"subject":"alice"`) {
		t.Fatal("API exchange failed", w.Code, w.Body.String())
	}
	var out bytes.Buffer
	input := filepath.Join(t.TempDir(), "start.json")
	if err = os.WriteFile(input, []byte(`{}`), 0600); err != nil {
		t.Fatal(err)
	}
	if err = s.trustActionsCLI([]string{"trust", "oidc-start"}, input, &out); err != nil {
		t.Fatal("CLI start failed", err)
	}
	if json.Unmarshal(out.Bytes(), &login) != nil || login.State == "" {
		t.Fatal("CLI start did not return a login")
	}
	response, err = client.Get(login.AuthorizationURL)
	if err != nil {
		t.Fatal(err)
	}
	response.Body.Close()
	u, _ = url.Parse(response.Header.Get("Location"))
	raw, _ = json.Marshal(trustOIDCCommand{State: login.State, Binding: login.Binding, Code: u.Query().Get("code")})
	os.WriteFile(input, raw, 0600)
	out.Reset()
	if err = s.trustActionsCLI([]string{"trust", "oidc-finish"}, input, &out); err != nil || !strings.Contains(out.String(), `"subject": "alice"`) {
		t.Fatal("CLI finish failed", err)
	}
	r := httptest.NewRequest("GET", "http://localhost:18787/trust-oidc-callback.html?code=secret&state=state", nil)
	w = httptest.NewRecorder()
	h.ServeHTTP(w, r)
	if w.Code != 200 || strings.Contains(w.Body.String(), "secret") || !strings.Contains(w.Body.String(), "trust-oidc-callback.js") {
		t.Fatal("safe callback landing failed", w.Code)
	}
}

func TestActionsConfianceT3OIDCConfigurationValidationAndPreview(t *testing.T) {
	f := newOIDCFixture(t)
	s := oidcConfiguredStore(t, f)
	cases := map[string]func(*TrustOIDCConfig){
		"zero-timeout":          func(c *TrustOIDCConfig) { c.RequestTimeoutSeconds = 0 },
		"negative-session":      func(c *TrustOIDCConfig) { c.SessionTTLSeconds = -1 },
		"zero-login":            func(c *TrustOIDCConfig) { c.LoginTTLSeconds = 0 },
		"remote-http-redirect":  func(c *TrustOIDCConfig) { c.RedirectURI = "http://example.com/callback" },
		"duplicate-mapping":     func(c *TrustOIDCConfig) { c.Mappings = append(c.Mappings, c.Mappings[0]) },
		"unknown-local-subject": func(c *TrustOIDCConfig) { c.Mappings[0].Subject = "unknown" },
		"issuer-credentials":    func(c *TrustOIDCConfig) { c.Issuer = "https://secret@example.com" },
	}
	for name, modify := range cases {
		t.Run(name, func(t *testing.T) {
			st, _ := s.loadTrustState()
			c := lifecycleChange(t, "alice", false, 2)
			cfg := *st.OIDC
			cfg.Mappings = append([]TrustOIDCMapping{}, cfg.Mappings...)
			modify(&cfg)
			c.OIDC = &cfg
			path, _, _ := s.trustPaths()
			before, _ := os.ReadFile(path)
			if _, err := s.trustConfig(c, true); err == nil {
				t.Fatal("invalid OIDC preview accepted")
			}
			if _, err := s.trustConfig(c, false); err == nil {
				t.Fatal("invalid OIDC apply accepted")
			}
			after, _ := os.ReadFile(path)
			if !bytes.Equal(before, after) {
				t.Fatal("invalid config mutated state")
			}
		})
	}
	st, _ := s.loadTrustState()
	c := lifecycleChange(t, "alice", false, 2)
	cfg := *st.OIDC
	cfg.Issuer = "http://127.0.0.1:1"
	c.OIDC = &cfg
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	if _, err := s.trustConfig(c, true); err != nil {
		t.Fatal("preview attempted network connection", err)
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("preview applied config")
	}
}

func TestActionsConfianceT3OIDCEnabledRequiresBoolean(t *testing.T) {
	for _, value := range []string{`null`, `"true"`, `1`, `{}`, `[]`} {
		t.Run(value, func(t *testing.T) {
			var config TrustOIDCConfig
			if err := strict([]byte(`{"enabled":`+value+`}`), &config); err == nil {
				t.Fatal("nonboolean enabled accepted")
			}
		})
	}
}

func TestActionsConfianceT3OIDCRequestDeadline(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { <-r.Context().Done() }))
	defer server.Close()
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.OIDC = &TrustOIDCConfig{Enabled: true, Issuer: server.URL, ClientID: "swarm", RedirectURI: "http://127.0.0.1/trust-oidc-callback.html", RequestTimeoutSeconds: 1, LoginTTLSeconds: 30, SessionTTLSeconds: 60, Mappings: []TrustOIDCMapping{{ExternalSubject: "e", Subject: "alice"}}}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	started := time.Now()
	if _, err := s.trustOIDCStart(context.Background()); commandFailure(err).Code != "authorization_required" {
		t.Fatal("stalled provider did not fail", err)
	}
	elapsed := time.Since(started)
	if elapsed < 900*time.Millisecond || elapsed > 3*time.Second {
		t.Fatal("configured deadline not observed", elapsed)
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("failed discovery mutated state")
	}
}
