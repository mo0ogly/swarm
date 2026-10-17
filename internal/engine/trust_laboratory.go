//go:build linux

package engine

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"
)

type TrustLaboratoryPolicy struct {
	Enabled           trustOIDCEnabled `json:"enabled"`
	TimeoutSeconds    int              `json:"timeout_seconds"`
	MaxConcurrentRuns int              `json:"max_concurrent_runs"`
}

func (p *TrustLaboratoryPolicy) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "enabled", "timeout_seconds", "max_concurrent_runs"); err != nil {
		return err
	}
	type plain TrustLaboratoryPolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustLaboratoryPolicy(v)
	return validateTrustLaboratoryPolicy(p)
}
func laboratoryPolicy(p *TrustLaboratoryPolicy) TrustLaboratoryPolicy {
	if p == nil {
		return TrustLaboratoryPolicy{TimeoutSeconds: 30, MaxConcurrentRuns: 1}
	}
	return *p
}
func validateTrustLaboratoryPolicy(p *TrustLaboratoryPolicy) error {
	if p != nil && (p.TimeoutSeconds < 1 || int64(p.TimeoutSeconds) > trustPolicyBounds()["laboratory_timeout_seconds"]["max"] || p.MaxConcurrentRuns < 1 || int64(p.MaxConcurrentRuns) > trustPolicyBounds()["laboratory_max_concurrent_runs"]["max"]) {
		return fmt.Errorf("laboratoire : délai et concurrence entiers positifs requis ; 0 refusé")
	}
	return nil
}

type TrustLaboratoryInput struct {
	Version   int      `json:"corpus_version"`
	Digest    string   `json:"corpus_digest"`
	Seed      string   `json:"seed"`
	Scenarios []string `json:"scenario_ids"`
	Connector string   `json:"connector_id"`
}
type TrustLaboratoryCommand struct {
	Schema           int                    `json:"schema_version,omitempty"`
	EventID          string                 `json:"event_id,omitempty"`
	ExpectedRevision *int                   `json:"expected_revision,omitempty"`
	Reason           string                 `json:"reason,omitempty"`
	Policy           *TrustLaboratoryPolicy `json:"policy,omitempty"`
	Input            *TrustLaboratoryInput  `json:"input,omitempty"`
	RunID            string                 `json:"run_id,omitempty"`
}
type TrustLaboratoryCase struct {
	ID       string         `json:"scenario_id"`
	Passed   bool           `json:"passed"`
	Observed map[string]any `json:"observed"`
}
type TrustLaboratoryResult struct {
	Input               TrustLaboratoryInput  `json:"input"`
	Cases               []TrustLaboratoryCase `json:"cases"`
	Containment         trustSealedResult     `json:"containment"`
	DeterministicDigest string                `json:"deterministic_digest"`
	Simulation          bool                  `json:"simulation"`
}
type TrustLaboratoryRun struct {
	Corpus     json.RawMessage        `json:"corpus"`
	ID         string                 `json:"run_id"`
	Parent     string                 `json:"replay_of,omitempty"`
	Input      TrustLaboratoryInput   `json:"input"`
	Policy     TrustLaboratoryPolicy  `json:"policy"`
	Revision   int                    `json:"configuration_revision"`
	Principal  string                 `json:"principal_id"`
	Authority  string                 `json:"authority_digest"`
	Status     string                 `json:"status"`
	StartedAt  time.Time              `json:"started_at"`
	FinishedAt *time.Time             `json:"finished_at,omitempty"`
	OwnerPID   int                    `json:"owner_pid"`
	OwnerStart string                 `json:"owner_start"`
	Result     *TrustLaboratoryResult `json:"result,omitempty"`
	Diagnostic string                 `json:"diagnostic_code,omitempty"`
}
type trustLaboratoryState struct {
	Runs    map[string]TrustLaboratoryRun `json:"runs"`
	Events  map[string]string             `json:"events"`
	Digests map[string]string             `json:"digests"`
}

func laboratoryInvalid() error {
	return fmt.Errorf("laboratoire : commande, corpus ou entrée invalide")
}
func laboratoryDenied() error {
	return &CommandError{Code: "authorization_required", Message: "laboratoire refusé : identité I3, politique ou révision non concordante"}
}
func laboratoryStoredInput(v *TrustLaboratoryInput) bool {
	if v == nil || v.Version < 1 || v.Scenarios == nil || len(v.Scenarios) == 0 || !safeName(v.Connector) {
		return false
	}
	digest, e := hex.DecodeString(v.Digest)
	if e != nil || len(digest) != 32 {
		return false
	}
	n, e := strconv.ParseUint(v.Seed, 10, 64)
	if e != nil || strconv.FormatUint(n, 10) != v.Seed {
		return false
	}
	seen := map[string]bool{}
	for _, id := range v.Scenarios {
		if !safeName(id) || seen[id] {
			return false
		}
		seen[id] = true
	}
	return true
}
func laboratoryInputValid(v *TrustLaboratoryInput) bool {
	if !laboratoryStoredInput(v) || v.Version != 1 || v.Digest != hashMust(laboratoryCorpus()) || v.Connector != "local-ledger-simulation-v1" || v.Scenarios == nil || len(v.Scenarios) == 0 {
		return false
	}
	n, err := strconv.ParseUint(v.Seed, 10, 64)
	if err != nil || strconv.FormatUint(n, 10) != v.Seed {
		return false
	}
	seen := map[string]bool{}
	for _, id := range v.Scenarios {
		if !containsTrust(laboratoryScenarioIDs, id) || seen[id] {
			return false
		}
		seen[id] = true
	}
	return true
}
func parseLaboratory(op string, raw []byte) (TrustLaboratoryCommand, error) {
	var c TrustLaboratoryCommand
	if len(raw) > 1<<20 || strict(raw, &c) != nil {
		return c, laboratoryInvalid()
	}
	var fields map[string]json.RawMessage
	if json.Unmarshal(raw, &fields) != nil || fields == nil {
		return c, laboratoryInvalid()
	}
	allowed := []string{}
	switch op {
	case "show":
	case "result":
		allowed = []string{"run_id"}
	case "policy-preview", "policy-apply":
		allowed = []string{"schema_version", "event_id", "expected_revision", "reason", "policy"}
	case "run":
		allowed = []string{"schema_version", "event_id", "expected_revision", "reason", "input"}
	case "replay":
		allowed = []string{"schema_version", "event_id", "expected_revision", "reason", "run_id"}
	default:
		return c, laboratoryInvalid()
	}
	for k, v := range fields {
		if !containsTrust(allowed, k) || strings.TrimSpace(string(v)) == "null" {
			return c, laboratoryInvalid()
		}
	}
	if op != "show" && op != "result" && (c.Schema != 1 || !safeName(c.EventID) || len(strings.TrimSpace(c.Reason)) < 8 || c.ExpectedRevision == nil) {
		return c, laboratoryInvalid()
	}
	if op == "run" && !laboratoryInputValid(c.Input) {
		return c, laboratoryInvalid()
	}
	return c, nil
}
