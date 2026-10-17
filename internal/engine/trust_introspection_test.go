//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	jose "github.com/go-jose/go-jose/v4"
	"github.com/go-jose/go-jose/v4/jwt"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

func TestActionsConfianceT3IntrospectionPolicyContract(t *testing.T) {
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	raw, _ := json.Marshal(c)
	var fields map[string]any
	if err := json.Unmarshal(raw, &fields); err != nil {
		t.Fatal(err)
	}
	fields["workload_introspection"] = map[string]any{"enabled": false, "endpoint": "", "client_id": "", "timeout_seconds": 15, "validation_window_seconds": 5, "max_sessions": 64, "max_token_bytes": 16384}
	raw, _ = json.Marshal(fields)
	var change TrustConfigChange
	if err := strict(raw, &change); err != nil {
		t.Fatalf("introspection policy contract absent: %v", err)
	}
	if _, err := s.trustConfig(change, true); err != nil {
		t.Fatal(err)
	}
}

func introspectionConfigured(t *testing.T, modify ...func(*TrustIntrospectionPolicy)) (*Store, *workloadFixture, string, *atomic.Bool, *atomic.Int64) {
	t.Helper()
	f := newWorkloadFixture(t)
	s := workloadConfiguredStore(t, f)
	c := lifecycleChange(t, "alice", false, 2)
	p := trustIntrospectionPolicyValue(nil)
	p.Enabled = true
	p.Endpoint = f.server.URL + "/introspect"
	p.ClientID = "swarm-resource"
	for _, fn := range modify {
		fn(&p)
	}
	c.WorkloadIntrospection = &p
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	keyValue, err := s.trustIntrospectionKey(trustAdminToken)
	if err != nil {
		t.Fatal(err)
	}
	key := keyValue.(map[string]any)["public_key"].(jose.JSONWebKey)
	active := &atomic.Bool{}
	active.Store(true)
	calls := &atomic.Int64{}
	old := f.server.Config.Handler
	f.server.Config.Handler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/introspect" {
			old.ServeHTTP(w, r)
			return
		}
		calls.Add(1)
		if r.Method != "POST" || r.Header.Get("Content-Type") != "application/x-www-form-urlencoded" {
			t.Error("wrong introspection transport")
			http.Error(w, "bad transport", 400)
			return
		}
		r.ParseForm()
		signed, err := jose.ParseSigned(r.Form.Get("client_assertion"), []jose.SignatureAlgorithm{jose.ES256})
		if err != nil {
			t.Error("missing distinct client assertion")
			http.Error(w, "auth", 401)
			return
		}
		payload, err := signed.Verify(key.Key)
		var auth jwt.Claims
		if err != nil || json.Unmarshal(payload, &auth) != nil || auth.ValidateWithLeeway(jwt.Expected{Issuer: p.ClientID, Subject: p.ClientID, AnyAudience: jwt.Audience{p.Endpoint}, Time: time.Now()}, 0) != nil || auth.ID == "" || r.Form.Get("client_id") != p.ClientID {
			t.Error("bad resource authentication")
			http.Error(w, "auth", 401)
			return
		}
		token, err := jose.ParseSigned(r.Form.Get("token"), []jose.SignatureAlgorithm{jose.ES256})
		if err != nil {
			t.Error("missing access token")
			return
		}
		payload, err = token.Verify(&f.issuerKey.PublicKey)
		if err != nil {
			t.Error("wrong access token")
			return
		}
		var claims map[string]any
		json.Unmarshal(payload, &claims)
		claims["active"] = active.Load()
		json.NewEncoder(w).Encode(claims)
	})
	v, err := s.trustWorkloadLogin(context.Background(), workloadAssertion(t, f, nil))
	if err != nil {
		t.Fatal(err)
	}
	return s, f, v.(map[string]any)["session_token"].(string), active, calls
}

func TestActionsConfianceT3IntrospectionActiveRevokedAndRestart(t *testing.T) {
	s, _, token, active, calls := introspectionConfigured(t)
	scope, err := s.trustServiceScope(context.Background(), token)
	if err != nil {
		t.Fatal(err)
	}
	v, err := scope.trustIdentityContext([]byte("{}"), token, time.Time{})
	if err != nil || v.(*TrustIdentityContext).PrincipalKind != "service" {
		t.Fatalf("active service denied: %v", err)
	}
	if calls.Load() != 1 {
		t.Fatal("introspection did not run once")
	}
	if _, err = s.trustIdentityContext([]byte("{}"), token, time.Time{}); err == nil {
		t.Fatal("unguarded engine access bypassed introspection")
	}
	if _, err = s.trustServiceScope(context.Background(), trustEffectToken); err != nil {
		t.Fatal(err)
	}
	if _, err = s.trustIdentityContext([]byte("{}"), trustEffectToken, time.Time{}); err == nil {
		t.Fatal("static token bypassed mapped service guard")
	}
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	active.Store(false)
	if _, err = s.trustServiceScope(context.Background(), token); err == nil {
		t.Fatal("revoked service accepted")
	}
	if _, err = scope.trustIdentityContext([]byte("{}"), token, time.Time{}); err == nil {
		t.Fatal("positive in-flight proof survived observed issuer revocation")
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("revocation refusal mutated business state")
	}
	restarted, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer restarted.db.Close()
	if _, err = restarted.trustServiceScope(context.Background(), token); err == nil {
		t.Fatal("restart recovered raw provider token")
	}
	if strings.Contains(string(after), "access_token\"") {
		t.Fatal("raw token key persisted")
	}
}

