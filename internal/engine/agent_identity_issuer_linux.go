//go:build linux

package engine

import (
	"context"
	"time"
)

// A controller input, NOT a configuration inferred from a login, business role,
// request, imported registry or first caller. Its protected enrollment, durable
// revocation and independent freshness source are still deployment prerequisites.
// Private Go fields are not an OS isolation boundary.
type agentIdentityIssuerAuthority struct {
	projectRoot      string
	principal        agentIdentityPrincipal
	scope            agentIdentityScope
	trustRevision    int
	enrollmentDigest string
	permissions      []agentIdentityPermission
	delegable        []agentIdentityPermission
	notBefore        time.Time
	expiresAt        time.Time
}

type agentIdentityPublicationIssuerProof struct {
	authority        agentIdentityIssuerAuthority
	tokenDigest      string
	issuedAt         time.Time
	sessionExpiresAt time.Time
	authorizedUntil  time.Time
}

func (a agentIdentityIssuerAuthority) owned() agentIdentityIssuerAuthority {
	a.permissions = append([]agentIdentityPermission(nil), a.permissions...)
	a.delegable = append([]agentIdentityPermission(nil), a.delegable...)
	return a
}

func validateAgentIdentityIssuer(root string, caller agentIdentityCaller, a agentIdentityIssuerAuthority, scope agentIdentityScope, at time.Time) error {
	if caller.actor.Principal.Kind != "human" || caller.actor.InstanceID != "" || caller.bindingID != "" {
		return agentIdentityError("agent_identity_human_issuer_required")
	}
	permissions, valid := agentIdentityPermissionSet(a.permissions)
	_, delegable := agentIdentityPermissionSet(a.delegable)
	if !a.principal.valid() || a.principal.Kind != "human" || a.principal != caller.actor.Principal ||
		a.projectRoot != root || caller.projectRoot != root || !scope.valid() || a.scope != scope ||
		a.trustRevision <= 0 || a.trustRevision != caller.trustRevision || !agentIdentityHashValid(a.enrollmentDigest) ||
		!valid || len(permissions) == 0 || !delegable || !agentIdentitySubset(a.delegable, permissions) {
		return agentIdentityError("agent_identity_issuer_authority_required")
	}
	if at.IsZero() || !caller.verifiedAt.Equal(at) || caller.expiresAt == nil || !at.Before(*caller.expiresAt) ||
		a.notBefore.IsZero() || !a.notBefore.Before(a.expiresAt) || at.Before(a.notBefore) || !at.Before(a.expiresAt) {
		return agentIdentityError("agent_identity_issuer_authority_stale")
	}
	return nil
}

// Publication authority is explicit and separate from its ceiling on new root
// grants. Retained history is not rewritten. Activating a previously unlisted
// root must pass the same ceiling as adding a new root.
func validateAgentIdentityIssuerRoots(a agentIdentityIssuerAuthority, snapshot agentIdentitySnapshot, previous agentIdentitySnapshot, roots, oldRoots []string) error {
	permissions, _ := agentIdentityPermissionSet(a.permissions)
	delegable, _ := agentIdentityPermissionSet(a.delegable)
	active := map[string]bool{}
	for _, id := range oldRoots {
		active[id] = true
	}
	check := func(g agentIdentityGrant) error {
		if g.Issuer != (agentIdentityActor{Principal: a.principal}) || g.Scope != a.scope || g.TrustRevision != a.trustRevision ||
			!agentIdentitySubset(g.Permissions, permissions) || !agentIdentitySubset(g.Delegable, delegable) ||
			g.NotBefore.Before(a.notBefore) || g.ExpiresAt.After(a.expiresAt) {
			return agentIdentityError("agent_identity_issuer_ceiling_exceeded")
		}
		return nil
	}
	for id, g := range snapshot.Grants {
		if _, exists := previous.Grants[id]; !exists && g.ParentID == "" {
			if err := check(g); err != nil {
				return err
			}
		}
	}
	for _, id := range roots {
		if !active[id] {
			if err := check(snapshot.Grants[id]); err != nil {
				return err
			}
		}
	}
	return nil
}

