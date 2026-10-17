//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

func lifecycleChange(t *testing.T, subject string, disabled any, revision int) TrustConfigChange {
	t.Helper()
	inputs := []map[string]any{
		{"subject": "admin", "token": trustAdminToken, "integrity": 3, "roles": []string{"admin"}},
		{"subject": "alice", "token": trustRequesterToken, "integrity": 1, "roles": []string{"requester"}},
		{"subject": "bob", "token": trustApproverToken, "integrity": 2, "roles": []string{"approver"}},
		{"subject": "effect-service", "token": trustEffectToken, "integrity": 2, "roles": []string{"effect"}},
	}
	for _, in := range inputs {
		if in["subject"] == subject {
			in["disabled"] = disabled
		}
	}
	raw, _ := json.Marshal(map[string]any{"schema_version": 1, "event_id": newID("identity-lifecycle-"), "expected_revision": revision, "policy": defaultTrustPolicy(), "identities": inputs, "reason": "suspend or restore the selected identity explicitly"})
	var change TrustConfigChange
	if err := strict(raw, &change); err != nil {
		t.Fatal(err)
	}
	return change
}

func TestActionsConfianceT3IdentitySuspensionPersistsAndRejectsWithoutMutation(t *testing.T) {
	for _, item := range []struct{ subject, token string }{{"alice", trustRequesterToken}, {"bob", trustApproverToken}, {"effect-service", trustEffectToken}, {"admin", trustAdminToken}} {
		t.Run(item.subject, func(t *testing.T) {
			s, when := trustFixture(t)
			if _, err := s.trustConfig(lifecycleChange(t, item.subject, true, 1), false); err != nil {
				t.Fatal(err)
			}
			path, _, _ := s.trustPaths()
			before, _ := os.ReadFile(path)
			if _, err := s.trustOperate("request", trustCmd("suspended-request"), item.token, when); commandFailure(err).Code != "authorization_required" {
				t.Fatal("suspended identity authenticated", err)
			}
			after, _ := os.ReadFile(path)
			if string(before) != string(after) {
				t.Fatal("suspended request mutated state")
			}
			if err := s.db.Close(); err != nil {
				t.Fatal(err)
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			if _, err = reopened.trustOperate("request", trustCmd("suspended-after-reopen"), item.token, when); commandFailure(err).Code != "authorization_required" {
				t.Fatal("suspension lost on reopen", err)
			}
			if _, err = reopened.trustConfig(lifecycleChange(t, item.subject, false, 2), false); err != nil {
				t.Fatal(err)
			}
			st, err := reopened.loadTrustState()
			if err != nil {
				t.Fatal(err)
			}
			if _, err = findTrustIdentity(&st, item.token); err != nil {
				t.Fatal("explicit reactivation failed", err)
			}
		})
	}
}

func TestActionsConfianceT3IdentitySuspensionPreviewDoesNotApply(t *testing.T) {
	s, _ := trustFixture(t)
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	preview, err := s.trustConfig(lifecycleChange(t, "alice", true, 1), true)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(preview)
	if !strings.Contains(string(raw), `"disabled":true`) || strings.Contains(string(raw), trustRequesterToken) || strings.Contains(string(raw), "token_hash") {
		t.Fatal("preview did not show redacted suspension", string(raw))
	}
	after, _ := os.ReadFile(path)
	if string(before) != string(after) {
		t.Fatal("preview applied suspension")
	}
	st, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := findTrustIdentity(&st, trustRequesterToken); err != nil {
		t.Fatal("preview revoked identity", err)
	}
}

func TestActionsConfianceT3SuspensionBlocksHistoricalReplay(t *testing.T) {
	s, when := trustFixture(t)
	createMandate(t, s, when, "lifecycle-replay", when.Add(time.Hour))
	a := createAction(t, s, when, "original-request", "lifecycle-replay", "lifecycle-key", 100)
	c := trustCmd("original-request")
	c.MandateID, c.Resource, c.Amount, c.Currency, c.Beneficiary, c.IdempotencyKey = a.MandateID, a.Resource, a.Amount, a.Currency, a.Beneficiary, a.IdempotencyKey
	if _, err := s.trustConfig(lifecycleChange(t, "alice", true, 1), false); err != nil {
		t.Fatal(err)
	}
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	if _, err := s.trustOperate("request", c, trustRequesterToken, when); commandFailure(err).Code != "authorization_required" {
		t.Fatal("historical reply bypassed suspension", err)
	}
	after, _ := os.ReadFile(path)
	if string(before) != string(after) {
		t.Fatal("refused replay changed history")
	}
}

func TestActionsConfianceT3SuspensionRequiresBoolean(t *testing.T) {
	for _, raw := range []string{`null`, `"true"`, `1`, `{}`, `[]`} {
		t.Run(raw, func(t *testing.T) {
			var identity TrustIdentityInput
			if err := strict([]byte(`{"subject":"alice","disabled":`+raw+`}`), &identity); err == nil {
				t.Fatal("invalid suspension flag accepted")
			}
		})
	}
}
