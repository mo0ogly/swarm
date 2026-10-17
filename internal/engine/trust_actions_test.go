//go:build linux

package engine

import (
	"bytes"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

const (
	trustAdminToken     = "admin-token-0123456789"
	trustRequesterToken = "requester-token-012345"
	trustApproverToken  = "approver-token-0123456"
	trustEffectToken    = "effect-token-0123456789"
)

func trustFixture(t *testing.T) (*Store, time.Time) {
	t.Helper()
	s, err := openStore(t.TempDir(), true)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = s.db.Close() })
	now := time.Date(2026, 10, 14, 12, 0, 0, 0, time.UTC)
	change := TrustConfigChange{Schema: 1, EventID: "config-1", ExpectedRevision: 0, Policy: TrustPolicy{MandateTTLSeconds: 3600, UnknownReconcileSeconds: 0, EvidenceRetentionDays: 365, RequireSeparation: true}, Reason: "configuration initiale du pilote", Identities: []TrustIdentityInput{
		{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}},
		{Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}},
		{Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}},
		{Subject: "effect-service", Token: trustEffectToken, Integrity: 2, Roles: []string{"effect"}},
	}}
	if _, err = s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	return s, now
}

func trustCmd(event string) TrustCommand { return TrustCommand{Schema: 1, EventID: event} }

func createMandate(t *testing.T, s *Store, now time.Time, id string, expires time.Time) TrustMandate {
	t.Helper()
	c := trustCmd("mandate-" + id)
	c.MandateID = id
	c.Subject = "alice"
	c.Actions = []string{"settle"}
	c.Resource = "invoice/acme"
	c.MaxAmount = 50000
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.StartsAt = now.Add(-time.Minute).Format(time.RFC3339)
	c.ExpiresAt = expires.Format(time.RFC3339)
	v, err := s.trustOperate("mandate-create", c, trustAdminToken, now)
	if err != nil {
		t.Fatal(err)
	}
	return v.(TrustMandate)
}

func createAction(t *testing.T, s *Store, now time.Time, event, mandate, key string, amount int64) TrustAction {
	t.Helper()
	c := trustCmd(event)
	c.MandateID = mandate
	c.Resource = "invoice/acme"
	c.Amount = amount
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.IdempotencyKey = key
	v, err := s.trustOperate("request", c, trustRequesterToken, now)
	if err != nil {
		t.Fatal(err)
	}
	a, ok := v.(TrustAction)
	if !ok {
		t.Fatalf("demande refusée: %#v", v)
	}
	return a
}

func assertTrustApprovalDigest(t *testing.T) {
	t.Helper()
	s, at := trustFixture(t)
	createMandate(t, s, at, "approval-digest", at.Add(time.Hour))
	a := createAction(t, s, at, "digest-first", "approval-digest", "digest-key", 100)
	other := createAction(t, s, at, "digest-second", "approval-digest", "digest-other-key", 100)
	for i, digest := range []string{"", strings.Repeat("0", 64), other.ContentHash, a.ContentHash} {
		c := trustCmd(fmt.Sprintf("digest-approval-%d", i))
		c.ActionID, c.MandateID, c.Amount, c.Currency, c.Beneficiary = a.ID, a.MandateID, a.Amount, a.Currency, a.Beneficiary
		if digest != "" {
			raw, _ := json.Marshal(c)
			var fields map[string]json.RawMessage
			if err := json.Unmarshal(raw, &fields); err != nil {
				t.Fatal(err)
			}
			fields["content_digest"], _ = json.Marshal(digest)
			raw, _ = json.Marshal(fields)
			if err := strict(raw, &c); err != nil {
				t.Fatal(err)
			}
		}
		got, err := s.trustOperate("approve", c, trustApproverToken, at)
		if err != nil {
			t.Fatal(err)
		}
		view, err := s.trustView()
		if err != nil {
			t.Fatal(err)
		}
		state := view.(map[string]any)
		if i < 3 {
			denied, ok := got.(map[string]any)
			if !ok || denied["state"] != "denied" || state["actions"].(map[string]TrustAction)[a.ID] != a || ledgerCount(t, s) != 0 {
				t.Fatalf("missing/foreign approval digest admitted (%d): %#v", i, got)
			}
		} else if approved, ok := got.(TrustAction); !ok || approved.State != "approved" || approved.ContentHash != digest {
			t.Fatalf("exact approval digest refused: %#v", got)
		}
	}
}

