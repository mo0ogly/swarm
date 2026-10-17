//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"mime"
	"net/http"
	"strings"
	"time"

	"github.com/coreos/go-oidc/v3/oidc"
	jose "github.com/go-jose/go-jose/v4"
)

const trustOIDCLogoutPath = "/api/v1/trust-actions/oidc-backchannel-logout"
const trustOIDCLogoutEvent = "http://schemas.openid.net/event/backchannel-logout"

type TrustOIDCLogoutPolicy struct {
	Enabled                trustOIDCEnabled `json:"enabled"`
	MaxAgeSeconds          int              `json:"max_age_seconds"`
	ReplayRetentionSeconds int              `json:"replay_retention_seconds"`
}

func (p *TrustOIDCLogoutPolicy) UnmarshalJSON(data []byte) error {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(data, &fields); err != nil {
		return err
	}
	for _, key := range []string{"enabled", "max_age_seconds", "replay_retention_seconds"} {
		if value, ok := fields[key]; !ok || bytes.Equal(bytes.TrimSpace(value), []byte("null")) {
			return fmt.Errorf("déconnexion OIDC : champs explicites non null requis")
		}
	}
	type policy TrustOIDCLogoutPolicy
	var decoded policy
	if err := strict(data, &decoded); err != nil {
		return err
	}
	*p = TrustOIDCLogoutPolicy(decoded)
	return nil
}

func defaultTrustOIDCLogoutPolicy() TrustOIDCLogoutPolicy {
	return TrustOIDCLogoutPolicy{MaxAgeSeconds: 300, ReplayRetentionSeconds: 600}
}
func effectiveTrustOIDCLogout(st *trustState) TrustOIDCLogoutPolicy {
	if st.OIDCLogout == nil {
		return defaultTrustOIDCLogoutPolicy()
	}
	return *st.OIDCLogout
}
func validateTrustOIDCLogout(p *TrustOIDCLogoutPolicy, c *TrustOIDCConfig) error {
	if p == nil {
		return nil
	}
	max := trustPolicyBounds()["oidc_logout_max_age_seconds"]["max"]
	if p.MaxAgeSeconds < 1 || int64(p.MaxAgeSeconds) > max || p.ReplayRetentionSeconds < 0 || int64(p.ReplayRetentionSeconds) > max || p.ReplayRetentionSeconds > 0 && p.ReplayRetentionSeconds < p.MaxAgeSeconds {
		return fmt.Errorf("déconnexion OIDC : âge positif et conservation nulle ou supérieure à l’âge requis")
	}
	if bool(p.Enabled) && (c == nil || !bool(c.Enabled)) {
		return fmt.Errorf("déconnexion OIDC : connexion OIDC active requise")
	}
	return nil
}
func trustOIDCLogoutDenied() error {
	return &CommandError{Code: "invalid_logout_token", Message: "déconnexion fournisseur invalide ou rejouée"}
}

type trustOIDCLogoutCommand struct {
	LogoutToken string `json:"logout_token"`
}
type trustOIDCLogoutClaims struct {
	SID       string                     `json:"sid"`
	JTI       string                     `json:"jti"`
	AZP       string                     `json:"azp"`
	NotBefore int64                      `json:"nbf"`
	Events    map[string]json.RawMessage `json:"events"`
}

