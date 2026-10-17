//go:build linux

package engine

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"
)

func TestActionsConfianceT2EmptyLedgerIsAnArrayWithoutMutation(t *testing.T) {
	s, _ := trustFixture(t)
	path := filepath.Join(s.root, ".swarm", "trust-actions.json")
	before, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	view, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	raw, err := json.Marshal(view)
	if err != nil {
		t.Fatal(err)
	}
	var public map[string]json.RawMessage
	if err = json.Unmarshal(raw, &public); err != nil {
		t.Fatal(err)
	}
	if string(public["ledger"]) != "[]" {
		t.Fatalf("empty ledger must be an array, got %s", public["ledger"])
	}
	after, err := os.ReadFile(path)
	if err != nil || !bytes.Equal(before, after) {
		t.Fatal("public read changed persisted state", err)
	}
}

func TestActionsConfianceT2PolicyHasNoArbitraryBusinessCeilings(t *testing.T) {
	for _, policy := range []TrustPolicy{
		{MandateTTLSeconds: 31536001, EvidenceRetentionDays: 365, RequireSeparation: true},
		{UnknownReconcileSeconds: 604801, EvidenceRetentionDays: 365, RequireSeparation: true},
		{EvidenceRetentionDays: 36501, RequireSeparation: true},
	} {
		if err := validateTrustPolicy(policy); err != nil {
			t.Errorf("representable operator setting denied by an arbitrary ceiling: %+v: %v", policy, err)
		}
	}
	for _, policy := range []TrustPolicy{
		{MandateTTLSeconds: -1, EvidenceRetentionDays: 365},
		{UnknownReconcileSeconds: -1, EvidenceRetentionDays: 365},
		{EvidenceRetentionDays: 0},
	} {
		if err := validateTrustPolicy(policy); err == nil {
			t.Errorf("invalid or overflowing policy accepted: %+v", policy)
		}
	}
	for _, field := range []string{"mandate_ttl_seconds", "unknown_reconcile_seconds"} {
		raw, err := json.Marshal(map[string]any{field: int64(^uint64(0)>>1)/int64(time.Second) + 1, "evidence_retention_days": 365})
		if err != nil {
			t.Fatal(err)
		}
		var policy TrustPolicy
		err = strict(raw, &policy)
		if err == nil {
			err = validateTrustPolicy(policy)
		}
		if err == nil {
			t.Fatalf("unrepresentable policy field admitted: %s", field)
		}
	}
}

func TestActionsConfianceT2ReconcileHonorsDelayWithoutAnotherEffect(t *testing.T) {
	s, at := trustFixture(t)
	if _, err := s.withTrustLock(true, func(st *trustState) (any, error) { st.Policy.UnknownReconcileSeconds = 3600; return nil, nil }); err != nil {
		t.Fatal(err)
	}
	createMandate(t, s, at, "delay-mandate", at.Add(2*time.Hour))
	action := approveAction(t, s, at, "delay-approve", createAction(t, s, at, "delay-request", "delay-mandate", "delay-key", 100))
	c := trustCmd("delay-commit")
	c.ActionID = action.ID
	c.LoseResponse = true
	if _, err := s.trustOperate("commit", c, trustEffectToken, at); err != nil {
		t.Fatal(err)
	}
	c = trustCmd("delay-early")
	c.ActionID = action.ID
	got, err := s.trustOperate("reconcile", c, trustEffectToken, at.Add(3599*time.Second))
	if err != nil {
		t.Fatal(err)
	}
	if state, ok := got.(map[string]any); !ok || state["state"] != "denied" {
		t.Fatalf("early reconcile admitted: %#v", got)
	}
	view, _ := s.trustView()
	if len(view.(map[string]any)["ledger"].([]TrustLedgerEntry)) != 1 || view.(map[string]any)["actions"].(map[string]TrustAction)[action.ID].State != "unknown" {
		t.Fatal("early refusal changed business state")
	}
	c = trustCmd("delay-on-time")
	c.ActionID = action.ID
	got, err = s.trustOperate("reconcile", c, trustEffectToken, at.Add(time.Hour))
	if err != nil || got.(TrustAction).State != "reconciled" {
		t.Fatal("due reconciliation refused", got, err)
	}
	view, _ = s.trustView()
	if len(view.(map[string]any)["ledger"].([]TrustLedgerEntry)) != 1 {
		t.Fatal("reconcile duplicated effect")
	}
}

func TestActionsConfianceT2HTTPVerifyAndStrictInput(t *testing.T) {
	s, at := trustFixture(t)
	createMandate(t, s, at, "verify-http", at.Add(time.Hour))
	exp, _ := s.trustExport()
	anchor, _ := s.trustAnchor()
	_, _, keyPath := s.trustPaths()
	key, _ := os.ReadFile(keyPath)
	h := newWebHandler(s, "localhost:18787", "csrf-test-token")
	call := func(action string, raw []byte) *httptest.ResponseRecorder {
		r := httptest.NewRequest(http.MethodPost, "http://localhost:18787/api/v1/trust-actions?action="+action, bytes.NewReader(raw))
		r.Host = "localhost:18787"
		r.Header.Set("Origin", "http://localhost:18787")
		r.Header.Set("X-Swarm-CSRF", "csrf-test-token")
		r.Header.Set("Authorization", "Bearer "+trustRequesterToken)
		r.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(r.Host))[:12], Value: "csrf-test-token"})
		w := httptest.NewRecorder()
		h.ServeHTTP(w, r)
		return w
	}
	c := trustCmd("http-proof")
	c.Export = &exp
	c.Anchor = &anchor
	c.VerificationKey = strings.TrimSpace(string(key))
	raw, _ := json.Marshal(c)
	if w := call("verify", raw); w.Code != http.StatusOK || !strings.Contains(w.Body.String(), `"verified":true`) {
		t.Fatal("HTTP proof verification parity missing", w.Code, w.Body.String())
	}
	statePath := filepath.Join(s.root, ".swarm/trust-actions.json")
	before, _ := os.ReadFile(statePath)
	for _, action := range []string{"verify", "request", "config-preview", "config-apply"} {
		if w := call(action, append(append([]byte{}, raw...), []byte(` {"unexpected":true}`)...)); w.Code == http.StatusOK {
			t.Fatal("trailing JSON accepted", action)
		}
	}
	after, _ := os.ReadFile(statePath)
	if !bytes.Equal(before, after) {
		t.Fatal("invalid HTTP input mutated state")
	}
}

func TestActionsConfianceT2HTTPRejectsTrailingJSONWithoutMutation(t *testing.T) {
	s, _ := trustFixture(t)
	at := time.Now().UTC()
	createMandate(t, s, at, "json-http", at.Add(time.Hour))
	h := newWebHandler(s, "localhost:18787", "csrf-test-token")
	c := trustCmd("http-json")
	c.MandateID = "json-http"
	c.Resource = "invoice/acme"
	c.Amount = 100
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	c.IdempotencyKey = "http-json-key"
	raw, _ := json.Marshal(c)
	statePath := filepath.Join(s.root, ".swarm/trust-actions.json")
	before, _ := os.ReadFile(statePath)
	r := httptest.NewRequest(http.MethodPost, "http://localhost:18787/api/v1/trust-actions?action=request", bytes.NewReader(append(raw, []byte(` {"unexpected":true}`)...)))
	r.Host = "localhost:18787"
	r.Header.Set("Origin", "http://localhost:18787")
	r.Header.Set("X-Swarm-CSRF", "csrf-test-token")
	r.Header.Set("Authorization", "Bearer "+trustRequesterToken)
	r.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(r.Host))[:12], Value: "csrf-test-token"})
	w := httptest.NewRecorder()
	h.ServeHTTP(w, r)
	after, _ := os.ReadFile(statePath)
	if w.Code == http.StatusOK || !bytes.Equal(before, after) {
		t.Fatal("extra HTTP JSON accepted or mutation occurred", w.Code, w.Body.String())
	}
}

