//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
)

type TrustDataResource struct {
	Resource       string `json:"resource"`
	Classification string `json:"classification"`
}

type TrustDataPolicy struct {
	Resources       []TrustDataResource `json:"resources"`
	Enabled         trustOIDCEnabled    `json:"enabled"`
	Classifications []string            `json:"classifications"`
	Destinations    []string            `json:"destinations"`
	Providers       []string            `json:"providers"`
	Tools           []string            `json:"tools"`
}

type TrustDataContext struct {
	Classification string `json:"classification"`
	Destination    string `json:"destination"`
	Provider       string `json:"provider"`
	Tool           string `json:"tool"`
}

func trustExplicitFields(raw []byte, names ...string) error {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fields); err != nil {
		return err
	}
	for _, name := range names {
		v, ok := fields[name]
		if !ok || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return fmt.Errorf("données : champs explicites non null requis")
		}
	}
	return nil
}
func (p *TrustDataPolicy) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "enabled", "classifications", "destinations", "providers", "tools", "resources"); err != nil {
		return err
	}
	type plain TrustDataPolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustDataPolicy(v)
	return nil
}
func (c *TrustDataContext) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "classification", "destination", "provider", "tool"); err != nil {
		return err
	}
	type plain TrustDataContext
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*c = TrustDataContext(v)
	return nil
}
func defaultTrustDataPolicy() TrustDataPolicy {
	return TrustDataPolicy{Resources: []TrustDataResource{}, Classifications: []string{}, Destinations: []string{}, Providers: []string{}, Tools: []string{}}
}
func effectiveTrustDataPolicy(st *trustState) TrustDataPolicy {
	if st.DataPolicy == nil {
		return defaultTrustDataPolicy()
	}
	return *st.DataPolicy
}
func trustDataCapabilities() any {
	return map[string]any{"classifications": []string{"public", "internal"}, "destination": "local-ledger", "provider": "swarm", "tool": "simulated-settlement", "secret_delivery": map[string]string{"classification": "restricted", "destination": "local-attestation", "provider": "swarm", "tool": "sealed-attestation", "scope": "fixed_helper_only"}, "external_containment_verified": false, "secret_grants_supported": false, "local_paths": []any{map[string]string{"destination": "local-access-state", "provider": "swarm", "tool": "simulated-access-grant"}, map[string]string{"destination": "local-ledger", "provider": "swarm", "tool": "simulated-settlement"}, map[string]string{"destination": "local-process-state", "provider": "swarm", "tool": "process-workflow"}}}
}
func trustDataPolicyValue(p *TrustDataPolicy) TrustDataPolicy {
	if p == nil {
		return defaultTrustDataPolicy()
	}
	return *p
}
func validateTrustDataPolicy(p *TrustDataPolicy) error {
	if p == nil {
		return nil
	}
	lists := []struct{ values, known []string }{
		{p.Classifications, []string{"public", "internal", "confidential", "restricted"}},
		{p.Destinations, []string{"local-ledger", "local-process-state", "local-access-state", "local-attestation"}},
		{p.Providers, []string{"swarm"}},
		{p.Tools, []string{"simulated-settlement", "process-workflow", "simulated-access-grant", "sealed-attestation"}},
	}
	for _, list := range lists {
		if list.values == nil {
			return fmt.Errorf("données : listes explicites non null requises")
		}
		seen := map[string]bool{}
		for _, value := range list.values {
			if !containsTrust(list.known, value) || seen[value] {
				return fmt.Errorf("données : valeurs connues et uniques requises")
			}
			seen[value] = true
		}
	}
	if p.Resources == nil {
		return fmt.Errorf("données : listes explicites non null requises")
	}
	resources := map[string]bool{}
	for _, rule := range p.Resources {
		if !trustGroupLabelValid(rule.Resource) || resources[rule.Resource] || !containsTrust([]string{"public", "internal", "confidential", "restricted"}, rule.Classification) {
			return fmt.Errorf("données : règles de ressources explicites et uniques requises")
		}
		resources[rule.Resource] = true
	}
	return nil
}
func trustDataAdmission(st *trustState, operation string, cmd TrustCommand) error {
	deny := func() error {
		return &CommandError{Code: "authorization_required", Message: "données : contexte ou chemin non autorisé ; aucune donnée conservée"}
	}
	if operation != "request" {
		if cmd.DataContext != nil {
			return deny()
		}
		return nil
	}
	p := effectiveTrustDataPolicy(st)
	if !bool(p.Enabled) {
		if cmd.DataContext != nil {
			return deny()
		}
		return nil
	}
	return trustConnectorDataContextAllowed(p, cmd.DataContext, cmd.Resource, cmd.Connector)
}
func trustDataContextAllowed(p TrustDataPolicy, c *TrustDataContext, resource string) error {
	return trustDataContextAllowedPath(p, c, resource, "local-ledger", "simulated-settlement")
}
func trustConnectorDataContextAllowed(p TrustDataPolicy, c *TrustDataContext, resource string, selection *TrustConnectorSelection) error {
	if trustConnectorID(selection) == "local-access-simulation" {
		return trustDataContextAllowedPath(p, c, resource, "local-access-state", "simulated-access-grant")
	}
	return trustDataContextAllowed(p, c, resource)
}
func trustDataContextAllowedPath(p TrustDataPolicy, c *TrustDataContext, resource, destination, tool string) error {
	deny := func() error {
		return &CommandError{Code: "authorization_required", Message: "données : contexte ou chemin non autorisé ; aucune donnée conservée"}
	}
	if !bool(p.Enabled) {
		if c == nil {
			return nil
		}
		return deny()
	}
	if c == nil || !containsTrust(p.Classifications, c.Classification) || !containsTrust(p.Destinations, c.Destination) || !containsTrust(p.Providers, c.Provider) || !containsTrust(p.Tools, c.Tool) {
		return deny()
	}
	classified := false
	for _, rule := range p.Resources {
		if rule.Resource == resource && rule.Classification == c.Classification {
			classified = true
			break
		}
	}
	if !classified {
		return deny()
	}
	if (c.Classification != "public" && c.Classification != "internal") || c.Destination != destination || c.Provider != "swarm" || c.Tool != tool {
		return deny()
	}
	return nil
}
func trustDataActionAdmission(st *trustState, operation string, cmd TrustCommand) error {
	if operation != "approve" && operation != "commit" {
		return nil
	}
	a, ok := st.Actions[trustActionTarget(cmd)]
	if !ok {
		return nil
	}
	if operation == "commit" && (a.State == "committed" || a.State == "reconciled" || a.State == "unknown") {
		return nil
	}
	return trustConnectorDataContextAllowed(effectiveTrustDataPolicy(st), a.DataContext, a.Resource, a.Connector)
}
