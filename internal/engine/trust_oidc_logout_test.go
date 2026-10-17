//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	jose "github.com/go-jose/go-jose/v4"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

func TestActionsConfianceT3OIDCLogoutPolicySurface(t *testing.T) {
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.OIDC = &TrustOIDCConfig{Enabled: true, Issuer: "http://127.0.0.1:1", ClientID: "swarm", RedirectURI: "http://127.0.0.1/callback", RequestTimeoutSeconds: 5, LoginTTLSeconds: 300, SessionTTLSeconds: 600, Mappings: []TrustOIDCMapping{{ExternalSubject: "employee-1", Subject: "alice"}}}
	raw, _ := json.Marshal(c)
	var input map[string]any
	json.Unmarshal(raw, &input)
	input["oidc_logout"] = map[string]any{"enabled": true, "max_age_seconds": 300, "replay_retention_seconds": 600}
	raw, _ = json.Marshal(input)
	var decoded TrustConfigChange
	if err := strict(raw, &decoded); err != nil {
		t.Fatal("provider logout policy absent", err)
	}
	before, _ := s.loadTrustState()
	v, err := s.trustConfig(decoded, true)
	if err != nil {
		t.Fatal(err)
	}
	after, _ := s.loadTrustState()
	b, _ := json.Marshal(before)
	a, _ := json.Marshal(after)
	if string(b) != string(a) || v.(map[string]any)["oidc_logout"] == nil {
		t.Fatal("logout preview mutated state or omitted policy")
	}
}

func oidcLogoutFixture(t *testing.T) (*Store, *oidcFixture) {
	t.Helper()
	f := newOIDCFixture(t)
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.OIDC = &TrustOIDCConfig{Enabled: true, Issuer: f.server.URL, ClientID: "swarm", RedirectURI: "http://127.0.0.1/callback", RequestTimeoutSeconds: 5, LoginTTLSeconds: 300, SessionTTLSeconds: 600, Mappings: []TrustOIDCMapping{{ExternalSubject: "employee-1", Subject: "alice"}, {ExternalSubject: "employee-2", Subject: "bob"}}}
	c.OIDCLogout = &TrustOIDCLogoutPolicy{Enabled: true, MaxAgeSeconds: 300, ReplayRetentionSeconds: 600}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	return s, f
}

func oidcLogoutLogin(t *testing.T, s *Store, f *oidcFixture, sub, sid string) string {
	t.Helper()
	f.mu.Lock()
	f.modify = func(c map[string]any) {
		c["sub"] = sub
		c["sid"] = sid
		c["auth_time"] = time.Now().Unix()
		c["acr"] = "urn:example:strong"
	}
	f.mu.Unlock()
	v, err := s.trustOIDCFinish(context.Background(), oidcAuthorize(t, s))
	if err != nil {
		t.Fatal(err)
	}
	return v.(map[string]any)["session_token"].(string)
}

func oidcLogoutJWT(t *testing.T, f *oidcFixture, mutate func(map[string]any), typ string) string {
	t.Helper()
	now := time.Now().Unix()
	c := map[string]any{"iss": f.server.URL, "aud": "swarm", "iat": now, "exp": now + 300, "jti": newID("logout-"), "events": map[string]any{trustOIDCLogoutEvent: map[string]any{}}, "sub": "employee-1"}
	if mutate != nil {
		mutate(c)
	}
	opts := (&jose.SignerOptions{}).WithHeader("kid", f.kid)
	if typ != "" {
		opts = opts.WithType(jose.ContentType(typ))
	}
	signer, err := jose.NewSigner(jose.SigningKey{Algorithm: jose.ES256, Key: f.key}, opts)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(c)
	signed, err := signer.Sign(raw)
	if err != nil {
		t.Fatal(err)
	}
	token, err := signed.CompactSerialize()
	if err != nil {
		t.Fatal(err)
	}
	return token
}