func TestActionsConfianceT2ApprovalRejectsContradictoryProtectedResource(t *testing.T) {
	assertTrustApprovalDigest(t)
	assertTrustActionResourceGuards(t)
	s, at := trustFixture(t)
	createMandate(t, s, at, "exact-resource", at.Add(time.Hour))
	a := createAction(t, s, at, "resource-request", "exact-resource", "resource-key", 100)
	c := trustCmd("wrong-resource-approval")
	c.ActionID = a.ID
	c.Resource = "invoice/other"
	c.MandateID = a.MandateID
	c.Amount = a.Amount
	c.Currency = a.Currency
	c.Beneficiary = a.Beneficiary
	c.ContentHash = a.ContentHash
	got, err := s.trustOperate("approve", c, trustApproverToken, at)
	if err != nil {
		t.Fatal(err)
	}
	if result, ok := got.(map[string]any); !ok || result["state"] != "denied" || result["reason"] != "ressource explicite contradictoire" {
		t.Fatalf("contradictory approval resource admitted: %#v", got)
	}
	view, _ := s.trustView()
	if view.(map[string]any)["actions"].(map[string]TrustAction)[a.ID] != a {
		t.Fatal("contradictory approval changed action")
	}
	c.EventID = "exact-resource-approval"
	c.Resource = a.Resource
	c.ContentHash = a.ContentHash
	got, err = s.trustOperate("approve", c, trustApproverToken, at)
	if err != nil || got.(TrustAction).State != "approved" {
		t.Fatal("exact resource not approved", got, err)
	}
}

func TestActionsConfianceT2ApprovalCannotRegressEffectState(t *testing.T) {
	for _, state := range []string{"committed", "unknown", "reconciled"} {
		t.Run(state, func(t *testing.T) {
			s, at := trustFixture(t)
			createMandate(t, s, at, "terminal-mandate", at.Add(time.Hour))
			a := approveAction(t, s, at, "terminal-approve", createAction(t, s, at, "terminal-request", "terminal-mandate", "terminal-key", 100))
			c := trustCmd("terminal-commit")
			c.ActionID = a.ID
			c.LoseResponse = state != "committed"
			got, err := s.trustOperate("commit", c, trustEffectToken, at)
			if err != nil {
				t.Fatal(err)
			}
			if state == "reconciled" {
				c = trustCmd("terminal-reconcile")
				c.ActionID = a.ID
				got, err = s.trustOperate("reconcile", c, trustEffectToken, at)
				if err != nil {
					t.Fatal(err)
				}
			}
			before := got.(TrustAction)
			if before.State != state {
				t.Fatal("wrong fixture effect state", before.State)
			}
			c = trustCmd("terminal-reapproval")
			c.ActionID = a.ID
			c.Resource = a.Resource
			c.MandateID = a.MandateID
			c.Amount = a.Amount
			c.Currency = a.Currency
			c.Beneficiary = a.Beneficiary
			c.ContentHash = a.ContentHash
			got, err = s.trustOperate("approve", c, trustApproverToken, at)
			if err != nil {
				t.Fatal(err)
			}
			if result, ok := got.(map[string]any); !ok || result["state"] != "denied" || result["reason"] != "action n’attend plus d’approbation" {
				t.Fatalf("terminal effect regressed through approval: %#v", got)
			}
			view, _ := s.trustView()
			if view.(map[string]any)["actions"].(map[string]TrustAction)[a.ID] != before || len(view.(map[string]any)["ledger"].([]TrustLedgerEntry)) != 1 {
				t.Fatal("reapproval changed action or ledger")
			}
		})
	}
}

func TestActionsConfianceT2ConfigReplayRetainsOriginalAcrossLaterChanges(t *testing.T) {
	s, at := trustFixture(t)
	mandate := trustCmd("wide-amount-mandate")
	mandate.MandateID, mandate.Subject, mandate.Actions = "wide-amount", "alice", []string{"settle"}
	mandate.Resource, mandate.MaxAmount, mandate.Currency, mandate.Beneficiary = "invoice/wide", 9007199254740993, "EUR", "ACME"
	if _, err := s.trustOperate("mandate-create", mandate, trustAdminToken, at); err != nil {
		t.Fatal(err)
	}
	identities := []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}}
	first := TrustConfigChange{Schema: 1, EventID: "replay-first", ExpectedRevision: 1, Policy: defaultTrustPolicy(), Identities: identities, Reason: "retain first event response"}
	original, err := s.trustConfig(first, false)
	if err != nil {
		t.Fatal(err)
	}
	second := first
	second.EventID = "replay-second"
	second.ExpectedRevision = 2
	second.Policy.MandateTTLSeconds = 7200
	if _, err = s.trustConfig(second, false); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	statePath := filepath.Join(s.root, ".swarm/trust-actions.json")
	before, _ := os.ReadFile(statePath)
	replay, err := reopened.trustConfig(first, false)
	if err != nil {
		t.Fatal(err)
	}
	a, _ := json.Marshal(original)
	b, _ := json.Marshal(replay)
	if !bytes.Equal(a, b) || !bytes.Contains(b, []byte("9007199254740993")) {
		t.Fatalf("replay changed the original state or integer precision: original=%s replay=%s", a, b)
	}
	after, _ := os.ReadFile(statePath)
	if !bytes.Equal(before, after) {
		t.Fatal("replay mutated current state")
	}
	first.Policy.MandateTTLSeconds++
	if _, err = reopened.trustConfig(first, false); err == nil {
		t.Fatal("changed payload admitted for original event")
	}
	first.Policy.MandateTTLSeconds--
	if _, err = reopened.withTrustLock(true, func(st *trustState) (any, error) { delete(st.EventReplies, first.EventID); return nil, nil }); err != nil {
		t.Fatal(err)
	}
	before, _ = os.ReadFile(statePath)
	if _, err = reopened.trustConfig(first, false); err == nil {
		t.Fatal("missing historical response replaced with current state")
	}
	after, _ = os.ReadFile(statePath)
	if !bytes.Equal(before, after) {
		t.Fatal("unavailable original response mutated state")
	}
}

func TestActionsConfianceT2HTTPRequiresBearerAndAuthenticatesReplays(t *testing.T) {
	for _, header := range []string{trustRequesterToken, "Basic " + trustRequesterToken, "Bearer  " + trustRequesterToken, "Bearer ", "", "Bearer " + trustRequesterToken} {
		t.Run(header, func(t *testing.T) {
			s, at := trustFixture(t)
			createMandate(t, s, at, "header-mandate", at.Add(time.Hour))
			a := createAction(t, s, at, "header-existing", "header-mandate", "header-key", 100)
			c := trustCmd("header-existing")
			c.MandateID = a.MandateID
			c.Resource = a.Resource
			c.Amount = a.Amount
			c.Currency = a.Currency
			c.Beneficiary = a.Beneficiary
			c.IdempotencyKey = a.IdempotencyKey
			raw, _ := json.Marshal(c)
			h := newWebHandler(s, "localhost:18787", "csrf-header-token")
			r := httptest.NewRequest(http.MethodPost, "http://localhost:18787/api/v1/trust-actions?action=request", bytes.NewReader(raw))
			r.Host = "localhost:18787"
			r.Header.Set("Origin", "http://localhost:18787")
			r.Header.Set("X-Swarm-CSRF", "csrf-header-token")
			r.Header.Set("Authorization", header)
			r.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(r.Host))[:12], Value: "csrf-header-token"})
			statePath := filepath.Join(s.root, ".swarm/trust-actions.json")
			before, _ := os.ReadFile(statePath)
			w := httptest.NewRecorder()
			h.ServeHTTP(w, r)
			after, _ := os.ReadFile(statePath)
			if (w.Code == http.StatusOK) != (header == "Bearer "+trustRequesterToken) || !bytes.Equal(before, after) {
				t.Fatal("authorization did not enforce Bearer or mutated state", w.Code)
			}
			if _, err := s.trustOperate("request", c, "", at); err == nil {
				t.Fatal("CLI replay bypassed authentication")
			}
		})
	}
}

