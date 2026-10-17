//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"time"
)

type TrustDelegationPolicy struct {
	MaxDepth int `json:"max_depth"`
}

func (p *TrustDelegationPolicy) UnmarshalJSON(data []byte) error {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(data, &fields); err != nil {
		return err
	}
	value, present := fields["max_depth"]
	if !present || bytes.Equal(bytes.TrimSpace(value), []byte("null")) {
		return fmt.Errorf("délégation : max_depth doit être un entier explicite")
	}
	type policy TrustDelegationPolicy
	var decoded policy
	if err := strict(data, &decoded); err != nil {
		return err
	}
	*p = TrustDelegationPolicy(decoded)
	return nil
}

func effectiveTrustDelegation(st *trustState) TrustDelegationPolicy {
	if st.Delegation == nil {
		return TrustDelegationPolicy{}
	}
	return *st.Delegation
}

func validateTrustDelegation(p *TrustDelegationPolicy) error {
	if p == nil {
		return nil
	}
	bound := trustPolicyBounds()["max_delegation_depth"]["max"]
	if p.MaxDepth < 0 || int64(p.MaxDepth) > bound {
		return fmt.Errorf("délégation : profondeur entière entre 0 et %d requise ; 0 désactive", bound)
	}
	return nil
}

func trustDelegationIdentityActive(st *trustState, subject string) bool {
	for _, id := range st.Identities {
		if id.Subject == subject {
			return !bool(id.Disabled)
		}
	}
	return false
}

func trustDelegationNarrows(child, parent TrustMandate) bool {
	if !trustConnectorEqual(child.Connector, parent.Connector) || child.Resource != parent.Resource || child.Currency != parent.Currency || child.Beneficiary != parent.Beneficiary || (trustConnectorID(child.Connector) != "local-access-simulation" && child.MaxAmount <= 0) || child.MaxAmount > parent.MaxAmount || len(child.Actions) == 0 {
		return false
	}
	for _, action := range child.Actions {
		if !containsTrust(parent.Actions, action) {
			return false
		}
	}
	start, err := time.Parse(time.RFC3339, child.StartsAt)
	if err != nil {
		return false
	}
	end, err := time.Parse(time.RFC3339, child.ExpiresAt)
	if err != nil {
		return false
	}
	parentStart, err := time.Parse(time.RFC3339, parent.StartsAt)
	if err != nil {
		return false
	}
	parentEnd, err := time.Parse(time.RFC3339, parent.ExpiresAt)
	if err != nil {
		return false
	}
	return end.After(start) && !start.Before(parentStart) && !end.After(parentEnd)
}

func trustDelegatedMandateReason(st *trustState, mandate TrustMandate, now time.Time) string {
	if mandate.ParentID == "" {
		if mandate.ParentHash != "" {
			return "référence de mandat parent invalide"
		}
		return ""
	}
	policy := effectiveTrustDelegation(st)
	if policy.MaxDepth == 0 {
		return "délégations désactivées"
	}
	current := mandate
	seen := map[string]bool{}
	depth := 0
	for {
		if seen[current.ID] {
			return "cycle dans la chaîne de délégation"
		}
		seen[current.ID] = true
		if current.ContentHash != trustMandateContentHash(current) {
			return "empreinte de mandat délégué altérée"
		}
		if !trustDelegationIdentityActive(st, current.Subject) {
			return "identité de la chaîne de délégation absente ou suspendue"
		}
		if current.ParentID == "" {
			return ""
		}
		depth++
		if depth > policy.MaxDepth {
			return "profondeur de délégation dépassée"
		}
		parent, ok := st.Mandates[current.ParentID]
		if !ok {
			return "mandat parent absent"
		}
		switch trustActiveMandateBaseReason(st, parent.ID, now) {
		case "mandat révoqué":
			return "mandat parent révoqué"
		case "mandat inactif":
			return "mandat parent inactif"
		case "mandat expiré":
			return "mandat parent expiré"
		case "révision de configuration du mandat périmée":
			return "révision de configuration du mandat parent périmée"
		case "":
		default:
			return "mandat parent invalide"
		}
		if current.ParentHash != parent.ContentHash {
			return "empreinte du mandat parent périmée"
		}
		if !trustDelegationNarrows(current, parent) {
			return "délégation plus large que le mandat parent"
		}
		current = parent
	}
}

func trustActiveMandateReason(st *trustState, id string, now time.Time) string {
	if reason := trustActiveMandateBaseReason(st, id, now); reason != "" {
		return reason
	}
	return trustDelegatedMandateReason(st, st.Mandates[id], now)
}

func trustDelegationPublicState(st *trustState) (map[string]string, map[string]string) {
	amounts, reasons := map[string]string{}, map[string]string{}
	now := time.Now().UTC()
	for id, m := range st.Mandates {
		amounts[id] = fmt.Sprint(m.MaxAmount)
		reasons[id] = trustActiveMandateReason(st, id, now)
	}
	return amounts, reasons
}