func assertTrustIgnoredClaimsReplay(t *testing.T) {
	t.Helper()
	s, at := trustFixture(t)
	createMandate(t, s, at, "ignored-claims", at.Add(time.Hour))
	c := trustCmd("ignored-claims-request")
	c.Actor, c.Integrity = "admin", 3
	c.MandateID, c.Resource, c.Amount, c.Currency, c.Beneficiary, c.IdempotencyKey = "ignored-claims", "invoice/acme", 100, "EUR", "ACME", "ignored-key"
	c.original, _ = json.Marshal(c)
	first, err := s.trustOperate("request", c, trustRequesterToken, at)
	if err != nil {
		t.Fatal(err)
	}
	path, _, _ := s.trustPaths()
	before, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	for _, variant := range []struct {
		actor     string
		integrity int
	}{{"", 0}, {"forged", -99}, {"admin", 3}} {
		replay := c
		replay.Actor, replay.Integrity = variant.actor, variant.integrity
		got, err := s.trustOperate("request", replay, trustRequesterToken, at)
		firstRaw, firstErr := json.Marshal(first)
		gotRaw, gotErr := json.Marshal(got)
		if err != nil || firstErr != nil || gotErr != nil || !bytes.Equal(firstRaw, gotRaw) {
			t.Fatalf("ignored claims changed event replay: %#v %v", got, err)
		}
		after, err := os.ReadFile(path)
		if err != nil || !bytes.Equal(before, after) {
			t.Fatal("ignored claims replay mutated history", err)
		}
		if _, err = s.trustOperate("request", replay, trustApproverToken, at); err == nil || commandFailure(err).Code != "authorization_required" {
			t.Fatal("ignored claims changed authority", err)
		}
	}
	changed := c
	changed.Amount++
	if _, err := s.trustOperate("request", changed, trustRequesterToken, at); err == nil || commandFailure(err).Code != "event_conflict" {
		t.Fatal("business change accepted for same event", err)
	}
	exported, err := s.trustExport()
	if err != nil || !bytes.Equal(exported.Originals[hash(c.original)], c.original) {
		t.Fatal("ignored claims original lost", err)
	}
	if _, err = s.withTrustLock(true, func(st *trustState) (any, error) {
		st.EventDigests[c.EventID] = trustLegacyCommandDigest("request", c)
		return nil, nil
	}); err != nil {
		t.Fatal(err)
	}
	before, err = os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	legacy := c
	legacy.Actor, legacy.Integrity = "", 0
	if _, err = s.trustOperate("request", legacy, trustRequesterToken, at); err != nil {
		t.Fatal("legacy original did not resolve ignored claims", err)
	}
	after, err := os.ReadFile(path)
	if err != nil || !bytes.Equal(before, after) {
		t.Fatal("legacy digest was rewritten", err)
	}
	if _, err = s.withTrustLock(true, func(st *trustState) (any, error) { delete(st.Originals, hash(c.original)); return nil, nil }); err != nil {
		t.Fatal(err)
	}
	before, err = os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.trustOperate("request", legacy, trustRequesterToken, at); err == nil || commandFailure(err).Code != "event_conflict" {
		t.Fatal("legacy normalization accepted without its original", err)
	}
	if _, err = s.trustOperate("request", c, trustRequesterToken, at); err != nil {
		t.Fatal("unchanged legacy event no longer replays", err)
	}
	after, err = os.ReadFile(path)
	if err != nil || !bytes.Equal(before, after) {
		t.Fatal("legacy original gap was hidden or changed", err)
	}
}

