//go:build linux

package engine

import (
	"context"
	"crypto/ecdsa"
	"crypto/ed25519"
	"crypto/elliptic"
	"crypto/rsa"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/coreos/go-oidc/v3/oidc"
	jose "github.com/go-jose/go-jose/v4"
	"github.com/go-jose/go-jose/v4/jwt"
	"golang.org/x/oauth2"
)

type trustWorkloadEnabled bool

func (enabled *trustWorkloadEnabled) UnmarshalJSON(raw []byte) error {
	switch string(raw) {
	case "true":
		*enabled = true
	case "false":
		*enabled = false
	default:
		return fmt.Errorf("Services : enabled doit être un booléen true ou false")
	}
	return nil
}

type TrustWorkloadClient struct {
	ClientID        string            `json:"client_id"`
	ExternalSubject string            `json:"external_subject"`
	Subject         string            `json:"subject"`
	PublicKeys      []jose.JSONWebKey `json:"public_keys"`
}

type TrustWorkloadConfig struct {
	Enabled               trustWorkloadEnabled  `json:"enabled"`
	Issuer                string                `json:"issuer"`
	Audience              string                `json:"audience"`
	RequestTimeoutSeconds int                   `json:"request_timeout_seconds"`
	AssertionTTLSeconds   int                   `json:"assertion_ttl_seconds"`
	SessionTTLSeconds     int                   `json:"session_ttl_seconds"`
	Clients               []TrustWorkloadClient `json:"clients"`
}

type trustWorkloadCommand struct {
	ClientID        string `json:"client_id,omitempty"`
	ClientAssertion string `json:"client_assertion,omitempty"`
	SessionToken    string `json:"session_token,omitempty"`
}

type trustWorkloadClaims struct {
	jwt.Claims
	ClientID string `json:"client_id"`
}

func defaultTrustWorkloadConfig() TrustWorkloadConfig {
	return TrustWorkloadConfig{RequestTimeoutSeconds: 15, AssertionTTLSeconds: 300, SessionTTLSeconds: 900, Clients: []TrustWorkloadClient{}}
}

func trustWorkloadPublicKeyValid(key jose.JSONWebKey) bool {
	if !key.IsPublic() || strings.TrimSpace(key.KeyID) == "" || (key.Use != "" && key.Use != "sig") {
		return false
	}
	switch public := key.Key.(type) {
	case *rsa.PublicKey:
		return public != nil && key.Valid() && key.Algorithm == "RS256" && public.N.BitLen() >= 2048 && public.E >= 3 && public.E%2 == 1
	case *ecdsa.PublicKey:
		return public != nil && key.Valid() && key.Algorithm == "ES256" && public.Curve == elliptic.P256() && public.Curve.IsOnCurve(public.X, public.Y)
	case ed25519.PublicKey:
		return key.Valid() && key.Algorithm == "EdDSA"
	}
	return false
}

func validateTrustWorkloads(c *TrustWorkloadConfig, ids []trustIdentity) error {
	if c == nil {
		return nil
	}
	for _, client := range c.Clients {
		kids := map[string]bool{}
		for _, key := range client.PublicKeys {
			if !trustWorkloadPublicKeyValid(key) || kids[key.KeyID] {
				return fmt.Errorf("Services : clé publique identifiée RS256, ES256 ou EdDSA requise ; clé privée refusée")
			}
			kids[key.KeyID] = true
		}
	}
	if !bool(c.Enabled) {
		return nil
	}
	if !trustOIDCURL(c.Issuer) || strings.TrimSpace(c.Audience) == "" || strings.ContainsAny(c.Audience, " \t\r\n") {
		return fmt.Errorf("Services : issuer sécurisé et audience explicite requis")
	}
	for _, seconds := range []int{c.RequestTimeoutSeconds, c.AssertionTTLSeconds, c.SessionTTLSeconds} {
		if seconds < 1 || int64(seconds) > int64((1<<63-1)/int64(time.Second)) {
			return fmt.Errorf("Services : durées en secondes strictement positives et représentables requises ; 0 refusé")
		}
	}
	if len(c.Clients) == 0 {
		return fmt.Errorf("Services : au moins un client explicitement associé est requis")
	}
	subjects, clients := map[string]bool{}, map[string]bool{}
	for _, id := range ids {
		subjects[id.Subject] = true
	}
	for _, client := range c.Clients {
		if strings.TrimSpace(client.ClientID) == "" || strings.ContainsAny(client.ClientID, " \t\r\n") || strings.TrimSpace(client.ExternalSubject) == "" || clients[client.ClientID] || !subjects[client.Subject] || len(client.PublicKeys) == 0 {
			return fmt.Errorf("Services : client, sujet ou clés publiques invalides ou dupliqués")
		}
		clients[client.ClientID] = true
	}
	return nil
}

