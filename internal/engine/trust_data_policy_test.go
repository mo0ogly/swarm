//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"testing"
	"time"
)

func dataPolicyChange(t *testing.T, s *Store, policy any) TrustConfigChange {
	t.Helper()
	v, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	c := lifecycleChange(t, "alice", false, v.(map[string]any)["revision"].(int))
	raw, _ := json.Marshal(c)
	var fields map[string]json.RawMessage
	if err = json.Unmarshal(raw, &fields); err != nil {
		t.Fatal(err)
	}
	fields["data_policy"], err = json.Marshal(policy)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ = json.Marshal(fields)
	if err = strict(raw, &c); err != nil {
		t.Fatal("data policy contract missing", err)
	}
	return c
}

func TestActionsConfianceT4DataPolicyPublicContract(t *testing.T) {
	s, _ := trustFixture(t)
	p, _, _ := s.trustPaths()
	before, _ := os.ReadFile(p)
	c := dataPolicyChange(t, s, map[string]any{"enabled": false, "classifications": []string{}, "destinations": []string{}, "providers": []string{}, "tools": []string{}, "resources": []any{}})
	v, err := s.trustConfig(c, true)
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := v.(map[string]any)["data_policy"]; !ok {
		t.Fatal("preview omits effective data policy")
	}
	after, _ := os.ReadFile(p)
	if !bytes.Equal(before, after) {
		t.Fatal("preview mutated state")
	}
}
func activeDataPolicy() *TrustDataPolicy {
	return &TrustDataPolicy{Resources: []TrustDataResource{{Resource: "invoice/acme", Classification: "internal"}}, Enabled: true, Classifications: []string{"public", "internal", "confidential", "restricted"}, Destinations: []string{"local-ledger"}, Providers: []string{"swarm"}, Tools: []string{"simulated-settlement"}}
}
func allowedDataContext() *TrustDataContext {
	return &TrustDataContext{Classification: "internal", Destination: "local-ledger", Provider: "swarm", Tool: "simulated-settlement"}
}
func enableDataPolicy(t *testing.T, s *Store) {
	t.Helper()
	if _, err := s.trustConfig(dataPolicyChange(t, s, activeDataPolicy()), false); err != nil {
		t.Fatal(err)
	}
}
func dataRequest(event string) TrustCommand {
	c := trustCmd(event)
	c.MandateID = "data-mandate"
	c.Resource = "invoice/acme"
	c.Amount = 100
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.IdempotencyKey = event
	c.DataContext = allowedDataContext()
	return c
}
func TestActionsConfianceT4DataPolicyAdmissionRejectsWithoutCapture(t *testing.T) {
	cases := map[string]func(*TrustCommand){
		"absent":       func(c *TrustCommand) { c.DataContext = nil },
		"url":          func(c *TrustCommand) { c.DataContext.Destination = "https://example.invalid/leak" },
		"path":         func(c *TrustCommand) { c.DataContext.Destination = "../../private" },
		"symlink":      func(c *TrustCommand) { c.DataContext.Destination = "linked-secret" },
		"binary":       func(c *TrustCommand) { c.DataContext.Tool = "/bin/cat" },
		"provider":     func(c *TrustCommand) { c.DataContext.Provider = "external" },
		"confidential": func(c *TrustCommand) { c.DataContext.Classification = "confidential" },
		"restricted":   func(c *TrustCommand) { c.DataContext.Classification = "restricted" },
	}
	for name, change := range cases {
		t.Run(name, func(t *testing.T) {
			s, at := trustFixture(t)
			enableDataPolicy(t, s)
			createMandate(t, s, at, "data-mandate", at.Add(time.Hour))
			c := dataRequest("data-denied")
			c.Beneficiary = "SENSITIVE-SENTINEL"
			change(&c)
			p, _, _ := s.trustPaths()
			before, _ := os.ReadFile(p)
			if _, err := s.trustOperate("request", c, trustRequesterToken, at); commandFailure(err).Code != "authorization_required" {
				t.Fatal("path not refused", err)
			}
			after, _ := os.ReadFile(p)
			if !bytes.Equal(before, after) || bytes.Contains(after, []byte("SENSITIVE-SENTINEL")) {
				t.Fatal("denied data persisted")
			}
		})
	}
}
func TestActionsConfianceT4DataPolicyExactActionAndRecovery(t *testing.T) {
	s, at := trustFixture(t)
	enableDataPolicy(t, s)
	createMandate(t, s, at, "data-mandate", at.Add(time.Hour))
	c := dataRequest("data-request")
	v, err := s.trustOperate("request", c, trustRequesterToken, at)
	if err != nil {
		t.Fatal(err)
	}
	a := v.(TrustAction)
	c.DataContext.Classification = "public"
	c.EventID = "data-other-content"
	if _, err = s.trustOperate("request", c, trustRequesterToken, at); commandFailure(err).Code != "authorization_required" {
		t.Fatal("caller downgraded resource classification", err)
	}
	a = approveAction(t, s, at, "data-approve", a)
	commit := trustCmd("data-commit")
	commit.ActionID = a.ID
	commit.LoseResponse = true
	if _, err = s.trustOperate("commit", commit, trustEffectToken, at); err != nil {
		t.Fatal(err)
	}
	p, _, _ := s.trustPaths()
	raw, _ := os.ReadFile(p)
	var st trustState
	if strict(raw, &st) != nil || st.Actions[a.ID].DataContext.Classification != "internal" {
		t.Fatal("context not persisted")
	}
	c2 := dataPolicyChange(t, s, activeDataPolicy())
	c2.DataPolicy.Classifications = []string{}
	if _, err = s.trustConfig(c2, false); err != nil {
		t.Fatal(err)
	}
	r := trustCmd("data-reconcile")
	r.ActionID = a.ID
	v, err = s.trustOperate("reconcile", r, trustEffectToken, at.Add(time.Second))
	if err != nil || v.(TrustAction).State != "reconciled" {
		t.Fatal("committed effect lost", err)
	}
	v, err = s.trustOperate("commit", commit, trustEffectToken, at)
	if err != nil {
		t.Fatal("historical replay lost", err)
	}
	view, _ := s.trustView()
	if len(view.(map[string]any)["ledger"].([]TrustLedgerEntry)) != 1 {
		t.Fatal("effect duplicated")
	}
}