func assertTrustRequestKeyConflict(t *testing.T) {
	t.Helper()
	for _, terminal := range []bool{false, true} {
		s, at := trustFixture(t)
		createMandate(t, s, at, "request-key", at.Add(time.Hour))
		a := createAction(t, s, at, "key-first", "request-key", "one-key", 100)
		if terminal {
			a = approveAction(t, s, at, "key-approve", a)
			c := trustCmd("key-effect")
			c.ActionID = a.ID
			if _, err := s.trustOperate("commit", c, trustEffectToken, at); err != nil {
				t.Fatal(err)
			}
		}
		view, _ := s.trustView()
		before, _ := json.Marshal(view.(map[string]any)["actions"])
		c := trustCmd("key-conflict")
		c.MandateID, c.Resource, c.Amount, c.Currency, c.Beneficiary, c.IdempotencyKey = a.MandateID, a.Resource, a.Amount+1, a.Currency, a.Beneficiary, a.IdempotencyKey
		if _, err := s.trustOperate("request", c, trustRequesterToken, at); err == nil || commandFailure(err).Code != "idempotency_conflict" {
			t.Fatal("request accepted different content with the same key", err)
		}
		view, _ = s.trustView()
		after, _ := json.Marshal(view.(map[string]any)["actions"])
		if !bytes.Equal(before, after) || len(view.(map[string]any)["actions"].(map[string]TrustAction)) != 1 {
			t.Fatal("request conflict changed actions")
		}
		c.EventID, c.Amount = "key-same-content", a.Amount
		got, err := s.trustOperate("request", c, trustRequesterToken, at)
		if err != nil || got.(TrustAction).ID != a.ID {
			t.Fatal("same key/content did not return original action", got, err)
		}
	}
	s, at := trustFixture(t)
	createMandate(t, s, at, "key-concurrency", at.Add(time.Hour))
	type result struct {
		value any
		err   error
	}
	results := make(chan result, 2)
	for i := 0; i < 2; i++ {
		go func(i int) {
			c := trustCmd(fmt.Sprintf("key-race-%d", i))
			c.MandateID, c.Resource, c.Amount, c.Currency, c.Beneficiary, c.IdempotencyKey = "key-concurrency", "invoice/acme", int64(100+i), "EUR", "ACME", "concurrent-key"
			v, err := s.trustOperate("request", c, trustRequesterToken, at)
			results <- result{v, err}
		}(i)
	}
	admitted, conflicts := 0, 0
	for i := 0; i < 2; i++ {
		r := <-results
		if r.err == nil {
			if _, ok := r.value.(TrustAction); !ok {
				t.Fatal(r.value)
			}
			admitted++
		} else if commandFailure(r.err).Code == "idempotency_conflict" {
			conflicts++
		} else {
			t.Fatal(r.err)
		}
	}
	view, _ := s.trustView()
	if admitted != 1 || conflicts != 1 || len(view.(map[string]any)["actions"].(map[string]TrustAction)) != 1 || ledgerCount(t, s) != 0 {
		t.Fatal("concurrent conflicting requests were not serialized", admitted, conflicts)
	}
}

func approveAction(t *testing.T, s *Store, now time.Time, event string, a TrustAction) TrustAction {
	t.Helper()
	c := trustCmd(event)
	c.Resource = a.ID
	c.MandateID = a.MandateID
	c.Amount = a.Amount
	c.Currency = a.Currency
	c.Beneficiary = a.Beneficiary
	c.ContentHash = a.ContentHash
	v, err := s.trustOperate("approve", c, trustApproverToken, now)
	if err != nil {
		t.Fatal(err)
	}
	approved, ok := v.(TrustAction)
	if !ok {
		t.Fatalf("accord refusé: %#v", v)
	}
	return approved
}

func ledgerCount(t *testing.T, s *Store) int {
	t.Helper()
	v, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	return len(v.(map[string]any)["ledger"].([]TrustLedgerEntry))
}