func trustWorkloadDenied() error {
	return &CommandError{Code: "authorization_required", Message: "connexion de service refusée ; vérifiez la configuration et fournissez une nouvelle assertion"}
}

func trustWorkloadSigned(raw string) (*jose.JSONWebSignature, error) {
	if strings.Count(raw, ".") != 2 {
		return nil, trustWorkloadDenied()
	}
	signed, err := jose.ParseSigned(raw, []jose.SignatureAlgorithm{jose.RS256, jose.ES256, jose.EdDSA})
	if err != nil || len(signed.Signatures) != 1 || signed.Signatures[0].Header.KeyID == "" {
		return nil, trustWorkloadDenied()
	}
	return signed, nil
}

func trustWorkloadValidClaims(c trustWorkloadClaims, issuer, subject, audience string, now time.Time) bool {
	if c.IssuedAt == nil || c.Expiry == nil || strings.TrimSpace(c.ID) == "" || !c.Expiry.Time().After(c.IssuedAt.Time()) || !now.Before(c.Expiry.Time()) {
		return false
	}
	expected := jwt.Expected{Issuer: issuer, Subject: subject, Time: now}
	if audience != "" {
		expected.AnyAudience = jwt.Audience{audience}
	}
	return c.Claims.ValidateWithLeeway(expected, 0) == nil
}

func trustWorkloadAssertion(raw string, client TrustWorkloadClient, audience string, maxSeconds int, now time.Time) (trustWorkloadClaims, error) {
	var claims trustWorkloadClaims
	signed, err := trustWorkloadSigned(raw)
	if err != nil {
		return claims, err
	}
	header := signed.Signatures[0].Header
	for _, key := range client.PublicKeys {
		if key.KeyID != header.KeyID || key.Algorithm != header.Algorithm {
			continue
		}
		payload, err := signed.Verify(key.Key)
		if err != nil || json.Unmarshal(payload, &claims) != nil {
			return claims, trustWorkloadDenied()
		}
		if !trustWorkloadValidClaims(claims, client.ClientID, client.ClientID, audience, now) || claims.Expiry.Time().Sub(claims.IssuedAt.Time()) > time.Duration(maxSeconds)*time.Second {
			return claims, trustWorkloadDenied()
		}
		return claims, nil
	}
	return claims, trustWorkloadDenied()
}

func trustWorkloadProvider(ctx context.Context, c TrustWorkloadConfig) (context.Context, *oidc.Provider, string, error) {
	ctx = trustIdentityClientContext(ctx, c.RequestTimeoutSeconds)
	p, err := oidc.NewProvider(ctx, c.Issuer)
	if err != nil {
		return ctx, nil, "", trustWorkloadDenied()
	}
	var metadata struct {
		JWKSURI string   `json:"jwks_uri"`
		Methods []string `json:"token_endpoint_auth_methods_supported"`
	}
	if p.Claims(&metadata) != nil || !trustOIDCURL(metadata.JWKSURI) || !trustOIDCURL(p.Endpoint().TokenURL) {
		return ctx, nil, "", trustWorkloadDenied()
	}
	found := false
	for _, method := range metadata.Methods {
		if method == "private_key_jwt" {
			found = true
		}
	}
	if !found {
		return ctx, nil, "", trustWorkloadDenied()
	}
	return ctx, p, metadata.JWKSURI, nil
}

