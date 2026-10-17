//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"sync"
	"testing"
	"time"
)

func connectorFixtureAction(t *testing.T, s *Store, at time.Time, access bool) TrustAction {
	t.Helper()
	c := trustCmd("connector-mandate")
	c.MandateID = "connector-mandate"
	c.Subject = "alice"
	c.Resource = "invoice/acme"
	c.Actions = []string{"settle"}
	c.MaxAmount = 1000
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.StartsAt = at.Add(-time.Minute).Format(time.RFC3339)
	c.ExpiresAt = at.Add(time.Hour).Format(time.RFC3339)
	if access {
		c.Connector = &TrustConnectorSelection{ID: "local-access-simulation", Version: 1, Access: &TrustAccessParameters{Subject: "employee", Role: "reader"}}
		c.Actions = []string{"grant-access"}
		c.MaxAmount = 0
		c.Currency = ""
		c.Beneficiary = ""
	}
	if v, e := s.trustOperate("mandate-create", c, trustAdminToken, at); e != nil {
		t.Fatal(e)
	} else if _, ok := v.(TrustMandate); !ok {
		t.Fatalf("mandate refused: %v", v)
	}
	c.EventID = "connector-request"
	c.Amount = 100
	c.MaxAmount = 0
	c.Subject = ""
	c.IdempotencyKey = "connector-key"
	if access {
		c.Amount = 0
	}
	v, e := s.trustOperate("request", c, trustRequesterToken, at)
	if e != nil {
		t.Fatal(e)
	}
	a, ok := v.(TrustAction)
	if !ok {
		t.Fatalf("request refused: %v", v)
	}
	return a
}
func connectorApprove(t *testing.T, s *Store, at time.Time, a TrustAction) TrustAction {
	t.Helper()
	c := trustCmd("connector-approve")
	c.Connector = a.Connector
	c.ActionID = a.ID
	c.ContentHash = a.ContentHash
	c.MandateID = a.MandateID
	c.Amount = a.Amount
	c.Currency = a.Currency
	c.Beneficiary = a.Beneficiary
	v, e := s.trustOperate("approve", c, trustApproverToken, at)
	if e != nil {
		t.Fatal(e)
	}
	a, ok := v.(TrustAction)
	if !ok {
		t.Fatalf("approve refused: %v", v)
	}
	return a
}
func TestActionsConfianceT6ConnectorContract(t *testing.T) {
	for _, access := range []bool{false, true} {
		t.Run(fmt.Sprint(access), func(t *testing.T) {
			s, at := trustFixture(t)
			a := connectorApprove(t, s, at, connectorFixtureAction(t, s, at, access))
			c := trustCmd("connector-commit")
			c.ActionID = a.ID
			c.LoseResponse = true
			v, e := s.trustOperate("commit", c, trustEffectToken, at)
			if e != nil || v.(TrustAction).State != "unknown" {
				t.Fatalf("commit: %v %v", v, e)
			}
			c.EventID = "connector-retry"
			v, e = s.trustOperate("commit", c, trustEffectToken, at)
			if e != nil || v.(TrustAction).State != "unknown" {
				t.Fatal(v, e)
			}
			c.EventID = "connector-readback"
			c.LoseResponse = false
			v, e = s.trustOperate("reconcile", c, trustEffectToken, at)
			if e != nil || v.(TrustAction).State != "reconciled" {
				t.Fatal(v, e)
			}
			_, e = s.withTrustLock(false, func(st *trustState) (any, error) {
				if len(st.Ledger)+len(st.AccessLedger) != 1 || len(st.Receipts) != 1 {
					t.Fatal("effect duplicated")
				}
				r := st.Receipts[a.IdempotencyKey]
				if r.Intent == nil || r.Result == nil || r.Result.State != "reconciled" || r.Intent.ActionHash != a.ContentHash || r.Intent.MandateHash != st.Mandates[a.MandateID].ContentHash || !trustConnectorReadback(st, a, r) {
					t.Fatalf("invalid intent/result: %+v", r)
				}
				if access && len(st.Ledger) != 0 {
					t.Fatal("access changed billing")
				}
				return nil, nil
			})
			if e != nil {
				t.Fatal(e)
			}
		})
	}
}
func TestActionsConfianceT6ConnectorGuards(t *testing.T) {
	for _, access := range []bool{false, true} {
		t.Run(fmt.Sprint(access), func(t *testing.T) {
			s, at := trustFixture(t)
			a := connectorFixtureAction(t, s, at, access)
			t.Run("direct-bypass", func(t *testing.T) {
				_, e := s.withTrustLock(false, func(st *trustState) (any, error) {
					actor, e := findTrustIdentityAt(st, trustEffectToken, at)
					if e != nil {
						return nil, e
					}
					if trustConnectorCommit(st, a, actor, at, false) == nil || len(st.Ledger)+len(st.AccessLedger) != 0 {
						t.Fatal("unapproved intent executed")
					}
					return nil, nil
				})
				if e != nil {
					t.Fatal(e)
				}
			})
			a = connectorApprove(t, s, at, a)
			t.Run("wrong-role", func(t *testing.T) {
				_, e := s.withTrustLock(false, func(st *trustState) (any, error) {
					actor, e := findTrustIdentityAt(st, trustRequesterToken, at)
					if e != nil {
						return nil, e
					}
					if trustConnectorCommit(st, a, actor, at, false) == nil {
						t.Fatal("requester executed")
					}
					return nil, nil
				})
				if e != nil {
					t.Fatal(e)
				}
			})
			t.Run("content-drift", func(t *testing.T) {
				_, e := s.withTrustLock(false, func(st *trustState) (any, error) {
					actor, _ := findTrustIdentityAt(st, trustEffectToken, at)
					bad := a
					bad.Resource = "other"
					if trustConnectorCommit(st, bad, actor, at, false) == nil {
						t.Fatal("drift executed")
					}
					return nil, nil
				})
				if e != nil {
					t.Fatal(e)
				}
			})
			t.Run("concurrency", func(t *testing.T) {
				var wg sync.WaitGroup
				errs := make(chan error, 8)
				for i := 0; i < 8; i++ {
					wg.Add(1)
					go func(i int) {
						defer wg.Done()
						c := trustCmd(fmt.Sprintf("parallel-%d", i))
						c.ActionID = a.ID
						_, e := s.trustOperate("commit", c, trustEffectToken, at)
						errs <- e
					}(i)
				}
				wg.Wait()
				close(errs)
				for e := range errs {
					if e != nil {
						t.Fatal(e)
					}
				}
			})
			t.Run("readback-alteration", func(t *testing.T) {
				_, e := s.withTrustLock(false, func(st *trustState) (any, error) {
					r := st.Receipts[a.IdempotencyKey]
					if !trustConnectorReadback(st, a, r) {
						t.Fatal("good receipt refused")
					}
					if access {
						st.AccessLedger[0].Access.Role = "admin"
					} else {
						st.Ledger[0].Amount++
					}
					if trustConnectorReadback(st, a, r) {
						t.Fatal("altered effect accepted")
					}
					return nil, nil
				})
				if e != nil {
					t.Fatal(e)
				}
			})
		})
	}
}
func TestActionsConfianceT6ConnectorInvalid(t *testing.T) {
	for _, raw := range []string{
		`{"id":"production","version":1}`, `{"id":"local-access-simulation","version":2,"access":{"subject":"a","role":"b"}}`,
		`{"id":"local-access-simulation","version":1}`, `{"id":"local-ledger-simulation","version":1,"access":{"subject":"a","role":"b"}}`,
		`{"id":"local-access-simulation","version":1,"access":{"subject":"../a","role":"b"}}`,
	} {
		t.Run(raw, func(t *testing.T) {
			var selection TrustConnectorSelection
			if e := json.Unmarshal([]byte(raw), &selection); e != nil {
				t.Fatal(e)
			}
			if trustConnectorCommandValid("request", TrustCommand{Connector: &selection}) == nil {
				t.Fatal("invalid capability admitted")
			}
		})
	}
}
func TestActionsConfianceT6ConnectorStrictJSON(t *testing.T) {
	for _, raw := range []string{`{"connector":null}`, `{"connector":{"id":"local-access-simulation","version":1,"access":{"subject":null,"role":"reader"}}}`, `{"connector":{"id":"local-access-simulation","version":1,"capabilities":{"real":true}}}`, `{"connector":{"id":"local-access-simulation","version":1.5}}`} {
		t.Run(raw, func(t *testing.T) {
			var c TrustCommand
			err := strict([]byte(raw), &c)
			c.original = []byte(raw)
			if err == nil {
				err = trustConnectorCommandValid("request", c)
			}
			if err == nil {
				t.Fatal("invalid JSON admitted")
			}
		})
	}
}
func TestActionsConfianceT6ConnectorDataBinding(t *testing.T) {
	p := TrustDataPolicy{Enabled: true, Classifications: []string{"internal"}, Destinations: []string{"local-ledger", "local-access-state"}, Providers: []string{"swarm"}, Tools: []string{"simulated-settlement", "simulated-access-grant"}, Resources: []TrustDataResource{{Resource: "vault/model", Classification: "internal"}}}
	c := &TrustConnectorSelection{ID: "local-access-simulation", Version: 1, Access: &TrustAccessParameters{Subject: "member", Role: "reader"}}
	ctx := &TrustDataContext{Classification: "internal", Destination: "local-ledger", Provider: "swarm", Tool: "simulated-settlement"}
	if trustConnectorDataContextAllowed(p, ctx, "vault/model", c) == nil {
		t.Fatal("wrong destination admitted")
	}
	ctx.Destination = "local-access-state"
	ctx.Tool = "simulated-access-grant"
	if e := trustConnectorDataContextAllowed(p, ctx, "vault/model", c); e != nil {
		t.Fatal(e)
	}
	if trustConnectorDataContextAllowed(p, ctx, "vault/model", nil) == nil {
		t.Fatal("access context admitted for settlement")
	}
	ctx.Classification = "restricted"
	if trustConnectorDataContextAllowed(p, ctx, "vault/model", c) == nil {
		t.Fatal("sensitive provider qualified")
	}
}
func TestActionsConfianceT6ConnectorCrossKeyAndApproval(t *testing.T) {
	s, at := trustFixture(t)
	a := connectorFixtureAction(t, s, at, true)
	c := trustCmd("wrong-connector-approval")
	c.ActionID = a.ID
	c.MandateID = a.MandateID
	c.ContentHash = a.ContentHash
	v, e := s.trustOperate("approve", c, trustApproverToken, at)
	if e != nil || v.(map[string]any)["state"] != "denied" {
		t.Fatal("non-exact connector approval accepted", v, e)
	}
	createMandate(t, s, at, "ledger-mandate", at.Add(time.Hour))
	c = trustCmd("cross-connector-key")
	c.MandateID = "ledger-mandate"
	c.Resource = "invoice/acme"
	c.Amount = 100
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.IdempotencyKey = a.IdempotencyKey
	_, e = s.trustOperate("request", c, trustRequesterToken, at)
	if e == nil || commandFailure(e).Code != "idempotency_conflict" {
		t.Fatal("connector reset idempotency domain", e)
	}
}