func TestActionsConfianceT2IdentityMandateApprovalAndBiba(t *testing.T) {
	s, now := trustFixture(t)
	createMandate(t, s, now, "mandate-main", now.Add(time.Hour))
	missing := trustCmd("missing-auth")
	missing.MandateID = "mandate-main"
	missing.Resource = "invoice/acme"
	missing.Amount = 1000
	missing.Currency = "EUR"
	missing.Beneficiary = "ACME"
	missing.IdempotencyKey = "key-missing"
	if _, err := s.trustOperate("request", missing, "", now); commandFailure(err).Code != "authorization_required" {
		t.Fatalf("auth absente non refusée: %v", err)
	}
	forged := missing
	forged.EventID = "request-good"
	forged.Actor = "admin"
	forged.Integrity = 3
	forged.IdempotencyKey = "key-good"
	v, err := s.trustOperate("request", forged, trustRequesterToken, now)
	if err != nil {
		t.Fatal(err)
	}
	a := v.(TrustAction)
	if a.Requester != "alice" {
		t.Fatalf("actor forgé accepté: %+v", a)
	}
	wrong := trustCmd("approval-wrong")
	wrong.Resource = a.ID
	wrong.MandateID = a.MandateID
	wrong.Amount = a.Amount + 1
	wrong.Currency = a.Currency
	wrong.Beneficiary = a.Beneficiary
	wrong.ContentHash = a.ContentHash
	v, err = s.trustOperate("approve", wrong, trustApproverToken, now)
	if err != nil {
		t.Fatal(err)
	}
	if v.(map[string]any)["state"] != "denied" || ledgerCount(t, s) != 0 {
		t.Fatalf("accord dérivé a muté le métier: %#v", v)
	}
	self := wrong
	self.EventID = "approval-self"
	self.Amount = a.Amount
	v, err = s.trustOperate("approve", self, trustRequesterToken, now)
	if err != nil {
		t.Fatal(err)
	}
	if v.(map[string]any)["state"] != "denied" {
		t.Fatalf("auto-approbation acceptée: %#v", v)
	}
	a = approveAction(t, s, now, "approval-good", a)
	commit := trustCmd("commit-good")
	commit.Resource = a.ID
	if _, err = s.trustOperate("commit", commit, trustEffectToken, now); err != nil {
		t.Fatal(err)
	}
	if ledgerCount(t, s) != 1 {
		t.Fatal("effet autorisé absent")
	}
	for _, tc := range []struct {
		name  string
		setup func() string
	}{
		{"absent", func() string { return "missing-mandate" }},
		{"expired", func() string {
			createMandate(t, s, now, "mandate-expired", now.Add(-time.Second))
			return "mandate-expired"
		}},
		{"revoked", func() string {
			createMandate(t, s, now, "mandate-revoked", now.Add(time.Hour))
			r := trustCmd("revoke")
			r.MandateID = "mandate-revoked"
			if _, e := s.trustOperate("mandate-revoke", r, trustAdminToken, now); e != nil {
				t.Fatal(e)
			}
			return "mandate-revoked"
		}},
	} {
		t.Run(tc.name, func(t *testing.T) {
			before := ledgerCount(t, s)
			c := trustCmd("refusal-" + tc.name)
			c.MandateID, c.Resource, c.Amount, c.Currency, c.Beneficiary, c.IdempotencyKey = tc.setup(), "invoice/acme", 100, "EUR", "ACME", "key-"+tc.name
			v, e := s.trustOperate("request", c, trustRequesterToken, now)
			if e != nil {
				t.Fatal(e)
			}
			if v.(map[string]any)["state"] != "denied" || ledgerCount(t, s) != before {
				t.Fatalf("refus %s incorrect: %#v", tc.name, v)
			}
		})
	}
	for _, op := range []string{"direct-write", "elevate-integrity"} {
		c := trustCmd("deny-" + op)
		c.Resource = "official-policy"
		v, e := s.trustOperate(op, c, trustRequesterToken, now)
		if e != nil {
			t.Fatal(e)
		}
		if v.(map[string]any)["state"] != "denied" || ledgerCount(t, s) != 1 {
			t.Fatalf("contournement accepté: %s %#v", op, v)
		}
	}
}