func (s *Store) trustWorkloadLogin(ctx context.Context, cmd trustWorkloadCommand) (any, error) {
	if cmd.ClientID == "" || cmd.ClientAssertion == "" || cmd.SessionToken != "" {
		return nil, trustWorkloadDenied()
	}
	var c TrustWorkloadConfig
	var client TrustWorkloadClient
	var revision int
	var introspection TrustIntrospectionPolicy
	_, err := s.withTrustLock(false, func(st *trustState) (any, error) {
		if st.Workloads == nil || !bool(st.Workloads.Enabled) {
			return nil, trustWorkloadDenied()
		}
		introspection = trustIntrospectionPolicyValue(st.WorkloadIntrospection)
		c = *st.Workloads
		revision = st.Revision
		for _, candidate := range c.Clients {
			if candidate.ClientID == cmd.ClientID {
				client = candidate
			}
		}
		if client.ClientID == "" {
			return nil, trustWorkloadDenied()
		}
		for _, id := range st.Identities {
			if id.Subject == client.Subject && !id.Disabled {
				return nil, validateTrustWorkloads(&c, st.Identities)
			}
		}
		return nil, trustWorkloadDenied()
	})
	if err != nil {
		return nil, err
	}
	if bool(introspection.Enabled) {
		if s.serviceRuntime == nil {
			return nil, trustIntrospectionDenied()
		}
		b := s.serviceRuntime
		b.mu.Lock()
		b.purge(revision, time.Now().UTC())
		ready := b.revision == revision && b.key != nil
		b.mu.Unlock()
		if !ready {
			return nil, trustIntrospectionDenied()
		}
		if err := trustSecretHarden(); err != nil {
			return nil, err
		}
	}
	ctx, cancel := context.WithTimeout(ctx, time.Duration(c.RequestTimeoutSeconds)*time.Second)
	defer cancel()
	assertion, err := trustWorkloadAssertion(cmd.ClientAssertion, client, "", c.AssertionTTLSeconds, time.Now().UTC())
	if err != nil {
		return nil, err
	}
	ctx, p, jwks, err := trustWorkloadProvider(ctx, c)
	if err != nil {
		return nil, err
	}
	if !assertion.Audience.Contains(p.Endpoint().TokenURL) {
		return nil, trustWorkloadDenied()
	}
	usedID := hashMust(struct{ Issuer, ClientID, JTI string }{c.Issuer, client.ClientID, assertion.ID})
	_, err = s.withTrustLock(true, func(st *trustState) (any, error) {
		if st.Revision != revision || !time.Now().Before(assertion.Expiry.Time()) {
			return nil, trustWorkloadDenied()
		}
		if st.WorkloadUsed == nil {
			st.WorkloadUsed = map[string]time.Time{}
		}
		for key, expires := range st.WorkloadUsed {
			if !time.Now().Before(expires) {
				delete(st.WorkloadUsed, key)
			}
		}
		if _, used := st.WorkloadUsed[usedID]; used {
			return nil, trustWorkloadDenied()
		}
		st.WorkloadUsed[usedID] = assertion.Expiry.Time()
		return nil, nil
	})
	if err != nil {
		return nil, err
	}
	form := url.Values{"grant_type": {"client_credentials"}, "client_id": {client.ClientID}, "client_assertion_type": {"urn:ietf:params:oauth:client-assertion-type:jwt-bearer"}, "client_assertion": {cmd.ClientAssertion}, "resource": {c.Audience}}
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, p.Endpoint().TokenURL, strings.NewReader(form.Encode()))
	if err != nil {
		return nil, trustWorkloadDenied()
	}
	request.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	request.Header.Set("Accept", "application/json")
	response, err := oauth2.NewClient(ctx, nil).Do(request)
	if err != nil {
		return nil, trustWorkloadDenied()
	}
	defer response.Body.Close()
	body, err := io.ReadAll(response.Body)
	if err != nil || response.StatusCode != http.StatusOK {
		return nil, trustWorkloadDenied()
	}
	var token struct {
		AccessToken string `json:"access_token"`
		TokenType   string `json:"token_type"`
	}
	if json.Unmarshal(body, &token) != nil || !strings.EqualFold(token.TokenType, "Bearer") {
		return nil, trustWorkloadDenied()
	}
	signed, err := trustWorkloadSigned(token.AccessToken)
	if err != nil {
		return nil, err
	}
	typ, _ := signed.Signatures[0].Header.ExtraHeaders[jose.HeaderType].(string)
	if !strings.EqualFold(typ, "at+jwt") && !strings.EqualFold(typ, "application/at+jwt") {
		return nil, trustWorkloadDenied()
	}
	payload, err := oidc.NewRemoteKeySet(ctx, jwks).VerifySignature(ctx, token.AccessToken)
	if err != nil {
		return nil, trustWorkloadDenied()
	}
	var claims trustWorkloadClaims
	now := time.Now().UTC()
	if json.Unmarshal(payload, &claims) != nil || !trustWorkloadValidClaims(claims, c.Issuer, client.ExternalSubject, c.Audience, now) || claims.ClientID != client.ClientID {
		return nil, trustWorkloadDenied()
	}
	session, err := trustOIDCRandom()
	if err != nil {
		return nil, err
	}
	expires := now.Add(time.Duration(c.SessionTTLSeconds) * time.Second)
	if claims.Expiry.Time().Before(expires) {
		expires = claims.Expiry.Time()
	}
	result, loginErr := s.withTrustLock(true, func(st *trustState) (any, error) {
		if st.Revision != revision || ctx.Err() != nil || !time.Now().Before(expires) {
			return nil, trustWorkloadDenied()
		}
		var identity trustIdentity
		for _, id := range st.Identities {
			if id.Subject == client.Subject {
				identity = id
			}
		}
		if identity.Subject == "" || identity.Disabled {
			return nil, trustWorkloadDenied()
		}
		var signedClaims map[string]json.RawMessage
		if json.Unmarshal(payload, &signedClaims) != nil {
			return nil, trustWorkloadDenied()
		}
		identity, mappingDigest, err := mapTrustIdentityGroups(effectiveTrustGroupMapping(st), signedClaims, identity)
		if err != nil {
			return nil, err
		}
		if st.WorkloadSessions == nil {
			st.WorkloadSessions = map[string]trustOIDCSession{}
		}
		for key, value := range st.WorkloadSessions {
			if !now.Before(value.ExpiresAt) || value.Revision != revision {
				delete(st.WorkloadSessions, key)
			}
		}
		if err := s.trustServiceKeep(st, trustTokenHash(session), []byte(token.AccessToken), expires); err != nil {
			return nil, err
		}
		st.WorkloadSessions[trustTokenHash(session)] = trustOIDCSession{ClientID: client.ClientID, AccessTokenDigest: hash([]byte(token.AccessToken)), AccessTokenExpiresAt: claims.Expiry.Time(), MappedRoles: groupSessionRoles(identity, mappingDigest), GroupMappingDigest: mappingDigest, AuthenticatedAt: time.Now().UTC(), Issuer: claims.Issuer, ExternalSubject: claims.Subject, Subject: identity.Subject, Revision: revision, ExpiresAt: expires}
		return map[string]any{"subject": identity.Subject, "client_id": client.ClientID, "revision": revision, "session_token": session, "expires_at": expires, "roles": identity.Roles, "integrity": identity.Integrity}, nil
	})
	if loginErr != nil {
		s.trustServiceForget(trustTokenHash(session))
	}
	return result, loginErr
}

func (s *Store) trustWorkloadOperation(ctx context.Context, action string, raw []byte) (any, error) {
	var cmd trustWorkloadCommand
	if err := strict(raw, &cmd); err != nil {
		return nil, err
	}
	if action == "workload-logout" {
		if !trustCredentialValid(cmd.SessionToken) || cmd.ClientID != "" || cmd.ClientAssertion != "" {
			return nil, trustWorkloadDenied()
		}
		return s.withTrustLock(true, func(st *trustState) (any, error) {
			s.trustServiceForget(trustTokenHash(cmd.SessionToken))
			delete(st.WorkloadSessions, trustTokenHash(cmd.SessionToken))
			return map[string]any{"logged_out": true}, nil
		})
	}
	return s.trustWorkloadLogin(ctx, cmd)
}
