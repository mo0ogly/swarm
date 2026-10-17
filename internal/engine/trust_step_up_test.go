//go:build linux

package engine

import (
	"context"
	"encoding/json"
	jose "github.com/go-jose/go-jose/v4"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"reflect"
	"testing"
	"time"
)

func TestActionsConfianceT3StepUpPolicySurface(t *testing.T) {
	s, _ := trustFixture(t)
	change := lifecycleChange(t, "alice", false, 1)
	change.OIDC = &TrustOIDCConfig{Enabled: true, Issuer: "http://127.0.0.1:1", ClientID: "swarm", RedirectURI: "http://127.0.0.1/callback", RequestTimeoutSeconds: 5, LoginTTLSeconds: 300, SessionTTLSeconds: 600, Mappings: []TrustOIDCMapping{{ExternalSubject: "employee-1", Subject: "bob"}}}
	raw, _ := json.Marshal(change)
	var input map[string]any
	if err := json.Unmarshal(raw, &input); err != nil {
		t.Fatal(err)
	}
	input["step_up"] = map[string]any{"enabled": true, "operations": []string{"approve"}, "acr_values": []string{"urn:example:strong"}, "max_age_seconds": 0, "proof_ttl_seconds": 300}
	raw, _ = json.Marshal(input)
	var decoded TrustConfigChange
	if err := strict(raw, &decoded); err != nil {
		t.Fatal("required step-up configuration surface absent", err)
	}
	state := filepath.Join(s.root, ".swarm", "trust-actions.json")
	before, _ := os.ReadFile(state)
	preview, err := s.trustConfig(decoded, true)
	if err != nil {
		t.Fatal(err)
	}
	after, _ := os.ReadFile(state)
	if string(before) != string(after) {
		t.Fatal("step-up preview mutated state")
	}
	if preview.(map[string]any)["step_up"] == nil {
		t.Fatal("step-up policy missing from preview")
	}
}

func stepUpFixture(t *testing.T) (*Store, *oidcFixture, TrustAction) {
	t.Helper()
	f := newOIDCFixture(t)
	f.modify = func(c map[string]any) { c["auth_time"] = time.Now().Unix(); c["acr"] = "urn:example:strong" }
	s, _ := trustFixture(t)
	change := lifecycleChange(t, "alice", false, 1)
	change.OIDC = &TrustOIDCConfig{Enabled: true, Issuer: f.server.URL, ClientID: "swarm", RedirectURI: "http://127.0.0.1/callback", RequestTimeoutSeconds: 5, LoginTTLSeconds: 300, SessionTTLSeconds: 600, Mappings: []TrustOIDCMapping{{ExternalSubject: "employee-1", Subject: "bob"}, {ExternalSubject: "employee-2", Subject: "alice"}}}
	change.StepUp = &TrustStepUpPolicy{Enabled: true, Operations: []string{"approve"}, ACRValues: []string{"urn:example:strong"}, ProofTTLSeconds: 300}
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	now := time.Now().UTC()
	createMandate(t, s, now, "step-up-root", now.Add(time.Hour))
	v, err := s.trustOperate("request", delegatedRequest("step-up-request", "step-up-root"), trustRequesterToken, now)
	if err != nil {
		t.Fatal(err)
	}
	return s, f, v.(TrustAction)
}

func stepUpApproveCommand(a TrustAction, event string) TrustCommand {
	c := trustCmd(event)
	c.ActionID = a.ID
	c.MandateID = a.MandateID
	c.ContentHash = a.ContentHash
	c.Amount = a.Amount
	c.Currency = a.Currency
	c.Beneficiary = a.Beneficiary
	return c
}