func TestActionsConfianceT3OIDCLogoutSessionAndSubject(t *testing.T) {
	s, f := oidcLogoutFixture(t)
	one := oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
	two := oidcLogoutLogin(t, s, f, "employee-1", "sid-two")
	other := oidcLogoutLogin(t, s, f, "employee-2", "sid-other")
	before, _ := s.loadTrustState()
	token := oidcLogoutJWT(t, f, func(c map[string]any) { c["sid"] = "sid-one" }, "logout+jwt")
	v, err := s.trustOIDCBackchannelLogout(context.Background(), token)
	if err != nil || v.(map[string]any)["revoked_sessions"] != 1 {
		t.Fatal("session logout failed", err)
	}
	st, _ := s.loadTrustState()
	if _, err := findTrustIdentity(&st, one); err == nil {
		t.Fatal("revoked session remains authenticated")
	}
	for _, kept := range []string{two, other} {
		if _, err := findTrustIdentity(&st, kept); err != nil {
			t.Fatal("unrelated session removed", err)
		}
	}
	if !reflect.DeepEqual(before.Actions, st.Actions) || !reflect.DeepEqual(before.Mandates, st.Mandates) || !reflect.DeepEqual(before.Ledger, st.Ledger) || !reflect.DeepEqual(before.Evidence, st.Evidence) {
		t.Fatal("logout mutated business objects")
	}
	token = oidcLogoutJWT(t, f, nil, "JWT")
	v, err = s.trustOIDCBackchannelLogout(context.Background(), token)
	if err != nil || v.(map[string]any)["revoked_sessions"] != 1 {
		t.Fatal("subject logout failed", err)
	}
	st, _ = s.loadTrustState()
	if _, err := findTrustIdentity(&st, two); err == nil {
		t.Fatal("subject session retained")
	}
	if _, err := findTrustIdentity(&st, other); err != nil {
		t.Fatal("other subject session removed")
	}
}

func TestActionsConfianceT3OIDCLogoutRejectsInvalidTokensWithoutMutation(t *testing.T) {
	cases := map[string]func(map[string]any){"issuer": func(c map[string]any) { c["iss"] = "https://other.invalid" }, "audience": func(c map[string]any) { c["aud"] = "other" }, "expired": func(c map[string]any) { c["exp"] = time.Now().Add(-time.Minute).Unix() }, "future-iat": func(c map[string]any) { c["iat"] = time.Now().Add(time.Minute).Unix() }, "old-iat": func(c map[string]any) { c["iat"] = time.Now().Add(-time.Hour).Unix() }, "no-exp": func(c map[string]any) { delete(c, "exp") }, "no-jti": func(c map[string]any) { delete(c, "jti") }, "nonce": func(c map[string]any) { c["nonce"] = "" }, "no-events": func(c map[string]any) { delete(c, "events") }, "wrong-event": func(c map[string]any) { c["events"] = map[string]any{"other": map[string]any{}} }, "null-event": func(c map[string]any) { c["events"] = map[string]any{trustOIDCLogoutEvent: nil} }, "no-sub-or-sid": func(c map[string]any) { delete(c, "sub") }, "empty-sid": func(c map[string]any) { c["sid"] = "" }, "empty-sub": func(c map[string]any) { c["sub"] = ""; c["sid"] = "sid-one" }, "null-sub": func(c map[string]any) { c["sub"] = nil; c["sid"] = "sid-one" }, "future-nbf": func(c map[string]any) { c["nbf"] = time.Now().Add(time.Minute).Unix() }, "multiple-aud-without-azp": func(c map[string]any) { c["aud"] = []string{"swarm", "other"} }, "wrong-azp": func(c map[string]any) { c["azp"] = "other" }, "empty-azp": func(c map[string]any) { c["azp"] = "" }}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			s, f := oidcLogoutFixture(t)
			oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
			token := oidcLogoutJWT(t, f, mutate, "logout+jwt")
			before, _ := s.loadTrustState()
			if _, err := s.trustOIDCBackchannelLogout(context.Background(), token); err == nil {
				t.Fatal("invalid logout token accepted")
			}
			after, _ := s.loadTrustState()
			if !reflect.DeepEqual(before, after) {
				t.Fatal("invalid logout mutated state")
			}
		})
	}
}

