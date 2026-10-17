//go:build linux

package engine

import (
	"bytes"
	"crypto/ed25519"
	"encoding/base64"
	"encoding/json"
	"strings"
)

func parseTrustEvidence(op string, raw []byte) (TrustEvidenceCommand, error) {
	var c TrustEvidenceCommand
	limit := 1 << 20
	if op == "verify" {
		limit = 2 * trustStateMaxBytes
	}
	if len(raw) > limit || !bytes.HasPrefix(bytes.TrimSpace(raw), []byte("{")) || strict(raw, &c) != nil {
		return c, trustEvidenceInvalid()
	}
	var fields map[string]json.RawMessage
	if json.Unmarshal(raw, &fields) != nil {
		return c, trustEvidenceInvalid()
	}
	allowed := []string{}
	switch op {
	case "show":
	case "archive-show":
		allowed = []string{"archive_id"}
	case "archive-preview", "archive-create":
		allowed = []string{"schema_version", "event_id", "expected_revision", "expected_sequence", "expected_hash", "anchor", "reason"}
	case "archive-release":
		allowed = []string{"schema_version", "event_id", "expected_revision", "archive_id", "reason"}
	case "policy-preview", "policy-apply":
		allowed = []string{"schema_version", "event_id", "expected_revision", "policy", "reason"}
	case "key-prepare":
		allowed = []string{"schema_version", "event_id", "expected_registry_revision", "reason"}
	case "key-activate", "key-revoke":
		allowed = []string{"schema_version", "event_id", "expected_registry_revision", "key_id", "key_digest", "reason"}
	case "anchor":
		allowed = []string{"expected_registry_revision", "key_id", "expected_sequence", "expected_hash"}
	case "verify":
		allowed = []string{"export", "anchor", "trusted_key", "trusted_checkpoint"}
	default:
		return c, trustEvidenceInvalid()
	}
	for k, v := range fields {
		if !containsTrust(allowed, k) || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return c, trustEvidenceInvalid()
		}
	}
	if op != "show" && op != "archive-show" && op != "anchor" && op != "verify" && (c.Schema != 1 || !safeName(c.EventID) || len(strings.TrimSpace(c.Reason)) < 8) {
		return c, trustEvidenceInvalid()
	}
	return c, nil
}
func validateTrustEvidenceRegistry(r *trustEvidenceRegistry) error {
	if r == nil {
		return nil
	}
	if r.Revision < 0 {
		return trustEvidenceInvalid()
	}
	for id, a := range r.Archives {
		if id != a.ID || !trustArchiveID(id) || a.Sequence < 1 || a.Hash == "" || a.KeyID == "" || a.CreatedAt.IsZero() || a.CreatedBy == "" || a.RetentionDays < 1 || a.PolicyRevision < 1 || (a.ReleasedAt == nil) != (a.ReleasedBy == "") {
			return trustEvidenceInvalid()
		}
		if a.ReleasedAt != nil && !archiveProtection(a, *a.ReleasedAt)["minimum_retention_satisfied"].(bool) {
			return trustEvidenceInvalid()
		}
	}
	active, pending := 0, 0
	for id, k := range r.Keys {
		if id != k.ID {
			return trustEvidenceInvalid()
		}
		if _, err := evidencePublicKey(k); err != nil {
			return err
		}
		switch k.State {
		case "active":
			active++
			if r.Active != id || k.ActivatedAt == nil || k.ActivatedBy == "" || k.ActivatedBy == k.Proposer {
				return trustEvidenceInvalid()
			}
		case "pending":
			pending++
			if r.Pending != id {
				return trustEvidenceInvalid()
			}
		case "retired":
			if k.RetiredAt == nil || k.ActivatedAt == nil {
				return trustEvidenceInvalid()
			}
		case "revoked":
			if k.RevokedAt == nil {
				return trustEvidenceInvalid()
			}
		case "abandoned":
		default:
			return trustEvidenceInvalid()
		}
	}
	if active > 1 || pending > 1 || (r.Active != "" && active != 1) || (r.Pending != "" && pending != 1) {
		return trustEvidenceInvalid()
	}
	return nil
}
func publicAnchorBytes(a TrustPublicAnchor) []byte {
	a.Signature = ""
	raw, _ := json.Marshal(a)
	return raw
}
func verifyPublicEvidence(c TrustEvidenceCommand) (any, error) {
	if c.Export == nil || c.Anchor == nil || c.TrustedKey == nil {
		return nil, trustEvidenceInvalid()
	}
	e, a, k := *c.Export, *c.Anchor, *c.TrustedKey
	if err := verifyPublicAnchor(a, k); err != nil {
		return nil, err
	}
	if a.Schema != 3 || a.Domain != "swarm-public-evidence-anchor-v1" || a.KeyID != k.ID || a.PolicyRevision != k.PolicyRevision || a.RegistryRevision < 1 || a.Sequence < 1 || a.ExportDigest != hashMust(e) || e.Schema != 2 || !e.Simulation || len(e.Evidence) != a.Sequence || k.ActivatedAt == nil || a.SignedAt.Before(*k.ActivatedAt) || !a.SignedAt.Before(k.NotAfter) || (k.RetiredAt != nil && a.SignedAt.After(*k.RetiredAt)) {
		return nil, trustEvidenceInvalid()
	}
	if err := verifyTrustEvidenceChain(e); err != nil {
		return nil, err
	}
	prev := ""
	for i, ev := range e.Evidence {
		if ev.Sequence != i+1 || ev.PreviousHash != prev || trustEvidenceHash(ev) != ev.Hash {
			return nil, trustEvidenceInvalid()
		}
		prev = ev.Hash
	}
	if prev != a.Hash {
		return nil, trustEvidenceInvalid()
	}

	if c.TrustedCheckpoint != nil {
		checkpoint := c.TrustedCheckpoint
		if checkpoint.Anchor == nil || checkpoint.TrustedKey == nil {
			return nil, trustEvidenceInvalid()
		}
		known := *checkpoint.Anchor
		if err := verifyPublicAnchor(known, *checkpoint.TrustedKey); err != nil {
			return nil, err
		}
		if known.Sequence > a.Sequence || e.Evidence[known.Sequence-1].Hash != known.Hash {
			return nil, trustEvidenceInvalid()
		}
		prefix := TrustProofExport{Schema: e.Schema, Simulation: e.Simulation, Evidence: e.Evidence[:known.Sequence], Originals: map[string][]byte{}}
		for _, ev := range prefix.Evidence {
			prefix.Originals[ev.OriginalHash] = e.Originals[ev.OriginalHash]
		}
		if hashMust(prefix) != known.ExportDigest {
			return nil, trustEvidenceInvalid()
		}
		return map[string]any{"verified": true, "sequence": a.Sequence, "key_id": k.ID, "key_state": k.State, "simulation": true, "trust_source": "caller_supplied_independent_public_key", "checkpoint_verified": true, "checkpoint_sequence": known.Sequence, "checkpoint_hash": known.Hash}, nil
	}
	return map[string]any{"checkpoint_verified": false, "verified": true, "sequence": a.Sequence, "key_id": k.ID, "key_state": k.State, "simulation": true, "trust_source": "caller_supplied_independent_public_key"}, nil
}