func (s *Store) trustOIDCBackchannelLogout(ctx context.Context, raw string) (any, error) {
	if raw == "" || strings.ContainsAny(raw, " \t\r\n") {
		return nil, trustOIDCLogoutDenied()
	}
	c, revision, err := s.trustOIDCConfigSnapshot()
	if err != nil {
		return nil, trustOIDCLogoutDenied()
	}
	var policy TrustOIDCLogoutPolicy
	_, err = s.withTrustLock(false, func(st *trustState) (any, error) {
		policy = effectiveTrustOIDCLogout(st)
		if st.Revision != revision || !bool(policy.Enabled) {
			return nil, trustOIDCLogoutDenied()
		}
		return nil, validateTrustOIDCLogout(&policy, &c)
	})
	if err != nil {
		return nil, err
	}
	signed, err := jose.ParseSigned(raw, []jose.SignatureAlgorithm{jose.RS256, jose.ES256, jose.EdDSA})
	if err != nil || len(signed.Signatures) != 1 || signed.Signatures[0].Header.KeyID == "" {
		return nil, trustOIDCLogoutDenied()
	}
	if value, present := signed.Signatures[0].Header.ExtraHeaders[jose.HeaderKey("typ")]; present {
		typ, ok := value.(string)
		if !ok || typ != "JWT" && typ != "logout+jwt" {
			return nil, trustOIDCLogoutDenied()
		}
	}
	ctx, cancel := context.WithTimeout(ctx, time.Duration(c.RequestTimeoutSeconds)*time.Second)
	defer cancel()
	ctx, provider, err := trustOIDCProvider(ctx, c)
	if err != nil {
		return nil, trustOIDCLogoutDenied()
	}
	verified, err := provider.Verifier(&oidc.Config{ClientID: c.ClientID, SupportedSigningAlgs: []string{"RS256", "ES256", "EdDSA"}}).Verify(ctx, raw)
	if err != nil {
		return nil, trustOIDCLogoutDenied()
	}
	var claims trustOIDCLogoutClaims
	var fields map[string]json.RawMessage
	if verified.Claims(&claims) != nil || verified.Claims(&fields) != nil {
		return nil, trustOIDCLogoutDenied()
	}
	if _, present := fields["nonce"]; present {
		return nil, trustOIDCLogoutDenied()
	}
	for _, key := range []string{"iss", "aud", "iat", "exp", "jti", "events"} {
		if value, ok := fields[key]; !ok || bytes.Equal(bytes.TrimSpace(value), []byte("null")) {
			return nil, trustOIDCLogoutDenied()
		}
	}
	for _, key := range []string{"sid", "sub", "azp", "nbf"} {
		if value, present := fields[key]; present && bytes.Equal(bytes.TrimSpace(value), []byte("null")) {
			return nil, trustOIDCLogoutDenied()
		}
	}
	for _, key := range []string{"sid", "sub", "azp"} {
		if rawValue, present := fields[key]; present {
			var value string
			if json.Unmarshal(rawValue, &value) != nil || value == "" {
				return nil, trustOIDCLogoutDenied()
			}
		}
	}
	now := time.Now().UTC()
	if verified.Issuer != c.Issuer || verified.IssuedAt.IsZero() || verified.IssuedAt.After(now) || now.Sub(verified.IssuedAt) > time.Duration(policy.MaxAgeSeconds)*time.Second || !now.Before(verified.Expiry) || !verified.Expiry.After(verified.IssuedAt) || claims.NotBefore > now.Unix() || claims.JTI == "" || verified.Subject == "" && claims.SID == "" || claims.AZP != "" && claims.AZP != c.ClientID || len(verified.Audience) > 1 && claims.AZP != c.ClientID {
		return nil, trustOIDCLogoutDenied()
	}
	var event map[string]json.RawMessage
	if json.Unmarshal(claims.Events[trustOIDCLogoutEvent], &event) != nil || event == nil {
		return nil, trustOIDCLogoutDenied()
	}
	return s.applyTrustOIDCLogout(c, revision, policy, verified, claims, now)
}

