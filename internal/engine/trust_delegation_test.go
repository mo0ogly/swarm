//go:build linux

package engine

import (
	"fmt"
	"reflect"
	"testing"
	"time"
)

func TestActionsConfianceT3DelegationConfigurationRejectsNegativeDepth(t *testing.T) {
	s, _ := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.Delegation = &TrustDelegationPolicy{MaxDepth: -1}
	if _, err := s.trustConfig(c, true); err == nil {
		t.Fatal("negative delegation depth accepted")
	}
}

const trustCarolToken = "carol-delegation-token-012345"
const trustDaveToken = "dave-delegation-token-0123456"

func delegationFixture(t *testing.T, depth int) (*Store, time.Time) {
	t.Helper()
	s, when := trustFixture(t)
	c := lifecycleChange(t, "alice", false, 1)
	c.Identities = append(c.Identities, TrustIdentityInput{Subject: "carol", Token: trustCarolToken, Integrity: 1, Roles: []string{"requester"}}, TrustIdentityInput{Subject: "dave", Token: trustDaveToken, Integrity: 1, Roles: []string{"requester"}})
	c.Delegation = &TrustDelegationPolicy{MaxDepth: depth}
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	return s, when
}

func delegationCommand(id, parent, subject string, when time.Time) TrustCommand {
	c := trustCmd("create-" + id)
	c.MandateID = id
	c.ParentMandateID = parent
	c.Subject = subject
	c.Actions = []string{"settle"}
	c.Resource = "invoice/acme"
	c.MaxAmount = 1000
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.StartsAt = when.Format(time.RFC3339)
	c.ExpiresAt = when.Add(30 * time.Minute).Format(time.RFC3339)
	return c
}

func createDelegatedMandate(t *testing.T, s *Store, c TrustCommand, when time.Time) TrustMandate {
	t.Helper()
	v, err := s.trustOperate("mandate-create", c, trustAdminToken, when)
	if err != nil {
		t.Fatal(err)
	}
	m, ok := v.(TrustMandate)
	if !ok {
		t.Fatal("delegation denied", v)
	}
	return m
}

func delegatedRequest(id, mandate string) TrustCommand {
	c := trustCmd(id)
	c.MandateID = mandate
	c.Resource = "invoice/acme"
	c.Amount = 100
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.IdempotencyKey = id
	return c
}

func TestActionsConfianceT3DelegationChainEffectPersistenceAndRevocation(t *testing.T) {
	s, when := delegationFixture(t, 2)
	root := createMandate(t, s, when, "root", when.Add(time.Hour))
	child := createDelegatedMandate(t, s, delegationCommand("child", "root", "carol", when), when)
	grand := createDelegatedMandate(t, s, delegationCommand("grand", "child", "dave", when), when)
	if child.ParentHash != root.ContentHash || grand.ParentHash != child.ContentHash || child.Delegator != "admin" {
		t.Fatal("parent digest or authenticated issuer lost")
	}
	v, err := s.trustOperate("request", delegatedRequest("delegate-request", "grand"), trustDaveToken, when)
	if err != nil {
		t.Fatal(err)
	}
	a, ok := v.(TrustAction)
	if !ok || a.Requester != "dave" {
		t.Fatal("delegated request denied", v)
	}
	approveAction(t, s, when, "delegate-approve", a)
	commit := trustCmd("delegate-commit")
	commit.ActionID = a.ID
	v, err = s.trustOperate("commit", commit, trustEffectToken, when)
	if err != nil || v.(TrustAction).State != "committed" {
		t.Fatal("delegated effect failed", err)
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	st, err := reopened.loadTrustState()
	if err != nil || st.Mandates["grand"].ParentHash != child.ContentHash {
		t.Fatal("chain lost after reopen", err)
	}
	revoke := trustCmd("revoke-root")
	revoke.MandateID = "root"
	if _, err = reopened.trustOperate("mandate-revoke", revoke, trustAdminToken, when); err != nil {
		t.Fatal(err)
	}
	denied, err := reopened.trustOperate("request", delegatedRequest("after-parent-revocation", "grand"), trustDaveToken, when)
	if err != nil || denied.(map[string]any)["reason"] != "mandat parent révoqué" {
		t.Fatal("ancestor revocation did not block child", denied, err)
	}
	rec := trustCmd("delegate-reconcile")
	rec.ActionID = a.ID
	if _, err = reopened.trustOperate("reconcile", rec, trustEffectToken, when); err != nil {
		t.Fatal("committed effect no longer reconcilable", err)
	}
	st, err = reopened.loadTrustState()
	if err != nil || len(st.Ledger) != 1 {
		t.Fatal("revocation erased or duplicated effect", err)
	}
}

func TestActionsConfianceT3DelegationRejectsAmplification(t *testing.T) {
	cases := map[string]func(*TrustCommand){"amount": func(c *TrustCommand) { c.MaxAmount = 50001 }, "resource": func(c *TrustCommand) { c.Resource = "invoice/other" }, "currency": func(c *TrustCommand) { c.Currency = "USD" }, "beneficiary": func(c *TrustCommand) { c.Beneficiary = "ATTACKER" }, "actions": func(c *TrustCommand) { c.Actions = []string{"settle", "other"} }, "start": func(c *TrustCommand) { c.StartsAt = "2026-10-14T11:00:00Z" }, "expiry": func(c *TrustCommand) { c.ExpiresAt = "2026-10-14T15:00:00Z" }, "missing-subject": func(c *TrustCommand) { c.Subject = "unknown" }, "missing-parent": func(c *TrustCommand) { c.ParentMandateID = "unknown" }}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			s, when := delegationFixture(t, 2)
			createMandate(t, s, when, "root", when.Add(time.Hour))
			c := delegationCommand("rejected", "root", "carol", when)
			mutate(&c)
			before, _ := s.loadTrustState()
			v, err := s.trustOperate("mandate-create", c, trustAdminToken, when)
			if err != nil {
				t.Fatal(err)
			}
			if v.(map[string]any)["state"] != "denied" {
				t.Fatal("amplification admitted")
			}
			after, _ := s.loadTrustState()
			if len(after.Mandates) != len(before.Mandates) || len(after.Actions) != 0 || len(after.Ledger) != 0 || len(after.Evidence) != len(before.Evidence)+1 {
				t.Fatal("denied delegation changed business state or lost proof")
			}
		})
	}
}

