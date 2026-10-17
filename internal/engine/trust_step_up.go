//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"strings"
	"time"
	"unicode"
)

type TrustStepUpPolicy struct {
	Enabled         trustOIDCEnabled `json:"enabled"`
	Operations      []string         `json:"operations"`
	ACRValues       []string         `json:"acr_values"`
	MaxAgeSeconds   int              `json:"max_age_seconds"`
	ProofTTLSeconds int              `json:"proof_ttl_seconds"`
}

func (p *TrustStepUpPolicy) UnmarshalJSON(data []byte) error {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(data, &fields); err != nil {
		return err
	}
	for _, key := range []string{"enabled", "operations", "acr_values", "max_age_seconds", "proof_ttl_seconds"} {
		if value, ok := fields[key]; !ok || bytes.Equal(bytes.TrimSpace(value), []byte("null")) {
			return fmt.Errorf("step-up : champs explicites non null requis")
		}
	}
	type policy TrustStepUpPolicy
	var decoded policy
	if err := strict(data, &decoded); err != nil {
		return err
	}
	*p = TrustStepUpPolicy(decoded)
	return nil
}

func defaultTrustStepUpPolicy() TrustStepUpPolicy {
	return TrustStepUpPolicy{Operations: []string{"approve"}, ACRValues: []string{}, ProofTTLSeconds: 300}
}
func effectiveTrustStepUp(st *trustState) TrustStepUpPolicy {
	if st.StepUp == nil {
		return defaultTrustStepUpPolicy()
	}
	return *st.StepUp
}

func validateTrustStepUp(p *TrustStepUpPolicy, c *TrustOIDCConfig) error {
	if p == nil {
		return nil
	}
	if p.Operations == nil || p.ACRValues == nil {
		return fmt.Errorf("step-up : champs explicites non null requis")
	}
	max := int64((1<<63 - 1) / int64(time.Second))
	if p.MaxAgeSeconds < 0 || int64(p.MaxAgeSeconds) > max || p.ProofTTLSeconds < 1 || int64(p.ProofTTLSeconds) > max {
		return fmt.Errorf("step-up : âge positif ou nul et durée de preuve strictement positive requis")
	}
	seen := map[string]bool{}
	for _, v := range p.Operations {
		if v != "approve" || seen[v] {
			return fmt.Errorf("step-up : opération approve unique requise")
		}
		seen[v] = true
	}
	if bool(p.Enabled) && (len(p.Operations) != 1 || c == nil || !bool(c.Enabled) || len(p.ACRValues) == 0) {
		return fmt.Errorf("step-up : OIDC actif, opération approve et ACR convenus requis")
	}
	seen = map[string]bool{}
	for _, v := range p.ACRValues {
		if v == "" || strings.IndexFunc(v, func(r rune) bool { return unicode.IsSpace(r) || unicode.IsControl(r) }) >= 0 || seen[v] {
			return fmt.Errorf("step-up : ACR non vides, sans espaces et uniques requis")
		}
		seen[v] = true
	}
	return nil
}

type trustStepUpBinding struct {
	ActionID      string    `json:"action_id"`
	ActionDigest  string    `json:"action_digest"`
	MandateDigest string    `json:"mandate_digest"`
	Subject       string    `json:"subject"`
	StartedAt     time.Time `json:"started_at"`
}

type trustStepUpProof struct {
	Binding  trustStepUpBinding `json:"binding"`
	AuthTime int64              `json:"auth_time"`
	ACR      string             `json:"acr"`
	Used     bool               `json:"used"`
}

func trustStepUpActionBinding(st *trustState, actionID string, actor trustIdentity, now time.Time) (trustStepUpBinding, error) {
	a, ok := st.Actions[actionID]
	if !ok || a.State != "pending_approval" || !trustHasRole(actor, "approver") || actor.Integrity < 2 || (st.Policy.RequireSeparation && actor.Subject == a.Requester) || trustActiveMandateReason(st, a.MandateID, now) != "" {
		return trustStepUpBinding{}, trustOIDCDenied()
	}
	return trustStepUpBinding{ActionID: a.ID, ActionDigest: a.ContentHash, MandateDigest: st.Mandates[a.MandateID].ContentHash, Subject: actor.Subject, StartedAt: now}, nil
}

func trustStepUpBindingMatches(st *trustState, b trustStepUpBinding, actor trustIdentity, now time.Time) bool {
	current, err := trustStepUpActionBinding(st, b.ActionID, actor, now)
	return err == nil && b.Subject == actor.Subject && b.ActionDigest == current.ActionDigest && b.MandateDigest == current.MandateDigest
}

func trustStepUpApprovalReason(st *trustState, token string, actor trustIdentity, a TrustAction, now time.Time) string {
	p := effectiveTrustStepUp(st)
	if !bool(p.Enabled) {
		return ""
	}
	session, ok := st.OIDCSessions[trustTokenHash(token)]
	if !ok || session.Revision != st.Revision || !now.Before(session.ExpiresAt) || session.Subject != actor.Subject || session.StepUp == nil || session.StepUp.Used || session.StepUp.Binding.ActionID != a.ID || !containsTrust(p.ACRValues, session.StepUp.ACR) || !trustStepUpBindingMatches(st, session.StepUp.Binding, actor, now) {
		return "authentification renforcée requise pour cette action exacte"
	}
	return ""
}

func (s *Store) trustStepUpStartBinding(actionID, token string, revision int) (*trustStepUpBinding, TrustStepUpPolicy, error) {
	var binding *trustStepUpBinding
	var policy TrustStepUpPolicy
	_, err := s.withTrustLock(false, func(st *trustState) (any, error) {
		policy = effectiveTrustStepUp(st)
		if !bool(policy.Enabled) || st.Revision != revision {
			return nil, trustOIDCDenied()
		}
		now := time.Now().UTC()
		actor, err := findTrustIdentityAt(st, token, now)
		if err != nil {
			return nil, err
		}
		b, err := trustStepUpActionBinding(st, actionID, actor, now)
		if err != nil {
			return nil, err
		}
		binding = &b
		return nil, nil
	})
	return binding, policy, err
}

func trustStepUpVerifyClaims(p TrustStepUpPolicy, b trustStepUpBinding, authTime int64, acr string, now time.Time) (*trustStepUpProof, error) {
	if authTime <= 0 || authTime > now.Unix() || !containsTrust(p.ACRValues, acr) {
		return nil, trustOIDCDenied()
	}
	if p.MaxAgeSeconds == 0 {
		if authTime < b.StartedAt.Unix() {
			return nil, trustOIDCDenied()
		}
	} else if now.Sub(time.Unix(authTime, 0)) > time.Duration(p.MaxAgeSeconds)*time.Second {
		return nil, trustOIDCDenied()
	}
	return &trustStepUpProof{Binding: b, AuthTime: authTime, ACR: acr}, nil
}
