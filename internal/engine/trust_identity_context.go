//go:build linux

package engine

import (
	"crypto/hmac"
	"fmt"
	"strings"
	"time"
)

type TrustIdentityAssurance struct {
	Method           string     `json:"method"`
	ACR              string     `json:"acr,omitempty"`
	ProviderAuthTime *time.Time `json:"provider_authenticated_at"`
}
type TrustIdentityAttributes struct {
	RoleMappingDigest string   `json:"role_mapping_digest,omitempty"`
	Roles             []string `json:"roles"`
	Integrity         int      `json:"integrity"`
}
type TrustIdentityContext struct {
	Schema          int                     `json:"schema_version"`
	Revision        int                     `json:"configuration_revision"`
	PrincipalID     string                  `json:"principal_id"`
	PrincipalKind   string                  `json:"principal_kind"`
	Issuer          *string                 `json:"issuer"`
	Subject         *string                 `json:"subject"`
	Assurance       TrustIdentityAssurance  `json:"assurance"`
	Attributes      TrustIdentityAttributes `json:"attributes"`
	DelegationID    *string                 `json:"delegation_id"`
	AuthenticatedAt *time.Time              `json:"authenticated_at"`
	ExpiresAt       *time.Time              `json:"expires_at"`
	TokenDigest     string                  `json:"token_digest"`
}

func trustContextString(v string) *string {
	if v == "" {
		return nil
	}
	return &v
}
func trustContextTime(v time.Time) *time.Time {
	if v.IsZero() {
		return nil
	}
	v = v.UTC()
	return &v
}
func resolveTrustIdentityAt(st *trustState, token string, now time.Time) (trustIdentity, error) {
	id, err := resolveTrustIdentityUnchecked(st, token, now)
	if err != nil {
		return id, err
	}
	if bool(trustIntrospectionPolicyValue(st.WorkloadIntrospection).Enabled) {
		if id.Context.PrincipalKind == "service" && !trustIntrospectionFresh(st, id.Context.TokenDigest, time.Now().UTC()) {
			return trustIdentity{}, trustIntrospectionDenied()
		}
		if id.Context.PrincipalKind == "local" && st.Workloads != nil {
			for _, c := range st.Workloads.Clients {
				if c.Subject == id.Subject {
					return trustIdentity{}, trustIntrospectionDenied()
				}
			}
		}
	}
	return id, nil
}
func resolveTrustIdentityUnchecked(st *trustState, token string, now time.Time) (trustIdentity, error) {
	denied := func() (trustIdentity, error) {
		return trustIdentity{}, &CommandError{Code: "authorization_required", Message: "identité non reconnue ou suspendue"}
	}
	if !trustCredentialValid(token) {
		return denied()
	}
	digest := trustTokenHash(token)
	var id trustIdentity
	var session trustOIDCSession
	kind := ""
	for _, candidate := range st.Identities {
		if hmac.Equal([]byte(candidate.TokenHash), []byte(digest)) {
			id = candidate
			kind = "local"
			break
		}
	}
	if kind == "" {
		if v, ok := st.OIDCSessions[digest]; ok && v.Revision == st.Revision && now.Before(v.ExpiresAt) && st.OIDC != nil && bool(st.OIDC.Enabled) {
			session = v
			kind = "human"
		}
		if kind == "" {
			if v, ok := st.WorkloadSessions[digest]; ok && v.Revision == st.Revision && now.Before(v.ExpiresAt) && st.Workloads != nil && bool(st.Workloads.Enabled) {
				session = v
				kind = "service"
			}
		}
		for _, candidate := range st.Identities {
			if candidate.Subject == session.Subject {
				id = candidate
				break
			}
		}
	}
	if kind == "" || id.Subject == "" || id.Disabled {
		return denied()
	}
	if kind != "local" {
		var err error
		id, err = trustSessionGroupRoles(st, session, id)
		if err != nil {
			return trustIdentity{}, err
		}
	}
	c := &TrustIdentityContext{Schema: 1, Revision: st.Revision, PrincipalID: id.Subject, PrincipalKind: kind, TokenDigest: digest, Attributes: TrustIdentityAttributes{Roles: append([]string{}, id.Roles...), Integrity: id.Integrity}}
	c.Attributes.RoleMappingDigest = session.GroupMappingDigest
	if kind == "local" {
		c.Issuer = trustContextString("urn:swarm:local")
		c.Subject = trustContextString(id.Subject)
		c.AuthenticatedAt = trustContextTime(now)
		c.Assurance.Method = "local_bearer"
	} else {
		c.Issuer = trustContextString(session.Issuer)
		c.Subject = trustContextString(session.ExternalSubject)
		c.AuthenticatedAt = trustContextTime(session.AuthenticatedAt)
		c.ExpiresAt = trustContextTime(session.ExpiresAt)
		c.Assurance.ACR = session.ACR
		c.Assurance.ProviderAuthTime = trustContextTime(session.ProviderAuthTime)
		if kind == "human" {
			c.Assurance.Method = "oidc_code_pkce"
		} else {
			c.Assurance.Method = "private_key_jwt"
		}
	}
	id.Context = c
	return id, nil
}
func (s *Store) trustIdentityContext(raw []byte, token string, now time.Time) (any, error) {
	var cmd struct {
		MandateID string `json:"mandate_id,omitempty"`
	}
	if !strings.HasPrefix(strings.TrimSpace(string(raw)), "{") {
		return nil, fmt.Errorf("objet JSON requis")
	}
	if err := strict(raw, &cmd); err != nil {
		return nil, err
	}
	return s.withTrustLock(false, func(st *trustState) (any, error) {
		if now.IsZero() {
			now = time.Now().UTC()
		}
		id, err := findTrustIdentityAt(st, token, now)
		if err != nil {
			return nil, err
		}
		if cmd.MandateID != "" {
			m, ok := st.Mandates[cmd.MandateID]
			if !ok || m.Subject != id.Subject || trustActiveMandateReason(st, m.ID, now) != "" {
				return nil, &CommandError{Code: "authorization_required", Message: "mandat actif du principal requis"}
			}
			id.Context.DelegationID = trustContextString(m.ID)
		}
		return id.Context, nil
	})
}

func trustEvidenceHash(ev TrustEvidence) string {
	_, digest := trustCanonical(struct {
		Sequence                                                int `json:"sequence"`
		PreviousHash, Kind, Actor                               string
		Integrity                                               int
		ObjectID, ContentHash, OriginalHash, Result, Reason, At string
		IdentityContext                                         *TrustIdentityContext    `json:"identity_context,omitempty"`
		PolicyContext                                           *TrustProofPolicyContext `json:"policy_context,omitempty"`
		EffectReceipt                                           *TrustEffectProof        `json:"effect_receipt,omitempty"`
	}{ev.Sequence, ev.PreviousHash, ev.Kind, ev.Actor, ev.Integrity, ev.ObjectID, ev.ContentHash, ev.OriginalHash, ev.Result, ev.Reason, ev.At, ev.IdentityContext, ev.PolicyContext, ev.EffectReceipt})
	return digest
}