func TestActionsConfianceT2OriginalBytesPersistAndVerify(t *testing.T) {
	assertTrustAuthenticatedValidationAudit(t)
	s, _ := trustFixture(t)
	c := trustCmd("original-wire")
	c.MandateID = "original-wire"
	c.Subject = "alice"
	c.Actions = []string{"settle"}
	c.Resource = "invoice/original"
	c.MaxAmount = 100
	c.Currency = "EUR"
	c.Beneficiary = "ACMÉ <&>"
	raw, _ := json.MarshalIndent(c, "", "  ")
	raw = append(raw, '\n')
	h := newWebHandler(s, "localhost:18787", "csrf-original")
	r := httptest.NewRequest(http.MethodPost, "http://localhost:18787/api/v1/trust-actions?action=mandate-create", bytes.NewReader(raw))
	r.Host = "localhost:18787"
	r.Header.Set("Origin", "http://localhost:18787")
	r.Header.Set("X-Swarm-CSRF", "csrf-original")
	r.Header.Set("Authorization", "Bearer "+trustAdminToken)
	r.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(r.Host))[:12], Value: "csrf-original"})
	w := httptest.NewRecorder()
	h.ServeHTTP(w, r)
	if w.Code != http.StatusOK {
		t.Fatal("original command refused", w.Code, w.Body.String())
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	exp, err := reopened.trustExport()
	if err != nil {
		t.Fatal(err)
	}
	encoded, _ := json.Marshal(exp)
	var original struct {
		Originals map[string][]byte `json:"originals"`
	}
	if err = json.Unmarshal(encoded, &original); err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(original.Originals[hash(raw)], raw) {
		t.Fatal("exact incoming original absent after restart")
	}
	anchor, err := reopened.trustAnchor()
	if err != nil {
		t.Fatal(err)
	}
	_, _, kp := s.trustPaths()
	key, _ := os.ReadFile(kp)
	if err = verifyTrustProof(exp, anchor, strings.TrimSpace(string(key))); err != nil {
		t.Fatal(err)
	}
	for _, mode := range []string{"altered", "missing", "extra"} {
		t.Run(mode, func(t *testing.T) {
			var data map[string]any
			if err := json.Unmarshal(encoded, &data); err != nil {
				t.Fatal(err)
			}
			orig := data["originals"].(map[string]any)
			switch mode {
			case "altered":
				orig[hash(raw)] = "e30="
			case "missing":
				delete(orig, hash(raw))
			case "extra":
				orig[hash([]byte(`{}`))] = "e30="
			}
			badJSON, _ := json.Marshal(data)
			var bad TrustProofExport
			if err := json.Unmarshal(badJSON, &bad); err != nil {
				t.Fatal(err)
			}
			if verifyTrustProof(bad, anchor, strings.TrimSpace(string(key))) == nil {
				t.Fatal("unbound or missing original accepted")
			}
		})
	}
	for _, secret := range []string{trustAdminToken, trustRequesterToken, strings.TrimSpace(string(key))} {
		if bytes.Contains(encoded, []byte(secret)) {
			t.Fatal("credential exposed in original export")
		}
	}
	if _, err = reopened.withTrustLock(true, func(st *trustState) (any, error) { delete(st.Originals, hash(raw)); return nil, nil }); err != nil {
		t.Fatal(err)
	}
	view, err := reopened.trustView()
	if err != nil || view.(map[string]any)["originals_missing"] != 1 {
		t.Fatal("legacy gap is hidden", view, err)
	}
	statePath := filepath.Join(s.root, ".swarm/trust-actions.json")
	before, _ := os.ReadFile(statePath)
	if _, err = reopened.trustOperate("direct-write", trustCmd("after-gap"), trustRequesterToken, time.Now().UTC()); err == nil {
		t.Fatal("operation continued with absent originals")
	}
	after, _ := os.ReadFile(statePath)
	if !bytes.Equal(before, after) {
		t.Fatal("missing-original refusal mutated history")
	}
}

func TestActionsConfianceT2OriginalsCoverDecisionsAndRejectCredentialFields(t *testing.T) {
	assertTrustDeniedActionTarget(t)
	s, at := trustFixture(t)
	createMandate(t, s, at, "source-mandate", at.Add(time.Hour))
	a := approveAction(t, s, at, "source-approval", createAction(t, s, at, "source-request", "source-mandate", "source-key", 100))
	c := trustCmd("source-effect")
	c.ActionID = a.ID
	c.LoseResponse = true
	if _, err := s.trustOperate("commit", c, trustEffectToken, at); err != nil {
		t.Fatal(err)
	}
	c = trustCmd("source-reconcile")
	c.ActionID = a.ID
	if _, err := s.trustOperate("reconcile", c, trustEffectToken, at); err != nil {
		t.Fatal(err)
	}
	exp, _ := s.trustExport()
	raw, _ := json.Marshal(exp)
	var wire struct {
		Originals map[string][]byte `json:"originals"`
	}
	json.Unmarshal(raw, &wire)
	seen := map[string]bool{}
	for _, ev := range exp.Evidence {
		source := wire.Originals[ev.OriginalHash]
		if len(source) == 0 || hash(source) != ev.OriginalHash {
			t.Fatal("decision has no retained original", ev.Kind)
		}
		seen[ev.OriginalHash] = true
	}
	if len(seen) != len(exp.Evidence) {
		t.Fatal("different decisions reused another command's original")
	}
	statePath := filepath.Join(s.root, ".swarm/trust-actions.json")
	before, _ := os.ReadFile(statePath)
	c = trustCmd("secret-in-business")
	c.Identities = []TrustIdentityInput{{Token: "do-not-retain-this-credential"}}
	if _, err := s.trustOperate("direct-write", c, trustRequesterToken, at); err == nil {
		t.Fatal("credential-bearing business command admitted")
	}
	after, _ := os.ReadFile(statePath)
	if !bytes.Equal(before, after) {
		t.Fatal("invalid credential fields changed state")
	}
}

func TestActionsConfianceT2BusinessReplayPreservesIntegerPrecision(t *testing.T) {
	s, at := trustFixture(t)
	c := trustCmd("wide-business-replay")
	c.MandateID = "wide-business"
	c.Subject = "alice"
	c.Actions = []string{"settle"}
	c.Resource = "invoice/wide"
	c.MaxAmount = 9007199254740993
	c.Currency = "EUR"
	c.Beneficiary = "ACME"
	if _, err := s.trustOperate("mandate-create", c, trustAdminToken, at); err != nil {
		t.Fatal(err)
	}
	got, err := s.trustOperate("mandate-create", c, trustAdminToken, at)
	if err != nil {
		t.Fatal(err)
	}
	raw, err := json.Marshal(got)
	if err != nil {
		t.Fatal(err)
	}
	var view struct {
		MaxAmount int64 `json:"max_amount_minor"`
	}
	if err = json.Unmarshal(raw, &view); err != nil || view.MaxAmount != c.MaxAmount {
		t.Fatal("replay rounded original business response", string(raw), err)
	}
	r := trustCmd("wide-public-action")
	r.MandateID, r.Resource, r.Amount, r.Currency, r.Beneficiary, r.IdempotencyKey = c.MandateID, c.Resource, c.MaxAmount, c.Currency, c.Beneficiary, "wide-public-key"
	created, err := s.trustOperate("request", r, trustRequesterToken, at)
	if err != nil {
		t.Fatal(err)
	}
	public, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	encoded, err := json.Marshal(public)
	if err != nil {
		t.Fatal(err)
	}
	var browser struct {
		Amounts map[string]string `json:"amount_minor_text"`
	}
	if err = json.Unmarshal(encoded, &browser); err != nil || browser.Amounts[created.(TrustAction).ID] != "9007199254740993" {
		t.Fatal("public browser projection rounded the authoritative amount", string(encoded), err)
	}
}

func TestActionsConfianceT2RevocationBindsUpdatedMandate(t *testing.T) {
	s, at := trustFixture(t)
	first := createMandate(t, s, at, "revoke-digest", at.Add(time.Hour))
	c := trustCmd("revoke-digest-event")
	c.MandateID = first.ID
	c.Reason = "revocation requested by owner"
	got, err := s.trustOperate("mandate-revoke", c, trustAdminToken, at.Add(time.Minute))
	if err != nil {
		t.Fatal(err)
	}
	revoked := got.(TrustMandate)
	exp, _ := s.trustExport()
	last := exp.Evidence[len(exp.Evidence)-1]
	if revoked.RevokedAt == "" || revoked.Version != first.Version+1 || revoked.ContentHash == first.ContentHash || last.ContentHash != revoked.ContentHash {
		t.Fatal("revocation attested the old mandate content")
	}
}