func stepUpAuthorize(t *testing.T, s *Store, a TrustAction) trustOIDCCommand {
	t.Helper()
	v, err := s.trustOIDCStartFor(context.Background(), trustOIDCCommand{StepUpActionID: a.ID}, trustApproverToken)
	if err != nil {
		t.Fatal(err)
	}
	login := v.(trustOIDCLogin)
	auth, _ := url.Parse(login.AuthorizationURL)
	q := auth.Query()
	if q.Get("max_age") != "0" || q.Get("acr_values") != "urn:example:strong" {
		t.Fatal("step-up parameters missing")
	}
	var claims map[string]map[string]json.RawMessage
	if json.Unmarshal([]byte(q.Get("claims")), &claims) != nil || string(claims["id_token"]["auth_time"]) != "{\"essential\":true}" {
		t.Fatal("essential claims missing")
	}
	client := http.Client{CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }}
	response, err := client.Get(login.AuthorizationURL)
	if err != nil {
		t.Fatal(err)
	}
	response.Body.Close()
	target, err := url.Parse(response.Header.Get("Location"))
	if err != nil || target.Query().Get("state") != login.State {
		t.Fatal("invalid authorization redirect")
	}
	return trustOIDCCommand{State: login.State, Binding: login.Binding, Code: target.Query().Get("code")}
}

func TestActionsConfianceT3StepUpExactApprovalPersistenceAndConsumption(t *testing.T) {
	s, _, a := stepUpFixture(t)
	now := time.Now().UTC()
	before, _ := s.loadTrustState()
	weak, err := s.trustOperate("approve", stepUpApproveCommand(a, "step-up-weak"), trustApproverToken, now)
	if err != nil || weak.(map[string]any)["state"] != "denied" {
		t.Fatal("static token bypassed step-up", err)
	}
	after, _ := s.loadTrustState()
	if !reflect.DeepEqual(before.Actions, after.Actions) || len(after.Ledger) != 0 {
		t.Fatal("weak approval mutated business state")
	}
	cmd := stepUpAuthorize(t, s, a)
	v, err := s.trustOIDCFinish(context.Background(), cmd)
	if err != nil {
		t.Fatal(err)
	}
	result := v.(map[string]any)
	token := result["session_token"].(string)
	if result["subject"] != "bob" || result["integrity"] != 2 || result["step_up_action_id"] != a.ID {
		t.Fatal("step-up changed local identity or lost action binding")
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	approve := stepUpApproveCommand(a, "step-up-exact")
	wrong := approve
	wrong.EventID = "step-up-wrong-amount"
	wrong.Amount++
	if v, err = reopened.trustOperate("approve", wrong, token, time.Now().UTC()); err != nil || v.(map[string]any)["reason"] != "accord non exact" {
		t.Fatal("inexact approval admitted", err)
	}
	if v, err = reopened.trustOperate("approve", approve, token, time.Now().UTC()); err != nil || v.(TrustAction).State != "approved" {
		t.Fatal("exact stepped approval failed", err)
	}
	st, _ := reopened.loadTrustState()
	if !st.OIDCSessions[trustTokenHash(token)].StepUp.Used {
		t.Fatal("proof not consumed")
	}
	if _, err = reopened.trustOperate("approve", approve, token, time.Now().UTC()); err != nil {
		t.Fatal("same authorized event replay failed", err)
	}
	commit := trustCmd("step-up-effect")
	commit.ActionID = a.ID
	if v, err = reopened.trustOperate("commit", commit, trustEffectToken, time.Now().UTC()); err != nil || v.(TrustAction).State != "committed" {
		t.Fatal("approved effect failed", err)
	}
}

func TestActionsConfianceT3StepUpRejectsWeakOrChangedClaims(t *testing.T) {
	cases := map[string]func(map[string]any){"missing-auth-time": func(c map[string]any) { delete(c, "auth_time") }, "old-auth-time": func(c map[string]any) { c["auth_time"] = time.Now().Add(-time.Hour).Unix() }, "future-auth-time": func(c map[string]any) { c["auth_time"] = time.Now().Add(time.Minute).Unix() }, "invalid-auth-time-type": func(c map[string]any) { c["auth_time"] = "now" }, "missing-acr": func(c map[string]any) { delete(c, "acr") }, "weak-acr": func(c map[string]any) { c["acr"] = "0" }, "invalid-acr-type": func(c map[string]any) { c["acr"] = []string{"urn:example:strong"} }, "changed-subject": func(c map[string]any) { c["sub"] = "employee-2" }, "wrong-nonce": func(c map[string]any) { c["nonce"] = "wrong" }}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			s, f, a := stepUpFixture(t)
			cmd := stepUpAuthorize(t, s, a)
			before, _ := s.loadTrustState()
			f.mu.Lock()
			f.modify = func(c map[string]any) { c["auth_time"] = time.Now().Unix(); c["acr"] = "urn:example:strong"; mutate(c) }
			f.mu.Unlock()
			if _, err := s.trustOIDCFinish(context.Background(), cmd); err == nil {
				t.Fatal("weak or changed signed claims admitted")
			}
			after, _ := s.loadTrustState()
			if !reflect.DeepEqual(before.Actions, after.Actions) || len(after.Ledger) != 0 || len(after.OIDCSessions) != 0 || len(after.OIDCPending) != 0 {
				t.Fatal("rejected login changed business state or created session")
			}
		})
	}
}

