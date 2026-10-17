//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"errors"
	"reflect"
	"testing"
	"time"
)

func TestAgentIdentityIssuanceReceiptSurvivesReopen(t *testing.T) {
	f := issuerFixture(t)
	permit := issuerMint(t, f)
	cp, err := f.store.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), f.token, permit, nil)
	if err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(f.store.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	var body []byte
	if err = reopened.db.QueryRow(`SELECT body FROM agent_identity_publications WHERE head_digest=?`, cp.headDigest).Scan(&body); err != nil {
		t.Fatal(err)
	}
	var head map[string]json.RawMessage
	if err = json.Unmarshal(body, &head); err != nil {
		t.Fatal(err)
	}
	receipt := head["issuance_receipt"]
	if len(receipt) == 0 || bytes.Equal(receipt, []byte("null")) {
		t.Fatal("published human authorization lost its issuance receipt after reopen")
	}
	if bytes.Contains(receipt, []byte(f.token)) {
		t.Fatal("raw session token persisted in issuance receipt")
	}
	var record agentIdentityIssuanceReceipt
	if err = strict(receipt, &record); err != nil || !reflect.DeepEqual(&record, permit.issuer.receipt()) {
		t.Fatal("receipt lost exact authority/session/dates: ", err)
	}
	if string(head["schema_version"]) != "2" {
		t.Fatal("human publication did not require receipt-bearing head format")
	}
	if _, _, err = reopened.readAgentIdentityRegistry(registryContext(t), cp); err != nil {
		t.Fatal("receipt-bearing publication is unreadable: ", err)
	}
}

