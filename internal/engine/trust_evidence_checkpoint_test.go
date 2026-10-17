//go:build linux

package engine

import (
	"bytes"
	"crypto/ed25519"
	"encoding/base64"
	"encoding/json"
	"testing"
)

func TestActionsConfianceT6EvidenceCheckpoint(t *testing.T) {
	s := evidenceFixture(t)
	k := evidencePrepare(t, s, "checkpoint-one", 0)
	evidenceActivate(t, s, k, "checkpoint-activate-one", 1)
	oldExport, oldAnchor, oldPin := evidenceAnchor(t, s)
	k2 := evidencePrepare(t, s, "checkpoint-two", 2)
	evidenceActivate(t, s, k2, "checkpoint-activate-two", 3)
	exp, anchor, pin := evidenceAnchor(t, s)
	before := secretStateBytes(t, s)
	verify := func(t *testing.T, e TrustProofExport, a TrustPublicAnchor, key TrustEvidenceKey, checkpoint any, want bool) {
		t.Helper()
		raw, _ := json.Marshal(map[string]any{"export": e, "anchor": a, "trusted_key": key, "trusted_checkpoint": checkpoint})
		cmd, err := parseTrustEvidence("verify", raw)
		var value any
		if err == nil {
			value, err = verifyPublicEvidence(cmd)
		}
		if (err == nil) != want {
			t.Fatalf("checkpoint acceptance=%t wanted=%t: %v", err == nil, want, err)
		}
		if want && value.(map[string]any)["checkpoint_verified"] != true {
			t.Fatal("checkpoint was not actually compared")
		}
		if !bytes.Equal(before, secretStateBytes(t, s)) {
			t.Fatal("verification mutated source state")
		}
	}
	checkpoint := map[string]any{"anchor": oldAnchor, "trusted_key": oldPin}
	t.Run("equal", func(t *testing.T) { verify(t, oldExport, oldAnchor, oldPin, checkpoint, true) })
	t.Run("rotation-extension", func(t *testing.T) { verify(t, exp, anchor, pin, checkpoint, true) })
	t.Run("rollback-signed-old-bundle", func(t *testing.T) {
		verify(t, oldExport, oldAnchor, oldPin, map[string]any{"anchor": anchor, "trusted_key": pin}, false)
	})
	t.Run("missing", func(t *testing.T) { verify(t, exp, anchor, pin, map[string]any{}, false) })
	t.Run("null", func(t *testing.T) { verify(t, exp, anchor, pin, nil, false) })
	t.Run("bad-signature", func(t *testing.T) {
		a := oldAnchor
		a.Signature = "invalid"
		verify(t, exp, anchor, pin, map[string]any{"anchor": a, "trusted_key": oldPin}, false)
	})
	t.Run("revoked-pin", func(t *testing.T) {
		p := oldPin
		p.State = "revoked"
		verify(t, exp, anchor, pin, map[string]any{"anchor": oldAnchor, "trusted_key": p}, false)
	})
	t.Run("wrong-pin", func(t *testing.T) {
		verify(t, exp, anchor, pin, map[string]any{"anchor": oldAnchor, "trusted_key": pin}, false)
	})
	t.Run("signed-divergent-history", func(t *testing.T) {
		raw, _ := json.Marshal(exp)
		var changed TrustProofExport
		_ = json.Unmarshal(raw, &changed)
		changed.Evidence[0].Reason = "different attributed history"
		previous := ""
		for i := range changed.Evidence {
			changed.Evidence[i].PreviousHash = previous
			changed.Evidence[i].Hash = trustEvidenceHash(changed.Evidence[i])
			previous = changed.Evidence[i].Hash
		}
		a := anchor
		a.Hash = previous
		a.ExportDigest = hashMust(changed)
		s.evidenceRuntime.mu.Lock()
		a.Signature = base64.RawURLEncoding.EncodeToString(ed25519.Sign(s.evidenceRuntime.keys[pin.ID], publicAnchorBytes(a)))
		s.evidenceRuntime.mu.Unlock()
		if _, err := verifyPublicEvidence(TrustEvidenceCommand{Export: &changed, Anchor: &a, TrustedKey: &pin}); err != nil {
			t.Fatal("counterexample is not a valid signed branch", err)
		}
		verify(t, changed, a, pin, checkpoint, false)
	})
}
