//go:build linux

package engine

import (
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"time"
)

var laboratoryScenarioIDs = []string{"biba-denials", "exact-approval", "duplicate-effect", "lost-response", "original-tampering"}

func laboratoryCorpus() any {
	return map[string]any{"version": 1, "scenario_ids": laboratoryScenarioIDs, "connector_id": "local-ledger-simulation-v1", "fixture_time": "2026-01-01T12:00:00Z", "fixture_policy": defaultTrustPolicy(), "production_effects": false, "scope": "fixed_corpus_only"}
}
func laboratoryToken(seed, who string) string {
	return hash([]byte("swarm-laboratory-fixture-v1/" + seed + "/" + who))
}
func laboratoryFixture(root, seed string) (*Store, time.Time, error) {
	if err := os.Mkdir(filepath.Join(root, ".swarm"), 0700); err != nil {
		return nil, time.Time{}, err
	}
	s := &Store{root: root}
	at := time.Date(2026, 1, 1, 12, 0, 0, 0, time.UTC)
	identities := []TrustIdentityInput{}
	for _, v := range []struct {
		subject   string
		integrity int
		role      string
	}{{"lab-admin", 3, "admin"}, {"lab-requester", 1, "requester"}, {"lab-approver", 2, "approver"}, {"lab-effect", 2, "effect"}} {
		identities = append(identities, TrustIdentityInput{Subject: v.subject, Token: laboratoryToken(seed, v.subject), Integrity: v.integrity, Roles: []string{v.role}})
	}
	_, err := s.trustConfig(TrustConfigChange{Schema: 1, EventID: "lab-config", ExpectedRevision: 0, Policy: defaultTrustPolicy(), Identities: identities, Reason: "fixed isolated laboratory fixture"}, false)
	return s, at, err
}
func laboratoryScenario(root string, input TrustLaboratoryInput, id string) (TrustLaboratoryCase, error) {
	result := TrustLaboratoryCase{ID: id, Observed: map[string]any{}}
	s, at, err := laboratoryFixture(root, input.Seed)
	if err != nil {
		return result, err
	}
	call := func(op string, c TrustCommand, who string) (any, error) {
		return s.trustOperate(op, c, laboratoryToken(input.Seed, who), at)
	}
	cmd := func(event string) TrustCommand { return TrustCommand{Schema: 1, EventID: event} }
	state := func() (trustState, error) { return s.loadTrustState() }
	if id == "biba-denials" {
		for _, op := range []string{"direct-write", "elevate-integrity"} {
			c := cmd(op)
			c.Resource = "laboratory/invoice"
			v, e := call(op, c, "lab-requester")
			if e != nil {
				return result, e
			}
			denied, ok := v.(map[string]any)
			if !ok || denied["state"] != "denied" {
				return result, laboratoryInvalid()
			}
		}
		st, e := state()
		if e != nil {
			return result, e
		}
		if len(st.Ledger) != 0 || len(st.Actions) != 0 {
			return result, laboratoryInvalid()
		}
		result.Passed = true
		result.Observed = map[string]any{"direct_write_denied": true, "elevation_denied": true, "effects": 0}
		return result, nil
	}
	c := cmd("lab-mandate")
	c.MandateID = "lab-mandate"
	c.Subject = "lab-requester"
	c.Actions = []string{"settle"}
	c.Resource = "laboratory/invoice"
	c.MaxAmount = 50000
	c.Currency = "EUR"
	c.Beneficiary = "LAB"
	c.StartsAt = at.Add(-time.Minute).Format(time.RFC3339)
	c.ExpiresAt = at.Add(time.Hour).Format(time.RFC3339)
	if _, err = call("mandate-create", c, "lab-admin"); err != nil {
		return result, err
	}
	c = cmd("lab-request")
	c.MandateID = "lab-mandate"
	c.Resource = "laboratory/invoice"
	seed, _ := strconv.ParseUint(input.Seed, 10, 64)
	c.Amount = 1 + int64(seed%100)
	c.Currency = "EUR"
	c.Beneficiary = "LAB"
	c.IdempotencyKey = "lab-exact-effect"
	v, err := call("request", c, "lab-requester")
	if err != nil {
		return result, err
	}
	action, ok := v.(TrustAction)
	if !ok {
		return result, laboratoryInvalid()
	}
	approve := cmd("lab-approve")
	approve.ActionID = action.ID
	approve.MandateID = action.MandateID
	approve.Amount = action.Amount
	approve.Currency = action.Currency
	approve.Beneficiary = action.Beneficiary
	approve.ContentHash = action.ContentHash
	if id == "exact-approval" {
		bad := approve
		bad.EventID = "lab-wrong-approve"
		bad.Beneficiary = "OTHER"
		if _, err = call("approve", bad, "lab-approver"); err != nil {
			return result, err
		}
		st, e := state()
		if e != nil {
			return result, e
		}
		if st.Actions[action.ID].State != "pending_approval" {
			return result, laboratoryInvalid()
		}
		result.Observed["changed_content_denied"] = true
	}
	if _, err = call("approve", approve, "lab-approver"); err != nil {
		return result, err
	}
	effect := cmd("lab-commit")
	effect.ActionID = action.ID
	effect.LoseResponse = id == "lost-response"
	if _, err = call("commit", effect, "lab-effect"); err != nil {
		return result, err
	}
	if id == "duplicate-effect" || id == "lost-response" {
		effect.EventID = "lab-commit-again"
		if _, err = call("commit", effect, "lab-effect"); err != nil {
			return result, err
		}
	}
	if id == "lost-response" {
		st, e := state()
		if e != nil {
			return result, e
		}
		if st.Actions[action.ID].State != "unknown" || len(st.Ledger) != 1 {
			return result, laboratoryInvalid()
		}
		effect.EventID = "lab-reconcile"
		if _, err = call("reconcile", effect, "lab-effect"); err != nil {
			return result, err
		}
		result.Observed["unknown_before_reconciliation"] = true
	}
	if id == "original-tampering" {
		exp, e := s.trustExport()
		if e != nil {
			return result, e
		}
		anchor, e := s.trustAnchor()
		if e != nil {
			return result, e
		}
		key, e := s.trustProofKey()
		if e != nil {
			return result, e
		}
		defer clear(key)
		if verifyTrustProof(exp, anchor, hex.EncodeToString(key)) != nil {
			return result, laboratoryInvalid()
		}
		first := exp.Evidence[0].OriginalHash
		exp.Originals[first] = []byte(`{"tampered":true}`)
		if verifyTrustProof(exp, anchor, hex.EncodeToString(key)) == nil {
			return result, laboratoryInvalid()
		}
		result.Observed["altered_original_denied"] = true
	}
	st, err := state()
	if err != nil {
		return result, err
	}
	wanted := "committed"
	if id == "lost-response" {
		wanted = "reconciled"
	}
	if len(st.Ledger) != 1 || st.Actions[action.ID].State != wanted {
		return result, laboratoryInvalid()
	}
	result.Passed = true
	result.Observed["effects"] = 1
	result.Observed["final_state"] = wanted
	result.Observed["amount_minor"] = action.Amount
	result.Observed["simulation"] = true
	return result, nil
}
func laboratoryExecute(work string, input TrustLaboratoryInput) ([]TrustLaboratoryCase, error) {
	cases := []TrustLaboratoryCase{}
	for i, id := range input.Scenarios {
		root := filepath.Join(work, fmt.Sprintf("scenario-%d", i))
		if err := os.Mkdir(root, 0700); err != nil {
			return nil, err
		}
		c, err := laboratoryScenario(root, input, id)
		if err != nil {
			return nil, err
		}
		cases = append(cases, c)
	}
	return cases, nil
}
func laboratoryResultDigest(v TrustLaboratoryResult) string {
	return hashMust(struct {
		Input TrustLaboratoryInput
		Cases []TrustLaboratoryCase
	}{v.Input, v.Cases})
}
func laboratoryCleanWork(work string) bool {
	info, err := os.Lstat(work)
	if err != nil || !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
		return false
	}
	entries, err := os.ReadDir(work)
	if err != nil || len(entries) != 2 {
		return false
	}
	names := map[string]bool{}
	for _, e := range entries {
		names[e.Name()] = true
	}
	return names["allowed"] && names["linked"]
}
func laboratoryPublicResult(raw []byte, input TrustLaboratoryInput) (TrustLaboratoryResult, error) {
	var result TrustLaboratoryResult
	if strict(raw, &result) != nil || !result.Simulation || hashMust(result.Input) != hashMust(input) || len(result.Cases) != len(input.Scenarios) || result.Containment.ABI < 6 || len(result.Containment.Checks) != len(trustSealedCheckNames) {
		return result, laboratoryInvalid()
	}
	for _, key := range trustSealedCheckNames {
		if !result.Containment.Checks[key] {
			return result, laboratoryInvalid()
		}
	}
	for i, c := range result.Cases {
		if c.ID != input.Scenarios[i] || !c.Passed || len(c.Observed) == 0 {
			return result, laboratoryInvalid()
		}
	}
	if result.DeterministicDigest != laboratoryResultDigest(result) {
		return result, laboratoryInvalid()
	}
	return result, nil
}