func TestActionsConfianceT3DelegationPreservesLegacyRootDigest(t *testing.T) {
	m := TrustMandate{ID: "legacy-root", Subject: "alice", Delegator: "admin", Actions: []string{"settle"}, Resource: "invoice/acme", MaxAmount: 1000, Currency: "EUR", Beneficiary: "ACME", Version: 1, PolicyRev: 1, Integrity: 2, StartsAt: "2026-10-14T12:00:00Z", ExpiresAt: "2026-10-14T13:00:00Z"}
	if got := trustMandateContentHash(m); got != "17749c31acf37938ebabe43b794adf5726fbc565905e6c0eb059afdae9ab5ad3" {
		t.Fatal("legacy root digest changed", got)
	}
}

func TestActionsConfianceT3DelegationDepthRequiresInteger(t *testing.T) {
	for _, value := range []string{`null`, `"2"`, `1.5`, `true`, `{}`, `[]`} {
		t.Run(value, func(t *testing.T) {
			var p TrustDelegationPolicy
			if strict([]byte(`{"max_depth":`+value+`}`), &p) == nil {
				t.Fatal("noninteger delegation depth accepted")
			}
		})
	}
}

func TestActionsConfianceT3DelegationDepthAndAuthority(t *testing.T) {
	for _, depth := range []int{0, 1} {
		t.Run(fmt.Sprint(depth), func(t *testing.T) {
			s, when := delegationFixture(t, depth)
			createMandate(t, s, when, "root", when.Add(time.Hour))
			c := delegationCommand("child", "root", "carol", when)
			v, err := s.trustOperate("mandate-create", c, trustRequesterToken, when)
			if err != nil || v.(map[string]any)["state"] != "denied" {
				t.Fatal("I1 endorsed an I2 delegation", v, err)
			}
			c.EventID = "admin-child-after-requester-refusal"
			if depth == 0 {
				v, err = s.trustOperate("mandate-create", c, trustAdminToken, when)
				if err != nil || v.(map[string]any)["reason"] != "délégations désactivées" {
					t.Fatal("disabled delegation admitted", v, err)
				}
				return
			}
			createDelegatedMandate(t, s, c, when)
			v, err = s.trustOperate("mandate-create", delegationCommand("too-deep", "child", "dave", when), trustAdminToken, when)
			if err != nil || v.(map[string]any)["reason"] != "profondeur de délégation dépassée" {
				t.Fatal("depth ceiling bypassed", v, err)
			}
		})
	}
}

