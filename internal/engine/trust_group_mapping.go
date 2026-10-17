//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"sort"
	"strings"
	"unicode"
)

type TrustGroupRule struct {
	Group string   `json:"group"`
	Roles []string `json:"roles"`
}
type TrustGroupMapping struct {
	Enabled trustOIDCEnabled `json:"enabled"`
	Claim   string           `json:"claim"`
	Rules   []TrustGroupRule `json:"rules"`
}

func (p *TrustGroupMapping) UnmarshalJSON(data []byte) error {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(data, &fields); err != nil {
		return err
	}
	for _, key := range []string{"enabled", "claim", "rules"} {
		if v, ok := fields[key]; !ok || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return fmt.Errorf("groupes : champs explicites non null requis")
		}
	}
	type plain TrustGroupMapping
	var v plain
	if err := strict(data, &v); err != nil {
		return err
	}
	*p = TrustGroupMapping(v)
	return nil
}
func defaultTrustGroupMapping() TrustGroupMapping {
	return TrustGroupMapping{Claim: "groups", Rules: []TrustGroupRule{}}
}
func effectiveTrustGroupMapping(st *trustState) TrustGroupMapping {
	if st.GroupMapping == nil {
		return defaultTrustGroupMapping()
	}
	return *st.GroupMapping
}
func trustGroupLabelValid(v string) bool {
	return strings.TrimSpace(v) == v && v != "" && strings.IndexFunc(v, unicode.IsControl) < 0
}
func validateTrustGroupMapping(p *TrustGroupMapping, oidc *TrustOIDCConfig, workloads *TrustWorkloadConfig) error {
	if p == nil {
		return nil
	}
	if !trustGroupLabelValid(p.Claim) || p.Rules == nil {
		return fmt.Errorf("groupes : claim non vide et liste de règles explicite requis")
	}
	groups := map[string]bool{}
	for _, rule := range p.Rules {
		if !trustGroupLabelValid(rule.Group) || groups[rule.Group] || len(rule.Roles) == 0 {
			return fmt.Errorf("groupes : règles non vides et groupes uniques requis")
		}
		groups[rule.Group] = true
		roles := map[string]bool{}
		for _, role := range rule.Roles {
			if (role != "requester" && role != "approver" && role != "effect" && role != "admin") || roles[role] {
				return fmt.Errorf("groupes : rôles locaux connus et uniques requis")
			}
			roles[role] = true
		}
	}
	if bool(p.Enabled) && (len(p.Rules) == 0 || (oidc == nil || !bool(oidc.Enabled)) && (workloads == nil || !bool(workloads.Enabled))) {
		return fmt.Errorf("groupes : connexion fédérée active et règles requises")
	}
	return nil
}
func mapTrustIdentityGroups(p TrustGroupMapping, claims map[string]json.RawMessage, id trustIdentity) (trustIdentity, string, error) {
	if !bool(p.Enabled) {
		return id, "", nil
	}
	denied := func() (trustIdentity, string, error) {
		return trustIdentity{}, "", &CommandError{Code: "authorization_required", Message: "groupes signés absents, invalides ou sans rôle local autorisé"}
	}
	raw, ok := claims[p.Claim]
	if !ok || bytes.Equal(bytes.TrimSpace(raw), []byte("null")) {
		return denied()
	}
	var groups []string
	if json.Unmarshal(raw, &groups) != nil {
		return denied()
	}
	selected := map[string]bool{}
	rules := make(map[string][]string, len(p.Rules))
	for _, rule := range p.Rules {
		rules[rule.Group] = rule.Roles
	}
	for _, group := range groups {
		if !trustGroupLabelValid(group) {
			return denied()
		}
		for _, role := range rules[group] {
			if containsTrust(id.Roles, role) {
				selected[role] = true
			}
		}
	}

	if len(selected) == 0 {
		return denied()
	}
	id.Roles = make([]string, 0, len(selected))
	for role := range selected {
		id.Roles = append(id.Roles, role)
	}
	sort.Strings(id.Roles)
	return id, hashMust(p), nil
}
func trustSessionGroupRoles(st *trustState, session trustOIDCSession, id trustIdentity) (trustIdentity, error) {
	p := effectiveTrustGroupMapping(st)
	if !bool(p.Enabled) {
		if session.GroupMappingDigest != "" {
			return trustIdentity{}, trustOIDCDenied()
		}
		return id, nil
	}
	if session.GroupMappingDigest != hashMust(p) || len(session.MappedRoles) == 0 {
		return trustIdentity{}, trustOIDCDenied()
	}
	roles := []string{}
	for _, role := range id.Roles {
		if containsTrust(session.MappedRoles, role) && !containsTrust(roles, role) {
			roles = append(roles, role)
		}
	}
	if len(roles) == 0 {
		return trustIdentity{}, trustOIDCDenied()
	}
	id.Roles = roles
	return id, nil
}
func groupSessionRoles(id trustIdentity, digest string) []string {
	if digest == "" {
		return nil
	}
	return append([]string{}, id.Roles...)
}
func trustGroupMappingValue(p *TrustGroupMapping) TrustGroupMapping {
	if p == nil {
		return defaultTrustGroupMapping()
	}
	return *p
}
