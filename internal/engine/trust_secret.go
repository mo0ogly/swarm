//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"time"
)

type TrustSecretPolicy struct {
	Enabled            trustOIDCEnabled `json:"enabled"`
	MaterialTTLSeconds int              `json:"material_ttl_seconds"`
	GrantTTLSeconds    int              `json:"grant_ttl_seconds"`
	MaxMaterialBytes   int              `json:"max_material_bytes"`
	MaxEntries         int              `json:"max_entries"`
}

func (p *TrustSecretPolicy) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "enabled", "material_ttl_seconds", "grant_ttl_seconds", "max_material_bytes", "max_entries"); err != nil {
		return err
	}
	type plain TrustSecretPolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustSecretPolicy(v)
	return nil
}
func trustSecretPolicyValue(p *TrustSecretPolicy) TrustSecretPolicy {
	if p == nil {
		return TrustSecretPolicy{MaterialTTLSeconds: 300, GrantTTLSeconds: 60, MaxMaterialBytes: 4096, MaxEntries: 16}
	}
	return *p
}
func validateTrustSecretPolicy(p *TrustSecretPolicy) error {
	if p == nil {
		return nil
	}
	maximum := int64((1<<63 - 1) / int64(time.Second))
	if p.MaterialTTLSeconds <= 0 || int64(p.MaterialTTLSeconds) > maximum || p.GrantTTLSeconds <= 0 || p.GrantTTLSeconds > p.MaterialTTLSeconds || p.MaxMaterialBytes <= 0 || p.MaxMaterialBytes > 1<<20 || p.MaxEntries <= 0 || int64(p.MaxEntries) > trustPolicyBounds()["secret_broker_max_entries"]["max"] {
		return fmt.Errorf("courtier : durées, capacité et taille strictement positives requises ; octroi borné par la matière, taille bornée par HTTP1Mio ; 0 refusé")
	}
	return nil
}

type TrustSecretGrant struct {
	DataContext *TrustDataContext `json:"data_context,omitempty"`
	ID          string            `json:"grant_id"`
	Revision    int               `json:"policy_revision"`
	ActionID    string            `json:"action_id"`
	Digest      string            `json:"action_digest"`
	Principal   string            `json:"principal_id"`
	TokenDigest string            `json:"token_digest"`
	Connector   string            `json:"connector_id"`
	Ref         string            `json:"secret_ref"`
	Destination string            `json:"destination"`
	IssuedAt    time.Time         `json:"issued_at"`
	ExpiresAt   time.Time         `json:"expires_at"`
	ConsumedAt  *time.Time        `json:"consumed_at"`
	State       string            `json:"state"`
	Attestation string            `json:"attestation,omitempty"`
}
type trustSecretRequest struct {
	ActionID    string `json:"action_id,omitempty"`
	Digest      string `json:"action_digest,omitempty"`
	Ref         string `json:"secret_ref,omitempty"`
	Connector   string `json:"connector_id,omitempty"`
	Destination string `json:"destination,omitempty"`
	GrantID     string `json:"grant_id,omitempty"`
}

func trustSecretDenied() error {
	return &CommandError{Code: "authorization_required", Message: "octroi secret refusé : vérifier identité, politique, portée, fraîcheur et matière disponible"}
}
func trustSecretObject(raw []byte) (trustSecretRequest, error) {
	var v trustSecretRequest
	if len(bytes.TrimSpace(raw)) == 0 || bytes.TrimSpace(raw)[0] != '{' {
		return v, fmt.Errorf("objet JSON de métadonnées requis")
	}
	if err := strict(raw, &v); err != nil {
		return v, fmt.Errorf("métadonnées d’octroi invalides")
	}
	return v, nil
}

func trustSecretPrepared(raw []byte) bool {
	var v struct {
		Schema    int  `json:"schema_version"`
		Ready     bool `json:"ready"`
		Revision  int  `json:"revision"`
		Protected bool `json:"server_memory_nondumpable"`
		MaxBytes  int  `json:"max_material_bytes"`
	}
	return json.Unmarshal(raw, &v) == nil && v.Schema == 1 && v.Ready && v.Protected && v.Revision > 0 && v.MaxBytes > 0 && v.MaxBytes <= 1<<20
}

func trustSecretDeliveryContext(st *trustState) (*TrustDataContext, error) {
	p := effectiveTrustDataPolicy(st)
	if !bool(p.Enabled) {
		return nil, nil
	}
	c := &TrustDataContext{Classification: "restricted", Destination: "local-attestation", Provider: "swarm", Tool: "sealed-attestation"}
	if !containsTrust(p.Classifications, c.Classification) || !containsTrust(p.Destinations, c.Destination) || !containsTrust(p.Providers, c.Provider) || !containsTrust(p.Tools, c.Tool) {
		return nil, trustSecretDenied()
	}
	return c, nil
}
func trustSecretDeliveryMatches(st *trustState, g TrustSecretGrant) bool {
	c, err := trustSecretDeliveryContext(st)
	if err != nil {
		return false
	}
	if c == nil || g.DataContext == nil {
		return c == nil && g.DataContext == nil
	}
	return *c == *g.DataContext
}