// Produce an exact private publication permit from an authenticated human and
// independently supplied enrollment. This is not a public bootstrap endpoint,
// human approval for a launch, plan-adoption attestation or process start.
func (s *Store) mintAgentIdentityRegistryPermit(ctx context.Context, raw []byte, token string, enrollment agentIdentityIssuerAuthority, base agentIdentityRegistryCheckpoint, eventID string, roots []string) (agentIdentityRegistryPermit, error) {
	var result agentIdentityRegistryPermit
	if ctx == nil || !base.valid() || base.projectRoot != s.root || !safeName(eventID) {
		return result, agentIdentityError("agent_identity_publication_permit_required")
	}
	a := enrollment.owned()
	roots = append([]string(nil), roots...)
	snapshot, err := decodeAgentIdentitySnapshot(raw)
	if err != nil {
		return result, err
	}
	digest, err := agentIdentityDigest(snapshot)
	if err != nil {
		return result, err
	}
	_, err = s.withTrustReadAvailable(func(state *trustState) (any, error) {
		// Reject before looking up a work or taking SQL resources. Repeat this
		// after the writer wait; this first check is never a cached decision.
		{
			at := time.Now().UTC()
			caller, err := resolveAgentIdentityCaller(s.root, state, token, at)
			if err != nil {
				return nil, err
			}
			if err = validateAgentIdentityIssuer(s.root, caller, a, a.scope, at); err != nil {
				return nil, err
			}
			if a.scope.WorkID != base.workID {
				return nil, agentIdentityError("agent_identity_issuer_authority_required")
			}
		}
		tx, err := s.db.BeginTx(ctx, nil)
		if err != nil {
			return nil, err
		}
		defer tx.Rollback()
		// Same order as the registry consumer: Trust -> SQLite writer -> time.
		if _, err = tx.ExecContext(ctx, `UPDATE works SET revision=revision WHERE id=?`, base.workID); err != nil {
			return nil, err
		}
		at := time.Now().UTC()
		caller, err := resolveAgentIdentityCaller(s.root, state, token, at)
		if err != nil {
			return nil, err
		}
		if err = validateAgentIdentityIssuer(s.root, caller, a, a.scope, at); err != nil {
			return nil, err
		}
		var version int
		if err = tx.QueryRowContext(ctx, "PRAGMA user_version").Scan(&version); err != nil {
			return nil, err
		}
		if version != schemaVersion {
			return nil, agentIdentityError("storage_version_conflict")
		}
		scope, err := readAgentIdentityRecordedPlan(ctx, tx, base.workID)
		if err != nil {
			return nil, err
		}
		if err = validateAgentIdentityIssuer(s.root, caller, a, scope, at); err != nil {
			return nil, err
		}
		head := agentIdentityRegistryHead{Schema: 1, ProjectDigest: hash([]byte(s.root)), Scope: scope, Revision: snapshot.Revision,
			SnapshotDigest: digest, TrustRevision: state.Revision, Roots: roots, Publisher: caller.actor.Principal, EventID: eventID, Previous: base.headDigest}
		if err = validateAgentIdentityRegistrySnapshot(snapshot, head); err != nil {
			return nil, err
		}
		current := state.AgentIdentities[base.workID]
		if current.Pending != nil {
			return nil, agentIdentityError("agent_identity_publication_pending")
		}
		var previous agentIdentitySnapshot
		var oldRoots []string
		if current.Head == nil {
			var count int
			if base.headDigest != "" || snapshot.Revision != 1 {
				return nil, agentIdentityError("agent_identity_checkpoint_mismatch")
			}
			if err = tx.QueryRowContext(ctx, `SELECT (SELECT count(*) FROM agent_identity_snapshots WHERE work_id=?)+(SELECT count(*) FROM agent_identity_publications WHERE work_id=?)`, base.workID, base.workID).Scan(&count); err != nil {
				return nil, err
			}
			if count != 0 {
				return nil, agentIdentityError("agent_identity_import_not_authority")
			}
		} else {
			previous, _, err = loadAgentIdentityRegistry(ctx, tx, state, base)
			if err != nil {
				return nil, err
			}
			if err = validateAgentIdentityTransition(previous, snapshot); err != nil {
				return nil, err
			}
			oldRoots = current.Head.Roots
		}
		if err = validateAgentIdentityIssuerRoots(a, snapshot, previous, roots, oldRoots); err != nil {
			return nil, err
		}
		// History reads may take time; refresh identity and enrollment at issuance.
		at = time.Now().UTC()
		caller, err = resolveAgentIdentityCaller(s.root, state, token, at)
		if err != nil {
			return nil, err
		}
		if err = validateAgentIdentityIssuer(s.root, caller, a, scope, at); err != nil {
			return nil, err
		}
		until := a.expiresAt
		if caller.expiresAt.Before(until) {
			until = *caller.expiresAt
		}
		result = agentIdentityRegistryPermit{base: base, nextDigest: digest, roots: roots, publisher: caller.actor.Principal,
			trustRevision: state.Revision, eventID: eventID, scope: scope,
			issuer: &agentIdentityPublicationIssuerProof{authority: a, tokenDigest: trustTokenHash(token), issuedAt: at, sessionExpiresAt: *caller.expiresAt, authorizedUntil: until}}
		return nil, tx.Commit()
	})
	if err != nil {
		return agentIdentityRegistryPermit{}, err
	}
	return result, nil
}

func validateAgentIdentityPublicationIssuer(permit agentIdentityRegistryPermit, caller agentIdentityCaller, token string, at time.Time) error {
	proof := permit.issuer
	if proof == nil {
		// Existing controller-capability fixtures predate the human producer.
		// There is no public adapter to this low-level publication API. A public
		// consumer must only receive permits from the human producer above.
		return nil
	}
	if proof.tokenDigest != trustTokenHash(token) || at.Before(proof.issuedAt) || !at.Before(proof.authorizedUntil) {
		return agentIdentityError("agent_identity_issuer_authority_stale")
	}
	return validateAgentIdentityIssuer(permit.base.projectRoot, caller, proof.authority, permit.scope, at)
}
