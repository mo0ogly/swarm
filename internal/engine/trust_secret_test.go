//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

func TestActionsConfianceT4SecretBrokerPolicy(t *testing.T) {
	s, _ := trustFixture(t)
	raw := []byte(`{"schema_version":1,"event_id":"broker-policy","expected_revision":1,"policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},"identities":[{"subject":"admin","token":"admin-token-0123456789","integrity":3,"roles":["admin"]}],"secret_broker":{"enabled":false,"material_ttl_seconds":300,"grant_ttl_seconds":60,"max_material_bytes":4096,"max_entries":16},"reason":"isolated broker policy"}`)
	var change TrustConfigChange
	if err := strict(raw, &change); err != nil {
		t.Fatal(err)
	}
	change.Identities = []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}, {Subject: "effect-service", Token: trustEffectToken, Integrity: 2, Roles: []string{"effect"}}}
	v, err := s.trustConfig(change, true)
	if err != nil {
		t.Fatal(err)
	}
	encoded, _ := json.Marshal(v)
	var view map[string]json.RawMessage
	if json.Unmarshal(encoded, &view) != nil || view["secret_broker"] == nil {
		t.Fatal("broker policy missing from preview")
	}
}

func trustSecretFixture(t *testing.T) (*Store, *trustSecretBroker, TrustAction) {
	t.Helper()
	s, _ := trustFixture(t)
	change := TrustConfigChange{Schema: 1, EventID: "broker-enabled", ExpectedRevision: 1, Reason: "isolated broker configuration", Policy: defaultTrustPolicy(), Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}, {Subject: "effect-service", Token: trustEffectToken, Integrity: 2, Roles: []string{"effect"}}}, SecretBroker: &TrustSecretPolicy{Enabled: true, MaterialTTLSeconds: 300, GrantTTLSeconds: 60, MaxMaterialBytes: 4096, MaxEntries: 2}, SealedExecution: &TrustSealedPolicy{Enabled: true, TimeoutSeconds: 15}}
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	now := time.Now().UTC()
	createMandate(t, s, now, "broker-mandate", now.Add(time.Hour))
	a := createAction(t, s, now, "broker-request", "broker-mandate", "broker-key", 100)
	cmd := trustCmd("broker-approval")
	cmd.ActionID = a.ID
	cmd.MandateID = a.MandateID
	cmd.Amount = a.Amount
	cmd.Currency = a.Currency
	cmd.Beneficiary = a.Beneficiary
	cmd.ContentHash = a.ContentHash
	v, err := s.trustOperate("approve", cmd, trustApproverToken, now)
	if err != nil {
		t.Fatal(err)
	}
	a = v.(TrustAction)
	b := newTrustSecretBroker(s)
	b.readyRevision = 2
	return s, b, a
}
func trustSecretIssue(t *testing.T, b *trustSecretBroker, a TrustAction) TrustSecretGrant {
	t.Helper()
	value := []byte("ephemeral-fixture-secret-0123456789abcdef")
	v, err := b.load(context.Background(), trustAdminToken, value)
	if err != nil {
		t.Fatal(err)
	}
	ref := v.(map[string]any)["secret_ref"].(string)
	cmd := trustSecretRequest{ActionID: a.ID, Digest: a.ContentHash, Ref: ref, Connector: "sealed-attestation", Destination: "local-attestation"}
	v, err = b.operation(context.Background(), "secret-grant", cmd, trustEffectToken)
	if err != nil {
		t.Fatal(err)
	}
	return v.(TrustSecretGrant)
}
func secretStateBytes(t *testing.T, s *Store) []byte {
	t.Helper()
	p, _, _ := s.trustPaths()
	v, e := os.ReadFile(p)
	if e != nil {
		t.Fatal(e)
	}
	return v
}
func TestActionsConfianceT4SecretBrokerScope(t *testing.T) {
	s, b, a := trustSecretFixture(t)
	g := trustSecretIssue(t, b, a)
	before := secretStateBytes(t, s)
	for _, token := range []string{trustAdminToken, trustRequesterToken, trustApproverToken, "wrong-token-value"} {
		if _, err := b.consume(context.Background(), trustSecretRequest{GrantID: g.ID}, token, func(context.Context, TrustSealedPolicy, []byte, TrustSecretGrant) (string, error) {
			t.Fatal("denied secret reached consumer")
			return "", nil
		}); err == nil {
			t.Fatal("wrong principal admitted")
		}
	}
	if !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("denial mutated durable state")
	}
	called := 0
	v, err := b.consume(context.Background(), trustSecretRequest{GrantID: g.ID}, trustEffectToken, func(ctx context.Context, p TrustSealedPolicy, value []byte, actual TrustSecretGrant) (string, error) {
		called++
		if string(value) != "ephemeral-fixture-secret-0123456789abcdef" || actual.Digest != a.ContentHash {
			t.Fatal("wrong material or content")
		}
		return strings.Repeat("a", 64), nil
	})
	if err != nil || v.(TrustSecretGrant).State != "consumed" || called != 1 {
		t.Fatalf("consumption failed: %v", err)
	}
	if _, err = b.consume(context.Background(), trustSecretRequest{GrantID: g.ID}, trustEffectToken, runTrustSecretConsumer); err == nil {
		t.Fatal("grant replay admitted")
	}
	if bytes.Contains(secretStateBytes(t, s), []byte("ephemeral-fixture-secret-0123456789abcdef")) {
		t.Fatal("raw secret persisted")
	}
	if _, err = s.trustExport(); err != nil {
		t.Fatalf("new audit broke original/proof chain: %v", err)
	}
}