func TestActionsConfianceT3OIDCLogoutWrongSignatureTypeAndUnknownSession(t *testing.T) {
	for _, variant := range []string{"signature", "access-token-type", "unknown-sid", "wrong-sub-for-sid", "typ-absent"} {
		t.Run(variant, func(t *testing.T) {
			s, f := oidcLogoutFixture(t)
			one := oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
			typ := "logout+jwt"
			if variant == "access-token-type" {
				typ = "at+jwt"
			}
			if variant == "typ-absent" {
				typ = ""
			}
			token := oidcLogoutJWT(t, f, func(c map[string]any) {
				if variant == "unknown-sid" {
					c["sid"] = "unknown"
				}
				if variant == "wrong-sub-for-sid" {
					c["sid"] = "sid-one"
					c["sub"] = "employee-2"
				}
			}, typ)
			if variant == "signature" {
				parts := strings.Split(token, ".")
				prefix := "A"
				if parts[2][0] == 'A' {
					prefix = "B"
				}
				parts[2] = prefix + parts[2][1:]
				token = strings.Join(parts, ".")
			}
			before, _ := s.loadTrustState()
			v, err := s.trustOIDCBackchannelLogout(context.Background(), token)
			after, _ := s.loadTrustState()
			if variant == "signature" || variant == "access-token-type" {
				if err == nil || !reflect.DeepEqual(before, after) {
					t.Fatal("invalid signature/type changed state")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			want := 0
			if variant == "typ-absent" {
				want = 1
			}
			if v.(map[string]any)["revoked_sessions"] != want {
				t.Fatal("wrong sessions revoked")
			}
			_, active := after.OIDCSessions[trustTokenHash(one)]
			if active != (want == 0) {
				t.Fatal("session selection mismatch")
			}
		})
	}
}

func TestActionsConfianceT3OIDCLogoutReplayPersistenceAndConfiguration(t *testing.T) {
	s, f := oidcLogoutFixture(t)
	oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
	token := oidcLogoutJWT(t, f, nil, "logout+jwt")
	if _, err := s.trustOIDCBackchannelLogout(context.Background(), token); err != nil {
		t.Fatal(err)
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	c := lifecycleChange(t, "alice", false, 2)
	c.OIDCLogout = &TrustOIDCLogoutPolicy{Enabled: true, MaxAgeSeconds: 900, ReplayRetentionSeconds: 900}
	if _, err = reopened.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	fresh := oidcLogoutLogin(t, reopened, f, "employee-1", "sid-one")
	before, _ := reopened.loadTrustState()
	if _, err = reopened.trustOIDCBackchannelLogout(context.Background(), token); err == nil {
		t.Fatal("old logout replayed after reconfiguration")
	}
	after, _ := reopened.loadTrustState()
	if !reflect.DeepEqual(before, after) {
		t.Fatal("replay changed state")
	}
	if _, err := findTrustIdentity(&after, fresh); err != nil {
		t.Fatal("replay revoked a new session")
	}
}

func TestActionsConfianceT3OIDCLogoutAnonymousHTTPAndCLI(t *testing.T) {
	s, f := oidcLogoutFixture(t)
	one := oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
	h := newWebHandler(s, "localhost:8080", "operator-token")
	token := oidcLogoutJWT(t, f, nil, "logout+jwt")
	r := httptest.NewRequest(http.MethodPost, "http://localhost:8080"+trustOIDCLogoutPath, strings.NewReader(url.Values{"logout_token": {token}, "ignored": {"value"}}.Encode()))
	r.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	w := httptest.NewRecorder()
	h.ServeHTTP(w, r)
	if w.Code != 200 {
		t.Fatal("signed callback failed without operator session", w.Code)
	}
	st, _ := s.loadTrustState()
	if _, err := findTrustIdentity(&st, one); err == nil {
		t.Fatal("HTTP logout had no effect")
	}
	for _, variant := range []string{"host", "get", "bad-token", "duplicate-token", "operator-api-still-private"} {
		t.Run(variant, func(t *testing.T) {
			target := "http://localhost:8080" + trustOIDCLogoutPath
			method := http.MethodPost
			body := "logout_token=bad"
			if variant == "host" {
				target = "http://other:8080" + trustOIDCLogoutPath
			}
			if variant == "get" {
				method = http.MethodGet
			}
			if variant == "duplicate-token" {
				body = "logout_token=bad&logout_token=bad"
			}
			if variant == "operator-api-still-private" {
				target = "http://localhost:8080/api/v1/trust-actions?action=oidc-backchannel-logout"
				body = `{"logout_token":"bad"}`
			}
			before, _ := s.loadTrustState()
			r := httptest.NewRequest(method, target, strings.NewReader(body))
			r.Header.Set("Content-Type", "application/x-www-form-urlencoded")
			w := httptest.NewRecorder()
			h.ServeHTTP(w, r)
			if w.Code < 400 {
				t.Fatal("invalid callback admitted")
			}
			after, _ := s.loadTrustState()
			if !reflect.DeepEqual(before, after) {
				t.Fatal("invalid callback mutated state")
			}
		})
	}
	input := filepath.Join(t.TempDir(), "logout.json")
	raw, _ := json.Marshal(trustOIDCLogoutCommand{LogoutToken: oidcLogoutJWT(t, f, nil, "logout+jwt")})
	os.WriteFile(input, raw, 0600)
	var out bytes.Buffer
	if err := s.trustActionsCLI([]string{"trust", "oidc-backchannel-logout"}, input, &out); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(out.String(), `"logged_out": true`) {
		t.Fatal("CLI omitted logout result")
	}
}

func TestActionsConfianceT3OIDCLogoutPolicyValidation(t *testing.T) {
	for _, p := range []TrustOIDCLogoutPolicy{{MaxAgeSeconds: 0, ReplayRetentionSeconds: 600}, {MaxAgeSeconds: -1, ReplayRetentionSeconds: 600}, {MaxAgeSeconds: 300, ReplayRetentionSeconds: 299}, {MaxAgeSeconds: 300, ReplayRetentionSeconds: -1}} {
		if validateTrustOIDCLogout(&p, nil) == nil {
			t.Fatal("invalid logout policy accepted")
		}
	}
	p := TrustOIDCLogoutPolicy{Enabled: true, MaxAgeSeconds: 300, ReplayRetentionSeconds: 600}
	if validateTrustOIDCLogout(&p, nil) == nil {
		t.Fatal("logout without OIDC accepted")
	}
	for _, raw := range []string{`{}`, `{"enabled":null,"max_age_seconds":300,"replay_retention_seconds":600}`, `{"enabled":false,"max_age_seconds":null,"replay_retention_seconds":600}`, `{"enabled":false,"max_age_seconds":300,"replay_retention_seconds":null}`, `{"enabled":false,"max_age_seconds":1.5,"replay_retention_seconds":600}`} {
		var p TrustOIDCLogoutPolicy
		if strict([]byte(raw), &p) == nil {
			t.Fatal("malformed logout policy decoded")
		}
	}
}

func TestActionsConfianceT3OIDCLogoutRetentionCoversSignedExpiry(t *testing.T) {
	for _, retain := range []int{0, 600} {
		t.Run(fmt.Sprint(retain), func(t *testing.T) {
			s, f := oidcLogoutFixture(t)
			c := lifecycleChange(t, "alice", false, 2)
			c.OIDCLogout = &TrustOIDCLogoutPolicy{Enabled: true, MaxAgeSeconds: 300, ReplayRetentionSeconds: retain}
			if _, err := s.trustConfig(c, false); err != nil {
				t.Fatal(err)
			}
			expiry := time.Now().Add(time.Hour).Unix()
			token := oidcLogoutJWT(t, f, func(c map[string]any) { c["exp"] = expiry }, "logout+jwt")
			if _, err := s.trustOIDCBackchannelLogout(context.Background(), token); err != nil {
				t.Fatal(err)
			}
			st, _ := s.loadTrustState()
			if len(st.OIDCLogoutUsed) != 1 {
				t.Fatal("missing replay record")
			}
			for _, until := range st.OIDCLogoutUsed {
				if retain == 0 && !until.IsZero() || retain > 0 && until.Before(time.Unix(expiry, 0)) {
					t.Fatal("replay guard expires before signed token")
				}
			}
		})
	}
}

func TestActionsConfianceT3OIDCLogoutRevokesStepUpAndKeepsEffect(t *testing.T) {
	s, f, _ := stepUpFixture(t)
	c := lifecycleChange(t, "alice", false, 2)
	c.OIDCLogout = &TrustOIDCLogoutPolicy{Enabled: true, MaxAgeSeconds: 300, ReplayRetentionSeconds: 600}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	now := time.Now().UTC()
	createMandate(t, s, now, "logout-step-up-root", now.Add(time.Hour))
	a := createAction(t, s, now, "logout-step-up-request", "logout-step-up-root", "logout-step-up-key", 100)
	f.mu.Lock()
	f.modify = func(c map[string]any) {
		c["auth_time"] = time.Now().Unix()
		c["acr"] = "urn:example:strong"
		c["sid"] = "step-up-sid"
	}
	f.mu.Unlock()
	v, err := s.trustOIDCFinish(context.Background(), stepUpAuthorize(t, s, a))
	if err != nil {
		t.Fatal(err)
	}
	token := v.(map[string]any)["session_token"].(string)
	if _, err = s.trustOperate("approve", stepUpApproveCommand(a, "logout-step-up-approve"), token, time.Now().UTC()); err != nil {
		t.Fatal(err)
	}
	commit := trustCmd("logout-step-up-commit")
	commit.ActionID = a.ID
	if _, err = s.trustOperate("commit", commit, trustEffectToken, time.Now().UTC()); err != nil {
		t.Fatal(err)
	}
	logout := oidcLogoutJWT(t, f, func(c map[string]any) { c["sid"] = "step-up-sid" }, "logout+jwt")
	if _, err = s.trustOIDCBackchannelLogout(context.Background(), logout); err != nil {
		t.Fatal(err)
	}
	if _, err = s.trustOperate("approve", stepUpApproveCommand(a, "logout-step-up-reuse"), token, time.Now().UTC()); commandFailure(err).Code != "authorization_required" {
		t.Fatal("revoked proof retained authority")
	}
	st, _ := s.loadTrustState()
	if len(st.Ledger) != 1 || st.Actions[a.ID].State != "committed" {
		t.Fatal("provider logout erased effect")
	}
}

func TestActionsConfianceT3OIDCLogoutConcurrentDelivery(t *testing.T) {
	s, f := oidcLogoutFixture(t)
	oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
	token := oidcLogoutJWT(t, f, nil, "logout+jwt")
	start := make(chan struct{})
	out := make(chan error, 2)
	for i := 0; i < 2; i++ {
		go func() { <-start; _, err := s.trustOIDCBackchannelLogout(context.Background(), token); out <- err }()
	}
	close(start)
	success, rejected := 0, 0
	for i := 0; i < 2; i++ {
		if err := <-out; err == nil {
			success++
		} else {
			rejected++
		}
	}
	if success != 1 || rejected != 1 {
		t.Fatal("concurrent logout was not applied exactly once")
	}
	st, _ := s.loadTrustState()
	if len(st.OIDCLogoutUsed) != 1 || len(st.OIDCSessions) != 0 {
		t.Fatal("concurrent logout lost replay guard or session removal")
	}
}

func TestActionsConfianceT3OIDCLogoutKeepsWorkloadSession(t *testing.T) {
	s, f := oidcLogoutFixture(t)
	wf := newWorkloadFixture(t)
	c := lifecycleChange(t, "alice", false, 2)
	c.Workloads = &TrustWorkloadConfig{Enabled: true, Issuer: wf.server.URL, Audience: "urn:swarm:trust", RequestTimeoutSeconds: 5, AssertionTTLSeconds: 300, SessionTTLSeconds: 600, Clients: []TrustWorkloadClient{{ClientID: "settlement-service", ExternalSubject: "service-principal", Subject: "effect-service", PublicKeys: []jose.JSONWebKey{{Key: &wf.clientKey.PublicKey, KeyID: wf.clientKid, Algorithm: "ES256", Use: "sig"}}}}}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	v, err := s.trustWorkloadLogin(context.Background(), workloadAssertion(t, wf, nil))
	if err != nil {
		t.Fatal(err)
	}
	token := v.(map[string]any)["session_token"].(string)
	oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
	before, _ := s.loadTrustState()
	if _, err = s.trustOIDCBackchannelLogout(context.Background(), oidcLogoutJWT(t, f, nil, "logout+jwt")); err != nil {
		t.Fatal(err)
	}
	after, _ := s.loadTrustState()
	id, err := findTrustIdentity(&after, token)
	if err != nil || id.Subject != "effect-service" || !reflect.DeepEqual(before.WorkloadSessions, after.WorkloadSessions) || !reflect.DeepEqual(before.WorkloadUsed, after.WorkloadUsed) {
		t.Fatal("human logout changed service authority or consumption", err)
	}
}

func TestActionsConfianceT3OIDCLogoutFencesEarlierLogin(t *testing.T) {
	for _, variant := range []string{"sid-and-sub", "sid-only", "sub-only", "other-subject", "other-sid"} {
		t.Run(variant, func(t *testing.T) {
			s, f := oidcLogoutFixture(t)
			sub, sid := "employee-1", "sid-one"
			if variant == "other-subject" {
				sub = "employee-2"
			}
			if variant == "other-sid" {
				sid = "sid-two"
			}
			f.mu.Lock()
			f.modify = func(c map[string]any) { c["sub"] = sub; c["sid"] = sid }
			f.mu.Unlock()
			pending := oidcAuthorize(t, s)
			logout := oidcLogoutJWT(t, f, func(c map[string]any) {
				if variant != "sub-only" {
					c["sid"] = "sid-one"
				}
				if variant == "sid-only" {
					delete(c, "sub")
				}
			}, "logout+jwt")
			if _, err := s.trustOIDCBackchannelLogout(context.Background(), logout); err != nil {
				t.Fatal(err)
			}
			_, err := s.trustOIDCFinish(context.Background(), pending)
			kept := variant == "other-subject" || variant == "other-sid"
			if (err == nil) != kept {
				t.Fatal("in-flight logout fence selected the wrong connection", err)
			}
			fresh := oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
			st, _ := s.loadTrustState()
			if _, err := findTrustIdentity(&st, fresh); err != nil {
				t.Fatal("new login after logout was blocked", err)
			}
		})
	}
}

func TestActionsConfianceT3OIDCLogoutDuringTokenExchange(t *testing.T) {
	f := newOIDCFixture(t)
	entered, release := make(chan struct{}), make(chan struct{})
	baseHandler := f.server.Config.Handler
	var proxy *httptest.Server
	proxy = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/.well-known/openid-configuration" {
			json.NewEncoder(w).Encode(map[string]any{"issuer": proxy.URL, "authorization_endpoint": proxy.URL + "/authorize", "token_endpoint": proxy.URL + "/token", "jwks_uri": proxy.URL + "/keys", "id_token_signing_alg_values_supported": []string{"ES256"}})
			return
		}
		if r.URL.Path == "/token" {
			close(entered)
			<-release
		}
		baseHandler.ServeHTTP(w, r)
	}))
	defer proxy.Close()
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.OIDC = &TrustOIDCConfig{Enabled: true, Issuer: proxy.URL, ClientID: "swarm", RedirectURI: "http://127.0.0.1/callback", RequestTimeoutSeconds: 5, LoginTTLSeconds: 300, SessionTTLSeconds: 600, Mappings: []TrustOIDCMapping{{ExternalSubject: "employee-1", Subject: "alice"}}}
	c.OIDCLogout = &TrustOIDCLogoutPolicy{Enabled: true, MaxAgeSeconds: 300, ReplayRetentionSeconds: 600}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	f.mu.Lock()
	f.modify = func(c map[string]any) { c["iss"] = proxy.URL; c["sid"] = "sid-one" }
	f.mu.Unlock()
	pending := oidcAuthorize(t, s)
	done := make(chan error, 1)
	go func() { _, err := s.trustOIDCFinish(context.Background(), pending); done <- err }()
	select {
	case <-entered:
	case <-time.After(3 * time.Second):
		close(release)
		t.Fatal("token exchange not entered")
	}
	token := oidcLogoutJWT(t, f, func(c map[string]any) { c["iss"] = proxy.URL; c["sid"] = "sid-one" }, "logout+jwt")
	_, err := s.trustOIDCBackchannelLogout(context.Background(), token)
	close(release)
	if err != nil {
		t.Fatal(err)
	}
	if err := <-done; err == nil {
		t.Fatal("token exchange recreated session after logout")
	}
	st, _ := s.loadTrustState()
	if len(st.OIDCSessions) != 0 {
		t.Fatal("in-flight revoked session persisted")
	}
}

func TestActionsConfianceT3OIDCLogoutFormContentTypeCase(t *testing.T) {
	for _, contentType := range []string{"Application/X-WWW-Form-Urlencoded; charset=UTF-8", "application/x-www-form-urlencoded ; charset=utf-8"} {
		t.Run(contentType, func(t *testing.T) {
			s, f := oidcLogoutFixture(t)
			one := oidcLogoutLogin(t, s, f, "employee-1", "sid-one")
			token := oidcLogoutJWT(t, f, nil, "logout+jwt")
			h := newWebHandler(s, "localhost:8080", "operator-token")
			r := httptest.NewRequest(http.MethodPost, "http://localhost:8080"+trustOIDCLogoutPath, strings.NewReader(url.Values{"logout_token": {token}}.Encode()))
			r.Header.Set("Content-Type", contentType)
			w := httptest.NewRecorder()
			h.ServeHTTP(w, r)
			if w.Code != 200 {
				t.Fatal("valid MIME type rejected", w.Code)
			}
			st, _ := s.loadTrustState()
			if _, err := findTrustIdentity(&st, one); err == nil {
				t.Fatal("accepted MIME callback did not revoke session")
			}
		})
	}
}
