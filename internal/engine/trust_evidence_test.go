//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"testing"
	"time"
)

func TestActionsConfianceT6EvidencePublicNamespace(t *testing.T) {
	s, _ := trustFixture(t)
	t.Setenv("SWARM_TRUST_TOKEN", trustAdminToken)
	var out, errs bytes.Buffer
	code := run([]string{"--root", s.root, "--json", "evidence", "show"}, &out, &errs)
	if code != 0 {
		t.Fatalf("evidence namespace missing: exit %d", code)
	}
}

func evidenceFixture(t *testing.T) *Store {
	t.Helper()
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.Identities = append(c.Identities, TrustIdentityInput{Subject: "admin-two", Token: "admin-two-evidence-test-token", Integrity: 3, Roles: []string{"admin"}})
	c.EvidenceSigning = &TrustEvidenceSigningPolicy{Enabled: true, KeyTTLSeconds: 3600}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	return s
}
func evidenceCall(t *testing.T, s *Store, op string, body map[string]any, token string) (any, error) {
	t.Helper()
	raw, err := json.Marshal(body)
	if err != nil {
		t.Fatal(err)
	}
	return s.trustEvidenceOperation(context.Background(), op, raw, token, true)
}
func evidencePrepare(t *testing.T, s *Store, event string, rev int) TrustEvidenceKey {
	t.Helper()
	v, err := evidenceCall(t, s, "key-prepare", map[string]any{"schema_version": 1, "event_id": event, "expected_registry_revision": rev, "reason": "prepare exact isolated test key"}, trustAdminToken)
	if err != nil {
		t.Fatal(err)
	}
	return v.(map[string]any)["key"].(TrustEvidenceKey)
}
func evidenceActivate(t *testing.T, s *Store, k TrustEvidenceKey, event string, rev int) {
	t.Helper()
	body := map[string]any{"schema_version": 1, "event_id": event, "expected_registry_revision": rev, "key_id": k.ID, "key_digest": k.Digest, "reason": "independent exact key activation"}
	before := secretStateBytes(t, s)
	if _, err := evidenceCall(t, s, "key-activate", body, trustAdminToken); err == nil {
		t.Fatal("proposer activated own key")
	}
	if !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("denial changed state")
	}
	if _, err := evidenceCall(t, s, "key-activate", body, "admin-two-evidence-test-token"); err != nil {
		t.Fatal(err)
	}
}
func evidenceAnchor(t *testing.T, s *Store) (TrustProofExport, TrustPublicAnchor, TrustEvidenceKey) {
	t.Helper()
	v, err := evidenceCall(t, s, "show", map[string]any{}, trustAdminToken)
	if err != nil {
		t.Fatal(err)
	}
	view := v.(map[string]any)
	v, err = evidenceCall(t, s, "anchor", map[string]any{"key_id": view["active_key_id"], "expected_registry_revision": view["registry_revision"], "expected_sequence": view["last_sequence"], "expected_hash": view["last_hash"]}, trustAdminToken)
	if err != nil {
		t.Fatal(err)
	}
	result := v.(map[string]any)
	k := view["keys"].(map[string]TrustEvidenceKey)[view["active_key_id"].(string)]
	return result["export"].(TrustProofExport), result["anchor"].(TrustPublicAnchor), k
}
func TestActionsConfianceT6EvidenceRotationRevokeAndRestart(t *testing.T) {
	s := evidenceFixture(t)
	k := evidencePrepare(t, s, "key-one", 0)
	evidenceActivate(t, s, k, "activate-one", 1)
	export, anchor, pin := evidenceAnchor(t, s)
	if _, err := verifyPublicEvidence(TrustEvidenceCommand{Export: &export, Anchor: &anchor, TrustedKey: &pin}); err != nil {
		t.Fatal(err)
	}
	k2 := evidencePrepare(t, s, "key-two", 2)
	evidenceActivate(t, s, k2, "activate-two", 3)
	state, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	retired := state.PublicEvidence.Keys[k.ID]
	if retired.State != "retired" {
		t.Fatal("old key not retired")
	}
	if _, err = verifyPublicEvidence(TrustEvidenceCommand{Export: &export, Anchor: &anchor, TrustedKey: &retired}); err != nil {
		t.Fatal("historical retired proof rejected", err)
	}
	e2, a2, pin2 := evidenceAnchor(t, s)
	body := map[string]any{"schema_version": 1, "event_id": "revoke-two", "expected_registry_revision": 4, "key_id": k2.ID, "key_digest": k2.Digest, "reason": "emergency key revocation test"}
	if _, err = evidenceCall(t, s, "key-revoke", body, trustAdminToken); err != nil {
		t.Fatal(err)
	}
	state, _ = s.loadTrustState()
	revoked := state.PublicEvidence.Keys[k2.ID]
	if _, err = verifyPublicEvidence(TrustEvidenceCommand{Export: &e2, Anchor: &a2, TrustedKey: &revoked}); err == nil {
		t.Fatal("revoked trust pin accepted")
	}
	if pin2.State != "active" {
		t.Fatal("independent old pin changed")
	}
	k3 := evidencePrepare(t, s, "key-three", 5)
	evidenceActivate(t, s, k3, "activate-three", 6)
	view, err := s.withTrustLock(false, func(st *trustState) (any, error) { return evidenceExport(st), nil })
	if err != nil {
		t.Fatal(err)
	}
	_ = view
	reopened := &Store{root: s.root, evidenceRuntime: &trustEvidenceRuntime{}}
	v, err := evidenceCall(t, reopened, "show", map[string]any{}, trustAdminToken)
	if err != nil || v.(map[string]any)["material_available"].(map[string]bool)[k3.ID] {
		t.Fatal("restart restored private material", err)
	}
	m := v.(map[string]any)
	before := secretStateBytes(t, s)
	if _, err = evidenceCall(t, reopened, "anchor", map[string]any{"key_id": k3.ID, "expected_registry_revision": 7, "expected_sequence": m["last_sequence"], "expected_hash": m["last_hash"]}, trustAdminToken); err == nil {
		t.Fatal("restart signed without private key")
	}
	if !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("missing key refusal changed state")
	}
}
func TestActionsConfianceT6EvidenceVerifyRejectsCorruption(t *testing.T) {
	s := evidenceFixture(t)
	k := evidencePrepare(t, s, "verify-key", 0)
	evidenceActivate(t, s, k, "verify-activate", 1)
	export, anchor, pin := evidenceAnchor(t, s)
	original := TrustEvidenceCommand{Export: &export, Anchor: &anchor, TrustedKey: &pin}
	raw, _ := json.Marshal(original)
	cases := map[string]func(*TrustEvidenceCommand){"domain": func(c *TrustEvidenceCommand) { c.Anchor.Domain = "other-purpose" }, "signature": func(c *TrustEvidenceCommand) { c.Anchor.Signature = "invalid" }, "wrong-key": func(c *TrustEvidenceCommand) { c.TrustedKey.ID = "wrong" }, "truncation": func(c *TrustEvidenceCommand) { c.Export.Evidence = c.Export.Evidence[:len(c.Export.Evidence)-1] }, "reorder": func(c *TrustEvidenceCommand) {
		c.Export.Evidence[0], c.Export.Evidence[1] = c.Export.Evidence[1], c.Export.Evidence[0]
	}, "payload": func(c *TrustEvidenceCommand) { c.Export.Evidence[0].Reason = "altered" }, "pin-revoked": func(c *TrustEvidenceCommand) { c.TrustedKey.State = "revoked" }, "pin-inconsistent-revocation": func(c *TrustEvidenceCommand) { now := time.Now(); c.TrustedKey.RevokedAt = &now }, "key-period": func(c *TrustEvidenceCommand) { c.TrustedKey.NotAfter = c.TrustedKey.CreatedAt }, "empty": func(c *TrustEvidenceCommand) { c.Export.Evidence = nil }}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			var c TrustEvidenceCommand
			if json.Unmarshal(raw, &c) != nil {
				t.Fatal("fixture decode")
			}
			mutate(&c)
			if _, err := verifyPublicEvidence(c); err == nil {
				t.Fatal("invalid public proof accepted")
			}
		})
	}
}
func TestActionsConfianceT6EvidencePolicyStrictAndPreserved(t *testing.T) {
	for _, raw := range []string{`{"enabled":null,"key_ttl_seconds":10}`, `{"enabled":true,"key_ttl_seconds":0}`, `{"enabled":false}`, `{"enabled":false,"key_ttl_seconds":null}`, `{"enabled":true,"key_ttl_seconds":1.5}`} {
		t.Run(raw, func(t *testing.T) {
			var p TrustEvidenceSigningPolicy
			if json.Unmarshal([]byte(raw), &p) == nil {
				t.Fatal("invalid policy accepted")
			}
		})
	}
	s := evidenceFixture(t)
	c := lifecycleChange(t, "alice", false, 2)
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	st, err := s.loadTrustState()
	if err != nil || st.EvidenceSigning == nil || !bool(st.EvidenceSigning.Enabled) || st.EvidenceSigning.KeyTTLSeconds != 3600 {
		t.Fatal("omitted policy lost", err)
	}
}
func TestActionsConfianceT6EvidencePrivateMaterialNotPersisted(t *testing.T) {
	s := evidenceFixture(t)
	k := evidencePrepare(t, s, "private-key", 0)
	evidenceActivate(t, s, k, "private-activate", 1)
	e, a, _ := evidenceAnchor(t, s)
	s.evidenceRuntime.mu.Lock()
	private := append([]byte{}, s.evidenceRuntime.keys[k.ID]...)
	s.evidenceRuntime.mu.Unlock()
	defer clear(private)
	encoded, _ := json.Marshal(map[string]any{"export": e, "anchor": a})
	for _, value := range [][]byte{private, []byte(base64.RawURLEncoding.EncodeToString(private)), []byte(hex.EncodeToString(private))} {
		if bytes.Contains(secretStateBytes(t, s), value) || bytes.Contains(encoded, value) {
			t.Fatal("private key reached durable output")
		}
	}
}
func TestActionsConfianceT6EvidenceExpiryAndCancellation(t *testing.T) {
	s := evidenceFixture(t)
	p := TrustEvidenceSigningPolicy{Enabled: true, KeyTTLSeconds: 1}
	body := map[string]any{"schema_version": 1, "event_id": "short-policy", "expected_revision": 2, "policy": p, "reason": "isolated one second expiry guard"}
	if _, err := evidenceCall(t, s, "policy-apply", body, trustAdminToken); err != nil {
		t.Fatal(err)
	}
	k := evidencePrepare(t, s, "short-key", 0)
	evidenceActivate(t, s, k, "short-activate", 1)
	time.Sleep(time.Until(k.NotAfter) + time.Millisecond)
	v, err := evidenceCall(t, s, "show", map[string]any{}, trustAdminToken)
	if err != nil {
		t.Fatal(err)
	}
	m := v.(map[string]any)
	before := secretStateBytes(t, s)
	if _, err = evidenceCall(t, s, "anchor", map[string]any{"key_id": k.ID, "expected_registry_revision": 2, "expected_sequence": m["last_sequence"], "expected_hash": m["last_hash"]}, trustAdminToken); err == nil {
		t.Fatal("expired private key signed")
	}
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if _, err = s.trustEvidenceOperation(ctx, "show", []byte("{}"), trustAdminToken, true); err == nil {
		t.Fatal("cancelled read accepted")
	}
	if !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("expiry/cancel refusal changed state")
	}
}
func TestActionsConfianceT6EvidenceRevokeWhileDisabled(t *testing.T) {
	s := evidenceFixture(t)
	k := evidencePrepare(t, s, "disabled-key", 0)
	evidenceActivate(t, s, k, "disabled-key-active", 1)
	if _, err := evidenceCall(t, s, "policy-apply", map[string]any{"schema_version": 1, "event_id": "disabled-key-policy", "expected_revision": 2, "policy": map[string]any{"enabled": false, "key_ttl_seconds": 3600}, "reason": "disable signing before emergency withdrawal"}, trustAdminToken); err != nil {
		t.Fatal(err)
	}
	if _, err := evidenceCall(t, s, "key-revoke", map[string]any{"schema_version": 1, "event_id": "disabled-key-revoke", "expected_registry_revision": 2, "key_id": k.ID, "key_digest": k.Digest, "reason": "emergency withdrawal with disabled policy"}, trustAdminToken); err != nil {
		t.Fatal("disabled signing blocked emergency withdrawal", err)
	}
	st, err := s.loadTrustState()
	if err != nil || st.PublicEvidence.Keys[k.ID].State != "revoked" || st.PublicEvidence.Active != "" {
		t.Fatal("withdrawal not applied", err)
	}
}

