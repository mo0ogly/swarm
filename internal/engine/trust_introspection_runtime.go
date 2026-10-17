//go:build linux

package engine

import (
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"encoding/json"
	jose "github.com/go-jose/go-jose/v4"
	"time"
)

func (b *trustServiceRuntime) purge(revision int, now time.Time) {
	if revision < b.revision {
		return
	}
	for k, m := range b.tokens {
		if m.revision != revision || !now.Before(m.expiresAt) {
			m.revoked.Store(true)
			clear(m.value)
			delete(b.tokens, k)
		}
	}
	if b.revision != revision {
		b.key = nil
		b.revision = revision
	}
}
func (s *Store) trustIntrospectionKey(token string) (any, error) {
	var revision int
	var p TrustIntrospectionPolicy
	_, err := s.withTrustReadAvailable(func(st *trustState) (any, error) {
		id, e := findTrustIdentityAt(st, token, time.Now().UTC())
		if e != nil || !trustSecretAdmin(id) {
			return nil, trustIntrospectionDenied()
		}
		revision = st.Revision
		p = trustIntrospectionPolicyValue(st.WorkloadIntrospection)
		if p.Endpoint == "" || p.ClientID == "" {
			return nil, trustIntrospectionDenied()
		}
		return nil, nil
	})
	if err != nil {
		return nil, err
	}
	if s.serviceRuntime == nil {
		return nil, trustIntrospectionDenied()
	}
	if err = trustSecretHarden(); err != nil {
		return nil, err
	}
	b := s.serviceRuntime
	b.mu.Lock()
	defer b.mu.Unlock()
	b.purge(revision, time.Now().UTC())
	if b.revision != revision {
		return nil, trustIntrospectionDenied()
	}
	if b.key == nil {
		b.key, err = ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
		if err != nil {
			return nil, trustIntrospectionDenied()
		}
	}
	key := jose.JSONWebKey{Key: &b.key.PublicKey, Algorithm: "ES256", Use: "sig"}
	raw, _ := json.Marshal(key)
	key.KeyID = hash(raw)
	return map[string]any{"schema_version": 1, "revision": revision, "client_id": p.ClientID, "endpoint": p.Endpoint, "public_key": key, "ephemeral": true, "server_memory_nondumpable": true}, nil
}
func (s *Store) trustServiceForget(digest string) {
	if s.serviceRuntime == nil {
		return
	}
	b := s.serviceRuntime
	b.mu.Lock()
	defer b.mu.Unlock()
	if m := b.tokens[digest]; m != nil {
		m.revoked.Store(true)
		clear(m.value)
		delete(b.tokens, digest)
	}
}
func (s *Store) trustServiceKeep(st *trustState, digest string, raw []byte, expires time.Time) error {
	p := trustIntrospectionPolicyValue(st.WorkloadIntrospection)
	if !bool(p.Enabled) {
		return nil
	}
	if s.serviceRuntime == nil || len(raw) == 0 || len(raw) > p.MaxTokenBytes {
		return trustIntrospectionDenied()
	}
	b := s.serviceRuntime
	b.mu.Lock()
	defer b.mu.Unlock()
	b.purge(st.Revision, time.Now().UTC())
	if b.revision != st.Revision || b.key == nil || len(b.tokens) >= p.MaxSessions {
		return trustIntrospectionDenied()
	}
	b.tokens[digest] = &trustServiceMaterial{value: append([]byte{}, raw...), revision: st.Revision, expiresAt: expires}
	time.AfterFunc(time.Until(expires), func() {
		b.mu.Lock()
		defer b.mu.Unlock()
		if m := b.tokens[digest]; m != nil && !time.Now().Before(m.expiresAt) {
			m.revoked.Store(true)
			clear(m.value)
			delete(b.tokens, digest)
		}
	})
	return nil
}
func (s *Store) trustServiceScope(ctx context.Context, token string) (*Store, error) {
	var stCopy trustState
	_, err := s.withTrustLock(false, func(st *trustState) (any, error) { stCopy = *st; return nil, nil })
	if err != nil {
		return nil, err
	}
	p := trustIntrospectionPolicyValue(stCopy.WorkloadIntrospection)
	if !bool(p.Enabled) {
		return s, nil
	}
	id, err := resolveTrustIdentityUnchecked(&stCopy, token, time.Now().UTC())
	if err != nil {
		return nil, err
	}
	if id.Context.PrincipalKind != "service" {
		return s, nil
	}
	proof, err := s.trustServiceIntrospect(ctx, &stCopy, token)
	if err != nil {
		return nil, err
	}
	scoped := *s
	scoped.serviceProof = proof
	return &scoped, nil
}

func (s *Store) trustServiceConsumption(revision int) any {
	count, size := 0, 0
	prepared := false
	if b := s.serviceRuntime; b != nil {
		b.mu.Lock()
		defer b.mu.Unlock()
		b.purge(revision, time.Now().UTC())
		prepared = b.revision == revision && b.key != nil
		for _, m := range b.tokens {
			count++
			size += len(m.value)
		}
	}
	return map[string]any{"prepared": prepared, "retained_sessions": count, "retained_token_bytes": size, "persistent_tokens": false}
}
