//go:build linux

package engine

import (
	"bytes"
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/rsa"
	"encoding/json"
	jose "github.com/go-jose/go-jose/v4"
	"github.com/go-jose/go-jose/v4/jwt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestActionsConfianceT3WorkloadConfigurationRejectsUnsafeInputs(t *testing.T) {
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.Workloads = &TrustWorkloadConfig{Enabled: true, Issuer: "http://example.com", Audience: "urn:swarm:trust", RequestTimeoutSeconds: 5, AssertionTTLSeconds: 300, SessionTTLSeconds: 600}
	if _, err := s.trustConfig(c, true); err == nil {
		t.Fatal("insecure workload configuration accepted")
	}
}

type workloadFixture struct {
	tokenEntered   chan struct{}
	tokenContinue  chan struct{}
	server         *httptest.Server
	mu             sync.Mutex
	clientKey      *ecdsa.PrivateKey
	issuerKey      *ecdsa.PrivateKey
	clientKid      string
	issuerKid      string
	used           map[string]bool
	exchanges      int
	discoveries    int
	modify         func(map[string]any)
	wrongSignature bool
	tokenType      string
	methods        []string
}

func newWorkloadFixture(t *testing.T) *workloadFixture {
	t.Helper()
	clientKey, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	issuerKey, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	f := &workloadFixture{clientKey: clientKey, issuerKey: issuerKey, clientKid: "service-key-1", issuerKid: "issuer-key-1", used: map[string]bool{}, tokenType: "at+jwt", methods: []string{"private_key_jwt"}}
	f.server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		f.mu.Lock()
		defer f.mu.Unlock()
		w.Header().Set("Content-Type", "application/json")
		issuer := f.server.URL
		switch r.URL.Path {
		case "/.well-known/openid-configuration":
			f.discoveries++
			json.NewEncoder(w).Encode(map[string]any{"issuer": issuer, "token_endpoint": issuer + "/token", "jwks_uri": issuer + "/keys", "token_endpoint_auth_methods_supported": f.methods})
		case "/keys":
			json.NewEncoder(w).Encode(jose.JSONWebKeySet{Keys: []jose.JSONWebKey{{Key: &f.issuerKey.PublicKey, KeyID: f.issuerKid, Algorithm: "ES256", Use: "sig"}}})
		case "/token":
			if f.tokenEntered != nil {
				close(f.tokenEntered)
				<-f.tokenContinue
			}
			r.ParseForm()
			f.exchanges++
			if r.Form.Get("grant_type") != "client_credentials" || r.Form.Get("client_assertion_type") != "urn:ietf:params:oauth:client-assertion-type:jwt-bearer" || r.Form.Get("client_id") != "settlement-service" || r.Header.Get("Authorization") != "" || r.Form.Get("resource") != "urn:swarm:trust" {
				http.Error(w, "invalid client authentication", 400)
				return
			}
			signed, err := jose.ParseSigned(r.Form.Get("client_assertion"), []jose.SignatureAlgorithm{jose.ES256})
			if err != nil {
				http.Error(w, "invalid assertion", 400)
				return
			}
			payload, err := signed.Verify(&f.clientKey.PublicKey)
			if err != nil {
				http.Error(w, "invalid signature", 400)
				return
			}
			var claims jwt.Claims
			if json.Unmarshal(payload, &claims) != nil || claims.ValidateWithLeeway(jwt.Expected{Issuer: "settlement-service", Subject: "settlement-service", AnyAudience: jwt.Audience{issuer + "/token"}, Time: time.Now()}, 0) != nil || claims.ID == "" || f.used[claims.ID] {
				http.Error(w, "invalid claims or replay", 400)
				return
			}
			f.used[claims.ID] = true
			now := time.Now().Unix()
			body := map[string]any{"iss": issuer, "sub": "service-principal", "client_id": "settlement-service", "aud": "urn:swarm:trust", "iat": now, "exp": now + 600, "jti": newID("access-"), "roles": []string{"admin"}, "integrity": 3}
			if f.modify != nil {
				f.modify(body)
			}
			signingKey := f.issuerKey
			if f.wrongSignature {
				signingKey, _ = ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
			}
			signer, err := jose.NewSigner(jose.SigningKey{Algorithm: jose.ES256, Key: signingKey}, (&jose.SignerOptions{}).WithType(jose.ContentType(f.tokenType)).WithHeader("kid", f.issuerKid))
			if err != nil {
				t.Error(err)
				http.Error(w, "signer", 500)
				return
			}
			raw, _ := json.Marshal(body)
			result, err := signer.Sign(raw)
			if err != nil {
				t.Error(err)
				return
			}
			token, _ := result.CompactSerialize()
			json.NewEncoder(w).Encode(map[string]any{"access_token": token, "token_type": "Bearer"})
		default:
			http.NotFound(w, r)
		}
	}))
	t.Cleanup(f.server.Close)
	return f
}