func TestActionsConfianceT2StateBoundIncludesFinalNewline(t *testing.T) {
	assertTrustStateReadUsesOneDescriptor(t)
	for _, delta := range []int{-1, 0, 1} {
		t.Run(map[int]string{-1: "last-readable-byte", 0: "newline-overflow", 1: "body-overflow"}[delta], func(t *testing.T) {
			s, _ := trustFixture(t)
			p := filepath.Join(s.root, ".swarm/trust-actions.json")
			before, _ := os.ReadFile(p)
			_, err := s.withTrustLock(true, func(st *trustState) (any, error) {
				st.EventDigests["padding"] = hash([]byte("fixture retained input"))
				st.EventReplies["padding"] = json.RawMessage(`""`)
				raw, _ := json.MarshalIndent(st, "", "  ")
				n := trustStateMaxBytes + delta - len(raw)
				if n < 0 {
					t.Fatal("fixture too large")
				}
				st.EventReplies["padding"] = json.RawMessage(`"` + strings.Repeat("x", n) + `"`)
				raw, _ = json.MarshalIndent(st, "", "  ")
				if len(raw) != trustStateMaxBytes+delta {
					t.Fatal("wrong boundary fixture", len(raw))
				}
				return nil, nil
			})
			after, _ := os.ReadFile(p)
			if delta < 0 {
				if err != nil || len(after) != trustStateMaxBytes {
					t.Fatal("last representable state refused", err, len(after))
				}
				if _, err = s.loadTrustState(); err != nil {
					t.Fatal("accepted state cannot reopen", err)
				}
			} else if err == nil || !bytes.Equal(before, after) {
				t.Fatal("oversize state accepted or history mutated", len(after), err)
			}
		})
	}
}

func TestActionsConfianceT2ExampleCredentialsAreRejectedWithoutMutation(t *testing.T) {
	for _, placeholder := range []string{"remplacer-par-un-secret-admin", "replace-with-a-secret-admin"} {
		t.Run(placeholder, func(t *testing.T) {
			s, _ := trustFixture(t)
			p := filepath.Join(s.root, ".swarm/trust-actions.json")
			before, _ := os.ReadFile(p)
			change := TrustConfigChange{Schema: 1, EventID: "example-credential", ExpectedRevision: 1, Policy: defaultTrustPolicy(), Reason: "operator must enter private credentials", Identities: []TrustIdentityInput{{Subject: "admin", Token: placeholder, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}}}
			if _, err := s.trustConfig(change, false); err == nil {
				t.Fatal("public example credential accepted")
			}
			after, _ := os.ReadFile(p)
			if !bytes.Equal(before, after) {
				t.Fatal("invalid example credential mutated state")
			}
			if _, err := s.withTrustLock(true, func(st *trustState) (any, error) {
				for i := range st.Identities {
					if st.Identities[i].Subject == "admin" {
						st.Identities[i].TokenHash = trustTokenHash(placeholder)
						return nil, nil
					}
				}
				t.Fatal("fixture admin missing")
				return nil, nil
			}); err != nil {
				t.Fatal(err)
			}
			before, _ = os.ReadFile(p)
			if _, err := s.trustOperate("direct-write", trustCmd("legacy-example-login"), placeholder, time.Now().UTC()); err == nil {
				t.Fatal("previously stored example credential still authenticates")
			}
			after, _ = os.ReadFile(p)
			if !bytes.Equal(before, after) {
				t.Fatal("rejected legacy credential mutated history")
			}
		})
	}
}

func TestActionsConfianceT2ReplayBindsAuthenticatedAuthority(t *testing.T) {
	assertTrustIgnoredClaimsReplay(t)
	s, at := trustFixture(t)
	m := createMandate(t, s, at, "principal-bound", at.Add(time.Hour))
	c := trustCmd("principal-bound-request")
	c.MandateID = m.ID
	c.Resource = m.Resource
	c.Amount = 100
	c.Currency = m.Currency
	c.Beneficiary = m.Beneficiary
	c.IdempotencyKey = "principal-bound-key"
	if _, err := s.trustOperate("request", c, trustRequesterToken, at); err != nil {
		t.Fatal(err)
	}
	state, _, _ := s.trustPaths()
	before, err := os.ReadFile(state)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := s.trustOperate("request", c, trustApproverToken, at); err == nil || commandFailure(err).Code != "authorization_required" {
		t.Fatal("another authenticated principal replayed the original authority or lost the shared API/CLI authorization code", err)
	}
	after, err := os.ReadFile(state)
	if err != nil || !bytes.Equal(before, after) {
		t.Fatal("foreign replay mutated state", err)
	}
	if _, err := s.trustOperate("request", c, trustRequesterToken, at); err != nil {
		t.Fatal("same authority replay refused", err)
	}
}

func TestActionsConfianceT2ProofKeyRejectsSymlinkWithoutMutation(t *testing.T) {
	for _, op := range []string{"anchor", "config"} {
		t.Run(op, func(t *testing.T) {
			s, at := trustFixture(t)
			createMandate(t, s, at, "key-symlink", at.Add(time.Hour))
			state, _, key := s.trustPaths()
			before, err := os.ReadFile(state)
			if err != nil {
				t.Fatal(err)
			}
			target := filepath.Join(t.TempDir(), "known-key")
			known := []byte(strings.Repeat("a", 64) + "\n")
			if err = os.WriteFile(target, known, 0600); err != nil {
				t.Fatal(err)
			}
			if err = os.Remove(key); err != nil {
				t.Fatal(err)
			}
			if err = os.Symlink(target, key); err != nil {
				t.Fatal(err)
			}
			if op == "anchor" {
				_, err = s.trustAnchor()
			} else {
				_, err = s.trustConfig(TrustConfigChange{Schema: 1, EventID: "key-symlink-config", ExpectedRevision: 1, Policy: defaultTrustPolicy(), Reason: "reject a preexisting symlink without mutation", Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}}}, false)
			}
			if err == nil {
				t.Fatal("preexisting proof-key symlink accepted")
			}
			after, readErr := os.ReadFile(state)
			if readErr != nil || !bytes.Equal(before, after) {
				t.Fatal("rejected key changed state", readErr)
			}
			current, readErr := os.ReadFile(target)
			if readErr != nil || !bytes.Equal(current, known) {
				t.Fatal("rejected key changed target", readErr)
			}
		})
	}
}

func TestActionsConfianceT2ReplayAuthorityChangeAndLegacyGap(t *testing.T) {
	for _, mode := range []string{"changed-role", "changed-integrity", "legacy-missing-binding", "credential-rotation"} {
		t.Run(mode, func(t *testing.T) {
			s, at := trustFixture(t)
			m := createMandate(t, s, at, "authority-variants", at.Add(time.Hour))
			c := trustCmd("authority-variant-request")
			c.MandateID = m.ID
			c.Resource = m.Resource
			c.Amount = 100
			c.Currency = m.Currency
			c.Beneficiary = m.Beneficiary
			c.IdempotencyKey = "authority-variant-key"
			first, err := s.trustOperate("request", c, trustRequesterToken, at)
			if err != nil {
				t.Fatal(err)
			}
			original, _ := json.Marshal(first)
			token := trustRequesterToken
			_, err = s.withTrustLock(true, func(st *trustState) (any, error) {
				if mode == "legacy-missing-binding" {
					delete(st.EventAuthorities, c.EventID)
				}
				for i := range st.Identities {
					if st.Identities[i].Subject != "alice" {
						continue
					}
					switch mode {
					case "changed-role":
						st.Identities[i].Roles = []string{"approver"}
					case "changed-integrity":
						st.Identities[i].Integrity = 2
					case "credential-rotation":
						token = "rotated-requester-credential"
						st.Identities[i].TokenHash = trustTokenHash(token)
					}
				}
				return nil, nil
			})
			if err != nil {
				t.Fatal(err)
			}
			file, _, _ := s.trustPaths()
			before, err := os.ReadFile(file)
			if err != nil {
				t.Fatal(err)
			}
			got, err := s.trustOperate("request", c, token, at)
			if mode == "credential-rotation" {
				raw, e := json.Marshal(got)
				if err != nil || e != nil || string(raw) != string(original) {
					t.Fatal("same authority credential rotation changed original response", err, e)
				}
			} else if err == nil {
				t.Fatal("changed or absent historical authority admitted")
			}
			after, e := os.ReadFile(file)
			if e != nil || !bytes.Equal(before, after) {
				t.Fatal("replay changed persisted state", e)
			}
		})
	}
}