func TestActionsConfianceT2IdempotencyRestartAndReconciliation(t *testing.T) {
	assertTrustRequestKeyConflict(t)
	s, now := trustFixture(t)
	createMandate(t, s, now, "mandate-recovery", now.Add(time.Hour))
	a := approveAction(t, s, now, "approve-recovery", createAction(t, s, now, "request-recovery", "mandate-recovery", "invoice-42", 4200))
	c := trustCmd("commit-lost")
	c.Resource = a.ID
	c.LoseResponse = true
	v, err := s.trustOperate("commit", c, trustEffectToken, now)
	if err != nil {
		t.Fatal(err)
	}
	if v.(TrustAction).State != "unknown" || ledgerCount(t, s) != 1 {
		t.Fatalf("réponse perdue incorrecte: %#v", v)
	}
	root := s.root
	_ = s.db.Close()
	reopened, err := openStore(root, false)
	if err != nil {
		t.Fatal(err)
	}
	s = reopened
	defer s.db.Close()
	r := trustCmd("reconcile-after-restart")
	r.Resource = a.ID
	v, err = s.trustOperate("reconcile", r, trustEffectToken, now.Add(time.Minute))
	if err != nil {
		t.Fatal(err)
	}
	if v.(TrustAction).State != "reconciled" || ledgerCount(t, s) != 1 {
		t.Fatalf("réconciliation incorrecte: %#v", v)
	}
	replay := trustCmd("commit-replay")
	replay.Resource = a.ID
	if _, err = s.trustOperate("commit", replay, trustEffectToken, now.Add(2*time.Minute)); err != nil || ledgerCount(t, s) != 1 {
		t.Fatalf("rejeu exact non idempotent: %v", err)
	}
	conflict := trustCmd("request-conflict")
	conflict.MandateID, conflict.Resource, conflict.Amount, conflict.Currency, conflict.Beneficiary, conflict.IdempotencyKey = "mandate-recovery", "invoice/acme", 4300, "EUR", "ACME", "invoice-42"
	if _, err = s.trustOperate("request", conflict, trustRequesterToken, now); err == nil || commandFailure(err).Code != "idempotency_conflict" || ledgerCount(t, s) != 1 {
		t.Fatalf("conflit de contenu non sûr: %v", err)
	}
	c2 := approveAction(t, s, now, "approve-concurrent", createAction(t, s, now, "request-concurrent", "mandate-recovery", "invoice-43", 4400))
	var wg sync.WaitGroup
	errs := make(chan error, 2)
	for i := 0; i < 2; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			cmd := trustCmd(fmt.Sprintf("commit-concurrent-%d", i))
			cmd.Resource = c2.ID
			_, e := s.trustOperate("commit", cmd, trustEffectToken, now)
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
	if ledgerCount(t, s) != 2 {
		t.Fatalf("engagement concurrent dupliqué: %d", ledgerCount(t, s))
	}
}