func TestActionsConfianceT4DataPolicyStrictValidation(t *testing.T) {
	for _, key := range []string{"enabled", "classifications", "destinations", "providers", "tools", "resources"} {
		for _, mode := range []string{"missing", "null"} {
			t.Run(key+"-"+mode, func(t *testing.T) {
				raw, _ := json.Marshal(activeDataPolicy())
				var fields map[string]json.RawMessage
				json.Unmarshal(raw, &fields)
				if mode == "missing" {
					delete(fields, key)
				} else {
					fields[key] = json.RawMessage(`null`)
				}
				raw, _ = json.Marshal(fields)
				var p TrustDataPolicy
				if strict(raw, &p) == nil {
					t.Fatal("incomplete policy accepted")
				}
			})
		}
	}
	for _, key := range []string{"classification", "destination", "provider", "tool"} {
		t.Run("context-"+key, func(t *testing.T) {
			raw, _ := json.Marshal(allowedDataContext())
			var fields map[string]json.RawMessage
			json.Unmarshal(raw, &fields)
			fields[key] = json.RawMessage(`null`)
			raw, _ = json.Marshal(fields)
			var c TrustDataContext
			if strict(raw, &c) == nil {
				t.Fatal("null context field accepted")
			}
		})
	}
	for _, edit := range []func(*TrustDataPolicy){func(p *TrustDataPolicy) { p.Tools = nil }, func(p *TrustDataPolicy) { p.Classifications = []string{"public", "public"} }, func(p *TrustDataPolicy) { p.Destinations = []string{"https://example.invalid"} }, func(p *TrustDataPolicy) { p.Providers = []string{"external"} }, func(p *TrustDataPolicy) { p.Tools = []string{"/bin/cat"} }} {
		s, _ := trustFixture(t)
		c := dataPolicyChange(t, s, activeDataPolicy())
		edit(c.DataPolicy)
		c.DataPolicy.Enabled = false
		p, _, _ := s.trustPaths()
		before, _ := os.ReadFile(p)
		if _, err := s.trustConfig(c, false); err == nil {
			t.Fatal("invalid typed policy accepted")
		}
		after, _ := os.ReadFile(p)
		if !bytes.Equal(before, after) {
			t.Fatal("invalid policy mutated")
		}
	}
}
func TestActionsConfianceT4DataPolicyChecksApprovalAndFirstEffect(t *testing.T) {
	for _, operation := range []string{"approve", "commit"} {
		t.Run(operation, func(t *testing.T) {
			s, at := trustFixture(t)
			createMandate(t, s, at, "data-mandate", at.Add(time.Hour))
			a := createAction(t, s, at, "old-request", "data-mandate", "old-key", 100)
			if operation == "commit" {
				a = approveAction(t, s, at, "old-approve", a)
			}
			enableDataPolicy(t, s)
			c := trustCmd("new-step")
			c.ActionID = a.ID
			c.ContentHash = a.ContentHash
			c.MandateID = a.MandateID
			c.Amount = a.Amount
			c.Currency = a.Currency
			c.Beneficiary = a.Beneficiary
			token := trustApproverToken
			if operation == "commit" {
				token = trustEffectToken
			}
			p, _, _ := s.trustPaths()
			before, _ := os.ReadFile(p)
			if _, err := s.trustOperate(operation, c, token, at); commandFailure(err).Code != "authorization_required" {
				t.Fatal("unclassified action proceeded", err)
			}
			after, _ := os.ReadFile(p)
			if !bytes.Equal(before, after) {
				t.Fatal("denied step mutated")
			}
		})
	}
}