func TestActionsConfianceT2ProofKeyRejectsInvalidFilesAndMissingHistoricalKey(t *testing.T) {
	for _, kind := range []string{"short", "invalid-hex", "public-permissions", "directory", "fifo", "missing"} {
		t.Run(kind, func(t *testing.T) {
			s, at := trustFixture(t)
			createMandate(t, s, at, "invalid-key", at.Add(time.Hour))
			state, _, key := s.trustPaths()
			before, err := os.ReadFile(state)
			if err != nil {
				t.Fatal(err)
			}
			if err = os.Remove(key); err != nil {
				t.Fatal(err)
			}
			switch kind {
			case "short":
				err = os.WriteFile(key, []byte("abcd"), 0600)
			case "invalid-hex":
				err = os.WriteFile(key, []byte(strings.Repeat("z", 64)), 0600)
			case "public-permissions":
				err = os.WriteFile(key, []byte(strings.Repeat("a", 64)), 0644)
			case "directory":
				err = os.Mkdir(key, 0700)
			case "fifo":
				err = syscall.Mkfifo(key, 0600)
			}
			if err != nil {
				t.Fatal(err)
			}
			if _, err = s.trustAnchor(); err == nil {
				t.Fatal("invalid proof key admitted")
			}
			if _, err = s.trustProofKey(); err == nil {
				t.Fatal("invalid historical key accepted or recreated")
			}
			if kind == "missing" {
				if _, err = os.Lstat(key); !os.IsNotExist(err) {
					t.Fatal("missing historical key recreated", err)
				}
			}
			after, err := os.ReadFile(state)
			if err != nil || !bytes.Equal(before, after) {
				t.Fatal("invalid key mutated proof state", err)
			}
		})
	}
}

func TestActionsConfianceT2InitialConfigurationRejectsUnattributedKey(t *testing.T) {
	for _, kind := range []string{"regular", "hardlink"} {
		t.Run(kind, func(t *testing.T) {
			s := storeTest(t)
			state, _, key := s.trustPaths()
			known := []byte(strings.Repeat("a", 64) + "\n")
			var err error
			if kind == "regular" {
				err = os.WriteFile(key, known, 0600)
			} else {
				target := filepath.Join(t.TempDir(), "known-private-key")
				if err = os.WriteFile(target, known, 0600); err == nil {
					err = os.Link(target, key)
				}
			}
			if err != nil {
				t.Fatal(err)
			}
			_, err = s.trustConfig(TrustConfigChange{Schema: 1, EventID: "reject-unattributed-key", ExpectedRevision: 0, Policy: defaultTrustPolicy(), Reason: "initial configuration must create its own key", Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}}}, false)
			if err == nil {
				t.Fatal("unattributed preexisting key adopted by initial configuration")
			}
			if _, err = os.Lstat(state); !os.IsNotExist(err) {
				t.Fatal("rejected key created configuration", err)
			}
			after, err := os.ReadFile(key)
			if err != nil || !bytes.Equal(after, known) {
				t.Fatal("rejected key overwritten", err)
			}
		})
	}
}

func assertTrustAuthenticatedValidationAudit(t *testing.T) {
	t.Helper()
	for _, variant := range []string{"resource", "start", "expiry", "duplicate"} {
		s, at := trustFixture(t)
		c := trustCmd("authenticated-rejection")
		c.MandateID, c.Subject, c.Resource, c.MaxAmount, c.Currency, c.Beneficiary = "audit-mandate", "alice", "invoice/audit", 100, "EUR", "ACME"
		c.Actions = []string{"settle"}
		switch variant {
		case "resource":
			c.Resource = "../outside"
		case "start":
			c.StartsAt = "not-a-date"
		case "expiry":
			c.ExpiresAt = at.Add(-time.Hour).Format(time.RFC3339)
		case "duplicate":
			createMandate(t, s, at, c.MandateID, at.Add(time.Hour))
		}
		raw, _ := json.Marshal(c)
		c.original = raw
		before, err := s.loadTrustState()
		if err != nil {
			t.Fatal(err)
		}
		businessBefore, _ := json.Marshal([]any{before.Mandates, before.Actions, before.Receipts, before.Ledger})
		_, rejected := s.trustOperate("mandate-create", c, trustAdminToken, at)
		if rejected == nil {
			t.Fatalf("%s invalid command admitted", variant)
		}
		after, err := s.loadTrustState()
		if err != nil {
			t.Fatal(err)
		}
		businessAfter, _ := json.Marshal([]any{after.Mandates, after.Actions, after.Receipts, after.Ledger})
		if !bytes.Equal(businessBefore, businessAfter) || len(after.Evidence) != len(before.Evidence)+1 {
			t.Fatalf("%s refusal lost its audit or changed business state: before=%d after=%d", variant, len(before.Evidence), len(after.Evidence))
		}
		ev := after.Evidence[len(after.Evidence)-1]
		if ev.Actor != "admin" || ev.Integrity != 3 || ev.Result != "denied" || ev.OriginalHash != hash(raw) || !bytes.Equal(after.Originals[ev.OriginalHash], raw) {
			t.Fatalf("%s audit lost original or authority", variant)
		}
		reopened, err := openStore(s.root, false)
		if err != nil {
			t.Fatal(err)
		}
		t.Cleanup(func() { _ = reopened.db.Close() })
		path := filepath.Join(s.root, ".swarm/trust-actions.json")
		retained, _ := os.ReadFile(path)
		_, replay := reopened.trustOperate("mandate-create", c, trustAdminToken, at.Add(time.Minute))
		if replay == nil || commandFailure(replay).Code != commandFailure(rejected).Code || replay.Error() != rejected.Error() {
			t.Fatalf("%s replay changed the original refusal: %v / %v", variant, rejected, replay)
		}
		if _, err = reopened.trustOperate("mandate-create", c, trustRequesterToken, at); err == nil || commandFailure(err).Code != "authorization_required" {
			t.Fatalf("%s foreign replay admitted: %v", variant, err)
		}
		unchanged, _ := os.ReadFile(path)
		if !bytes.Equal(retained, unchanged) {
			t.Fatalf("%s replay changed retained history", variant)
		}
		export, err := reopened.trustExport()
		if err != nil {
			t.Fatal(err)
		}
		anchor, err := reopened.trustAnchor()
		if err != nil {
			t.Fatal(err)
		}
		key, err := os.ReadFile(filepath.Join(s.root, ".swarm/trust-actions-proof.key"))
		if err != nil {
			t.Fatal(err)
		}
		if err = verifyTrustProof(export, anchor, strings.TrimSpace(string(key))); err != nil {
			t.Fatalf("%s proof invalid: %v", variant, err)
		}
	}
}

func assertTrustActionResourceGuards(t *testing.T) {
	t.Helper()
	for _, operation := range []string{"commit", "reconcile"} {
		for _, state := range []string{"committed", "unknown", "reconciled"} {
			s, at := trustFixture(t)
			createMandate(t, s, at, "target-mandate", at.Add(time.Hour))
			a := approveAction(t, s, at, "approve-target", createAction(t, s, at, "request-target", "target-mandate", "target-key", 100))
			seed := trustCmd("seed-target-effect")
			seed.ActionID, seed.LoseResponse = a.ID, state != "committed"
			if _, err := s.trustOperate("commit", seed, trustEffectToken, at); err != nil {
				t.Fatal(err)
			}
			if state == "reconciled" {
				seed = trustCmd("seed-target-reconcile")
				seed.ActionID = a.ID
				if _, err := s.trustOperate("reconcile", seed, trustEffectToken, at); err != nil {
					t.Fatal(err)
				}
			}
			before, err := s.loadTrustState()
			if err != nil {
				t.Fatal(err)
			}
			if before.Actions[a.ID].State != state {
				t.Fatalf("fixture state mismatch: got %s want %s", before.Actions[a.ID].State, state)
			}
			businessBefore, _ := json.Marshal([]any{before.Actions, before.Receipts, before.Ledger})
			bad := trustCmd("bad-explicit-target")
			bad.ActionID, bad.Resource = a.ID, "invoice/other"
			got, err := s.trustOperate(operation, bad, trustEffectToken, at)
			if err != nil {
				t.Fatal(err)
			}
			denied, ok := got.(map[string]any)
			if !ok || denied["state"] != "denied" {
				t.Fatalf("%s/%s contradictory resource returned known action: %#v", operation, state, got)
			}
			after, err := s.loadTrustState()
			if err != nil {
				t.Fatal(err)
			}
			businessAfter, _ := json.Marshal([]any{after.Actions, after.Receipts, after.Ledger})
			if !bytes.Equal(businessBefore, businessAfter) || len(after.Evidence) != len(before.Evidence)+1 {
				t.Fatalf("%s/%s changed business or omitted refusal proof", operation, state)
			}
			for _, legacy := range []bool{false, true} {
				valid := trustCmd(newID("valid-target-"))
				if legacy {
					valid.Resource = a.ID
				} else {
					valid.ActionID, valid.Resource = a.ID, a.Resource
				}
				result, err := s.trustOperate(operation, valid, trustEffectToken, at)
				if err != nil {
					t.Fatal(err)
				}
				if action, ok := result.(TrustAction); !ok || action.ID != a.ID || ledgerCount(t, s) != 1 {
					t.Fatalf("%s/%s valid target or legacy alias rejected: %#v", operation, state, result)
				}
			}
		}
	}
}

