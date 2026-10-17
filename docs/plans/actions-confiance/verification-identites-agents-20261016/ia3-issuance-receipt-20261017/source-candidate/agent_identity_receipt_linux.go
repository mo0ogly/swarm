//go:build linux

package engine

import "time"

// Historical controller-supplied authorization, NOT an independently signed
// enrollment or a capability reconstructed from imported/persisted data.
// The registry checkpoint authenticates the head containing this receipt.
type agentIdentityIssuanceReceipt struct {
	Schema           int                       `json:"schema_version"`
	ProjectDigest    string                    `json:"project_context_digest"`
	Principal        agentIdentityPrincipal    `json:"principal"`
	Scope            agentIdentityScope        `json:"scope"`
	TrustRevision    int                       `json:"trust_configuration_revision"`
	EnrollmentDigest string                    `json:"enrollment_digest"`
	Permissions      []agentIdentityPermission `json:"permissions"`
	Delegable        []agentIdentityPermission `json:"delegable"`
	NotBefore        time.Time                 `json:"not_before"`
	ExpiresAt        time.Time                 `json:"expires_at"`
	SessionDigest    string                    `json:"session_digest"`
	SessionExpiresAt time.Time                 `json:"session_expires_at"`
	IssuedAt         time.Time                 `json:"issued_at"`
	AuthorizedUntil  time.Time                 `json:"authorized_until"`
}

func (r *agentIdentityIssuanceReceipt) owned() *agentIdentityIssuanceReceipt {
	if r == nil {
		return nil
	}
	result := *r
	result.Permissions = append([]agentIdentityPermission(nil), r.Permissions...)
	result.Delegable = append([]agentIdentityPermission(nil), r.Delegable...)
	return &result
}

func (p *agentIdentityPublicationIssuerProof) receipt() *agentIdentityIssuanceReceipt {
	if p == nil {
		return nil
	}
	a := p.authority
	return (&agentIdentityIssuanceReceipt{Schema: 1, ProjectDigest: hash([]byte(a.projectRoot)), Principal: a.principal,
		Scope: a.scope, TrustRevision: a.trustRevision, EnrollmentDigest: a.enrollmentDigest, Permissions: a.permissions, Delegable: a.delegable,
		NotBefore: a.notBefore, ExpiresAt: a.expiresAt, SessionDigest: p.tokenDigest, SessionExpiresAt: p.sessionExpiresAt,
		IssuedAt: p.issuedAt, AuthorizedUntil: p.authorizedUntil}).owned()
}

// Validate historical consistency at issuance, not freshness at read time.
// A reader must still authenticate the entire head using its private checkpoint.
func (r *agentIdentityIssuanceReceipt) valid(h agentIdentityRegistryHead) bool {
	if r == nil {
		return false
	}
	permissions, valid := agentIdentityPermissionSet(r.Permissions)
	_, delegable := agentIdentityPermissionSet(r.Delegable)
	if r.Schema != 1 || r.ProjectDigest != h.ProjectDigest || r.Principal != h.Publisher || r.Principal.Kind != "human" || !r.Principal.valid() ||
		r.Scope != h.Scope || r.TrustRevision != h.TrustRevision || !agentIdentityHashValid(r.EnrollmentDigest) || !agentIdentityHashValid(r.SessionDigest) ||
		!valid || len(permissions) == 0 || !delegable || !agentIdentitySubset(r.Delegable, permissions) ||
		r.NotBefore.IsZero() || !r.NotBefore.Before(r.ExpiresAt) || r.IssuedAt.IsZero() || r.IssuedAt.Before(r.NotBefore) {
		return false
	}
	until := r.ExpiresAt
	if r.SessionExpiresAt.Before(until) {
		until = r.SessionExpiresAt
	}
	return r.AuthorizedUntil.Equal(until) && r.IssuedAt.Before(until)
}