func TestActionsConfianceT4SecretBrokerInvalidPolicy(t *testing.T) {
	base := map[string]any{"enabled": false, "material_ttl_seconds": 300, "grant_ttl_seconds": 60, "max_material_bytes": 4096, "max_entries": 16}
	for _, key := range []string{"enabled", "material_ttl_seconds", "grant_ttl_seconds", "max_material_bytes", "max_entries"} {
		for _, bad := range []any{nil, "", -1, 0, 1.5} {

			t.Run(key+"-"+fmt.Sprint(bad), func(t *testing.T) {
				v := map[string]any{}
				for k, x := range base {
					v[k] = x
				}
				v[key] = bad
				raw, _ := json.Marshal(v)
				var p TrustSecretPolicy
				err := strict(raw, &p)
				if err == nil {
					err = validateTrustSecretPolicy(&p)
				}
				if err == nil {
					t.Fatal("invalid policy admitted")
				}
			})
		}
	}
	for _, key := range []string{"enabled", "material_ttl_seconds", "grant_ttl_seconds", "max_material_bytes", "max_entries"} {
		v := map[string]any{}
		for k, x := range base {
			if k != key {
				v[k] = x
			}
		}
		raw, _ := json.Marshal(v)
		var p TrustSecretPolicy
		if strict(raw, &p) == nil {
			t.Fatal("missing policy field admitted")
		}
	}
	for _, p := range []TrustSecretPolicy{{MaterialTTLSeconds: 1, GrantTTLSeconds: 2, MaxMaterialBytes: 1, MaxEntries: 1}, {MaterialTTLSeconds: 3, GrantTTLSeconds: 2, MaxMaterialBytes: 1<<20 + 1, MaxEntries: 1}} {
		if validateTrustSecretPolicy(&p) == nil {
			t.Fatal("inconsistent/oversized policy admitted")
		}
	}
}
func TestActionsConfianceT4SecretBrokerClosedRecovery(t *testing.T) {
	for _, variant := range []string{"restart", "revoke", "expired", "failed-consumer", "configuration"} {
		t.Run(variant, func(t *testing.T) {
			s, b, a := trustSecretFixture(t)
			g := trustSecretIssue(t, b, a)
			called := 0
			runner := func(context.Context, TrustSealedPolicy, []byte, TrustSecretGrant) (string, error) {
				called++
				return "", trustSealedFailure()
			}
			switch variant {
			case "restart":
				b = newTrustSecretBroker(s)
			case "revoke":
				if _, err := b.operation(context.Background(), "secret-revoke", trustSecretRequest{GrantID: g.ID}, trustAdminToken); err != nil {
					t.Fatal(err)
				}
			case "expired":
				_, err := s.withTrustLock(true, func(st *trustState) (any, error) {
					v := st.SecretGrants[g.ID]
					v.ExpiresAt = time.Now().Add(-time.Second)
					st.SecretGrants[g.ID] = v
					return nil, nil
				})
				if err != nil {
					t.Fatal(err)
				}
			case "configuration":
				_, err := s.withTrustLock(true, func(st *trustState) (any, error) { st.Revision++; return nil, nil })
				if err != nil {
					t.Fatal(err)
				}
			}
			before := secretStateBytes(t, s)
			if _, err := b.consume(context.Background(), trustSecretRequest{GrantID: g.ID}, trustEffectToken, runner); err == nil {
				t.Fatal("closed grant admitted")
			}
			if variant != "failed-consumer" && (called != 0 || !bytes.Equal(before, secretStateBytes(t, s))) {
				t.Fatal("refused grant consumed or mutated state")
			}
			if variant == "failed-consumer" {
				if called != 1 {
					t.Fatal("consumer did not run")
				}
				if _, err := b.consume(context.Background(), trustSecretRequest{GrantID: g.ID}, trustEffectToken, runner); err == nil || called != 1 {
					t.Fatal("failed consumption replayed")
				}
			}
		})
	}
}
func TestActionsConfianceT4SecretBrokerConcurrency(t *testing.T) {
	s, b, a := trustSecretFixture(t)
	g := trustSecretIssue(t, b, a)
	started, release := make(chan bool, 1), make(chan struct{})
	done := make(chan error, 1)
	released := false
	defer func() {
		if !released {
			close(release)
		}
	}()
	go func() {
		_, err := b.consume(context.Background(), trustSecretRequest{GrantID: g.ID}, trustEffectToken, func(context.Context, TrustSealedPolicy, []byte, TrustSecretGrant) (string, error) {
			started <- true
			select {
			case <-release:
			case <-time.After(3 * time.Second):
				return "", trustSealedTimeout()
			}
			return strings.Repeat("b", 64), nil
		})
		done <- err
	}()
	select {
	case <-started:
	case err := <-done:
		t.Fatalf("consumer stopped before start: %v", err)
	case <-time.After(3 * time.Second):
		t.Fatal("consumer did not start")
	}
	before := secretStateBytes(t, s)
	if _, err := b.consume(context.Background(), trustSecretRequest{GrantID: g.ID}, trustEffectToken, runTrustSecretConsumer); err == nil {
		t.Fatal("concurrent reuse admitted")
	}
	if !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("concurrent denial mutated state")
	}
	close(release)
	released = true
	if err := <-done; err != nil {
		t.Fatal(err)
	}
}