func TestActionsConfianceT6EvidenceConcurrentPrepare(t *testing.T) {
	s := evidenceFixture(t)
	start := make(chan struct{})
	results := make(chan error, 8)
	for _, event := range []string{"prepare-a", "prepare-b", "prepare-c", "prepare-d", "prepare-e", "prepare-f", "prepare-g", "prepare-h"} {
		raw, err := json.Marshal(map[string]any{"schema_version": 1, "event_id": event, "expected_registry_revision": 0, "reason": "concurrent exact registry preparation"})
		if err != nil {
			t.Fatal(err)
		}
		go func(raw []byte) {
			<-start
			_, err := s.trustEvidenceOperation(context.Background(), "key-prepare", raw, trustAdminToken, true)
			results <- err
		}(raw)
	}
	close(start)
	accepted := 0
	for i := 0; i < 8; i++ {
		err := <-results
		if err == nil {
			accepted++
			continue
		}
		code := commandFailure(err).Code
		if code != "revision_conflict" && code != "workspace_wait" && code != "operation_pending" {
			t.Fatalf("unexpected preparation refusal: %s", code)
		}
	}
	st, err := s.loadTrustState()
	if err != nil || accepted != 1 || st.PublicEvidence.Revision != 1 || len(st.PublicEvidence.Keys) != 1 || st.PublicEvidence.Pending == "" {
		t.Fatal("concurrent prepares did not select one exact candidate", accepted, err)
	}
	s.evidenceRuntime.mu.Lock()
	defer s.evidenceRuntime.mu.Unlock()
	if len(s.evidenceRuntime.keys) != 1 || len(s.evidenceRuntime.keys[st.PublicEvidence.Pending]) != 64 {
		t.Fatal("private material disagrees with selected candidate")
	}
}