func TestAgentIdentityIssuanceReceiptValidityAndOwnership(t *testing.T) {
	at := time.Date(2026, 10, 17, 9, 0, 0, 0, time.UTC)
	a := agentIdentityIssuerAuthority{projectRoot: t.TempDir(), principal: agentIdentityPrincipal{"alice", "human", "https://oidc.invalid", "employee-1"},
		scope: agentIdentityScope{"work", 1, hash([]byte("plan"))}, trustRevision: 2, enrollmentDigest: hash([]byte("controller enrollment fixture")),
		permissions: []agentIdentityPermission{{"agent.create", "task"}}, delegable: []agentIdentityPermission{{"agent.create", "task"}},
		notBefore: at.Add(-time.Hour), expiresAt: at.Add(time.Hour)}
	p := &agentIdentityPublicationIssuerProof{authority: a, tokenDigest: trustTokenHash("session fixture"), issuedAt: at,
		sessionExpiresAt: at.Add(10 * time.Minute), authorizedUntil: at.Add(10 * time.Minute)}
	head := agentIdentityRegistryHead{Schema: 2, ProjectDigest: hash([]byte(a.projectRoot)), Publisher: a.principal, Scope: a.scope,
		TrustRevision: 2, Revision: 1, SnapshotDigest: hash([]byte("snapshot")), EventID: "publish", Roots: []string{"root"}, Issuance: p.receipt()}
	if !head.valid() {
		t.Fatal("valid historical receipt rejected")
	}
	// The historical validator deliberately has no current clock: expiry is not
	// a reason to rewrite or erase a previously authorized publication.
	owned := head.Issuance.owned()
	owned.Permissions[0].Resource, owned.Delegable[0].Resource = "changed", "changed"
	if head.Issuance.Permissions[0].Resource != "task" || a.permissions[0].Resource != "task" || a.delegable[0].Resource != "task" {
		t.Fatal("receipt retained a mutable alias")
	}
	variants := []struct {
		name string
		edit func(*agentIdentityRegistryHead)
	}{
		{"stripped", func(h *agentIdentityRegistryHead) { h.Issuance = nil }},
		{"legacy-with-receipt", func(h *agentIdentityRegistryHead) { h.Schema = 1 }},
		{"unknown-head-schema", func(h *agentIdentityRegistryHead) { h.Schema = 3 }},
		{"unknown-receipt-schema", func(h *agentIdentityRegistryHead) { h.Issuance.Schema = 2 }},
		{"project", func(h *agentIdentityRegistryHead) { h.Issuance.ProjectDigest = hash([]byte("other")) }},
		{"principal", func(h *agentIdentityRegistryHead) { h.Issuance.Principal.ID = "bob" }},
		{"issuer", func(h *agentIdentityRegistryHead) { h.Issuance.Principal.Issuer = "https://other.invalid" }},
		{"subject", func(h *agentIdentityRegistryHead) { h.Issuance.Principal.Subject = "employee-2" }},
		{"non-human", func(h *agentIdentityRegistryHead) {
			h.Issuance.Principal.Kind = "service"
			h.Publisher = h.Issuance.Principal
		}},
		{"work", func(h *agentIdentityRegistryHead) { h.Issuance.Scope.WorkID = "other" }},
		{"plan-generation", func(h *agentIdentityRegistryHead) { h.Issuance.Scope.PlanVersion++ }},
		{"plan-digest", func(h *agentIdentityRegistryHead) { h.Issuance.Scope.PlanDigest = hash([]byte("other")) }},
		{"trust-revision", func(h *agentIdentityRegistryHead) { h.Issuance.TrustRevision++ }},
		{"enrollment-reference", func(h *agentIdentityRegistryHead) { h.Issuance.EnrollmentDigest = "unverified" }},
		{"session-reference", func(h *agentIdentityRegistryHead) { h.Issuance.SessionDigest = "raw-token" }},
		{"no-permissions", func(h *agentIdentityRegistryHead) { h.Issuance.Permissions = nil }},
		{"duplicate-permissions", func(h *agentIdentityRegistryHead) {
			h.Issuance.Permissions = append(h.Issuance.Permissions, h.Issuance.Permissions[0])
		}},
		{"wildcard", func(h *agentIdentityRegistryHead) { h.Issuance.Permissions[0].Resource = "*" }},
		{"delegation-exceeds-permissions", func(h *agentIdentityRegistryHead) { h.Issuance.Delegable[0].Resource = "other" }},
		{"duplicate-delegation", func(h *agentIdentityRegistryHead) {
			h.Issuance.Delegable = append(h.Issuance.Delegable, h.Issuance.Delegable[0])
		}},
		{"no-start", func(h *agentIdentityRegistryHead) { h.Issuance.NotBefore = time.Time{} }},
		{"empty-interval", func(h *agentIdentityRegistryHead) { h.Issuance.ExpiresAt = h.Issuance.NotBefore }},
		{"no-issuance", func(h *agentIdentityRegistryHead) { h.Issuance.IssuedAt = time.Time{} }},
		{"before-enrollment", func(h *agentIdentityRegistryHead) { h.Issuance.IssuedAt = h.Issuance.NotBefore.Add(-time.Nanosecond) }},
		{"at-expiry", func(h *agentIdentityRegistryHead) { h.Issuance.IssuedAt = h.Issuance.AuthorizedUntil }},
		{"no-session-expiry", func(h *agentIdentityRegistryHead) { h.Issuance.SessionExpiresAt = time.Time{} }},
		{"shorter-than-minimum", func(h *agentIdentityRegistryHead) {
			h.Issuance.AuthorizedUntil = h.Issuance.AuthorizedUntil.Add(-time.Nanosecond)
		}},
		{"exceeds-session", func(h *agentIdentityRegistryHead) {
			h.Issuance.AuthorizedUntil = h.Issuance.AuthorizedUntil.Add(time.Nanosecond)
		}},
		{"exceeds-enrollment", func(h *agentIdentityRegistryHead) { h.Issuance.ExpiresAt = at.Add(time.Minute) }},
	}
	for _, variant := range variants {
		t.Run(variant.name, func(t *testing.T) {
			candidate := head
			candidate.Issuance = head.Issuance.owned()
			variant.edit(&candidate)
			if candidate.valid() {
				t.Fatal("invalid issuance receipt accepted")
			}
		})
	}
	legacy := head
	legacy.Schema, legacy.Issuance = 1, nil
	if !legacy.valid() {
		t.Fatal("existing private controller publication became unreadable")
	}
	// Either the enrollment or the session may be the tighter bound.
	head.Issuance.ExpiresAt, head.Issuance.AuthorizedUntil = at.Add(time.Minute), at.Add(time.Minute)
	if !head.valid() {
		t.Fatal("enrollment-bounded receipt rejected")
	}
	// A backwards wall clock cannot use a permit before its issuance. Validate
	// the caller at each tested instant, without database or timing sleeps.
	permit := agentIdentityRegistryPermit{base: agentIdentityRegistryCheckpoint{projectRoot: a.projectRoot}, scope: a.scope, issuer: p}
	for _, instant := range []time.Time{at.Add(-time.Nanosecond), at, p.authorizedUntil} {
		caller := agentIdentityCaller{projectRoot: a.projectRoot, actor: agentIdentityActor{Principal: a.principal}, trustRevision: a.trustRevision,
			verifiedAt: instant, expiresAt: &p.sessionExpiresAt}
		err := validateAgentIdentityPublicationIssuer(permit, caller, "session fixture", instant)
		if instant.Equal(at) && err != nil || !instant.Equal(at) && identityPlanFailure(err).Code != "agent_identity_issuer_authority_stale" {
			t.Fatal("publication did not respect issuance/expiry boundary: ", err)
		}
	}
}