func TestActionsConfianceT4SecretBrokerLoadBoundaries(t *testing.T) {
	s, b, _ := trustSecretFixture(t)
	for _, kind := range []string{"not-ready", "wrong-role", "oversized", "cancelled"} {
		t.Run(kind, func(t *testing.T) {
			ctx := context.Background()
			token := trustAdminToken
			raw := []byte("secret-load-boundary-fixture")
			switch kind {
			case "not-ready":
				b.readyRevision = 0
				defer func() { b.readyRevision = 2 }()
			case "wrong-role":
				token = trustRequesterToken
			case "oversized":
				raw = make([]byte, 4097)
			case "cancelled":
				var cancel context.CancelFunc
				ctx, cancel = context.WithCancel(ctx)
				cancel()
			}
			before := secretStateBytes(t, s)
			if _, err := b.load(ctx, token, raw); err == nil {
				t.Fatal("invalid load admitted")
			}
			if !bytes.Equal(raw, make([]byte, len(raw))) || !bytes.Equal(before, secretStateBytes(t, s)) || len(b.entries) != 0 {
				t.Fatal("refused material retained or state changed")
			}
		})
	}
}
func TestActionsConfianceT4SecretBrokerStatusAfterRestart(t *testing.T) {
	s, b, a := trustSecretFixture(t)
	g := trustSecretIssue(t, b, a)
	b = newTrustSecretBroker(s)
	v, err := b.operation(context.Background(), "secret-status", trustSecretRequest{}, trustEffectToken)
	if err != nil {
		t.Fatal(err)
	}
	if v.(map[string]any)["grant_material_available"].(map[string]bool)[g.ID] {
		t.Fatal("historical grant misrepresented as usable after restart")
	}
}