func assertTrustStateReadUsesOneDescriptor(t *testing.T) {
	t.Helper()
	s, _ := trustFixture(t)
	target, _ := trustFixture(t)
	change := TrustConfigChange{Schema: 1, EventID: "target-state-version", ExpectedRevision: 1, Policy: defaultTrustPolicy(), Reason: "different valid state for controlled replacement", Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}}}
	if _, err := target.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	path, _, _ := s.trustPaths()
	targetPath, _, _ := target.trustPaths()
	original, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	targetBytes, err := os.ReadFile(targetPath)
	if err != nil {
		t.Fatal(err)
	}
	backup := path + ".original"
	calls := 0
	state, err := s.loadTrustStateWithOpen(func(name string, flags int, perm os.FileMode) (*os.File, error) {
		calls++
		if name != path || calls != 1 {
			t.Fatalf("unexpected state opener %q call %d", name, calls)
		}
		if e := os.Rename(path, backup); e != nil {
			t.Fatal(e)
		}
		if e := os.Symlink(targetPath, path); e != nil {
			t.Fatal(e)
		}
		return os.OpenFile(name, flags, perm)
	})
	if err == nil {
		t.Fatalf("replacement after metadata validation followed another state inode: revision %d", state.Revision)
	}
	if after, e := os.ReadFile(targetPath); e != nil || !bytes.Equal(after, targetBytes) {
		t.Fatal("replacement target was modified", e)
	}
	if e := os.Remove(path); e != nil {
		t.Fatal(e)
	}
	if e := os.Rename(backup, path); e != nil {
		t.Fatal(e)
	}
	calls = 0
	state, err = s.loadTrustStateWithOpen(func(name string, flags int, perm os.FileMode) (*os.File, error) {
		calls++
		f, e := os.OpenFile(name, flags, perm)
		if e != nil {
			return nil, e
		}
		if e = os.Rename(path, backup); e != nil {
			f.Close()
			t.Fatal(e)
		}
		if e = os.Symlink(targetPath, path); e != nil {
			f.Close()
			t.Fatal(e)
		}
		return f, nil
	})
	if err != nil || state.Revision != 1 || calls != 1 {
		t.Fatal("opened inode was replaced by a second path read", state.Revision, calls, err)
	}
	if after, e := os.ReadFile(backup); e != nil || !bytes.Equal(after, original) {
		t.Fatal("opened original was modified", e)
	}
	for _, kind := range []string{"directory", "fifo", "symlink", "oversize"} {
		fixture, _ := trustFixture(t)
		name, _, _ := fixture.trustPaths()
		if e := os.Remove(name); e != nil {
			t.Fatal(e)
		}
		switch kind {
		case "directory":
			if e := os.Mkdir(name, 0700); e != nil {
				t.Fatal(e)
			}
		case "fifo":
			if e := syscall.Mkfifo(name, 0600); e != nil {
				t.Fatal(e)
			}
		case "symlink":
			if e := os.Symlink(targetPath, name); e != nil {
				t.Fatal(e)
			}
		case "oversize":
			f, e := os.OpenFile(name, os.O_CREATE|os.O_RDWR, 0600)
			if e != nil {
				t.Fatal(e)
			}
			e = f.Truncate(int64(trustStateMaxBytes) + 1)
			closeErr := f.Close()
			if e != nil || closeErr != nil {
				t.Fatal(e, closeErr)
			}
		}
		if _, e := fixture.loadTrustState(); e == nil {
			t.Fatal("invalid state file accepted", kind)
		}
	}
}

func assertTrustDeniedActionTarget(t *testing.T) {
	t.Helper()
	s, at := trustFixture(t)
	for _, op := range []string{"approve", "commit", "reconcile"} {
		for _, legacy := range []bool{false, true} {
			c := trustCmd(op + "-missing-action")
			if legacy {
				c.EventID += "-legacy"
				c.Resource = "absent-action-target"
			} else {
				c.ActionID = "absent-action-target"
			}
			c.original, _ = json.Marshal(c)
			token, subject := trustEffectToken, "effect-service"
			if op == "approve" {
				token, subject = trustApproverToken, "bob"
			}
			got, err := s.trustOperate(op, c, token, at)
			if err != nil {
				t.Fatal(err)
			}
			denied, ok := got.(map[string]any)
			if !ok || denied["state"] != "denied" || denied["business_mutated"] != false {
				t.Fatal("missing action was not explicitly denied", got)
			}
			proof, err := s.trustExport()
			if err != nil {
				t.Fatal(err)
			}
			ev := proof.Evidence[len(proof.Evidence)-1]
			if ev.ObjectID != "absent-action-target" || ev.Actor != subject || ev.Kind != op || ev.OriginalHash != hash(c.original) || !bytes.Equal(proof.Originals[ev.OriginalHash], c.original) {
				t.Fatalf("missing target lost from attributed refusal: %#v", ev)
			}
			path, _, _ := s.trustPaths()
			before, err := os.ReadFile(path)
			if err != nil {
				t.Fatal(err)
			}
			if _, err = s.trustOperate(op, c, token, at); err != nil {
				t.Fatal(err)
			}
			after, err := os.ReadFile(path)
			if err != nil || !bytes.Equal(before, after) {
				t.Fatal("missing-action replay changed evidence", err)
			}
			if ledgerCount(t, s) != 0 {
				t.Fatal("missing action wrote an effect")
			}
		}
	}
	for _, op := range []string{"commit", "reconcile"} {
		c := trustCmd(op + "-wrong-service")
		c.ActionID = "role-denied-target"
		got, err := s.trustOperate(op, c, trustRequesterToken, at)
		if err != nil || got.(map[string]any)["state"] != "denied" {
			t.Fatal("wrong service was not denied", got, err)
		}
		proof, err := s.trustExport()
		if err != nil || proof.Evidence[len(proof.Evidence)-1].ObjectID != c.ActionID {
			t.Fatal("role refusal lost explicit target", err)
		}
	}
}

func TestActionsConfianceT2ApprovalRequiresLiveMandate(t *testing.T) {
	for _, mode := range []string{"revoked", "expired", "missing", "not started"} {
		t.Run(mode, func(t *testing.T) {
			s, at := trustFixture(t)
			createMandate(t, s, at, "current-mandate", at.Add(time.Hour))
			a := createAction(t, s, at, "current-request", "current-mandate", "current-key", 100)
			when := at
			if mode == "not started" {
				when = at.Add(-24 * time.Hour)
			}
			if mode == "revoked" {
				cmd := trustCmd("revoke-before-approval")
				cmd.MandateID = a.MandateID
				if _, err := s.trustOperate("mandate-revoke", cmd, trustAdminToken, at); err != nil {
					t.Fatal(err)
				}
			}
			if mode == "expired" {
				when = at.Add(2 * time.Hour)
			}
			if mode == "missing" {
				if _, err := s.withTrustLock(true, func(st *trustState) (any, error) { delete(st.Mandates, a.MandateID); return nil, nil }); err != nil {
					t.Fatal(err)
				}
			}
			cmd := trustCmd("approval-after-mandate-change")
			cmd.ActionID = a.ID
			cmd.MandateID = a.MandateID
			cmd.Resource = a.Resource
			cmd.Amount = a.Amount
			cmd.Currency = a.Currency
			cmd.Beneficiary = a.Beneficiary
			cmd.ContentHash = a.ContentHash
			result, err := s.trustOperate("approve", cmd, trustApproverToken, when)
			if err != nil {
				t.Fatal(err)
			}
			denied, ok := result.(map[string]any)
			if !ok || denied["state"] != "denied" {
				t.Fatalf("approval admitted despite %s mandate: %#v", mode, result)
			}
			view, err := s.trustView()
			if err != nil {
				t.Fatal(err)
			}
			if view.(map[string]any)["actions"].(map[string]TrustAction)[a.ID] != a {
				t.Fatal("denied approval changed business state")
			}
		})
	}
}