func TestActionsConfianceT2ConfigProofAndInvalidInputs(t *testing.T) {
	assertTrustProofRejectsEmptyAndInvalidKeys(t)
	root := t.TempDir()
	s, err := openStore(root, true)
	if err != nil {
		t.Fatal(err)
	}
	defer s.db.Close()
	statePath, _, _ := s.trustPaths()
	change := TrustConfigChange{
		Schema: 1, EventID: "cfg-preview", ExpectedRevision: 0, Policy: defaultTrustPolicy(), Reason: "aperçu initial sans mutation",
		Identities: []TrustIdentityInput{
			{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}},
			{Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}},
			{Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}},
			{Subject: "effect-service", Token: trustEffectToken, Integrity: 2, Roles: []string{"effect"}},
		},
	}
	if _, err = s.trustConfig(change, true); err != nil {
		t.Fatal(err)
	}
	if _, err = os.Stat(statePath); !os.IsNotExist(err) {
		t.Fatalf("aperçu a muté l’état: %v", err)
	}
	change.EventID = "cfg-apply"
	if _, err = s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	replay, replayErr := s.trustConfig(change, false)
	replayJSON, marshalErr := json.Marshal(replay)
	var replayView struct {
		Revision int `json:"revision"`
	}
	decodeErr := json.Unmarshal(replayJSON, &replayView)
	if replayErr != nil || marshalErr != nil || decodeErr != nil || replayView.Revision != 1 {
		t.Fatalf("rejeu exact de configuration non idempotent: %s %v %v %v", replayJSON, replayErr, marshalErr, decodeErr)
	}
	stale := change
	stale.EventID = "cfg-stale"
	stale.Reason = "révision ancienne explicitement refusée"
	if _, err = s.trustConfig(stale, false); commandFailure(err).Code != "revision_conflict" {
		t.Fatalf("révision ancienne acceptée: %v", err)
	}
	invalidResource := trustCmd("invalid-resource")
	invalidResource.MandateID, invalidResource.Subject, invalidResource.Actions = "invalid-path", "alice", []string{"settle"}
	invalidResource.Resource, invalidResource.MaxAmount, invalidResource.Currency, invalidResource.Beneficiary = "../outside", 100, "EUR", "ACME"
	if _, err = s.trustOperate("mandate-create", invalidResource, trustAdminToken, time.Now().UTC()); err == nil {
		t.Fatal("ressource traversante acceptée")
	}
	raw, err := os.ReadFile(statePath)
	if err != nil {
		t.Fatal(err)
	}
	for _, secret := range []string{trustAdminToken, trustRequesterToken, trustApproverToken, trustEffectToken} {
		if bytes.Contains(raw, []byte(secret)) {
			t.Fatal("secret présent dans l’état")
		}
	}
	now := time.Date(2026, 10, 14, 12, 0, 0, 0, time.UTC)
	createMandate(t, s, now, "proof-mandate", now.Add(time.Hour))
	a := approveAction(t, s, now, "proof-approve", createAction(t, s, now, "proof-request", "proof-mandate", "proof-key", 100))
	c := trustCmd("proof-commit")
	c.Resource = a.ID
	if _, err = s.trustOperate("commit", c, trustEffectToken, now); err != nil {
		t.Fatal(err)
	}
	export, err := s.trustExport()
	if err != nil {
		t.Fatal(err)
	}
	anchor, err := s.trustAnchor()
	if err != nil {
		t.Fatal(err)
	}
	_, _, keyPath := s.trustPaths()
	keyRaw, err := os.ReadFile(keyPath)
	if err != nil {
		t.Fatal(err)
	}
	key := strings.TrimSpace(string(keyRaw))
	if _, err = hex.DecodeString(key); err != nil {
		t.Fatal(err)
	}
	if err = verifyTrustProof(export, anchor, key); err != nil {
		t.Fatal(err)
	}
	encoded, _ := json.Marshal(export)
	for _, secret := range []string{trustAdminToken, trustRequesterToken, key} {
		if bytes.Contains(encoded, []byte(secret)) {
			t.Fatal("secret présent dans l’export")
		}
	}
	tampered := export
	tampered.Evidence = append([]TrustEvidence(nil), export.Evidence...)
	tampered.Evidence[0].Reason = "altéré"
	if verifyTrustProof(tampered, anchor, key) == nil {
		t.Fatal("altération non détectée")
	}
	omitted := export
	omitted.Evidence = omitted.Evidence[:len(omitted.Evidence)-1]
	if verifyTrustProof(omitted, anchor, key) == nil {
		t.Fatal("omission non détectée")
	}
	if len(export.Evidence) > 1 {
		reordered := export
		reordered.Evidence = append([]TrustEvidence(nil), export.Evidence...)
		reordered.Evidence[0], reordered.Evidence[1] = reordered.Evidence[1], reordered.Evidence[0]
		if verifyTrustProof(reordered, anchor, key) == nil {
			t.Fatal("réordonnancement non détecté")
		}
	}
}

