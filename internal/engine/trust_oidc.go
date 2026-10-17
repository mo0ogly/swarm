//go:build linux

package engine

import (
	"context"
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/coreos/go-oidc/v3/oidc"
	jose "github.com/go-jose/go-jose/v4"
	"golang.org/x/oauth2"
)

type TrustOIDCMapping struct {
	ExternalSubject string `json:"external_subject"`
	Subject         string `json:"subject"`
}

type trustOIDCEnabled bool

func (enabled *trustOIDCEnabled) UnmarshalJSON(raw []byte) error {
	switch string(raw) {
	case "true":
		*enabled = true
	case "false":
		*enabled = false
	default:
		return fmt.Errorf("OIDC : enabled doit être un booléen true ou false")
	}
	return nil
}

type TrustOIDCConfig struct {
	Enabled               trustOIDCEnabled   `json:"enabled"`
	Issuer                string             `json:"issuer"`
	ClientID              string             `json:"client_id"`
	RedirectURI           string             `json:"redirect_uri"`
	RequestTimeoutSeconds int                `json:"request_timeout_seconds"`
	LoginTTLSeconds       int                `json:"login_ttl_seconds"`
	SessionTTLSeconds     int                `json:"session_ttl_seconds"`
	Mappings              []TrustOIDCMapping `json:"mappings"`
}

type trustOIDCPending struct {
	StartedAt   time.Time           `json:"started_at,omitempty"`
	StepUp      *trustStepUpBinding `json:"step_up,omitempty"`
	BindingHash string              `json:"binding_hash"`
	Nonce       string              `json:"nonce"`
	Verifier    string              `json:"verifier"`
	Revision    int                 `json:"revision"`
	ExpiresAt   time.Time           `json:"expires_at"`
}

type trustOIDCSession struct {
	ClientID             string            `json:"client_id,omitempty"`
	AccessTokenDigest    string            `json:"access_token_digest,omitempty"`
	AccessTokenExpiresAt time.Time         `json:"access_token_expires_at,omitempty"`
	MappedRoles          []string          `json:"mapped_roles,omitempty"`
	GroupMappingDigest   string            `json:"group_mapping_digest,omitempty"`
	AuthenticatedAt      time.Time         `json:"authenticated_at,omitempty"`
	ProviderAuthTime     time.Time         `json:"provider_authenticated_at,omitempty"`
	ACR                  string            `json:"acr,omitempty"`
	Issuer               string            `json:"issuer,omitempty"`
	ExternalSubject      string            `json:"external_subject,omitempty"`
	SID                  string            `json:"sid,omitempty"`
	StepUp               *trustStepUpProof `json:"step_up,omitempty"`
	Subject              string            `json:"subject"`
	Revision             int               `json:"revision"`
	ExpiresAt            time.Time         `json:"expires_at"`
}

type trustOIDCCommand struct {
	StepUpActionID string `json:"step_up_action_id,omitempty"`
	State          string `json:"state,omitempty"`
	Code           string `json:"code,omitempty"`
	Binding        string `json:"binding,omitempty"`
	SessionToken   string `json:"session_token,omitempty"`
}

type trustOIDCLogin struct {
	AuthorizationURL string `json:"authorization_url"`
	State            string `json:"state"`
	Binding          string `json:"binding"`
}

func trustOIDCURL(raw string) bool {
	u, err := url.Parse(raw)
	if err != nil || u.Host == "" || u.User != nil || u.Fragment != "" || u.RawQuery != "" {
		return false
	}
	if u.Scheme == "https" {
		return true
	}
	ip := net.ParseIP(u.Hostname())
	return u.Scheme == "http" && (u.Hostname() == "localhost" || ip != nil && ip.IsLoopback())
}

func defaultTrustOIDCConfig() TrustOIDCConfig {
	return TrustOIDCConfig{RequestTimeoutSeconds: 15, LoginTTLSeconds: 300, SessionTTLSeconds: 900, Mappings: []TrustOIDCMapping{}}
}

