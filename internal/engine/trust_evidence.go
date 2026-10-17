//go:build linux

package engine

import (
	"encoding/base64"
	"fmt"
	"time"
)

type TrustEvidenceSigningPolicy struct {
	Enabled       trustOIDCEnabled `json:"enabled"`
	KeyTTLSeconds int              `json:"key_ttl_seconds"`
}

func (p *TrustEvidenceSigningPolicy) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "enabled", "key_ttl_seconds"); err != nil {
		return err
	}
	type plain TrustEvidenceSigningPolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustEvidenceSigningPolicy(v)
	return validateTrustEvidencePolicy(p)
}
func trustEvidencePolicy(p *TrustEvidenceSigningPolicy) TrustEvidenceSigningPolicy {
	if p == nil {
		return TrustEvidenceSigningPolicy{KeyTTLSeconds: 86400}
	}
	return *p
}
func validateTrustEvidencePolicy(p *TrustEvidenceSigningPolicy) error {
	if p != nil && (p.KeyTTLSeconds <= 0 || int64(p.KeyTTLSeconds) > trustPolicyBounds()["evidence_key_ttl_seconds"]["max"]) {
		return fmt.Errorf("preuve publique : durée de clé en secondes strictement positive requise ; 0 refusé")
	}
	return nil
}

type TrustEvidenceKey struct {
	ID             string     `json:"key_id"`
	PublicKey      string     `json:"public_key"`
	Digest         string     `json:"key_digest"`
	PolicyRevision int        `json:"policy_revision"`
	Proposer       string     `json:"proposer"`
	CreatedAt      time.Time  `json:"created_at"`
	NotAfter       time.Time  `json:"not_after"`
	State          string     `json:"state"`
	ActivatedBy    string     `json:"activated_by,omitempty"`
	ActivatedAt    *time.Time `json:"activated_at,omitempty"`
	RetiredAt      *time.Time `json:"retired_at,omitempty"`
	RevokedAt      *time.Time `json:"revoked_at,omitempty"`
}
type trustEvidenceReply struct {
	Digest    string `json:"digest"`
	Authority string `json:"authority"`
	Value     []byte `json:"value"`
}
type trustEvidenceRegistry struct {
	Archives map[string]TrustEvidenceArchive `json:"archives,omitempty"`
	Revision int                             `json:"revision"`
	Active   string                          `json:"active_key_id"`
	Pending  string                          `json:"pending_key_id"`
	Keys     map[string]TrustEvidenceKey     `json:"keys"`
	Replies  map[string]trustEvidenceReply   `json:"replies"`
}
type TrustPublicAnchor struct {
	Schema           int       `json:"schema_version"`
	Domain           string    `json:"domain"`
	KeyID            string    `json:"key_id"`
	RegistryRevision int       `json:"registry_revision"`
	PolicyRevision   int       `json:"policy_revision"`
	Sequence         int       `json:"sequence"`
	Hash             string    `json:"hash"`
	ExportDigest     string    `json:"export_digest"`
	SignedAt         time.Time `json:"signed_at"`
	Signature        string    `json:"signature"`
}
type TrustEvidenceCheckpoint struct {
	Anchor     *TrustPublicAnchor `json:"anchor"`
	TrustedKey *TrustEvidenceKey  `json:"trusted_key"`
}

type TrustEvidenceCommand struct {
	ArchiveID                string                      `json:"archive_id,omitempty"`
	Schema                   int                         `json:"schema_version,omitempty"`
	EventID                  string                      `json:"event_id,omitempty"`
	ExpectedRevision         *int                        `json:"expected_revision,omitempty"`
	ExpectedRegistryRevision *int                        `json:"expected_registry_revision,omitempty"`
	Policy                   *TrustEvidenceSigningPolicy `json:"policy,omitempty"`
	KeyID                    string                      `json:"key_id,omitempty"`
	KeyDigest                string                      `json:"key_digest,omitempty"`
	Reason                   string                      `json:"reason,omitempty"`
	ExpectedSequence         *int                        `json:"expected_sequence,omitempty"`
	ExpectedHash             string                      `json:"expected_hash,omitempty"`
	Export                   *TrustProofExport           `json:"export,omitempty"`
	Anchor                   *TrustPublicAnchor          `json:"anchor,omitempty"`
	TrustedCheckpoint        *TrustEvidenceCheckpoint    `json:"trusted_checkpoint,omitempty"`
	TrustedKey               *TrustEvidenceKey           `json:"trusted_key,omitempty"`
}

func trustEvidenceDenied() error {
	return &CommandError{Code: "authorization_required", Message: "preuve publique refusée : vérifier identité, politique, clé, second administrateur, version et contenu exact"}
}
func trustEvidenceInvalid() error {
	return fmt.Errorf("preuve publique : commande, clé ou ancre invalide")
}
func evidenceKeyDigest(k TrustEvidenceKey) string {
	return hashMust(struct {
		ID, Public, Proposer string
		Revision             int
		Created, Expires     time.Time
	}{k.ID, k.PublicKey, k.Proposer, k.PolicyRevision, k.CreatedAt, k.NotAfter})
}
func evidencePublicKey(k TrustEvidenceKey) ([]byte, error) {
	raw, err := base64.RawURLEncoding.DecodeString(k.PublicKey)
	if err != nil || len(raw) != 32 || hash(raw) != k.ID || k.Digest != evidenceKeyDigest(k) || k.Proposer == "" || k.PolicyRevision < 1 || k.CreatedAt.IsZero() || !k.NotAfter.After(k.CreatedAt) {
		return nil, trustEvidenceInvalid()
	}
	return raw, nil
}