func TestActionsConfianceT2HTTPAuthenticationAndPersistence(t *testing.T) {
	s, _ := trustFixture(t)
	now := time.Now().UTC()
	createMandate(t, s, now, "http-mandate", now.Add(time.Hour))
	h := newWebHandler(s, "localhost:18787", "csrf-test-token")
	c := trustCmd("http-request")
	c.Actor = "admin"
	c.Integrity = 3
	c.MandateID = "http-mandate"
	c.Resource = "invoice/acme"
	c.Amount = 100
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.IdempotencyKey = "http-key"
	raw, _ := json.Marshal(c)
	request := func(token string) *httptest.ResponseRecorder {
		r := httptest.NewRequest(http.MethodPost, "http://localhost:18787/api/v1/trust-actions?action=request", bytes.NewReader(raw))
		r.Host = "localhost:18787"
		r.Header.Set("Origin", "http://localhost:18787")
		r.Header.Set("X-Swarm-CSRF", "csrf-test-token")
		r.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte("localhost:18787"))[:12], Value: "csrf-test-token"})
		if token != "" {
			r.Header.Set("Authorization", "Bearer "+token)
		}
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		return w
	}
	if w := request(""); w.Code != http.StatusForbidden {
		t.Fatalf("API sans identité: %d %s", w.Code, w.Body.String())
	}
	w := request(trustRequesterToken)
	if w.Code != http.StatusOK || !strings.Contains(w.Body.String(), `"requester":"alice"`) {
		t.Fatalf("API authentifiée: %d %s", w.Code, w.Body.String())
	}
	root := s.root
	_ = s.db.Close()
	reopened, err := openStore(root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	v, err := reopened.trustView()
	if err != nil {
		t.Fatal(err)
	}
	if len(v.(map[string]any)["actions"].(map[string]TrustAction)) != 1 {
		t.Fatal("action API non persistée après redémarrage")
	}
	if _, err = os.Stat(filepath.Join(root, ".swarm", "trust-actions.json")); err != nil {
		t.Fatal(err)
	}
}

func assertTrustProofRejectsEmptyAndInvalidKeys(t *testing.T) {
	t.Helper()
	empty := TrustProofExport{Schema: 2, Simulation: true, Originals: map[string][]byte{}, Evidence: []TrustEvidence{}}
	for _, key := range []string{" ", "00", strings.Repeat("00", 31), strings.Repeat("00", 32), strings.Repeat("00", 33)} {
		decoded, _ := hex.DecodeString(strings.TrimSpace(key))
		mac := hmac.New(sha256.New, decoded)
		mac.Write([]byte("0:"))
		anchor := TrustProofAnchor{Schema: 1, Sequence: 0, Hash: "", Signature: hex.EncodeToString(mac.Sum(nil))}
		c := trustCmd("empty-proof")
		c.Export, c.Anchor, c.VerificationKey = &empty, &anchor, key
		if got, err := verifyTrustCommand(c); err == nil {
			t.Fatalf("empty proof and invalid anchor accepted with key length %d: %#v", len(decoded), got)
		}
	}
	s, at := trustFixture(t)
	createMandate(t, s, at, "key-shape-mandate", at.Add(time.Hour))
	exported, err := s.trustExport()
	if err != nil {
		t.Fatal(err)
	}
	anchor, err := s.trustAnchor()
	if err != nil {
		t.Fatal(err)
	}
	for _, size := range []int{0, 1, 31, 33} {
		key := make([]byte, size)
		mac := hmac.New(sha256.New, key)
		fmt.Fprintf(mac, "%d:%s", anchor.Sequence, anchor.Hash)
		forged := anchor
		forged.Signature = hex.EncodeToString(mac.Sum(nil))
		if err = verifyTrustProof(exported, forged, hex.EncodeToString(key)); err == nil {
			t.Fatal("wrong verification-key size accepted", size)
		}
	}
	_, _, keyPath := s.trustPaths()
	key, err := os.ReadFile(keyPath)
	if err != nil {
		t.Fatal(err)
	}
	if len(strings.TrimSpace(string(key))) != 64 {
		t.Fatal("generated key is not the documented 32-byte key")
	}
	if err = verifyTrustProof(exported, anchor, string(key)); err != nil {
		t.Fatal("valid complete proof rejected", err)
	}
}