func validateTrustOIDC(c *TrustOIDCConfig, ids []trustIdentity) error {
	if c == nil || !bool(c.Enabled) {
		return nil
	}
	if !trustOIDCURL(c.Issuer) || !trustOIDCURL(c.RedirectURI) || strings.TrimSpace(c.ClientID) == "" || strings.ContainsAny(c.ClientID, "\r\n") {
		return fmt.Errorf("OIDC : issuer, client_id et redirect_uri HTTPS (ou boucle locale HTTP) requis")
	}
	for _, n := range []int{c.RequestTimeoutSeconds, c.LoginTTLSeconds, c.SessionTTLSeconds} {
		if n < 1 || int64(n) > int64((1<<63-1)/int64(time.Second)) {
			return fmt.Errorf("OIDC : durées en secondes strictement positives et représentables requises ; 0 refusé")
		}
	}
	if len(c.Mappings) == 0 {
		return fmt.Errorf("OIDC : mapping explicite requis")
	}
	subjects, external := map[string]bool{}, map[string]bool{}
	for _, id := range ids {
		subjects[id.Subject] = true
	}
	for _, m := range c.Mappings {
		if strings.TrimSpace(m.ExternalSubject) == "" || external[m.ExternalSubject] || !subjects[m.Subject] {
			return fmt.Errorf("OIDC : mapping inconnu, vide ou dupliqué")
		}
		external[m.ExternalSubject] = true
	}
	return nil
}

func trustOIDCRandom() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(b), nil
}

func trustOIDCDenied() error {
	return &CommandError{Code: "authorization_required", Message: "connexion OIDC refusée ; recommencez la connexion et vérifiez la configuration"}
}

type trustOIDCTransport struct{ base http.RoundTripper }

func (t trustOIDCTransport) RoundTrip(r *http.Request) (*http.Response, error) {
	u := *r.URL
	u.RawQuery = ""
	if !trustOIDCURL(u.String()) {
		return nil, fmt.Errorf("OIDC : transport non sécurisé refusé")
	}
	response, err := t.base.RoundTrip(r)
	if err != nil {
		return nil, err
	}
	if response.ContentLength > 1<<20 {
		response.Body.Close()
		return nil, fmt.Errorf("OIDC : réponse supérieure à 1 Mio")
	}
	response.Body = &trustOIDCBody{ReadCloser: response.Body, remaining: 1 << 20}
	return response, nil
}

type trustOIDCBody struct {
	io.ReadCloser
	remaining int64
}

func (b *trustOIDCBody) Read(p []byte) (int, error) {
	if b.remaining == 0 {
		var one [1]byte
		n, err := b.ReadCloser.Read(one[:])
		if n > 0 {
			return 0, fmt.Errorf("OIDC : réponse supérieure à 1 Mio")
		}
		return 0, err
	}
	if int64(len(p)) > b.remaining {
		p = p[:b.remaining]
	}
	n, err := b.ReadCloser.Read(p)
	b.remaining -= int64(n)
	return n, err
}

func trustIdentityClientContext(ctx context.Context, seconds int) context.Context {
	client := &http.Client{Timeout: time.Duration(seconds) * time.Second, Transport: trustOIDCTransport{http.DefaultTransport}, CheckRedirect: func(*http.Request, []*http.Request) error { return fmt.Errorf("OIDC : redirection HTTP refusée") }}
	return oidc.ClientContext(ctx, client)
}

func trustOIDCProvider(ctx context.Context, c TrustOIDCConfig) (context.Context, *oidc.Provider, error) {
	ctx = trustIdentityClientContext(ctx, c.RequestTimeoutSeconds)
	p, err := oidc.NewProvider(ctx, c.Issuer)
	if err != nil {
		return ctx, nil, trustOIDCDenied()
	}
	endpoints := p.Endpoint()
	if !trustOIDCURL(endpoints.AuthURL) || !trustOIDCURL(endpoints.TokenURL) {
		return ctx, nil, trustOIDCDenied()
	}
	return ctx, p, nil
}

func (s *Store) trustOIDCConfigSnapshot() (TrustOIDCConfig, int, error) {
	var c TrustOIDCConfig
	var revision int
	_, err := s.withTrustLock(false, func(st *trustState) (any, error) {
		if st.OIDC == nil || !bool(st.OIDC.Enabled) {
			return nil, trustOIDCDenied()
		}
		c = *st.OIDC
		c.Mappings = append([]TrustOIDCMapping{}, c.Mappings...)
		revision = st.Revision
		return nil, validateTrustOIDC(&c, st.Identities)
	})
	return c, revision, err
}

