//go:build linux

package engine

import (
	"fmt"
	"net/url"
	"strings"
	"time"
)

type TrustIntrospectionPolicy struct {
	Enabled                 trustOIDCEnabled `json:"enabled"`
	Endpoint                string           `json:"endpoint"`
	ClientID                string           `json:"client_id"`
	TimeoutSeconds          int              `json:"timeout_seconds"`
	ValidationWindowSeconds int              `json:"validation_window_seconds"`
	MaxSessions             int              `json:"max_sessions"`
	MaxTokenBytes           int              `json:"max_token_bytes"`
}

func (p *TrustIntrospectionPolicy) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "enabled", "endpoint", "client_id", "timeout_seconds", "validation_window_seconds", "max_sessions", "max_token_bytes"); err != nil {
		return err
	}
	type plain TrustIntrospectionPolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustIntrospectionPolicy(v)
	return nil
}
func trustIntrospectionPolicyValue(p *TrustIntrospectionPolicy) TrustIntrospectionPolicy {
	if p == nil {
		return TrustIntrospectionPolicy{TimeoutSeconds: 15, ValidationWindowSeconds: 5, MaxSessions: 64, MaxTokenBytes: 16384}
	}
	return *p
}
func validateTrustIntrospection(p *TrustIntrospectionPolicy, w *TrustWorkloadConfig) error {
	if p == nil {
		return nil
	}
	b := trustPolicyBounds()
	for key, v := range map[string]int{"introspection_timeout_seconds": p.TimeoutSeconds, "introspection_validation_window_seconds": p.ValidationWindowSeconds, "introspection_max_sessions": p.MaxSessions, "introspection_max_token_bytes": p.MaxTokenBytes} {
		if int64(v) < b[key]["min"] || int64(v) > b[key]["max"] {
			return fmt.Errorf("introspection : %s hors limites techniques ; entier positif requis, 0 refusé", key)
		}
	}
	if !bool(p.Enabled) && p.Endpoint == "" && p.ClientID == "" {
		return nil
	}
	if !trustOIDCURL(p.Endpoint) || (strings.TrimSpace(p.ClientID) == "" || strings.ContainsAny(p.ClientID, " \t\r\n")) || w == nil {
		return fmt.Errorf("introspection : endpoint exact et client_id requis")
	}
	u, e := url.Parse(p.Endpoint)
	issuer, x := url.Parse(w.Issuer)
	if e != nil || x != nil || u.Scheme != issuer.Scheme || u.Host != issuer.Host || (bool(p.Enabled) && !bool(w.Enabled)) {
		return fmt.Errorf("introspection : origine exacte de l’émetteur de services requise")
	}
	return nil
}
func trustIntrospectionDenied() error {
	return &CommandError{Code: "authorization_required", Message: "service refusé : introspection active requise ; vérifier configuration, connexion et disponibilité de l’émetteur"}
}
func trustIntrospectionFresh(st *trustState, digest string, now time.Time) bool {
	p := st.serviceProof
	return p != nil && p.material != nil && !p.material.revoked.Load() && p.requestContext != nil && p.requestContext.Err() == nil && p.tokenDigest == digest && p.revision == st.Revision && !now.Before(p.checkedAt) && now.Before(p.expiresAt)
}