func TestActionsConfianceT3StepUpWrongActionExpiryAndRevocation(t *testing.T) {
	for _, variant := range []string{"other-action", "expiry", "revoked-mandate", "suspended-identity", "changed-revision", "logout"} {
		t.Run(variant, func(t *testing.T) {
			s, _, a := stepUpFixture(t)
			command := stepUpAuthorize(t, s, a)
			v, err := s.trustOIDCFinish(context.Background(), command)
			if err != nil {
				t.Fatal(err)
			}
			token := v.(map[string]any)["session_token"].(string)
			now := time.Now().UTC()
			target := a
			switch variant {
			case "other-action":
				v, err = s.trustOperate("request", delegatedRequest("step-up-other-request", a.MandateID), trustRequesterToken, now)
				if err != nil {
					t.Fatal(err)
				}
				target = v.(TrustAction)
			case "expiry":
				now = now.Add(301 * time.Second)
			case "revoked-mandate":
				c := trustCmd("step-up-revoke")
				c.MandateID = a.MandateID
				if _, err = s.trustOperate("mandate-revoke", c, trustAdminToken, now); err != nil {
					t.Fatal(err)
				}
			case "suspended-identity", "changed-revision":
				c := lifecycleChange(t, "bob", variant == "suspended-identity", 2)
				if _, err = s.trustConfig(c, false); err != nil {
					t.Fatal(err)
				}
			case "logout":
				if _, err = s.trustOIDCOperation(context.Background(), "oidc-logout", []byte(`{"session_token":"`+token+`"}`)); err != nil {
					t.Fatal(err)
				}
			}
			before, _ := s.loadTrustState()
			v, err = s.trustOperate("approve", stepUpApproveCommand(target, "step-up-denied-"+variant), token, now)
			if err == nil {
				if r, ok := v.(map[string]any); !ok || r["state"] != "denied" {
					t.Fatal("invalid proof approved an action")
				}
			}
			after, _ := s.loadTrustState()
			if !reflect.DeepEqual(before.Actions, after.Actions) || len(after.Ledger) != 0 {
				t.Fatal("invalid proof mutated business state")
			}
		})
	}
}

func TestActionsConfianceT3StepUpStartRequiresAuthorityBeforeNetwork(t *testing.T) {
	s, f, a := stepUpFixture(t)
	for _, token := range []string{"", trustRequesterToken, trustEffectToken} {
		before, _ := s.loadTrustState()
		if _, err := s.trustOIDCStartFor(context.Background(), trustOIDCCommand{StepUpActionID: a.ID}, token); err == nil {
			t.Fatal("unauthorized step-up started")
		}
		after, _ := s.loadTrustState()
		if !reflect.DeepEqual(before, after) {
			t.Fatal("unauthorized start mutated state")
		}
	}
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.exchanges != 0 {
		t.Fatal("unauthorized start exchanged a token")
	}
}