func (s *Store) trustOIDCStart(ctx context.Context) (any, error) {
	return s.trustOIDCStartFor(ctx, trustOIDCCommand{}, "")
}

func (s *Store) trustOIDCStartFor(ctx context.Context, cmd trustOIDCCommand, token string) (any, error) {
	c, revision, err := s.trustOIDCConfigSnapshot()
	if err != nil {
		return nil, err
	}
	var stepUp *trustStepUpBinding
	var stepPolicy TrustStepUpPolicy
	if cmd.StepUpActionID != "" {
		stepUp, stepPolicy, err = s.trustStepUpStartBinding(cmd.StepUpActionID, token, revision)
		if err != nil {
			return nil, err
		}
	}
	ctx, cancel := context.WithTimeout(ctx, time.Duration(c.RequestTimeoutSeconds)*time.Second)
	defer cancel()
	_, p, err := trustOIDCProvider(ctx, c)
	if err != nil {
		return nil, err
	}
	state, err := trustOIDCRandom()
	if err != nil {
		return nil, err
	}
	binding, err := trustOIDCRandom()
	if err != nil {
		return nil, err
	}
	nonce, err := trustOIDCRandom()
	if err != nil {
		return nil, err
	}
	verifier, err := trustOIDCRandom()
	if err != nil {
		return nil, err
	}
	cfg := oauth2.Config{ClientID: c.ClientID, RedirectURL: c.RedirectURI, Endpoint: p.Endpoint(), Scopes: []string{oidc.ScopeOpenID}}
	opts := []oauth2.AuthCodeOption{oidc.Nonce(nonce), oauth2.S256ChallengeOption(verifier)}
	if stepUp != nil {
		claims, _ := json.Marshal(map[string]any{"id_token": map[string]any{"auth_time": map[string]any{"essential": true}, "acr": map[string]any{"essential": true, "values": stepPolicy.ACRValues}}})
		opts = append(opts, oauth2.SetAuthURLParam("max_age", fmt.Sprint(stepPolicy.MaxAgeSeconds)), oauth2.SetAuthURLParam("acr_values", strings.Join(stepPolicy.ACRValues, " ")), oauth2.SetAuthURLParam("claims", string(claims)))
	}
	login := trustOIDCLogin{State: state, Binding: binding, AuthorizationURL: cfg.AuthCodeURL(state, opts...)}
	return s.withTrustLock(true, func(st *trustState) (any, error) {
		if st.Revision != revision {
			return nil, trustOIDCDenied()
		}
		now := time.Now().UTC()
		if stepUp != nil {
			actor, err := findTrustIdentityAt(st, token, now)
			if err != nil || !trustStepUpBindingMatches(st, *stepUp, actor, now) {
				return nil, trustOIDCDenied()
			}
		}
		if st.OIDCPending == nil {
			st.OIDCPending = map[string]trustOIDCPending{}
		}
		for k, v := range st.OIDCPending {
			if !now.Before(v.ExpiresAt) {
				delete(st.OIDCPending, k)
			}
		}
		for k, v := range st.OIDCSessions {
			if !now.Before(v.ExpiresAt) || v.Revision != revision {
				delete(st.OIDCSessions, k)
			}
		}
		st.OIDCPending[trustTokenHash(state)] = trustOIDCPending{StartedAt: now, StepUp: stepUp, BindingHash: trustTokenHash(binding), Nonce: nonce, Verifier: verifier, Revision: revision, ExpiresAt: now.Add(time.Duration(c.LoginTTLSeconds) * time.Second)}
		return login, nil
	})
}