func TestActionsConfianceT3DelegationAncestorRevocationBeforeApprovalOrEffect(t *testing.T) {
	for _, phase := range []string{"approve", "commit"} {
		t.Run(phase, func(t *testing.T) {
			s, when := delegationFixture(t, 1)
			createMandate(t, s, when, "root", when.Add(time.Hour))
			createDelegatedMandate(t, s, delegationCommand("child", "root", "carol", when), when)
			v, err := s.trustOperate("request", delegatedRequest("before-revoke", "child"), trustCarolToken, when)
			if err != nil {
				t.Fatal(err)
			}
			a := v.(TrustAction)
			if phase == "commit" {
				approveAction(t, s, when, "before-approve", a)
			}
			revoke := trustCmd("ancestor-revoke")
			revoke.MandateID = "root"
			if _, err = s.trustOperate("mandate-revoke", revoke, trustAdminToken, when); err != nil {
				t.Fatal(err)
			}
			before, _ := s.loadTrustState()
			command := trustCmd("after-revoke")
			command.ActionID = a.ID
			token := trustEffectToken
			if phase == "approve" {
				token = trustApproverToken
				command.MandateID = a.MandateID
				command.Amount = a.Amount
				command.Currency = a.Currency
				command.Beneficiary = a.Beneficiary
				command.ContentHash = a.ContentHash
			}
			v, err = s.trustOperate(phase, command, token, when)
			if err != nil || v.(map[string]any)["reason"] != "mandat parent révoqué" {
				t.Fatal("revoked ancestor admitted at boundary", v, err)
			}
			after, _ := s.loadTrustState()
			if !reflect.DeepEqual(before.Actions, after.Actions) || len(after.Ledger) != 0 {
				t.Fatal("revoked ancestor mutated protected state")
			}
		})
	}
}

func TestActionsConfianceT3DelegationMalformedChainsTerminate(t *testing.T) {
	s, when := delegationFixture(t, 10)
	createMandate(t, s, when, "root", when.Add(time.Hour))
	child := createDelegatedMandate(t, s, delegationCommand("child", "root", "carol", when), when)
	cases := map[string]func(*trustState, *TrustMandate){"parent-digest": func(st *trustState, m *TrustMandate) {
		m.ParentHash = "wrong"
		m.ContentHash = trustMandateContentHash(*m)
	}, "cycle": func(st *trustState, m *TrustMandate) {
		m.ParentID = m.ID
		m.ContentHash = trustMandateContentHash(*m)
		st.Mandates[m.ID] = *m
	}, "suspended-ancestor": func(st *trustState, m *TrustMandate) {
		for i := range st.Identities {
			if st.Identities[i].Subject == "alice" {
				st.Identities[i].Disabled = true
			}
		}
	}, "missing-parent": func(st *trustState, m *TrustMandate) { delete(st.Mandates, "root") }, "altered-child": func(st *trustState, m *TrustMandate) { m.MaxAmount++ }}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			st, _ := s.loadTrustState()
			m := child
			mutate(&st, &m)
			if reason := trustDelegatedMandateReason(&st, m, when); reason == "" {
				t.Fatal("malformed delegation accepted")
			}
		})
	}
}

func TestActionsConfianceT3DelegationConcurrentRevocationAndCommit(t *testing.T) {
	s, when := delegationFixture(t, 1)
	createMandate(t, s, when, "root", when.Add(time.Hour))
	createDelegatedMandate(t, s, delegationCommand("child", "root", "carol", when), when)
	v, err := s.trustOperate("request", delegatedRequest("race-request", "child"), trustCarolToken, when)
	if err != nil {
		t.Fatal(err)
	}
	a := v.(TrustAction)
	approveAction(t, s, when, "race-approve", a)
	commit := trustCmd("race-commit")
	commit.ActionID = a.ID
	revoke := trustCmd("race-revoke")
	revoke.MandateID = "root"
	start := make(chan struct{})
	out := make(chan error, 2)
	go func() { <-start; _, e := s.trustOperate("commit", commit, trustEffectToken, when); out <- e }()
	go func() { <-start; _, e := s.trustOperate("mandate-revoke", revoke, trustAdminToken, when); out <- e }()
	close(start)
	for i := 0; i < 2; i++ {
		if e := <-out; e != nil {
			t.Fatal(e)
		}
	}
	st, _ := s.loadTrustState()
	if len(st.Ledger) > 1 || st.Mandates["root"].RevokedAt == "" {
		t.Fatal("concurrent chain state inconsistent")
	}
	if len(st.Ledger) == 1 && st.Actions[a.ID].State != "committed" {
		t.Fatal("ledger/action mismatch")
	}
	if len(st.Ledger) == 0 && st.Actions[a.ID].State != "approved" {
		t.Fatal("revocation changed approved action")
	}
}

func TestActionsConfianceT3DelegationDefaultExpiryBoundedByParent(t *testing.T) {
	s, when := delegationFixture(t, 1)
	parent := createMandate(t, s, when, "short-parent", when.Add(20*time.Minute))
	c := delegationCommand("default-child", "short-parent", "carol", when)
	c.ExpiresAt = ""
	v, err := s.trustOperate("mandate-create", c, trustAdminToken, when)
	if err != nil {
		t.Fatal(err)
	}
	child, ok := v.(TrustMandate)
	if !ok || child.ExpiresAt != parent.ExpiresAt {
		t.Fatal("default child lifetime is not capped by parent", v)
	}
}