func (s *Store) applyTrustOIDCLogout(c TrustOIDCConfig, revision int, p TrustOIDCLogoutPolicy, verified *oidc.IDToken, claims trustOIDCLogoutClaims, now time.Time) (any, error) {
	key := hashMust(struct{ Issuer, JTI string }{c.Issuer, claims.JTI})
	return s.withTrustLock(true, func(st *trustState) (any, error) {
		current := time.Now().UTC()
		if st.Revision != revision || !bool(effectiveTrustOIDCLogout(st).Enabled) || !current.Before(verified.Expiry) || current.Sub(verified.IssuedAt) > time.Duration(p.MaxAgeSeconds)*time.Second {
			return nil, trustOIDCLogoutDenied()
		}
		if until, present := st.OIDCLogoutUsed[key]; present && (until.IsZero() || current.Before(until)) {
			return nil, trustOIDCLogoutDenied()
		}
		until := time.Time{}
		if p.ReplayRetentionSeconds > 0 {
			until = now.Add(time.Duration(p.ReplayRetentionSeconds) * time.Second)
			if verified.Expiry.After(until) {
				until = verified.Expiry
			}
		}
		if st.OIDCLogoutUsed == nil {
			st.OIDCLogoutUsed = map[string]time.Time{}
		}
		for old, expiry := range st.OIDCLogoutUsed {
			if !expiry.IsZero() && !current.Before(expiry) {
				delete(st.OIDCLogoutUsed, old)
			}
		}
		st.OIDCLogoutUsed[key] = until
		if st.OIDCLogoutFences == nil {
			st.OIDCLogoutFences = map[string]trustOIDCLogoutFence{}
		}
		for old, fence := range st.OIDCLogoutFences {
			if !current.Before(fence.Until) {
				delete(st.OIDCLogoutFences, old)
			}
		}
		target := trustOIDCLogoutTarget(c.Issuer, claims.SID, verified.Subject)
		st.OIDCLogoutFences[target] = trustOIDCLogoutFence{ReceivedAt: current, Until: current.Add(time.Duration(c.LoginTTLSeconds) * time.Second).Add(time.Duration(c.RequestTimeoutSeconds) * time.Second)}

		removed := 0
		for hash, session := range st.OIDCSessions {
			if session.Issuer != c.Issuer {
				continue
			}
			if claims.SID != "" {
				if session.SID != claims.SID || verified.Subject != "" && session.ExternalSubject != verified.Subject {
					continue
				}
			} else if session.ExternalSubject != verified.Subject {
				continue
			}
			delete(st.OIDCSessions, hash)
			removed++
		}
		return map[string]any{"logged_out": true, "revoked_sessions": removed}, nil
	})
}

func (s *Store) trustOIDCLogoutOperation(ctx context.Context, raw []byte) (any, error) {
	var cmd trustOIDCLogoutCommand
	if err := strict(raw, &cmd); err != nil {
		return nil, err
	}
	return s.trustOIDCBackchannelLogout(ctx, cmd.LogoutToken)
}

func registerTrustOIDCLogoutHTTP(s *Store, mux *http.ServeMux) {
	mux.HandleFunc(trustOIDCLogoutPath, func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		w.Header().Set("Content-Type", "application/json")
		if r.Method != http.MethodPost {
			w.WriteHeader(http.StatusMethodNotAllowed)
			json.NewEncoder(w).Encode(map[string]string{"error": "POST requis"})
			return
		}
		r.Body = http.MaxBytesReader(w, r.Body, 1<<20)
		mediaType, _, mediaErr := mime.ParseMediaType(r.Header.Get("Content-Type"))
		if mediaErr != nil || mediaType != "application/x-www-form-urlencoded" || r.ParseForm() != nil || len(r.PostForm["logout_token"]) != 1 {
			w.WriteHeader(http.StatusBadRequest)
			io.WriteString(w, `{"error":"invalid_logout_token"}`)
			return
		}
		v, err := s.trustOIDCBackchannelLogout(r.Context(), r.PostForm.Get("logout_token"))
		if err != nil {
			w.WriteHeader(http.StatusBadRequest)
			io.WriteString(w, `{"error":"invalid_logout_token"}`)
			return
		}
		json.NewEncoder(w).Encode(v)
	})
}

type trustOIDCLogoutFence struct {
	ReceivedAt time.Time `json:"received_at"`
	Until      time.Time `json:"until"`
}

func trustOIDCLogoutTarget(issuer, sid, subject string) string {
	return hashMust(struct{ Issuer, SID, Subject string }{issuer, sid, subject})
}
func trustOIDCLogoutBlocksLogin(st *trustState, issuer, subject, sid string, pending trustOIDCPending, now time.Time) bool {
	keys := []string{trustOIDCLogoutTarget(issuer, "", subject)}
	if sid != "" {
		keys = append(keys, trustOIDCLogoutTarget(issuer, sid, ""), trustOIDCLogoutTarget(issuer, sid, subject))
	}
	for _, key := range keys {
		if fence, exists := st.OIDCLogoutFences[key]; exists && now.Before(fence.Until) && !pending.StartedAt.After(fence.ReceivedAt) {
			return true
		}
	}
	return false
}