func TestActionsConfianceT2InitialKeyPublicationFailure(t *testing.T) {
	if os.Getenv("SWARM_KEY_PUBLICATION_CHILD") != "1" {
		cmd := exec.Command(os.Args[0], "-test.run=^TestActionsConfianceT2InitialKeyPublicationFailure$", "-test.v")
		cmd.Env = append(os.Environ(), "SWARM_KEY_PUBLICATION_CHILD=1")
		out, err := cmd.CombinedOutput()
		if err != nil {
			t.Fatalf("isolated process: %v\n%s", err, out)
		}
		return
	}
	s := storeTest(t)
	state, _, key := s.trustPaths()
	change := TrustConfigChange{Schema: 1, EventID: "initial-publication", ExpectedRevision: 0, Policy: defaultTrustPolicy(), Reason: "initial publication must recover without adopting unknown keys", Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}}}
	var old syscall.Rlimit
	if err := syscall.Getrlimit(syscall.RLIMIT_FSIZE, &old); err != nil {
		t.Fatal(err)
	}
	limited := old
	limited.Cur = 256
	signal.Ignore(syscall.SIGXFSZ)
	if err := syscall.Setrlimit(syscall.RLIMIT_FSIZE, &limited); err != nil {
		t.Fatal(err)
	}
	_, first := s.trustConfig(change, false)
	if err := syscall.Setrlimit(syscall.RLIMIT_FSIZE, &old); err != nil {
		t.Fatal(err)
	}
	if first == nil {
		t.Fatal("publication failure was not exercised")
	}
	t.Logf("injected local file-size failure: %v", first)
	if _, err := os.Lstat(state); !os.IsNotExist(err) {
		t.Fatal("failed initial configuration published state", err)
	}
	if _, err := os.Lstat(key); !os.IsNotExist(err) {
		t.Errorf("failed publication left a key without configuration: %v", err)
	}
	if _, err := s.trustConfig(change, false); err != nil {
		t.Errorf("retry after repaired storage remains blocked: %v", err)
	}
}

func trustInitialPublicationFixture(t *testing.T) (trustInitialPublication, TrustConfigChange) {
	t.Helper()
	change := TrustConfigChange{Schema: 1, EventID: "recover-initial-publication", ExpectedRevision: 0, Policy: defaultTrustPolicy(), Reason: "initial publication must preserve exact authorized configuration", Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}}}
	source := storeTest(t)
	if _, err := source.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	state, _, _ := source.trustPaths()
	raw, err := os.ReadFile(state)
	if err != nil {
		t.Fatal(err)
	}
	key, err := source.trustProofKey()
	if err != nil {
		t.Fatal(err)
	}
	return trustInitialPublication{Schema: 1, State: raw, StateDigest: hash(raw), Key: key}, change
}

func TestActionsConfianceT2InitialPublicationRequiresExactOperatorRecovery(t *testing.T) {
	p, change := trustInitialPublicationFixture(t)
	for _, stage := range []string{"before key", "after key", "after state"} {
		t.Run(stage, func(t *testing.T) {
			s := storeTest(t)
			state, _, key := s.trustPaths()
			pending, _ := json.Marshal(p)
			if err := trustPublishPrivate(s.trustInitialPath(), pending); err != nil {
				t.Fatal(err)
			}
			if stage != "before key" {
				if err := trustPublishPrivate(key, []byte(hex.EncodeToString(p.Key)+"\n")); err != nil {
					t.Fatal(err)
				}
			}
			if stage == "after state" {
				if err := trustPublishPrivate(state, p.State); err != nil {
					t.Fatal(err)
				}
			}
			snapshot := func() map[string]string {
				m := map[string]string{}
				for _, name := range []string{state, key, s.trustInitialPath()} {
					raw, err := os.ReadFile(name)
					if err != nil && !os.IsNotExist(err) {
						t.Fatal(err)
					}
					m[name] = string(raw)
				}
				return m
			}
			before := snapshot()
			assertUnchanged := func(t *testing.T) {
				t.Helper()
				for name, raw := range snapshot() {
					if raw != before[name] {
						t.Fatalf("refused operation mutated initial publication: %s", filepath.Base(name))
					}
				}
			}
			for _, operation := range []string{"mandate-create", "mandate-revoke", "request", "approve", "commit", "reconcile"} {
				for _, token := range []string{"invalid-token", trustRequesterToken} {
					t.Run(operation+"/"+token, func(t *testing.T) {
						_, err := s.trustOperate(operation, TrustCommand{Schema: 1, EventID: "pending-business-operation"}, token, time.Now().UTC())
						if err == nil {
							t.Fatal("business operation completed incomplete configuration")
						}
						assertUnchanged(t)
					})
				}
			}
			t.Run("different operator request", func(t *testing.T) {
				other := change
				other.EventID = "different-authorized-configuration"
				if _, err := s.trustConfig(other, false); err == nil {
					t.Fatal("different request resumed initial configuration")
				}
				assertUnchanged(t)
			})
			if _, err := s.trustConfig(change, false); err != nil {
				t.Fatal("exact operator recovery failed", err)
			}
		})
	}
}

