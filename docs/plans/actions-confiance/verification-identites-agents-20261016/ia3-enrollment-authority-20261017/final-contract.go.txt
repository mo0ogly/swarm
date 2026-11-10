// Package identitycontract defines data and validation shared by the engine and
// enrollment wire schema. Valid data is not authenticated identity or authority.
package identitycontract

import (
	"encoding/hex"
	"strings"
	"unicode"
	"unicode/utf8"
)

type Principal struct {
	ID      string `json:"principal_id"`
	Kind    string `json:"principal_kind"`
	Issuer  string `json:"issuer"`
	Subject string `json:"subject"`
}

type Scope struct {
	WorkID      string `json:"work_id"`
	PlanVersion int    `json:"plan_version"`
	PlanDigest  string `json:"plan_digest"`
}

// Resources are exact identifiers, never glob or path-prefix policies.
type Permission struct {
	Operation string `json:"operation"`
	Resource  string `json:"resource"`
}

// Name preserves the engine's existing 100-byte ASCII identifier representation.
// It is not an OIDC issuer/subject validator or an operator execution policy.
func Name(s string) bool {
	if s == "" || len(s) > 100 {
		return false
	}
	for _, r := range s {
		if !(r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z' || r >= '0' && r <= '9' || r == '-' || r == '_') {
			return false
		}
	}
	return true
}

func Text(s string) bool {
	return s != "" && utf8.ValidString(s) && strings.TrimSpace(s) == s && strings.IndexFunc(s, unicode.IsControl) < 0
}

func Digest(s string) bool {
	b, err := hex.DecodeString(s)
	return err == nil && len(b) == 32 && hex.EncodeToString(b) == s
}

func (p Principal) Valid() bool {
	if !Name(p.ID) || !Text(p.Issuer) || !Text(p.Subject) {
		return false
	}
	switch p.Kind {
	case "local":
		return p.Issuer == "urn:swarm:local" && p.Subject == p.ID
	case "human", "service":
		return p.Issuer != "urn:swarm:local"
	}
	return false
}

func (s Scope) Valid() bool {
	return Name(s.WorkID) && s.PlanVersion > 0 && Digest(s.PlanDigest)
}

func (p Permission) Valid() bool {
	switch p.Operation {
	case "agent.create", "memory.read", "memory.summarize", "memory.transmit", "tool.invoke", "effect.execute":
		return Text(p.Resource) && !strings.ContainsAny(p.Resource, "*?[]")
	}
	return false
}
