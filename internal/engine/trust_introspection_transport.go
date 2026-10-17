//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	jose "github.com/go-jose/go-jose/v4"
	"github.com/go-jose/go-jose/v4/jwt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

func (s *Store) trustServiceIntrospect(ctx context.Context, st *trustState, token string) (*trustServiceProof, error) {
	requestContext := ctx
	p := trustIntrospectionPolicyValue(st.WorkloadIntrospection)
	digest := trustTokenHash(token)
	session, ok := st.WorkloadSessions[digest]
	if !ok || s.serviceRuntime == nil || session.ClientID == "" || session.AccessTokenDigest == "" {
		return nil, trustIntrospectionDenied()
	}
	b := s.serviceRuntime
	b.mu.Lock()
	b.purge(st.Revision, time.Now().UTC())
	m := b.tokens[digest]
	if m == nil || b.key == nil || b.revision != st.Revision {
		b.mu.Unlock()
		return nil, trustIntrospectionDenied()
	}
	raw := append([]byte{}, m.value...)
	key := b.key
	b.mu.Unlock()
	defer clear(raw)
	if hash(raw) != session.AccessTokenDigest {
		return nil, trustIntrospectionDenied()
	}
	public := jose.JSONWebKey{Key: &key.PublicKey, Algorithm: "ES256", Use: "sig"}
	pubRaw, _ := json.Marshal(public)
	signer, err := jose.NewSigner(jose.SigningKey{Algorithm: jose.ES256, Key: key}, (&jose.SignerOptions{}).WithType("JWT").WithHeader("kid", hash(pubRaw)))
	if err != nil {
		return nil, trustIntrospectionDenied()
	}
	now := time.Now().UTC()
	deadline := now.Add(time.Duration(p.TimeoutSeconds) * time.Second)
	claims := jwt.Claims{Issuer: p.ClientID, Subject: p.ClientID, Audience: jwt.Audience{p.Endpoint}, IssuedAt: jwt.NewNumericDate(now), Expiry: jwt.NewNumericDate(deadline), ID: newID("introspection-")}
	assertion, err := jwt.Signed(signer).Claims(claims).Serialize()
	if err != nil {
		return nil, trustIntrospectionDenied()
	}
	ctx, cancel := context.WithDeadline(ctx, deadline)
	defer cancel()
	form := url.Values{"token": {string(raw)}, "token_type_hint": {"access_token"}, "client_id": {p.ClientID}, "client_assertion_type": {"urn:ietf:params:oauth:client-assertion-type:jwt-bearer"}, "client_assertion": {assertion}}
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, p.Endpoint, strings.NewReader(form.Encode()))
	if err != nil {
		return nil, trustIntrospectionDenied()
	}
	request.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	request.Header.Set("Accept", "application/json")
	transport := &http.Transport{DisableKeepAlives: true, DisableCompression: true}
	defer transport.CloseIdleConnections()
	client := &http.Client{Transport: transport, CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }}
	response, err := client.Do(request)
	if err != nil {
		return nil, trustIntrospectionDenied()
	}
	defer response.Body.Close()
	body, err := io.ReadAll(io.LimitReader(response.Body, (1<<20)+1))
	defer clear(body)
	if err != nil || len(body) > 1<<20 || response.StatusCode != http.StatusOK || ctx.Err() != nil {
		return nil, trustIntrospectionDenied()
	}
	checked := time.Now().UTC()
	var status struct {
		Active *bool `json:"active"`
	}
	if json.Unmarshal(body, &status) == nil && status.Active != nil && !*status.Active {
		s.trustServiceForget(digest)
	}
	expiry, err := trustIntrospectionResponse(body, st.Workloads, session, checked)
	if err != nil {
		return nil, err
	}
	limit := checked.Add(time.Duration(p.ValidationWindowSeconds) * time.Second)
	if expiry.Before(limit) {
		limit = expiry
	}
	if session.ExpiresAt.Before(limit) {
		limit = session.ExpiresAt
	}
	return &trustServiceProof{material: m, requestContext: requestContext, tokenDigest: digest, revision: st.Revision, checkedAt: checked, expiresAt: limit}, nil
}

func trustIntrospectionResponse(raw []byte, c *TrustWorkloadConfig, session trustOIDCSession, now time.Time) (time.Time, error) {
	denied := func() (time.Time, error) { return time.Time{}, trustIntrospectionDenied() }
	d := json.NewDecoder(bytes.NewReader(raw))
	open, e := d.Token()
	if e != nil || open != json.Delim('{') {
		return denied()
	}
	fields := map[string]json.RawMessage{}
	for d.More() {
		k, e := d.Token()
		if e != nil {
			return denied()
		}
		key, ok := k.(string)
		if !ok {
			return denied()
		}
		if _, found := fields[key]; found {
			return denied()
		}
		var value json.RawMessage
		if d.Decode(&value) != nil {
			return denied()
		}
		fields[key] = value
	}
	if _, e = d.Token(); e != nil {
		return denied()
	}
	var extra any
	if d.Decode(&extra) != io.EOF {
		return denied()
	}
	var active bool
	if json.Unmarshal(fields["active"], &active) != nil || !active {
		return denied()
	}
	var claims jwt.Claims
	body, _ := json.Marshal(fields)
	if json.Unmarshal(body, &claims) != nil {
		return denied()
	}
	var clientID string
	if json.Unmarshal(fields["client_id"], &clientID) != nil || clientID != session.ClientID {
		return denied()
	}
	if claims.Issuer != session.Issuer || claims.Issuer != c.Issuer || claims.Subject != session.ExternalSubject || len(claims.Audience) != 1 || claims.Audience[0] != c.Audience || claims.Expiry == nil || !now.Before(claims.Expiry.Time()) || claims.Expiry.Time().After(session.AccessTokenExpiresAt) {
		return denied()
	}
	if claims.IssuedAt != nil && claims.IssuedAt.Time().After(now) {
		return denied()
	}
	if claims.NotBefore != nil && claims.NotBefore.Time().After(now) {
		return denied()
	}
	return claims.Expiry.Time(), nil
}