func TestActionsConfianceT3IntrospectionResponseRejectsAmbiguity(t *testing.T) {
	now := time.Now().UTC()
	c := TrustWorkloadConfig{Issuer: "https://issuer.example", Audience: "urn:swarm"}
	session := trustOIDCSession{Issuer: c.Issuer, ExternalSubject: "subject", ClientID: "client", AccessTokenExpiresAt: now.Add(time.Minute)}
	good := map[string]any{"active": true, "iss": c.Issuer, "sub": session.ExternalSubject, "client_id": "client", "aud": c.Audience, "exp": now.Add(30 * time.Second).Unix()}
	cases := map[string]func(map[string]any){"inactive": func(v map[string]any) { v["active"] = false }, "null-active": func(v map[string]any) { v["active"] = nil }, "string-active": func(v map[string]any) { v["active"] = "true" }, "issuer": func(v map[string]any) { v["iss"] = "https://other.example" }, "subject": func(v map[string]any) { v["sub"] = "other" }, "client": func(v map[string]any) { v["client_id"] = "other" }, "audience": func(v map[string]any) { v["aud"] = "other" }, "multiple-audiences": func(v map[string]any) { v["aud"] = []string{c.Audience, "other"} }, "expired": func(v map[string]any) { v["exp"] = now.Unix() }, "extended": func(v map[string]any) { v["exp"] = now.Add(time.Hour).Unix() }, "missing-exp": func(v map[string]any) { delete(v, "exp") }, "future-iat": func(v map[string]any) { v["iat"] = now.Add(time.Hour).Unix() }, "future-nbf": func(v map[string]any) { v["nbf"] = now.Add(time.Hour).Unix() }}
	raw, _ := json.Marshal(good)
	if _, err := trustIntrospectionResponse(raw, &c, session, now); err != nil {
		t.Fatal(err)
	}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			v := map[string]any{}
			for k, x := range good {
				v[k] = x
			}
			mutate(v)
			raw, _ := json.Marshal(v)
			if _, err := trustIntrospectionResponse(raw, &c, session, now); err == nil {
				t.Fatal("unsafe response accepted")
			}
		})
	}
	for _, raw := range []string{`{"active":false,"active":true}`, `{"active":true} {}`, `null`, `[]`, `{"active":true`} {
		if _, err := trustIntrospectionResponse([]byte(raw), &c, session, now); err == nil {
			t.Fatal("malformed/duplicate response accepted")
		}
	}
}

func TestActionsConfianceT3IntrospectionPolicyInvalidInputs(t *testing.T) {
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	p := trustIntrospectionPolicyValue(nil)
	c.WorkloadIntrospection = &p
	raw, _ := json.Marshal(c)
	var base map[string]json.RawMessage
	json.Unmarshal(raw, &base)
	var fields map[string]any
	json.Unmarshal(base["workload_introspection"], &fields)
	for _, key := range []string{"enabled", "endpoint", "client_id", "timeout_seconds", "validation_window_seconds", "max_sessions", "max_token_bytes"} {
		t.Run(key, func(t *testing.T) {
			copy := map[string]any{}
			for k, v := range fields {
				copy[k] = v
			}
			copy[key] = nil
			part, _ := json.Marshal(copy)
			base["workload_introspection"] = part
			all, _ := json.Marshal(base)
			var change TrustConfigChange
			if strict(all, &change) == nil {
				t.Fatal("null field accepted")
			}
		})
	}
	for _, key := range []string{"timeout_seconds", "validation_window_seconds", "max_sessions", "max_token_bytes"} {
		t.Run("zero-"+key, func(t *testing.T) {
			copy := map[string]any{}
			for k, v := range fields {
				copy[k] = v
			}
			copy[key] = 0
			part, _ := json.Marshal(copy)
			base["workload_introspection"] = part
			all, _ := json.Marshal(base)
			var change TrustConfigChange
			if strict(all, &change) == nil {
				if _, err := s.trustConfig(change, true); err == nil {
					t.Fatal("zero accepted while disabled")
				}
			}
		})
	}
}

