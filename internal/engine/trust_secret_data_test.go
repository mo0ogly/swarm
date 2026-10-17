//go:build linux

package engine

import (
	"bytes"
	"context"
	"strings"
	"testing"
	"time"
)

func trustSecretDataFixture(t *testing.T, allow bool) (*Store, *trustSecretBroker, TrustAction) {
	t.Helper()
	s, b, _ := trustSecretFixture(t)
	change := lifecycleChange(t, "alice", false, 2)
	change.DataPolicy = &TrustDataPolicy{Enabled: true, Classifications: []string{"internal"}, Destinations: []string{"local-ledger"}, Providers: []string{"swarm"}, Tools: []string{"simulated-settlement"}, Resources: []TrustDataResource{{Resource: "invoice/acme", Classification: "internal"}}}
	if allow {
		change.DataPolicy.Classifications = append(change.DataPolicy.Classifications, "restricted")
		change.DataPolicy.Destinations = append(change.DataPolicy.Destinations, "local-attestation")
		change.DataPolicy.Tools = append(change.DataPolicy.Tools, "sealed-attestation")
	}
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	now := time.Now().UTC()
	createMandate(t, s, now, "data-secret-mandate", now.Add(time.Hour))
	cmd := trustCmd("data-secret-request")
	cmd.MandateID = "data-secret-mandate"
	cmd.Resource = "invoice/acme"
	cmd.Amount = 100
	cmd.Currency = "EUR"
	cmd.Beneficiary = "ACME"
	cmd.IdempotencyKey = "data-secret-key"
	cmd.DataContext = &TrustDataContext{Classification: "internal", Destination: "local-ledger", Provider: "swarm", Tool: "simulated-settlement"}
	v, err := s.trustOperate("request", cmd, trustRequesterToken, now)
	if err != nil {
		t.Fatal(err)
	}
	a := v.(TrustAction)
	approval := trustCmd("data-secret-approve")
	approval.ActionID = a.ID
	approval.MandateID = a.MandateID
	approval.Amount = a.Amount
	approval.Currency = a.Currency
	approval.Beneficiary = a.Beneficiary
	approval.ContentHash = a.ContentHash
	v, err = s.trustOperate("approve", approval, trustApproverToken, now)
	if err != nil {
		t.Fatal(err)
	}
	b.readyRevision = 3
	return s, b, v.(TrustAction)
}
func TestActionsConfianceT4SecretDataPolicyDeniesUnlistedDelivery(t *testing.T) {
	s, b, a := trustSecretDataFixture(t, false)
	value := []byte("only-fixture-material-0123456789abcdef")
	initial := secretStateBytes(t, s)
	v, err := b.load(context.Background(), trustAdminToken, value)
	if err != nil {
		if commandFailure(err).Code != "authorization_required" {
			t.Fatal("unrelated error", err)
		}
		if !bytes.Equal(initial, secretStateBytes(t, s)) || len(b.entries) != 0 {
			t.Fatal("load refusal mutated state or material")
		}
		return
	}
	ref := v.(map[string]any)["secret_ref"].(string)
	before := secretStateBytes(t, s)
	_, err = b.operation(context.Background(), "secret-grant", trustSecretRequest{ActionID: a.ID, Digest: a.ContentHash, Ref: ref, Connector: "sealed-attestation", Destination: "local-attestation"}, trustEffectToken)
	if err == nil {
		t.Fatal("secret delivery bypasses active destination/tool/classification allowlists")
	}
	if !bytes.Equal(before, secretStateBytes(t, s)) || b.entries[ref].grant != "" {
		t.Fatal("denial mutated grant or reserved material")
	}
}

func TestActionsConfianceT4SecretDataPolicyAllowsExactHelper(t *testing.T) {
	s, b, a := trustSecretDataFixture(t, true)
	g := trustSecretIssue(t, b, a)
	want := TrustDataContext{Classification: "restricted", Destination: "local-attestation", Provider: "swarm", Tool: "sealed-attestation"}
	if g.DataContext == nil || *g.DataContext != want {
		t.Fatal("secret classification/delivery context missing")
	}
	message := trustSecretMessage(g)
	if !bytes.Contains(message, []byte("swarm-secret-delivery-v1")) {
		t.Fatal("delivery signature domain missing")
	}
	v, err := b.consume(context.Background(), trustSecretRequest{GrantID: g.ID}, trustEffectToken, func(ctx context.Context, p TrustSealedPolicy, value []byte, actual TrustSecretGrant) (string, error) {
		if actual.DataContext == nil || *actual.DataContext != want || actual.Digest != a.ContentHash || len(value) == 0 {
			t.Fatal("wrong actual delivery")
		}
		return strings.Repeat("a", 64), nil
	})
	if err != nil || v.(TrustSecretGrant).State != "consumed" {
		t.Fatal("allowed helper refused", err)
	}
	if bytes.Contains(secretStateBytes(t, s), []byte("ephemeral-fixture-secret-0123456789abcdef")) {
		t.Fatal("raw secret persisted")
	}
	copy := g
	other := *g.DataContext
	other.Destination = "other"
	copy.DataContext = &other
	if bytes.Equal(message, trustSecretMessage(copy)) {
		t.Fatal("delivery context not bound to attestation")
	}
}
func TestActionsConfianceT4SecretDataPolicyLists(t *testing.T) {
	for _, field := range []string{"classification", "destination", "provider", "tool"} {
		t.Run(field, func(t *testing.T) {
			p := TrustDataPolicy{Enabled: true, Classifications: []string{"restricted"}, Destinations: []string{"local-attestation"}, Providers: []string{"swarm"}, Tools: []string{"sealed-attestation"}, Resources: []TrustDataResource{}}
			st := &trustState{DataPolicy: &p}
			if c, err := trustSecretDeliveryContext(st); err != nil || c == nil {
				t.Fatal("valid fixed tuple refused")
			}
			switch field {
			case "classification":
				p.Classifications = []string{"internal"}
			case "destination":
				p.Destinations = []string{}
			case "provider":
				p.Providers = []string{}
			case "tool":
				p.Tools = []string{}
			}
			if _, err := trustSecretDeliveryContext(st); err == nil {
				t.Fatal("secret path bypasses list", field)
			}
		})
	}
	if _, err := trustSecretObject([]byte(`{"grant_id":"x","data_context":{"classification":"public"}}`)); err == nil {
		t.Fatal("caller classification accepted")
	}
}