func workloadConfiguredStore(t *testing.T, f *workloadFixture) *Store {
	t.Helper()
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.Workloads = &TrustWorkloadConfig{Enabled: true, Issuer: f.server.URL, Audience: "urn:swarm:trust", RequestTimeoutSeconds: 5, AssertionTTLSeconds: 300, SessionTTLSeconds: 600, Clients: []TrustWorkloadClient{{ClientID: "settlement-service", ExternalSubject: "service-principal", Subject: "effect-service", PublicKeys: []jose.JSONWebKey{{Key: &f.clientKey.PublicKey, KeyID: f.clientKid, Algorithm: "ES256", Use: "sig"}}}}}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	return s
}

func workloadAssertion(t *testing.T, f *workloadFixture, modify func(map[string]any)) trustWorkloadCommand {
	t.Helper()
	now := time.Now().Unix()
	claims := map[string]any{"iss": "settlement-service", "sub": "settlement-service", "aud": f.server.URL + "/token", "iat": now, "exp": now + 120, "jti": newID("assertion-")}
	if modify != nil {
		modify(claims)
	}
	signer, err := jose.NewSigner(jose.SigningKey{Algorithm: jose.ES256, Key: f.clientKey}, (&jose.SignerOptions{}).WithType("JWT").WithHeader("kid", f.clientKid))
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(claims)
	signed, err := signer.Sign(raw)
	if err != nil {
		t.Fatal(err)
	}
	token, err := signed.CompactSerialize()
	if err != nil {
		t.Fatal(err)
	}
	return trustWorkloadCommand{ClientID: "settlement-service", ClientAssertion: token}
}