func TestActionsConfianceT3IntrospectionNoNetworkLockAndFreshDecision(t *testing.T) {
	s, f, token, _, _ := introspectionConfigured(t)
	old := f.server.Config.Handler
	entered := make(chan struct{})
	release := make(chan struct{})
	var once sync.Once
	f.server.Config.Handler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/introspect" {
			once.Do(func() { close(entered) })
			select {
			case <-release:
			case <-r.Context().Done():
				return
			}
		}
		old.ServeHTTP(w, r)
	})
	result := make(chan *Store, 1)
	failure := make(chan error, 1)
	go func() {
		scope, err := s.trustServiceScope(context.Background(), token)
		if err != nil {
			failure <- err
			return
		}
		result <- scope
	}()
	select {
	case <-entered:
	case err := <-failure:
		t.Fatal(err)
	case <-time.After(5 * time.Second):
		t.Fatal("no introspection request")
	}
	_, err := s.withTrustPublicationLockUsing(true, nil, true, func(st *trustState) (any, error) { return nil, nil })
	close(release)
	if err != nil {
		t.Fatal("network held business lock", err)
	}
	var scope *Store
	select {
	case scope = <-result:
	case err := <-failure:
		t.Fatal(err)
	case <-time.After(5 * time.Second):
		t.Fatal("no completed introspection")
	}
	scope.serviceProof.expiresAt = time.Now().Add(-time.Second)
	if _, err = scope.trustIdentityContext([]byte("{}"), token, time.Time{}); err == nil {
		t.Fatal("expired authorization accepted")
	}
	scope, err = s.trustServiceScope(context.Background(), token)
	if err != nil {
		t.Fatal(err)
	}
	c := lifecycleChange(t, "alice", false, 3)
	if _, err = s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	if _, err = scope.trustIdentityContext([]byte("{}"), token, time.Time{}); err == nil {
		t.Fatal("reconfigured authorization accepted")
	}
	if !scope.serviceProof.material.revoked.Load() {
		t.Fatal("reconfiguration retained token material")
	}
}

func TestActionsConfianceT3IntrospectionTransportFailsClosed(t *testing.T) {
	for _, name := range []string{"unauthorized", "redirect", "malformed", "oversized", "timeout"} {
		t.Run(name, func(t *testing.T) {
			s, f, token, _, _ := introspectionConfigured(t, func(p *TrustIntrospectionPolicy) { p.TimeoutSeconds = 1 })
			var destinationCalls atomic.Int64
			other := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { destinationCalls.Add(1) }))
			defer other.Close()
			old := f.server.Config.Handler
			f.server.Config.Handler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				if r.URL.Path != "/introspect" {
					old.ServeHTTP(w, r)
					return
				}
				switch name {
				case "unauthorized":
					http.Error(w, "unauthorized", 401)
				case "redirect":
					http.Redirect(w, r, other.URL, http.StatusFound)
				case "malformed":
					w.Write([]byte(`{"active":"true"}`))
				case "oversized":
					w.Write(bytes.Repeat([]byte(" "), (1<<20)+1))
				case "timeout":
					select {
					case <-r.Context().Done():
					case <-time.After(2 * time.Second):
					}
				}
			})
			path, _, _ := s.trustPaths()
			before, _ := os.ReadFile(path)
			if _, err := s.trustServiceScope(context.Background(), token); err == nil {
				t.Fatal("failed introspection admitted service")
			}
			after, _ := os.ReadFile(path)
			if !bytes.Equal(before, after) {
				t.Fatal("transport failure mutated business state")
			}
			if destinationCalls.Load() != 0 {
				t.Fatal("redirect leaked provider token")
			}
		})
	}
}

func TestActionsConfianceT3IntrospectionDestinationAndCapacity(t *testing.T) {
	w := &TrustWorkloadConfig{Enabled: true, Issuer: "https://issuer.example"}
	p := trustIntrospectionPolicyValue(nil)
	p.Enabled = true
	p.ClientID = "swarm"
	p.Endpoint = w.Issuer + "/introspect"
	if err := validateTrustIntrospection(&p, w); err != nil {
		t.Fatal(err)
	}
	for _, endpoint := range []string{"https://other.example/introspect", "http://issuer.example/introspect", "https://issuer.example/introspect?token=x", "https://issuer.example/introspect#fragment", "https://user@issuer.example/introspect"} {
		t.Run(endpoint, func(t *testing.T) {
			bad := p
			bad.Endpoint = endpoint
			if validateTrustIntrospection(&bad, w) == nil {
				t.Fatal("uncontrolled destination accepted")
			}
		})
	}
	s, f, token, _, _ := introspectionConfigured(t, func(p *TrustIntrospectionPolicy) { p.MaxSessions = 1 })
	view, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	usage := view.(map[string]any)["workload_introspection_status"].(map[string]any)
	if usage["retained_sessions"] != 1 || usage["retained_token_bytes"].(int) <= 0 || usage["persistent_tokens"] != false {
		t.Fatal("memory consumption missing or mixed with policy")
	}
	if _, err = s.trustWorkloadLogin(context.Background(), workloadAssertion(t, f, nil)); err == nil {
		t.Fatal("memory capacity exceeded")
	}
	raw, _ := json.Marshal(trustWorkloadCommand{SessionToken: token})
	if _, err = s.trustWorkloadOperation(context.Background(), "workload-logout", raw); err != nil {
		t.Fatal(err)
	}
	if _, err = s.trustServiceScope(context.Background(), token); err == nil {
		t.Fatal("logged out service accepted")
	}
	view, err = s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	usage = view.(map[string]any)["workload_introspection_status"].(map[string]any)
	if usage["retained_sessions"] != 0 || usage["retained_token_bytes"] != 0 {
		t.Fatal("logout retained material")
	}
}