func (s *Store) trustOIDCFinish(ctx context.Context, cmd trustOIDCCommand) (any, error) {
	if !trustCredentialValid(cmd.State) || !trustCredentialValid(cmd.Binding) || cmd.Code == "" {
		return nil, trustOIDCDenied()
	}
	var pending trustOIDCPending
	var c TrustOIDCConfig
	var stepPolicy TrustStepUpPolicy
	_, err := s.withTrustLock(true, func(st *trustState) (any, error) {
		p, ok := st.OIDCPending[trustTokenHash(cmd.State)]
		if !ok || !hmac.Equal([]byte(p.BindingHash), []byte(trustTokenHash(cmd.Binding))) || p.Revision != st.Revision || !time.Now().Before(p.ExpiresAt) || st.OIDC == nil || !bool(st.OIDC.Enabled) {
			return nil, trustOIDCDenied()
		}
		pending = p
		stepPolicy = effectiveTrustStepUp(st)
		c = *st.OIDC
		c.Mappings = append([]TrustOIDCMapping{}, c.Mappings...)
		delete(st.OIDCPending, trustTokenHash(cmd.State))
		return nil, nil
	})
	if err != nil {
		return nil, err
	}
	ctx, cancel := context.WithTimeout(ctx, time.Duration(c.RequestTimeoutSeconds)*time.Second)
	defer cancel()
	ctx, p, err := trustOIDCProvider(ctx, c)
	if err != nil {
		return nil, err
	}
	endpoint := p.Endpoint()
	endpoint.AuthStyle = oauth2.AuthStyleInParams
	cfg := oauth2.Config{ClientID: c.ClientID, RedirectURL: c.RedirectURI, Endpoint: endpoint}
	token, err := cfg.Exchange(ctx, cmd.Code, oauth2.VerifierOption(pending.Verifier))
	if err != nil {
		return nil, trustOIDCDenied()
	}
	raw, ok := token.Extra("id_token").(string)
	if !ok {
		return nil, trustOIDCDenied()
	}
	signed, err := jose.ParseSigned(raw, []jose.SignatureAlgorithm{jose.RS256, jose.ES256, jose.EdDSA})
	if err != nil || len(signed.Signatures) != 1 || signed.Signatures[0].Header.KeyID == "" {
		return nil, trustOIDCDenied()
	}
	verified, err := p.Verifier(&oidc.Config{ClientID: c.ClientID, SupportedSigningAlgs: []string{"RS256", "ES256", "EdDSA"}}).Verify(ctx, raw)
	if err != nil {
		return nil, trustOIDCDenied()
	}
	now := time.Now().UTC()
	var claims struct {
		AZP       string `json:"azp"`
		NotBefore int64  `json:"nbf"`
		AuthTime  int64  `json:"auth_time"`
		SID       string `json:"sid"`
		ACR       string `json:"acr"`
	}
	if verified.Claims(&claims) != nil || verified.Subject == "" || verified.Nonce != pending.Nonce || verified.IssuedAt.IsZero() || verified.IssuedAt.After(now) || !now.Before(verified.Expiry) || claims.NotBefore > now.Unix() || claims.AZP != "" && claims.AZP != c.ClientID || len(verified.Audience) > 1 && claims.AZP != c.ClientID {
		return nil, trustOIDCDenied()
	}
	providerAuthTime := time.Time{}
	if claims.AuthTime < 0 || claims.AuthTime > now.Unix() {
		return nil, trustOIDCDenied()
	}
	if claims.AuthTime > 0 {
		providerAuthTime = time.Unix(claims.AuthTime, 0).UTC()
	}
	if verified.AccessTokenHash != "" {
		if verified.VerifyAccessToken(token.AccessToken) != nil {
			return nil, trustOIDCDenied()
		}
	}
	subject := ""
	for _, m := range c.Mappings {
		if m.ExternalSubject == verified.Subject {
			subject = m.Subject
		}
	}
	if subject == "" {
		return nil, trustOIDCDenied()
	}
	var stepProof *trustStepUpProof
	if pending.StepUp != nil {
		if !bool(stepPolicy.Enabled) || subject != pending.StepUp.Subject {
			return nil, trustOIDCDenied()
		}
		stepProof, err = trustStepUpVerifyClaims(stepPolicy, *pending.StepUp, claims.AuthTime, claims.ACR, now)
		if err != nil {
			return nil, err
		}
	}
	session, err := trustOIDCRandom()
	if err != nil {
		return nil, err
	}
	expires := now.Add(time.Duration(c.SessionTTLSeconds) * time.Second)
	if verified.Expiry.Before(expires) {
		expires = verified.Expiry
	}
	if stepProof != nil {
		proofExpiry := now.Add(time.Duration(stepPolicy.ProofTTLSeconds) * time.Second)
		if proofExpiry.Before(expires) {
			expires = proofExpiry
		}
	}
	return s.withTrustLock(true, func(st *trustState) (any, error) {
		if ctx.Err() != nil || !time.Now().Before(pending.ExpiresAt) || trustOIDCLogoutBlocksLogin(st, c.Issuer, verified.Subject, claims.SID, pending, time.Now().UTC()) {
			return nil, trustOIDCDenied()
		}
		if st.Revision != pending.Revision {
			return nil, trustOIDCDenied()
		}
		var id trustIdentity
		for _, candidate := range st.Identities {
			if candidate.Subject == subject {
				id = candidate
			}
		}
		if id.Subject == "" || id.Disabled {
			return nil, trustOIDCDenied()
		}
		var signedClaims map[string]json.RawMessage
		if err := verified.Claims(&signedClaims); err != nil {
			return nil, trustOIDCDenied()
		}
		id, mappingDigest, err := mapTrustIdentityGroups(effectiveTrustGroupMapping(st), signedClaims, id)
		if err != nil {
			return nil, err
		}
		if stepProof != nil && !trustStepUpBindingMatches(st, stepProof.Binding, id, time.Now().UTC()) {
			return nil, trustOIDCDenied()
		}
		if st.OIDCSessions == nil {
			st.OIDCSessions = map[string]trustOIDCSession{}
		}
		st.OIDCSessions[trustTokenHash(session)] = trustOIDCSession{MappedRoles: groupSessionRoles(id, mappingDigest), GroupMappingDigest: mappingDigest, AuthenticatedAt: time.Now().UTC(), ProviderAuthTime: providerAuthTime, ACR: claims.ACR, Issuer: verified.Issuer, ExternalSubject: verified.Subject, SID: claims.SID, StepUp: stepProof, Subject: subject, Revision: st.Revision, ExpiresAt: expires}
		result := map[string]any{"revision": st.Revision, "subject": subject, "session_token": session, "expires_at": expires, "roles": id.Roles, "integrity": id.Integrity}
		if stepProof != nil {
			result["step_up_action_id"] = stepProof.Binding.ActionID
		}
		return result, nil
	})
}