func TestActionsConfianceT3StepUpNormalOIDCIsNotActionProof(t *testing.T) {
	s, _, a := stepUpFixture(t)
	cmd := oidcAuthorize(t, s)
	v, err := s.trustOIDCFinish(context.Background(), cmd)
	if err != nil {
		t.Fatal(err)
	}
	token := v.(map[string]any)["session_token"].(string)
	v, err = s.trustOperate("approve", stepUpApproveCommand(a, "step-up-normal-session"), token, time.Now().UTC())
	if err != nil || v.(map[string]any)["reason"] != "authentification renforcée requise pour cette action exacte" {
		t.Fatal("ordinary OIDC session bypassed action proof", err)
	}
}

func TestActionsConfianceT3StepUpPolicyRejectsInvalidInputs(t *testing.T) {
	cases := map[string]func(*TrustStepUpPolicy){"negative-age": func(p *TrustStepUpPolicy) { p.MaxAgeSeconds = -1 }, "zero-proof-duration": func(p *TrustStepUpPolicy) { p.ProofTTLSeconds = 0 }, "unknown-operation": func(p *TrustStepUpPolicy) { p.Operations = []string{"commit"} }, "no-operation": func(p *TrustStepUpPolicy) { p.Operations = nil }, "duplicate-operation": func(p *TrustStepUpPolicy) { p.Operations = []string{"approve", "approve"} }, "empty-acr": func(p *TrustStepUpPolicy) { p.ACRValues = nil }, "whitespace-acr": func(p *TrustStepUpPolicy) { p.ACRValues = []string{"strong weak"} }, "duplicate-acr": func(p *TrustStepUpPolicy) { p.ACRValues = []string{"strong", "strong"} }}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			p := TrustStepUpPolicy{Enabled: true, Operations: []string{"approve"}, ACRValues: []string{"urn:example:strong"}, ProofTTLSeconds: 300}
			mutate(&p)
			if validateTrustStepUp(&p, &TrustOIDCConfig{Enabled: true}) == nil {
				t.Fatal("unsafe step-up policy accepted")
			}
		})
	}
	for _, raw := range []string{`{"enabled":null,"operations":["approve"],"acr_values":[],"max_age_seconds":0,"proof_ttl_seconds":300}`, `{"enabled":false,"operations":[],"acr_values":[],"max_age_seconds":null,"proof_ttl_seconds":300}`, `{"enabled":false,"operations":[],"acr_values":[],"max_age_seconds":0,"proof_ttl_seconds":null}`, `{}`, `{"enabled":"true"}`, `{"enabled":false,"operations":[],"acr_values":[],"max_age_seconds":1.5,"proof_ttl_seconds":300}`} {
		var p TrustStepUpPolicy
		if strict([]byte(raw), &p) == nil {
			t.Fatal("malformed policy decoded")
		}
	}
	p := defaultTrustStepUpPolicy()
	p.Enabled = true
	p.ACRValues = []string{"urn:example:strong"}
	if validateTrustStepUp(&p, nil) == nil {
		t.Fatal("step-up without OIDC enabled")
	}
}

func TestActionsConfianceT3StepUpFiniteAuthenticationAge(t *testing.T) {
	now := time.Now().UTC()
	b := trustStepUpBinding{StartedAt: now}
	p := TrustStepUpPolicy{MaxAgeSeconds: 120, ACRValues: []string{"urn:example:strong"}}
	if _, err := trustStepUpVerifyClaims(p, b, now.Add(-60*time.Second).Unix(), "urn:example:strong", now); err != nil {
		t.Fatal("recent strong authentication refused")
	}
	if _, err := trustStepUpVerifyClaims(p, b, now.Add(-121*time.Second).Unix(), "urn:example:strong", now); err == nil {
		t.Fatal("authentication older than persisted max age accepted")
	}
}