func TestActionsConfianceT2InitialPublicationRecovery(t *testing.T) {
	p, change := trustInitialPublicationFixture(t)
	for _, stage := range []string{"before key", "after key", "after state"} {
		t.Run(stage, func(t *testing.T) {
			s := storeTest(t)
			state, _, key := s.trustPaths()
			pending, _ := json.Marshal(p)
			if err := trustPublishPrivate(s.trustInitialPath(), pending); err != nil {
				t.Fatal(err)
			}
			if stage != "before key" {
				if err := trustPublishPrivate(key, []byte(hex.EncodeToString(p.Key)+"\n")); err != nil {
					t.Fatal(err)
				}
			}
			if stage == "after state" {
				if err := trustPublishPrivate(state, p.State); err != nil {
					t.Fatal(err)
				}
			}
			before, err := os.ReadFile(s.trustInitialPath())
			if err != nil {
				t.Fatal(err)
			}
			if _, err = s.trustConfig(change, true); err == nil {
				t.Fatal("preview hid incomplete publication")
			}
			after, err := os.ReadFile(s.trustInitialPath())
			if err != nil || !bytes.Equal(before, after) {
				t.Fatal("preview mutated pending original", err)
			}
			if err = s.db.Close(); err != nil {
				t.Fatal(err)
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			if _, err = reopened.trustConfig(change, false); err != nil {
				t.Fatal("exact initial configuration did not recover", err)
			}
			current, err := os.ReadFile(state)
			if err != nil || !bytes.Equal(current, p.State) {
				t.Fatal("recovery changed original configuration", err)
			}
			currentKey, err := reopened.trustProofKey()
			if err != nil || !bytes.Equal(currentKey, p.Key) {
				t.Fatal("recovery changed original proof key", err)
			}
			if _, err = os.Lstat(s.trustInitialPath()); !os.IsNotExist(err) {
				t.Fatal("publication did not finish", err)
			}
			if _, err = reopened.trustConfig(change, false); err != nil {
				t.Fatal("idempotent replay failed", err)
			}
			current, err = os.ReadFile(state)
			if err != nil || !bytes.Equal(current, p.State) {
				t.Fatal("replay rewrote history", err)
			}
		})
	}
}

func TestActionsConfianceT2InitialPublicationRejectsReplacement(t *testing.T) {
	original, change := trustInitialPublicationFixture(t)
	for _, mode := range []string{"state", "key", "digest", "public", "symlink"} {
		t.Run(mode, func(t *testing.T) {
			s := storeTest(t)
			state, _, key := s.trustPaths()
			p := original
			if mode == "digest" {
				p.StateDigest = strings.Repeat("0", 64)
			}
			pending, _ := json.Marshal(p)
			if mode == "state" {
				if err := os.WriteFile(state, []byte("foreign original state"), 0600); err != nil {
					t.Fatal(err)
				}
			}
			if mode == "key" {
				if err := os.WriteFile(key, []byte(strings.Repeat("a", 64)+"\n"), 0600); err != nil {
					t.Fatal(err)
				}
			}
			if mode == "symlink" {
				target := filepath.Join(t.TempDir(), "pending")
				if err := os.WriteFile(target, pending, 0600); err != nil {
					t.Fatal(err)
				}
				if err := os.Symlink(target, s.trustInitialPath()); err != nil {
					t.Fatal(err)
				}
			} else {
				if err := os.WriteFile(s.trustInitialPath(), pending, 0600); err != nil {
					t.Fatal(err)
				}
			}
			if mode == "public" {
				if err := os.Chmod(s.trustInitialPath(), 0644); err != nil {
					t.Fatal(err)
				}
			}
			beforeState, _ := os.ReadFile(state)
			beforeKey, _ := os.ReadFile(key)
			beforePending, _ := os.ReadFile(s.trustInitialPath())
			if _, err := s.trustConfig(change, false); err == nil {
				t.Fatal("untrusted replacement admitted")
			}
			afterState, _ := os.ReadFile(state)
			afterKey, _ := os.ReadFile(key)
			afterPending, _ := os.ReadFile(s.trustInitialPath())
			if !bytes.Equal(beforeState, afterState) || !bytes.Equal(beforeKey, afterKey) || !bytes.Equal(beforePending, afterPending) {
				t.Fatal("rejected publication mutated an original")
			}
		})
	}
}

func TestActionsConfianceT2InitialPublicationConcurrent(t *testing.T) {
	_, change := trustInitialPublicationFixture(t)
	s := storeTest(t)
	var wg sync.WaitGroup
	errs := make(chan error, 2)
	for i := 0; i < 2; i++ {
		wg.Add(1)
		go func() { defer wg.Done(); _, err := s.trustConfig(change, false); errs <- err }()
	}
	wg.Wait()
	close(errs)
	for err := range errs {
		if err != nil {
			t.Fatal(err)
		}
	}
	st, err := s.loadTrustState()
	if err != nil || st.Revision != 1 || len(st.History) != 1 {
		t.Fatal("concurrent initial configuration duplicated history", err)
	}
	if _, err = s.trustProofKey(); err != nil {
		t.Fatal(err)
	}
	if _, err = os.Lstat(s.trustInitialPath()); !os.IsNotExist(err) {
		t.Fatal("pending publication remained", err)
	}
}

func TestActionsConfianceT2RemovedRequesterInvalidatesMandate(t *testing.T) {
	for _, operation := range []string{"approve", "commit"} {
		t.Run(operation, func(t *testing.T) {
			s, at := trustFixture(t)
			createMandate(t, s, at, "epoch-mandate", at.Add(time.Hour))
			a := createAction(t, s, at, "epoch-request", "epoch-mandate", "epoch-key", 100)
			if operation == "commit" {
				a = approveAction(t, s, at, "epoch-approval", a)
			}
			change := TrustConfigChange{Schema: 1, EventID: "revoke-requester-authority", ExpectedRevision: 1, Policy: defaultTrustPolicy(), Reason: "Remove requester authority before a new approval or first effect", Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}, {Subject: "effect-service", Token: trustEffectToken, Integrity: 2, Roles: []string{"effect"}}}}
			if _, err := s.trustConfig(change, false); err != nil {
				t.Fatal(err)
			}
			cmd := trustCmd("after-authority-change")
			cmd.ActionID = a.ID
			token := trustEffectToken
			if operation == "approve" {
				token = trustApproverToken
				cmd.MandateID = a.MandateID
				cmd.Resource = a.Resource
				cmd.Amount = a.Amount
				cmd.Currency = a.Currency
				cmd.Beneficiary = a.Beneficiary
				cmd.ContentHash = a.ContentHash
			}
			got, err := s.trustOperate(operation, cmd, token, at)
			if err != nil {
				t.Fatal(err)
			}
			denied, ok := got.(map[string]any)
			if !ok || denied["state"] != "denied" {
				t.Fatalf("old mandate authority admitted after requester removal: %s %#v", operation, got)
			}
			view, err := s.trustView()
			if err != nil {
				t.Fatal(err)
			}
			state := view.(map[string]any)
			if state["actions"].(map[string]TrustAction)[a.ID] != a || len(state["ledger"].([]TrustLedgerEntry)) != 0 {
				t.Fatal("stale authority mutated business state")
			}
		})
	}
}

func TestActionsConfianceT2ConfigurationEpochControlsUnspentOperations(t *testing.T) {
	for _, operation := range []string{"request", "approve", "commit"} {
		t.Run(operation, func(t *testing.T) {
			s, at := trustFixture(t)
			createMandate(t, s, at, "old-epoch", at.Add(time.Hour))
			a := createAction(t, s, at, "before-epoch", "old-epoch", "before-epoch-key", 100)
			if operation == "commit" {
				a = approveAction(t, s, at, "before-epoch-approval", a)
			}
			change := TrustConfigChange{Schema: 1, EventID: "new-authorization-epoch", ExpectedRevision: 1, Policy: defaultTrustPolicy(), Reason: "Apply another explicit configuration revision before an unspent operation", Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "alice", Token: trustRequesterToken, Integrity: 1, Roles: []string{"requester"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}, {Subject: "effect-service", Token: trustEffectToken, Integrity: 2, Roles: []string{"effect"}}}}
			change.Policy.MandateTTLSeconds++
			if _, err := s.trustConfig(change, false); err != nil {
				t.Fatal(err)
			}
			cmd := trustCmd("old-epoch-new-operation")
			cmd.ActionID = a.ID
			cmd.MandateID = a.MandateID
			cmd.Resource = a.Resource
			cmd.Amount = a.Amount
			cmd.Currency = a.Currency
			cmd.Beneficiary = a.Beneficiary
			cmd.ContentHash = a.ContentHash
			cmd.IdempotencyKey = "old-epoch-new-key"
			token := trustApproverToken
			if operation == "request" {
				token = trustRequesterToken
			} else if operation == "commit" {
				token = trustEffectToken
			}
			got, err := s.trustOperate(operation, cmd, token, at)
			if err != nil {
				t.Fatal(err)
			}
			denied, ok := got.(map[string]any)
			if !ok || denied["state"] != "denied" || denied["reason"] != "révision de configuration du mandat périmée" {
				t.Fatal("old configuration authority admitted", got)
			}
			view, err := s.trustView()
			if err != nil {
				t.Fatal(err)
			}
			state := view.(map[string]any)
			if state["actions"].(map[string]TrustAction)[a.ID] != a || len(state["actions"].(map[string]TrustAction)) != 1 || len(state["ledger"].([]TrustLedgerEntry)) != 0 {
				t.Fatal("denied stale authority changed business state")
			}
			mandate := createMandate(t, s, at, "renewed-epoch", at.Add(time.Hour))
			if mandate.PolicyRev != 2 {
				t.Fatal("renewed mandate not bound to current revision")
			}
			renewed := createAction(t, s, at, "renewed-request", mandate.ID, "renewed-key", 100)
			renewed = approveAction(t, s, at, "renewed-approval", renewed)
			commit := trustCmd("renewed-commit")
			commit.ActionID = renewed.ID
			result, err := s.trustOperate("commit", commit, trustEffectToken, at)
			if err != nil || result.(TrustAction).State != "committed" {
				t.Fatal("renewed authorization failed", result, err)
			}
		})
	}
}

func TestActionsConfianceT2ConfigurationEpochPreservesCommittedRecovery(t *testing.T) {
	s, at := trustFixture(t)
	createMandate(t, s, at, "committed-epoch", at.Add(time.Hour))
	a := approveAction(t, s, at, "committed-epoch-approval", createAction(t, s, at, "committed-epoch-request", "committed-epoch", "committed-epoch-key", 100))
	commit := trustCmd("lost-before-configuration")
	commit.ActionID = a.ID
	commit.LoseResponse = true
	if _, err := s.trustOperate("commit", commit, trustEffectToken, at); err != nil {
		t.Fatal(err)
	}
	change := TrustConfigChange{Schema: 1, EventID: "configuration-after-effect", ExpectedRevision: 1, Policy: defaultTrustPolicy(), Reason: "Remove requester after a committed effect without losing reconciliation", Identities: []TrustIdentityInput{{Subject: "admin", Token: trustAdminToken, Integrity: 3, Roles: []string{"admin"}}, {Subject: "bob", Token: trustApproverToken, Integrity: 2, Roles: []string{"approver"}}, {Subject: "effect-service", Token: trustEffectToken, Integrity: 2, Roles: []string{"effect"}}}}
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	reconcile := trustCmd("reconcile-after-configuration")
	reconcile.ActionID = a.ID
	got, err := s.trustOperate("reconcile", reconcile, trustEffectToken, at)
	if err != nil || got.(TrustAction).State != "reconciled" {
		t.Fatal("configuration prevented recovery of a committed effect", got, err)
	}
	view, err := s.trustView()
	if err != nil || len(view.(map[string]any)["ledger"].([]TrustLedgerEntry)) != 1 {
		t.Fatal("configuration duplicated a committed effect", err)
	}
}