func TestAgentIdentityIssuanceReceiptRecoveryAtBothBoundaries(t *testing.T) {
	for _, stage := range []string{"intent_persisted", "sqlite_committed"} {
		t.Run(stage, func(t *testing.T) {
			f := issuerFixture(t)
			// Preserve an existing schema-1 controller publication, then advance
			// through the human producer to schema 2. Recovery must verify both.
			previous, err := f.store.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), f.token, f.permit, nil)
			if err != nil {
				t.Fatal(err)
			}
			f.snapshot.Revision++
			f.permit.base, f.permit.eventID = previous, "human-after-controller"
			registryApproveFixture(t, &f.agentIdentityRegistryFixture)
			permit := issuerMint(t, f)
			interrupted := errors.New("receipt fixture interruption")
			_, err = f.store.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), f.token, permit, func(at string) error {
				if at == stage {
					return interrupted
				}
				return nil
			})
			if !errors.Is(err, interrupted) {
				t.Fatal("durability boundary not reached: ", err)
			}
			var expected []byte
			_, err = f.store.withTrustLock(true, func(st *trustState) (any, error) {
				pending := st.AgentIdentities[permit.base.workID].Pending
				if pending == nil || pending.Head.Issuance == nil {
					t.Fatal("authorized intent lost its receipt")
				}
				expected, err = json.Marshal(pending.Head)
				session := st.OIDCSessions[trustTokenHash(f.token)]
				session.ExpiresAt = time.Now().UTC().Add(-time.Second)
				st.OIDCSessions[trustTokenHash(f.token)] = session
				return nil, err
			})
			if err != nil {
				t.Fatal(err)
			}
			reopened, err := openStore(f.store.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			cp, err := reopened.recoverAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), permit)
			if err != nil {
				t.Fatal("exact historical receipt did not recover: ", err)
			}
			var body []byte
			if err = reopened.db.QueryRow(`SELECT body FROM agent_identity_publications WHERE head_digest=?`, cp.headDigest).Scan(&body); err != nil || !bytes.Equal(body, expected) {
				t.Fatal("recovery changed the original head/receipt: ", err)
			}
			if _, _, err = reopened.readAgentIdentityRegistry(registryContext(t), cp); err != nil {
				t.Fatal(err)
			}
			before := registryTrustBytes(t, reopened)
			if _, err = reopened.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), f.token, permit, nil); err == nil {
				t.Fatal("historical recovery renewed an expired human session")
			}
			registryAssertUnchanged(t, reopened, before, 2, 2)
		})
	}
}

func TestAgentIdentityIssuanceReceiptJournalTamperRejected(t *testing.T) {
	f := issuerFixture(t)
	permit := issuerMint(t, f)
	cp, err := f.store.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), f.token, permit, nil)
	if err != nil {
		t.Fatal(err)
	}
	var original []byte
	if err = f.store.db.QueryRow(`SELECT body FROM agent_identity_publications WHERE head_digest=?`, cp.headDigest).Scan(&original); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"removed", "coherent-reference-rewrite"} {
		t.Run(name, func(t *testing.T) {
			var head agentIdentityRegistryHead
			if err := strict(original, &head); err != nil {
				t.Fatal(err)
			}
			if name == "removed" {
				head.Issuance = nil
			} else {
				head.Issuance.EnrollmentDigest = hash([]byte("forged enrollment reference"))
				if !head.valid() {
					t.Fatal("fixture must pass structural validation to exercise checkpoint binding")
				}
			}
			body, err := json.Marshal(head)
			if err != nil {
				t.Fatal(err)
			}
			if _, err = f.store.db.Exec(`UPDATE agent_identity_publications SET body=? WHERE head_digest=?`, body, cp.headDigest); err != nil {
				t.Fatal(err)
			}
			if _, _, err = f.store.readAgentIdentityRegistry(registryContext(t), cp); identityPlanFailure(err).Code != "agent_identity_registry_history_changed" {
				t.Fatal("modified receipt was not rejected by protected history: ", err)
			}
		})
	}
}