func TestActionsConfianceT4SecretBrokerGrantBoundaries(t *testing.T) {
	s, b, a := trustSecretFixture(t)
	loaded, err := b.load(context.Background(), trustAdminToken, []byte("grant-boundary-ephemeral-fixture"))
	if err != nil {
		t.Fatal(err)
	}
	valid := trustSecretRequest{ActionID: a.ID, Digest: a.ContentHash, Ref: loaded.(map[string]any)["secret_ref"].(string), Connector: "sealed-attestation", Destination: "local-attestation"}
	for _, kind := range []string{"digest", "destination", "connector", "action", "ref", "role"} {
		t.Run(kind, func(t *testing.T) {
			cmd := valid
			token := trustEffectToken
			switch kind {
			case "digest":
				cmd.Digest = strings.Repeat("0", 64)
			case "destination":
				cmd.Destination = "https://forbidden.invalid"
			case "connector":
				cmd.Connector = "external"
			case "action":
				cmd.ActionID = "missing"
			case "ref":
				cmd.Ref = "missing"
			case "role":
				token = trustRequesterToken
			}
			before := secretStateBytes(t, s)
			if _, err := b.operation(context.Background(), "secret-grant", cmd, token); err == nil {
				t.Fatal("out-of-scope grant admitted")
			}
			if !bytes.Equal(before, secretStateBytes(t, s)) {
				t.Fatal("grant refusal mutated state")
			}
		})
	}
	if _, err = b.operation(context.Background(), "secret-grant", valid, trustEffectToken); err != nil {
		t.Fatal(err)
	}
	before := secretStateBytes(t, s)
	if _, err = b.operation(context.Background(), "secret-grant", valid, trustEffectToken); err == nil || !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("reference reused or refusal mutated")
	}
}

func TestActionsConfianceT4SecretBrokerPreparation(t *testing.T) {
	for _, raw := range []string{`{}`, `{"ready":false}`, `{"schema_version":1,"ready":true,"revision":1,"server_memory_nondumpable":false,"max_material_bytes":256}`, `{"schema_version":1,"ready":true,"revision":1,"server_memory_nondumpable":true,"max_material_bytes":0}`} {
		if trustSecretPrepared([]byte(raw)) {
			t.Fatal("incomplete/refused preparation accepted")
		}
	}
	if !trustSecretPrepared([]byte(`{"schema_version":1,"ready":true,"revision":1,"server_memory_nondumpable":true,"max_material_bytes":256}`)) {
		t.Fatal("valid preparation refused")
	}
	var calls atomic.Int32
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Query().Get("action") == "secret-load" {
			calls.Add(1)
		}
		_, _ = w.Write([]byte(`{"ready":false}`))
	}))
	defer server.Close()
	var out bytes.Buffer
	err := trustRemoteCLI(context.Background(), []string{"trust", "secret-load"}, "-", server.URL, trustRemoteFixtureSession, trustAdminToken, &out)
	if err == nil || calls.Load() != 0 || out.Len() != 0 {
		t.Fatal("refused preparation sent raw material")
	}
}