func TestActionsConfianceT3StepUpWorkloadApproverCannotBypass(t *testing.T) {
	s, _, _ := stepUpFixture(t)
	f := newWorkloadFixture(t)
	change := lifecycleChange(t, "alice", false, 2)
	change.Workloads = &TrustWorkloadConfig{Enabled: true, Issuer: f.server.URL, Audience: "urn:swarm:trust", RequestTimeoutSeconds: 5, AssertionTTLSeconds: 300, SessionTTLSeconds: 600, Clients: []TrustWorkloadClient{{ClientID: "settlement-service", ExternalSubject: "service-principal", Subject: "bob", PublicKeys: []jose.JSONWebKey{{Key: &f.clientKey.PublicKey, KeyID: f.clientKid, Algorithm: "ES256", Use: "sig"}}}}}
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	now := time.Now().UTC()
	createMandate(t, s, now, "service-approver-root", now.Add(time.Hour))
	a := createAction(t, s, now, "service-approver-request", "service-approver-root", "service-approver-key", 100)
	v, err := s.trustWorkloadLogin(context.Background(), workloadAssertion(t, f, nil))
	if err != nil {
		t.Fatal(err)
	}
	token := v.(map[string]any)["session_token"].(string)
	v, err = s.trustOperate("approve", stepUpApproveCommand(a, "service-approver-attempt"), token, now)
	if err != nil || v.(map[string]any)["reason"] != "authentification renforcée requise pour cette action exacte" {
		t.Fatal("valid service approver bypassed step-up", err)
	}
}

func TestActionsConfianceT3StepUpConcurrentApprovalConsumesOnce(t *testing.T) {
	s, _, a := stepUpFixture(t)
	v, err := s.trustOIDCFinish(context.Background(), stepUpAuthorize(t, s, a))
	if err != nil {
		t.Fatal(err)
	}
	token := v.(map[string]any)["session_token"].(string)
	start := make(chan struct{})
	out := make(chan any, 2)
	for _, event := range []string{"step-up-race-1", "step-up-race-2"} {
		go func(event string) {
			<-start
			v, err := s.trustOperate("approve", stepUpApproveCommand(a, event), token, time.Now().UTC())
			if err != nil {
				out <- err
			} else {
				out <- v
			}
		}(event)
	}
	close(start)
	approved, denied := 0, 0
	for i := 0; i < 2; i++ {
		switch r := (<-out).(type) {
		case TrustAction:
			if r.State == "approved" {
				approved++
			}
		case map[string]any:
			if r["state"] == "denied" {
				denied++
			}
		default:
			t.Fatal("concurrent approval failed")
		}
	}
	if approved != 1 || denied != 1 {
		t.Fatal("proof did not authorize exactly one approval", approved, denied)
	}
}

func TestActionsConfianceT3StepUpMandateRevokedBeforeFinish(t *testing.T) {
	s, _, a := stepUpFixture(t)
	cmd := stepUpAuthorize(t, s, a)
	revoke := trustCmd("step-up-revoke-during-login")
	revoke.MandateID = a.MandateID
	if _, err := s.trustOperate("mandate-revoke", revoke, trustAdminToken, time.Now().UTC()); err != nil {
		t.Fatal(err)
	}
	if _, err := s.trustOIDCFinish(context.Background(), cmd); err == nil {
		t.Fatal("revoked mandate produced a usable proof")
	}
	st, _ := s.loadTrustState()
	if len(st.OIDCSessions) != 0 || st.Actions[a.ID].State != "pending_approval" {
		t.Fatal("refused finish changed protected state")
	}
}

func TestActionsConfianceT3StepUpTypedPolicyRoundTrip(t *testing.T) {
	for _, field := range []string{"operations", "acr_values"} {
		t.Run(field, func(t *testing.T) {
			s, _ := trustFixture(t)
			change := lifecycleChange(t, "alice", false, 1)
			p := defaultTrustStepUpPolicy()
			if field == "operations" {
				p.Operations = nil
			} else {
				p.ACRValues = nil
			}
			change.StepUp = &p
			before, _ := s.loadTrustState()
			_, err := s.trustConfig(change, false)
			if err == nil {
				_, roundTrip := s.loadTrustState()
				t.Fatal("typed null policy accepted; persisted round trip:", roundTrip)
			}
			after, err := s.loadTrustState()
			if err != nil || !reflect.DeepEqual(before, after) {
				t.Fatal("invalid typed policy mutated configuration", err)
			}
		})
	}
}
