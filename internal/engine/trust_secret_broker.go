//go:build linux

package engine

import (
	"context"
	"encoding/json"
	"fmt"
	"golang.org/x/sys/unix"
	"sync"
	"time"
)

type trustSecretMaterial struct {
	value    []byte
	revision int
	expires  time.Time
	grant    string
}
type trustSecretBroker struct {
	s *Store
	*trustSecretMemory
}
type trustSecretMemory struct {
	mu            sync.Mutex
	readyRevision int
	entries       map[string]*trustSecretMaterial
}

func newTrustSecretBroker(s *Store) *trustSecretBroker {
	return &trustSecretBroker{s: s, trustSecretMemory: &trustSecretMemory{entries: map[string]*trustSecretMaterial{}}}
}
func (b *trustSecretBroker) lock() error {
	if !b.mu.TryLock() {
		return &CommandError{Code: "workspace_wait", Message: "courtier occupé ; vérifier son statut avant reprise"}
	}
	return nil
}
func (b *trustSecretBroker) remove(ref string) {
	if m := b.entries[ref]; m != nil {
		clear(m.value)
		delete(b.entries, ref)
	}
}
func (b *trustSecretBroker) purge(revision int, now time.Time) {
	for ref, m := range b.entries {
		if m.revision != revision || !now.Before(m.expires) {
			b.remove(ref)
		}
	}
}
func trustSecretAdmin(id trustIdentity) bool  { return id.Integrity == 3 && trustHasRole(id, "admin") }
func trustSecretEffect(id trustIdentity) bool { return id.Integrity >= 2 && trustHasRole(id, "effect") }
func trustSecretAudit(st *trustState, kind string, id trustIdentity, g any, object, state string, now time.Time) {
	raw, _ := json.Marshal(g)
	digest := hash(raw)
	if st.Originals == nil {
		st.Originals = map[string][]byte{}
	}
	st.Originals[digest] = raw
	appendTrustEvidence(st, kind, id, object, digest, digest, state, "canal secret vers helper fixe ; aucune matière dans la preuve", now)
}
func (b *trustSecretBroker) withState(write bool, fn func(*trustState, TrustSecretPolicy, time.Time) (any, error)) (any, error) {
	return b.s.withTrustPublicationLockUsing(write, nil, true, func(st *trustState) (any, error) {
		now := time.Now().UTC()
		p := trustSecretPolicyValue(st.SecretBroker)
		if err := validateTrustSecretPolicy(&p); err != nil {
			return nil, err
		}
		b.purge(st.Revision, now)
		return fn(st, p, now)
	})
}
func (b *trustSecretBroker) prepare(ctx context.Context, token string) (any, error) {
	var revision int
	_, err := b.s.withTrustReadAvailable(func(st *trustState) (any, error) {
		id, e := findTrustIdentityAt(st, token, time.Now().UTC())
		if e != nil || !trustSecretAdmin(id) || !bool(trustSecretPolicyValue(st.SecretBroker).Enabled) {
			return nil, trustSecretDenied()
		}
		if _, err := trustSecretDeliveryContext(st); err != nil {
			return nil, err
		}
		revision = st.Revision
		return nil, nil
	})
	if err != nil {
		return nil, err
	}
	if _, err = b.s.trustSealedCheck(ctx, []byte("{}")); err != nil {
		return nil, err
	}
	if err = trustSecretHarden(); err != nil {
		return nil, err
	}
	if err = b.lock(); err != nil {
		return nil, err
	}
	defer b.mu.Unlock()
	return b.withState(false, func(st *trustState, p TrustSecretPolicy, now time.Time) (any, error) {
		id, e := findTrustIdentityAt(st, token, now)
		if ctx.Err() != nil || e != nil || !trustSecretAdmin(id) || !bool(p.Enabled) || st.Revision != revision {
			return nil, trustSecretDenied()
		}
		if _, err := trustSecretDeliveryContext(st); err != nil {
			return nil, err
		}
		b.readyRevision = revision
		return map[string]any{"schema_version": 1, "ready": true, "revision": revision, "server_memory_nondumpable": true, "max_material_bytes": p.MaxMaterialBytes}, nil
	})
}
func (b *trustSecretBroker) load(ctx context.Context, token string, raw []byte) (any, error) {
	defer clear(raw)
	if err := b.lock(); err != nil {
		return nil, err
	}
	defer b.mu.Unlock()
	var ref string
	var material *trustSecretMaterial
	result, err := b.withState(true, func(st *trustState, p TrustSecretPolicy, now time.Time) (any, error) {
		id, e := findTrustIdentityAt(st, token, now)
		if ctx.Err() != nil || e != nil || !trustSecretAdmin(id) || !bool(p.Enabled) || b.readyRevision != st.Revision || len(raw) == 0 || len(raw) > p.MaxMaterialBytes || len(b.entries) >= p.MaxEntries {
			return nil, trustSecretDenied()
		}
		if _, err := trustSecretDeliveryContext(st); err != nil {
			return nil, err
		}
		ref = newID("secret-")
		material = &trustSecretMaterial{value: append([]byte(nil), raw...), revision: st.Revision, expires: now.Add(time.Duration(p.MaterialTTLSeconds) * time.Second)}
		meta := map[string]any{"schema_version": 1, "secret_ref": ref, "policy_revision": st.Revision, "expires_at": material.expires}
		trustSecretAudit(st, "secret-load", id, meta, ref, "loaded", now)
		return meta, nil
	})
	if err != nil {
		if material != nil {
			clear(material.value)
		}
		return nil, err
	}
	b.entries[ref] = material
	time.AfterFunc(time.Until(material.expires), func() {
		b.mu.Lock()
		defer b.mu.Unlock()
		if b.entries[ref] == material {
			b.remove(ref)
		}
	})
	return result, nil
}
func trustSecretAction(st *trustState, id trustIdentity, cmd trustSecretRequest, now time.Time) (TrustAction, error) {
	a, ok := st.Actions[cmd.ActionID]
	if !ok || !trustSecretEffect(id) || a.State != "approved" || a.ContentHash != cmd.Digest || cmd.Connector != "sealed-attestation" || cmd.Destination != "local-attestation" || trustActiveMandateReason(st, a.MandateID, now) != "" {
		return TrustAction{}, trustSecretDenied()
	}
	if _, err := trustSecretDeliveryContext(st); err != nil {
		return TrustAction{}, err
	}
	if err := trustDataActionAdmission(st, "approve", TrustCommand{ActionID: a.ID}); err != nil {
		return TrustAction{}, err
	}
	return a, nil
}
func (b *trustSecretBroker) operation(ctx context.Context, op string, cmd trustSecretRequest, token string) (any, error) {
	if op == "secret-consume" {
		return b.consume(ctx, cmd, token, runTrustSecretConsumer)
	}
	if err := b.lock(); err != nil {
		return nil, err
	}
	defer b.mu.Unlock()
	var reserveRef, reserveID, removeRef string
	result, err := b.withState(op != "secret-status", func(st *trustState, p TrustSecretPolicy, now time.Time) (any, error) {
		id, e := findTrustIdentityAt(st, token, now)
		if ctx.Err() != nil || e != nil {
			return nil, trustSecretDenied()
		}
		if op == "secret-status" {
			if cmd != (trustSecretRequest{}) || (!trustSecretAdmin(id) && !trustSecretEffect(id)) {
				return nil, trustSecretDenied()
			}
			grants := []TrustSecretGrant{}
			available := map[string]bool{}
			for _, g := range st.SecretGrants {
				if trustSecretAdmin(id) || g.Principal == id.Subject {
					grants = append(grants, g)
					m := b.entries[g.Ref]
					available[g.ID] = g.State == "issued" && g.Revision == st.Revision && now.Before(g.ExpiresAt) && m != nil && m.grant == g.ID
				}
			}
			bytes := 0
			for _, m := range b.entries {
				bytes += len(m.value)
			}
			return map[string]any{"schema_version": 1, "scope": "sealed_attestation_only", "grants": grants, "grant_material_available": available, "loaded_entries": len(b.entries), "loaded_bytes": bytes, "policy": p, "provider_containment": false}, nil
		}
		if !bool(p.Enabled) || !bool(trustSealedPolicyValue(st.SealedExecution).Enabled) {
			return nil, trustSecretDenied()
		}
		switch op {
		case "secret-grant":
			if cmd.GrantID != "" {
				return nil, trustSecretDenied()
			}
			a, e := trustSecretAction(st, id, cmd, now)
			if e != nil {
				return nil, e
			}
			m := b.entries[cmd.Ref]
			if m == nil || m.grant != "" {
				return nil, trustSecretDenied()
			}
			mandateExpiry, e := time.Parse(time.RFC3339, st.Mandates[a.MandateID].ExpiresAt)
			if e != nil {
				return nil, trustSecretDenied()
			}
			expiry := now.Add(time.Duration(p.GrantTTLSeconds) * time.Second)
			for _, bound := range []time.Time{m.expires, mandateExpiry} {
				if bound.Before(expiry) {
					expiry = bound
				}
			}
			if id.Context.ExpiresAt != nil && id.Context.ExpiresAt.Before(expiry) {
				expiry = *id.Context.ExpiresAt
			}
			if !now.Before(expiry) {
				return nil, trustSecretDenied()
			}
			delivery, _ := trustSecretDeliveryContext(st)
			g := TrustSecretGrant{DataContext: delivery, ID: newID("grant-"), Revision: st.Revision, ActionID: a.ID, Digest: a.ContentHash, Principal: id.Subject, TokenDigest: trustTokenHash(token), Connector: cmd.Connector, Ref: cmd.Ref, Destination: cmd.Destination, IssuedAt: now, ExpiresAt: expiry, State: "issued"}
			if st.SecretGrants == nil {
				st.SecretGrants = map[string]TrustSecretGrant{}
			}
			st.SecretGrants[g.ID] = g
			trustSecretAudit(st, op, id, g, g.ID, g.State, now)
			reserveRef, reserveID = g.Ref, g.ID
			return g, nil
		case "secret-revoke":
			if !trustSecretAdmin(id) || cmd.ActionID != "" || cmd.Digest != "" || cmd.Connector != "" || cmd.Destination != "" || (cmd.Ref == "") == (cmd.GrantID == "") {
				return nil, trustSecretDenied()
			}
			if cmd.Ref != "" {
				if b.entries[cmd.Ref] == nil {
					return nil, trustSecretDenied()
				}
				removeRef = cmd.Ref
				for key, g := range st.SecretGrants {
					if g.Ref == cmd.Ref {
						g.State = "revoked"
						st.SecretGrants[key] = g
					}
				}
			} else {
				g, ok := st.SecretGrants[cmd.GrantID]
				if !ok {
					return nil, trustSecretDenied()
				}
				g.State = "revoked"
				st.SecretGrants[g.ID] = g
				removeRef = g.Ref
			}
			meta := map[string]any{"schema_version": 1, "grant_id": cmd.GrantID, "secret_ref": removeRef, "state": "revoked"}
			trustSecretAudit(st, op, id, meta, cmd.GrantID+removeRef, "revoked", now)
			return meta, nil
		default:
			return nil, fmt.Errorf("opération de courtier inconnue")
		}
	})
	if err == nil {
		if reserveRef != "" {
			b.entries[reserveRef].grant = reserveID
		}
		if removeRef != "" {
			b.remove(removeRef)
		}
	}
	return result, err
}
func (b *trustSecretBroker) consume(ctx context.Context, cmd trustSecretRequest, token string, runner func(context.Context, TrustSealedPolicy, []byte, TrustSecretGrant) (string, error)) (any, error) {
	if cmd.GrantID == "" || cmd != (trustSecretRequest{GrantID: cmd.GrantID}) {
		return nil, trustSecretDenied()
	}
	if err := b.lock(); err != nil {
		return nil, err
	}
	var grant TrustSecretGrant
	var actor trustIdentity
	var value []byte
	var sealed TrustSealedPolicy
	_, err := b.withState(true, func(st *trustState, p TrustSecretPolicy, now time.Time) (any, error) {
		id, e := findTrustIdentityAt(st, token, now)
		g, ok := st.SecretGrants[cmd.GrantID]
		if ctx.Err() != nil || e != nil || !ok || !bool(p.Enabled) || g.State != "issued" || !trustSecretDeliveryMatches(st, g) || g.Revision != st.Revision || g.Principal != id.Subject || g.TokenDigest != trustTokenHash(token) || !now.Before(g.ExpiresAt) {
			return nil, trustSecretDenied()
		}
		check := trustSecretRequest{ActionID: g.ActionID, Digest: g.Digest, Connector: g.Connector, Destination: g.Destination}
		if _, e = trustSecretAction(st, id, check, now); e != nil {
			return nil, e
		}
		m := b.entries[g.Ref]
		if m == nil || m.grant != g.ID {
			return nil, trustSecretDenied()
		}
		sealed = trustSealedPolicyValue(st.SealedExecution)
		if !bool(sealed.Enabled) {
			return nil, trustSecretDenied()
		}
		g.State = "consuming"
		g.ConsumedAt = &now
		st.SecretGrants[g.ID] = g
		grant = g
		actor = id
		trustSecretAudit(st, "secret-consume", id, g, g.ID, g.State, now)
		return g, nil
	})
	if err == nil {
		m := b.entries[grant.Ref]
		value = m.value
		m.value = nil
		delete(b.entries, grant.Ref)
	}
	b.mu.Unlock()
	if err != nil {
		return nil, err
	}
	defer clear(value)
	attestation, runErr := runner(ctx, sealed, value, grant)
	if err = b.lock(); err != nil {
		return nil, err
	}
	defer b.mu.Unlock()
	var rejected error
	result, err := b.withState(true, func(st *trustState, p TrustSecretPolicy, now time.Time) (any, error) {
		id, e := findTrustIdentityAt(st, token, now)
		g, ok := st.SecretGrants[grant.ID]
		if !ok || g.State != "consuming" {
			return nil, trustSecretDenied()
		}
		check := trustSecretRequest{ActionID: g.ActionID, Digest: g.Digest, Connector: g.Connector, Destination: g.Destination}
		_, actionErr := trustSecretAction(st, id, check, now)
		if runErr != nil || ctx.Err() != nil || e != nil || actionErr != nil || !trustSecretDeliveryMatches(st, g) || g.Revision != st.Revision || !bool(p.Enabled) || !now.Before(g.ExpiresAt) {
			g.State = "failed"
			rejected = trustSecretDenied()
			if runErr != nil {
				rejected = runErr
			}
		} else {
			g.State = "consumed"
			g.Attestation = attestation
		}
		st.SecretGrants[g.ID] = g
		trustSecretAudit(st, "secret-consume-result", actor, g, g.ID, g.State, now)
		return g, nil
	})
	if err != nil {
		return nil, err
	}
	if rejected != nil {
		return nil, rejected
	}
	return result, nil
}

func trustSecretHarden() error {
	if unix.Prctl(unix.PR_SET_DUMPABLE, 0, 0, 0, 0) != nil {
		return trustSealedFailure()
	}
	dump, e := unix.PrctlRetInt(unix.PR_GET_DUMPABLE, 0, 0, 0, 0)
	if e != nil || dump != 0 {
		return trustSealedFailure()
	}
	return nil
}