func TestActionsConfianceT4DataPolicyPersistenceAndDisabled(t *testing.T) {
	s, at := trustFixture(t)
	c := dataRequest("disabled-data")
	p, _, _ := s.trustPaths()
	before, _ := os.ReadFile(p)
	if _, err := s.trustOperate("request", c, trustRequesterToken, at); commandFailure(err).Code != "authorization_required" {
		t.Fatal("disabled policy accepted labelled input", err)
	}
	after, _ := os.ReadFile(p)
	if !bytes.Equal(before, after) {
		t.Fatal("disabled admission mutated")
	}
	enableDataPolicy(t, s)
	v, _ := s.trustView()
	change := lifecycleChange(t, "alice", false, v.(map[string]any)["revision"].(int))
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	v, err = reopened.trustView()
	if err != nil || !bool(v.(map[string]any)["data_policy"].(TrustDataPolicy).Enabled) {
		t.Fatal("policy not preserved", err)
	}
	c.DataContext = allowedDataContext()
	c.ActionID = "not-present"
	before, _ = os.ReadFile(p)
	if _, err = s.trustOperate("approve", c, trustApproverToken, at); commandFailure(err).Code != "authorization_required" {
		t.Fatal("approval replaced context", err)
	}
	after, _ = os.ReadFile(p)
	if !bytes.Equal(before, after) {
		t.Fatal("substitution mutated")
	}
}

func TestActionsConfianceT4DataPolicyResourceClassification(t *testing.T) {
	for _, mode := range []string{"uncatalogued", "downgraded", "empty-rules"} {
		t.Run(mode, func(t *testing.T) {
			s, at := trustFixture(t)
			policy := activeDataPolicy()
			if mode == "uncatalogued" {
				policy.Resources[0].Resource = "other"
			}
			if mode == "downgraded" {
				policy.Resources[0].Classification = "restricted"
			}
			if mode == "empty-rules" {
				policy.Resources = []TrustDataResource{}
			}
			if _, err := s.trustConfig(dataPolicyChange(t, s, policy), false); err != nil {
				t.Fatal(err)
			}
			createMandate(t, s, at, "data-mandate", at.Add(time.Hour))
			c := dataRequest("classification-denied")
			p, _, _ := s.trustPaths()
			before, _ := os.ReadFile(p)
			if _, err := s.trustOperate("request", c, trustRequesterToken, at); commandFailure(err).Code != "authorization_required" {
				t.Fatal("resource classification not enforced", err)
			}
			after, _ := os.ReadFile(p)
			if !bytes.Equal(before, after) {
				t.Fatal("classification denial mutated state")
			}
		})
	}
	for _, rules := range [][]TrustDataResource{nil, {{Resource: "", Classification: "internal"}}, {{Resource: "invoice/acme", Classification: ""}}, {{Resource: "invoice/acme", Classification: "unknown"}}, {{Resource: "invoice/acme", Classification: "internal"}, {Resource: "invoice/acme", Classification: "public"}}} {
		s, _ := trustFixture(t)
		policy := activeDataPolicy()
		policy.Enabled = false
		policy.Resources = rules
		c := dataPolicyChange(t, s, activeDataPolicy())
		c.DataPolicy = policy
		p, _, _ := s.trustPaths()
		before, _ := os.ReadFile(p)
		if _, err := s.trustConfig(c, false); err == nil {
			t.Fatal("invalid resource policy accepted")
		}
		after, _ := os.ReadFile(p)
		if !bytes.Equal(before, after) {
			t.Fatal("invalid resource policy mutated state")
		}
	}
}