func TestActionsConfianceT3WorkloadAuthenticatesEffectAndPersists(t *testing.T) {
	f := newWorkloadFixture(t)
	s := workloadConfiguredStore(t, f)
	cmd := workloadAssertion(t, f, nil)
	value, err := s.trustWorkloadLogin(context.Background(), cmd)
	if err != nil {
		t.Fatal(err)
	}
	result := value.(map[string]any)
	token := result["session_token"].(string)
	if result["subject"] != "effect-service" || result["integrity"] != 2 {
		t.Fatal("provider claims elevated authority")
	}
	now := time.Now().UTC()
	createMandate(t, s, now, "service-mandate", now.Add(time.Hour))
	action := createAction(t, s, now, "service-request", "service-mandate", "service-action", 100)
	approveAction(t, s, now, "service-approve", action)
	commit := trustCmd("service-commit")
	commit.ActionID = action.ID
	v, err := s.trustOperate("commit", commit, token, now)
	if err != nil || v.(TrustAction).State != "committed" {
		t.Fatal("service could not commit exact approved action", err)
	}
	path, _, _ := s.trustPaths()
	raw, _ := os.ReadFile(path)
	if strings.Contains(string(raw), token) || strings.Contains(string(raw), cmd.ClientAssertion) {
		t.Fatal("raw credential stored")
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	if _, err = reopened.trustOperate("commit", commit, token, now); err != nil {
		t.Fatal("session/replay lost after reopen", err)
	}
	if _, err = reopened.trustWorkloadLogin(context.Background(), cmd); err == nil {
		t.Fatal("assertion replay accepted after reopen")
	}
	before, _ := reopened.trustView()
	if len(before.(map[string]any)["ledger"].([]TrustLedgerEntry)) != 1 {
		t.Fatal("duplicate effect")
	}
	logout, _ := json.Marshal(trustWorkloadCommand{SessionToken: token})
	if _, err = reopened.trustWorkloadOperation(context.Background(), "workload-logout", logout); err != nil {
		t.Fatal(err)
	}
	if _, err = reopened.trustOperate("commit", commit, token, now); commandFailure(err).Code != "authorization_required" {
		t.Fatal("logout did not revoke session", err)
	}
}

func TestActionsConfianceT3WorkloadRejectsMalformedPublicKeys(t *testing.T) {
	for name, key := range map[string]jose.JSONWebKey{"nil-rsa": {Key: (*rsa.PublicKey)(nil), KeyID: "key", Algorithm: "RS256"}, "nil-ec": {Key: (*ecdsa.PublicKey)(nil), KeyID: "key", Algorithm: "ES256"}, "empty": {}, "empty-ec": {Key: &ecdsa.PublicKey{}, KeyID: "key", Algorithm: "ES256"}} {
		t.Run(name, func(t *testing.T) {
			if trustWorkloadPublicKeyValid(key) {
				t.Fatal("malformed key accepted")
			}
		})
	}
}

func TestActionsConfianceT3WorkloadRejectsInvalidAssertionsWithoutMutation(t *testing.T) {
	cases := map[string]func(map[string]any){
		"issuer": func(c map[string]any) { c["iss"] = "forged-client" }, "subject": func(c map[string]any) { c["sub"] = "alice" }, "audience": func(c map[string]any) { c["aud"] = "urn:wrong-token-endpoint" }, "missing-jti": func(c map[string]any) { delete(c, "jti") }, "missing-iat": func(c map[string]any) { delete(c, "iat") }, "future-iat": func(c map[string]any) { c["iat"] = time.Now().Add(time.Hour).Unix() }, "expired": func(c map[string]any) { c["exp"] = time.Now().Add(-time.Minute).Unix() }, "excessive-lifetime": func(c map[string]any) { c["exp"] = time.Now().Add(time.Hour).Unix() }, "future-nbf": func(c map[string]any) { c["nbf"] = time.Now().Add(time.Hour).Unix() }, "signature": nil, "unknown-key": nil,
	}
	for name, modify := range cases {
		t.Run(name, func(t *testing.T) {
			f := newWorkloadFixture(t)
			s := workloadConfiguredStore(t, f)
			if name == "signature" {
				f.clientKey, _ = ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
			}
			if name == "unknown-key" {
				f.clientKid = "unknown-key"
			}
			cmd := workloadAssertion(t, f, modify)
			path, _, _ := s.trustPaths()
			before, _ := os.ReadFile(path)
			if _, err := s.trustWorkloadLogin(context.Background(), cmd); commandFailure(err).Code != "authorization_required" {
				t.Fatal("invalid assertion authenticated", err)
			}
			after, _ := os.ReadFile(path)
			if !bytes.Equal(before, after) || f.exchanges != 0 {
				t.Fatal("invalid assertion mutated state or reached token endpoint")
			}
		})
	}
}

func TestActionsConfianceT3WorkloadRejectsInvalidAccessTokens(t *testing.T) {
	cases := map[string]func(map[string]any){
		"issuer": func(c map[string]any) { c["iss"] = "https://wrong.example" }, "subject": func(c map[string]any) { c["sub"] = "human-principal" }, "audience": func(c map[string]any) { c["aud"] = "another-api" }, "client": func(c map[string]any) { c["client_id"] = "another-client" }, "missing-jti": func(c map[string]any) { delete(c, "jti") }, "missing-iat": func(c map[string]any) { delete(c, "iat") }, "expired": func(c map[string]any) { c["exp"] = time.Now().Add(-time.Minute).Unix() }, "future-iat": func(c map[string]any) { c["iat"] = time.Now().Add(time.Hour).Unix() }, "future-nbf": func(c map[string]any) { c["nbf"] = time.Now().Add(time.Hour).Unix() }, "human-id-token": nil, "signature": nil, "missing-key-id": nil,
	}
	for name, modify := range cases {
		t.Run(name, func(t *testing.T) {
			f := newWorkloadFixture(t)
			f.modify = modify
			f.wrongSignature = name == "signature"
			if name == "human-id-token" {
				f.tokenType = "JWT"
			}
			if name == "missing-key-id" {
				f.issuerKid = ""
			}
			s := workloadConfiguredStore(t, f)
			cmd := workloadAssertion(t, f, nil)
			before, _ := s.trustView()
			beforeRaw, _ := json.Marshal(before)
			if _, err := s.trustWorkloadLogin(context.Background(), cmd); commandFailure(err).Code != "authorization_required" {
				t.Fatal("invalid access token authenticated", err)
			}
			after, _ := s.trustView()
			afterRaw, _ := json.Marshal(after)
			if !bytes.Equal(beforeRaw, afterRaw) {
				t.Fatal("invalid access token mutated business state")
			}
			st, _ := s.loadTrustState()
			if len(st.WorkloadSessions) != 0 || len(st.WorkloadUsed) != 1 {
				t.Fatal("invalid token created session or lost assertion consumption")
			}
			if _, err := s.trustWorkloadLogin(context.Background(), cmd); err == nil || f.exchanges != 1 {
				t.Fatal("failed token exchange assertion replayed", err)
			}
		})
	}
}

func TestActionsConfianceT3WorkloadPrivateKeysRejectedWhenDisabled(t *testing.T) {
	f := newWorkloadFixture(t)
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.Workloads = &TrustWorkloadConfig{Enabled: false, Clients: []TrustWorkloadClient{{ClientID: "service", Subject: "effect-service", PublicKeys: []jose.JSONWebKey{{Key: f.clientKey, KeyID: "private-key", Algorithm: "ES256"}}}}}
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	for _, preview := range []bool{true, false} {
		if _, err := s.trustConfig(c, preview); err == nil {
			t.Fatal("private key accepted in disabled configuration")
		}
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("rejected private key mutated state")
	}
}

func TestActionsConfianceT3WorkloadReplayConcurrencyAndRevocation(t *testing.T) {
	t.Run("single-exchange", func(t *testing.T) {
		f := newWorkloadFixture(t)
		s := workloadConfiguredStore(t, f)
		cmd := workloadAssertion(t, f, nil)
		results := make(chan error, 2)
		for i := 0; i < 2; i++ {
			go func() { _, err := s.trustWorkloadLogin(context.Background(), cmd); results <- err }()
		}
		successes := 0
		for i := 0; i < 2; i++ {
			if <-results == nil {
				successes++
			}
		}
		if successes != 1 || f.exchanges != 1 {
			t.Fatal("assertion exchanged more than once", successes, f.exchanges)
		}
	})
	t.Run("suspension-and-replay-survive-config", func(t *testing.T) {
		f := newWorkloadFixture(t)
		s := workloadConfiguredStore(t, f)
		cmd := workloadAssertion(t, f, nil)
		value, err := s.trustWorkloadLogin(context.Background(), cmd)
		if err != nil {
			t.Fatal(err)
		}
		token := value.(map[string]any)["session_token"].(string)
		if _, err = s.trustConfig(lifecycleChange(t, "effect-service", true, 2), false); err != nil {
			t.Fatal(err)
		}
		path, _, _ := s.trustPaths()
		before, _ := os.ReadFile(path)
		if _, err = s.trustOperate("commit", trustCmd("revoked-service"), token, time.Now()); commandFailure(err).Code != "authorization_required" {
			t.Fatal("suspended session authenticated", err)
		}
		after, _ := os.ReadFile(path)
		if !bytes.Equal(before, after) {
			t.Fatal("suspended operation mutated state")
		}
		if _, err = s.trustConfig(lifecycleChange(t, "effect-service", false, 3), false); err != nil {
			t.Fatal(err)
		}
		if _, err = s.trustWorkloadLogin(context.Background(), cmd); err == nil || f.exchanges != 1 {
			t.Fatal("config erased consumed assertion", err)
		}
		if _, err = s.trustWorkloadLogin(context.Background(), workloadAssertion(t, f, nil)); err != nil {
			t.Fatal("fresh assertion after reactivation rejected", err)
		}
	})
	t.Run("method-required", func(t *testing.T) {
		f := newWorkloadFixture(t)
		f.methods = []string{"client_secret_basic"}
		s := workloadConfiguredStore(t, f)
		if _, err := s.trustWorkloadLogin(context.Background(), workloadAssertion(t, f, nil)); err == nil || f.exchanges != 0 {
			t.Fatal("unannounced private_key_jwt accepted", err)
		}
	})
	t.Run("key-rotation", func(t *testing.T) {
		f := newWorkloadFixture(t)
		s := workloadConfiguredStore(t, f)
		first := workloadAssertion(t, f, nil)
		if _, err := s.trustWorkloadLogin(context.Background(), first); err != nil {
			t.Fatal(err)
		}
		f.clientKey, _ = ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
		f.clientKid = "rotated-client"
		f.issuerKey, _ = ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
		f.issuerKid = "rotated-issuer"
		st, _ := s.loadTrustState()
		c := lifecycleChange(t, "alice", false, 2)
		cfg := *st.Workloads
		cfg.Clients = append([]TrustWorkloadClient{}, cfg.Clients...)
		cfg.Clients[0].PublicKeys = []jose.JSONWebKey{{Key: &f.clientKey.PublicKey, KeyID: f.clientKid, Algorithm: "ES256"}}
		c.Workloads = &cfg
		if _, err := s.trustConfig(c, false); err != nil {
			t.Fatal(err)
		}
		if _, err := s.trustWorkloadLogin(context.Background(), first); err == nil {
			t.Fatal("removed key accepted")
		}
		if _, err := s.trustWorkloadLogin(context.Background(), workloadAssertion(t, f, nil)); err != nil {
			t.Fatal("rotated keys rejected", err)
		}
	})
}

func TestActionsConfianceT3WorkloadConfigurationAndBoolean(t *testing.T) {
	for _, value := range []string{`null`, `"true"`, `1`, `{}`, `[]`} {
		t.Run(value, func(t *testing.T) {
			var c TrustWorkloadConfig
			if strict([]byte(`{"enabled":`+value+`}`), &c) == nil {
				t.Fatal("nonboolean enabled accepted")
			}
		})
	}
	f := newWorkloadFixture(t)
	s := workloadConfiguredStore(t, f)
	st, _ := s.loadTrustState()
	cases := map[string]func(*TrustWorkloadConfig){"zero-timeout": func(c *TrustWorkloadConfig) { c.RequestTimeoutSeconds = 0 }, "zero-assertion": func(c *TrustWorkloadConfig) { c.AssertionTTLSeconds = 0 }, "negative-session": func(c *TrustWorkloadConfig) { c.SessionTTLSeconds = -1 }, "duplicate-client": func(c *TrustWorkloadConfig) { c.Clients = append(c.Clients, c.Clients[0]) }, "unknown-subject": func(c *TrustWorkloadConfig) { c.Clients[0].Subject = "unknown" }, "missing-kid": func(c *TrustWorkloadConfig) { c.Clients[0].PublicKeys[0].KeyID = "" }, "private-key": func(c *TrustWorkloadConfig) { c.Clients[0].PublicKeys[0].Key = f.clientKey }}
	for name, modify := range cases {
		t.Run(name, func(t *testing.T) {
			raw, _ := json.Marshal(st.Workloads)
			var cfg TrustWorkloadConfig
			if json.Unmarshal(raw, &cfg) != nil {
				t.Fatal("fixture config unreadable")
			}
			modify(&cfg)
			c := lifecycleChange(t, "alice", false, 2)
			c.Workloads = &cfg
			path, _, _ := s.trustPaths()
			before, _ := os.ReadFile(path)
			for _, preview := range []bool{true, false} {
				if _, err := s.trustConfig(c, preview); err == nil {
					t.Fatal("invalid configuration accepted")
				}
			}
			after, _ := os.ReadFile(path)
			if !bytes.Equal(before, after) {
				t.Fatal("invalid config mutated state")
			}
		})
	}
}

func TestActionsConfianceT3WorkloadInflightConfigurationChange(t *testing.T) {
	f := newWorkloadFixture(t)
	f.tokenEntered = make(chan struct{})
	f.tokenContinue = make(chan struct{})
	s := workloadConfiguredStore(t, f)
	cmd := workloadAssertion(t, f, nil)
	done := make(chan error, 1)
	go func() { _, err := s.trustWorkloadLogin(context.Background(), cmd); done <- err }()
	select {
	case <-f.tokenEntered:
	case <-time.After(5 * time.Second):
		t.Fatal("token exchange did not start")
	}
	if _, err := s.trustConfig(lifecycleChange(t, "effect-service", true, 2), false); err != nil {
		close(f.tokenContinue)
		t.Fatal(err)
	}
	close(f.tokenContinue)
	if err := <-done; commandFailure(err).Code != "authorization_required" {
		t.Fatal("configuration change did not revoke inflight login", err)
	}
	st, _ := s.loadTrustState()
	if len(st.WorkloadSessions) != 0 || len(st.WorkloadUsed) != 1 {
		t.Fatal("inflight login published or erased consumed assertion")
	}
}

func TestActionsConfianceT3WorkloadPublicAPIAndCLI(t *testing.T) {
	f := newWorkloadFixture(t)
	s := workloadConfiguredStore(t, f)
	h := newWebHandler(s, "localhost:18787", "workload-operator")
	cmd := workloadAssertion(t, f, nil)
	body, _ := json.Marshal(cmd)
	call := func(auth bool) *httptest.ResponseRecorder {
		r := httptest.NewRequest("POST", "http://localhost:18787/api/v1/trust-actions?action=workload-login", bytes.NewReader(body))
		r.Header.Set("Origin", "http://localhost:18787")
		r.Header.Set("X-Swarm-CSRF", "workload-operator")
		if auth {
			r.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte("localhost:18787"))[:12], Value: "workload-operator"})
		}
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		return w
	}
	if w := call(false); w.Code != 403 || f.exchanges != 0 {
		t.Fatal("operator session bypassed", w.Code)
	}
	w := call(true)
	if w.Code != 200 || w.Header().Get("Cache-Control") != "no-store" {
		t.Fatal("public workload API failed", w.Code, w.Body.String())
	}
	var result struct {
		SessionToken string `json:"session_token"`
		Subject      string `json:"subject"`
	}
	if json.Unmarshal(w.Body.Bytes(), &result) != nil || result.Subject != "effect-service" {
		t.Fatal("wrong API identity")
	}
	st, _ := s.loadTrustState()
	if _, err := findTrustIdentityAt(&st, result.SessionToken, time.Now().Add(time.Hour)); err == nil {
		t.Fatal("expired service session authenticated")
	}
	input := filepath.Join(t.TempDir(), "logout.json")
	raw, _ := json.Marshal(trustWorkloadCommand{SessionToken: result.SessionToken})
	if os.WriteFile(input, raw, 0600) != nil {
		t.Fatal("fixture input unavailable")
	}
	var out bytes.Buffer
	if err := s.trustActionsCLI([]string{"trust", "workload-logout"}, input, &out); err != nil || !strings.Contains(out.String(), `"logged_out": true`) {
		t.Fatal("CLI logout failed", err)
	}
}

func TestActionsConfianceT3WorkloadDeadlineAndPreview(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { <-r.Context().Done() }))
	defer server.Close()
	f := newWorkloadFixture(t)
	s := workloadConfiguredStore(t, f)
	st, _ := s.loadTrustState()
	c := lifecycleChange(t, "alice", false, 2)
	config := *st.Workloads
	config.Issuer = server.URL
	config.RequestTimeoutSeconds = 1
	c.Workloads = &config
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	if _, err := s.trustConfig(c, true); err != nil {
		t.Fatal("preview attempted network", err)
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("preview mutated state")
	}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	cmd := workloadAssertion(t, f, nil)
	before, _ = os.ReadFile(path)
	start := time.Now()
	if _, err := s.trustWorkloadLogin(context.Background(), cmd); err == nil {
		t.Fatal("stalled issuer accepted")
	}
	elapsed := time.Since(start)
	if elapsed < 900*time.Millisecond || elapsed > 3*time.Second {
		t.Fatal("configured timeout not enforced", elapsed)
	}
	after, _ = os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("failed discovery mutated state")
	}
}