func verifyPublicAnchor(a TrustPublicAnchor, k TrustEvidenceKey) error {
	pub, err := evidencePublicKey(k)
	if err != nil {
		return err
	}
	if (k.State != "active" && k.State != "retired") || k.RevokedAt != nil || k.ActivatedAt == nil || k.ActivatedBy == "" || k.ActivatedBy == k.Proposer || k.ActivatedAt.Before(k.CreatedAt) || !k.ActivatedAt.Before(k.NotAfter) || (k.State == "retired" && k.RetiredAt == nil) || (k.State == "active" && k.RetiredAt != nil) {
		return trustEvidenceDenied()
	}
	if a.Schema != 3 || a.Domain != "swarm-public-evidence-anchor-v1" || a.KeyID != k.ID || a.PolicyRevision != k.PolicyRevision || a.RegistryRevision < 1 || a.Sequence < 1 || a.Hash == "" || a.ExportDigest == "" || a.SignedAt.Before(*k.ActivatedAt) || !a.SignedAt.Before(k.NotAfter) || (k.RetiredAt != nil && a.SignedAt.After(*k.RetiredAt)) {
		return trustEvidenceInvalid()
	}
	sig, err := base64.RawURLEncoding.DecodeString(a.Signature)
	if err != nil || len(sig) != ed25519.SignatureSize || !ed25519.Verify(pub, publicAnchorBytes(a), sig) {
		return trustEvidenceInvalid()
	}
	return nil
}