func (s *Store) trustOIDCOperation(ctx context.Context, action string, raw []byte, tokens ...string) (any, error) {
	var cmd trustOIDCCommand
	if err := strict(raw, &cmd); err != nil {
		return nil, err
	}
	if action == "oidc-logout" || action == "oidc-session" {
		if !trustCredentialValid(cmd.SessionToken) || cmd.State != "" || cmd.Code != "" || cmd.Binding != "" || cmd.StepUpActionID != "" {
			return nil, trustOIDCDenied()
		}
		if action == "oidc-session" {
			return s.withTrustLock(false, func(st *trustState) (any, error) {
				session, exists := st.OIDCSessions[trustTokenHash(cmd.SessionToken)]
				_, err := findTrustIdentity(st, cmd.SessionToken)
				return map[string]any{"active": exists && err == nil && session.Revision == st.Revision && st.OIDC != nil && bool(st.OIDC.Enabled) && time.Now().Before(session.ExpiresAt)}, nil
			})
		}
		return s.withTrustLock(true, func(st *trustState) (any, error) {
			delete(st.OIDCSessions, trustTokenHash(cmd.SessionToken))
			return map[string]any{"logged_out": true}, nil
		})
	}
	if cmd.SessionToken != "" {
		return nil, trustOIDCDenied()
	}
	if action == "oidc-start" {
		if cmd.State != "" || cmd.Code != "" || cmd.Binding != "" {
			return nil, trustOIDCDenied()
		}
		token := ""
		if len(tokens) > 0 {
			token = tokens[0]
		}
		return s.trustOIDCStartFor(ctx, cmd, token)
	}
	if cmd.StepUpActionID != "" {
		return nil, trustOIDCDenied()
	}
	return s.trustOIDCFinish(ctx, cmd)
}

func trustOIDCChallenge(verifier string) string {
	digest := sha256.Sum256([]byte(verifier))
	return base64.RawURLEncoding.EncodeToString(digest[:])
}
