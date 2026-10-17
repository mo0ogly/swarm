//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"time"
)

type TrustConnectorSelection struct {
	ID      string                 `json:"id"`
	Version int                    `json:"version"`
	Access  *TrustAccessParameters `json:"access,omitempty"`
}
type TrustAccessParameters struct {
	Subject string `json:"subject"`
	Role    string `json:"role"`
}
type TrustAccessEntry struct {
	Sequence       int                   `json:"sequence"`
	ActionID       string                `json:"action_id"`
	ContentHash    string                `json:"content_digest"`
	IdempotencyKey string                `json:"idempotency_key"`
	Resource       string                `json:"resource"`
	Access         TrustAccessParameters `json:"access"`
	CommittedAt    string                `json:"committed_at"`
	Simulation     bool                  `json:"simulation"`
}
type TrustEffectIntent struct {
	Schema           int             `json:"schema_version"`
	ID               string          `json:"intent_id"`
	ActionHash       string          `json:"action_digest"`
	MandateHash      string          `json:"mandate_digest"`
	ApprovalHashes   []string        `json:"approval_digests"`
	ConnectorID      string          `json:"connector_id"`
	ConnectorVersion int             `json:"connector_version"`
	Resource         string          `json:"resource"`
	Parameters       json.RawMessage `json:"canonical_parameters"`
	IdempotencyKey   string          `json:"idempotency_key"`
	Deadline         string          `json:"deadline"`
}
type TrustEffectResult struct {
	Schema                 int    `json:"schema_version"`
	IntentID               string `json:"intent_id"`
	Receipt                string `json:"connector_receipt"`
	State                  string `json:"state"`
	CommittedAt            string `json:"committed_at"`
	ObservedAt             string `json:"observed_at"`
	Digest                 string `json:"result_digest"`
	ReconciliationRequired bool   `json:"reconciliation_required"`
	Diagnostic             string `json:"diagnostic_code"`
}

func (c *TrustConnectorSelection) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "id", "version"); err != nil {
		return err
	}
	type plain TrustConnectorSelection
	var value plain
	if err := strict(raw, &value); err != nil {
		return err
	}
	*c = TrustConnectorSelection(value)
	return nil
}
func (c *TrustAccessParameters) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "subject", "role"); err != nil {
		return err
	}
	type plain TrustAccessParameters
	var value plain
	if err := strict(raw, &value); err != nil {
		return err
	}
	*c = TrustAccessParameters(value)
	return nil
}
func trustConnectorID(c *TrustConnectorSelection) string {
	if c == nil {
		return "local-ledger-simulation"
	}
	return c.ID
}
func trustConnectorSelectionValid(c *TrustConnectorSelection) bool {
	if c == nil {
		return true
	}
	if c.Version != 1 {
		return false
	}
	switch c.ID {
	case "local-ledger-simulation":
		return c.Access == nil
	case "local-access-simulation":
		return c.Access != nil && safeName(c.Access.Subject) && safeName(c.Access.Role)
	}
	return false
}
func trustConnectorEqual(a, b *TrustConnectorSelection) bool {
	return hashMust(a) == hashMust(b)
}
func trustConnectorCatalog() any {
	rows := []map[string]any{}
	for _, id := range []string{"local-ledger-simulation", "local-access-simulation"} {
		rows = append(rows, map[string]any{"id": id, "version": 1, "simulation": true, "idempotency": true, "readback": true, "external_effects": false, "provider_containment": false, "secret_grants": false})
	}
	return map[string]any{"schema_version": 1, "connectors": rows, "arbitrary_plugins": false}
}
func trustConnectorCommandValid(op string, c TrustCommand) error {
	if len(c.original) > 0 {
		var fields map[string]json.RawMessage
		if err := json.Unmarshal(c.original, &fields); err != nil {
			return err
		}
		if v, ok := fields["connector"]; ok && bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return fmt.Errorf("connecteur : sélecteur null interdit")
		}
	}
	if !trustConnectorSelectionValid(c.Connector) {
		return fmt.Errorf("connecteur : version ou paramètres invalides")
	}
	if c.Connector != nil && op != "mandate-create" && op != "request" && op != "approve" {
		return fmt.Errorf("connecteur : sélecteur non admis pour cette opération")
	}
	if trustConnectorID(c.Connector) == "local-access-simulation" && (c.Amount != 0 || c.MaxAmount != 0 || c.Currency != "" || c.Beneficiary != "") {
		return fmt.Errorf("connecteur : paramètres monétaires interdits pour un accès")
	}
	return nil
}

func trustActionContentHash(c TrustCommand, requester string) string {
	base := hashMust(struct {
		MandateID, Requester, Resource        string
		Amount                                int64
		Currency, Beneficiary, IdempotencyKey string
		DataContext                           *TrustDataContext `json:"data_context,omitempty"`
	}{c.MandateID, requester, c.Resource, c.Amount, c.Currency, c.Beneficiary, c.IdempotencyKey, c.DataContext})
	if c.Connector == nil {
		return base
	}
	return hashMust(struct {
		Base      string
		Connector *TrustConnectorSelection
	}{base, c.Connector})
}
func trustConnectorRequestWithin(m TrustMandate, c TrustCommand) bool {
	if !trustConnectorEqual(m.Connector, c.Connector) {
		return false
	}
	if trustConnectorID(c.Connector) == "local-access-simulation" {
		return containsTrust(m.Actions, "grant-access")
	}
	return c.Amount > 0 && c.Amount <= m.MaxAmount && containsTrust(m.Actions, "settle")
}
func trustConnectorCommit(st *trustState, a TrustAction, actor trustIdentity, now time.Time, lost bool) error {
	if err := verifyTrustEvidenceChain(TrustProofExport{Evidence: st.Evidence, Originals: st.Originals}); err != nil {
		return err
	}
	deny := func() error {
		return &CommandError{Code: "authorization_required", Message: "connecteur : intention protégée requise"}
	}
	stored, ok := st.Actions[a.ID]
	if !ok || hashMust(stored) != hashMust(a) || a.State != "approved" || !trustHasRole(actor, "effect") || actor.Integrity < 2 || !trustConnectorSelectionValid(a.Connector) {
		return deny()
	}
	if trustActiveMandateReason(st, a.MandateID, now) != "" {
		return deny()
	}
	m := st.Mandates[a.MandateID]
	c := TrustCommand{Connector: a.Connector, DataContext: a.DataContext, MandateID: a.MandateID, Resource: a.Resource, Amount: a.Amount, Currency: a.Currency, Beneficiary: a.Beneficiary, IdempotencyKey: a.IdempotencyKey}
	if a.ContentHash != trustActionContentHash(c, a.Requester) || m.Subject != a.Requester || m.Resource != a.Resource || m.Currency != a.Currency || m.Beneficiary != a.Beneficiary || !trustConnectorRequestWithin(m, c) {
		return deny()
	}
	if err := trustDataActionAdmission(st, "commit", TrustCommand{ActionID: a.ID}); err != nil {
		return err
	}
	approvals := []string{}
	for _, ev := range st.Evidence {
		if ev.Kind == "approve" && ev.ObjectID == a.ID && ev.ContentHash == a.ContentHash && ev.Actor == a.Approver && ev.Result == "approved" {
			approvals = append(approvals, ev.Hash)
		}
	}
	if len(approvals) == 0 || a.Approver == "" || (st.Policy.RequireSeparation && a.Approver == a.Requester) {
		return deny()
	}
	if _, exists := st.Receipts[a.IdempotencyKey]; exists {
		return deny()
	}
	connector, err := trustConnectorFor(a.Connector)
	if err != nil {
		return err
	}
	parameters, _ := json.Marshal(connector.parameters(a))
	intent := &TrustEffectIntent{Schema: 1, ID: "intent-" + a.ContentHash, ActionHash: a.ContentHash, MandateHash: m.ContentHash, ApprovalHashes: approvals, ConnectorID: trustConnectorID(a.Connector), ConnectorVersion: 1, Resource: a.Resource, Parameters: parameters, IdempotencyKey: a.IdempotencyKey, Deadline: m.ExpiresAt}
	at := now.UTC().Format(time.RFC3339Nano)
	digest := connector.commit(st, a, at)
	result := &TrustEffectResult{Schema: 1, IntentID: intent.ID, Receipt: "receipt-" + hashMust(intent), State: "committed", CommittedAt: at, ObservedAt: at, Digest: digest}
	if lost {
		result.State = "unknown"
		result.ReconciliationRequired = true
		result.Diagnostic = "response_lost"
	}
	st.Receipts[a.IdempotencyKey] = trustReceipt{ContentHash: a.ContentHash, ActionID: a.ID, State: "committed", Intent: intent, Result: result}
	return nil
}

type trustConnectorSPI interface {
	parameters(TrustAction) any
	commit(*trustState, TrustAction, string) string
	readback(*trustState, TrustAction) (string, bool)
}
type trustLedgerConnector struct{}
type trustAccessConnector struct{}

func trustConnectorFor(c *TrustConnectorSelection) (trustConnectorSPI, error) {
	if !trustConnectorSelectionValid(c) {
		return nil, fmt.Errorf("connecteur : version ou paramètres invalides")
	}
	if trustConnectorID(c) == "local-access-simulation" {
		return trustAccessConnector{}, nil
	}
	return trustLedgerConnector{}, nil
}
func (trustLedgerConnector) parameters(a TrustAction) any {
	return struct {
		Amount      int64  `json:"amount_minor"`
		Currency    string `json:"currency"`
		Beneficiary string `json:"beneficiary"`
	}{a.Amount, a.Currency, a.Beneficiary}
}
func (trustAccessConnector) parameters(a TrustAction) any { return *a.Connector.Access }
func (trustLedgerConnector) commit(st *trustState, a TrustAction, at string) string {
	e := TrustLedgerEntry{Sequence: len(st.Ledger) + 1, ActionID: a.ID, ContentHash: a.ContentHash, IdempotencyKey: a.IdempotencyKey, Amount: a.Amount, Currency: a.Currency, Beneficiary: a.Beneficiary, Resource: a.Resource, CommittedAt: at, Simulation: true}
	st.Ledger = append(st.Ledger, e)
	return hashMust(e)
}
func (trustAccessConnector) commit(st *trustState, a TrustAction, at string) string {
	e := TrustAccessEntry{Sequence: len(st.AccessLedger) + 1, ActionID: a.ID, ContentHash: a.ContentHash, IdempotencyKey: a.IdempotencyKey, Resource: a.Resource, Access: *a.Connector.Access, CommittedAt: at, Simulation: true}
	st.AccessLedger = append(st.AccessLedger, e)
	return hashMust(e)
}
func (trustLedgerConnector) readback(st *trustState, a TrustAction) (string, bool) {
	count := 0
	digest := ""
	for _, e := range st.Ledger {
		if e.IdempotencyKey == a.IdempotencyKey {
			count++
			if e.ActionID != a.ID || e.ContentHash != a.ContentHash || e.Resource != a.Resource || e.Amount != a.Amount || e.Currency != a.Currency || e.Beneficiary != a.Beneficiary || !e.Simulation {
				return "", false
			}
			digest = hashMust(e)
		}
	}
	return digest, count == 1
}
func (trustAccessConnector) readback(st *trustState, a TrustAction) (string, bool) {
	count := 0
	digest := ""
	for _, e := range st.AccessLedger {
		if e.IdempotencyKey == a.IdempotencyKey {
			count++
			if e.ActionID != a.ID || e.ContentHash != a.ContentHash || e.Resource != a.Resource || e.Access != *a.Connector.Access || !e.Simulation {
				return "", false
			}
			digest = hashMust(e)
		}
	}
	return digest, count == 1
}
func trustConnectorReadback(st *trustState, a TrustAction, r trustReceipt) bool {
	if r.ActionID != a.ID || r.ContentHash != a.ContentHash {
		return false
	}
	c, err := trustConnectorFor(a.Connector)
	if err != nil {
		return false
	}
	digest, ok := c.readback(st, a)
	if !ok {
		return false
	}
	if r.Intent == nil || r.Result == nil {
		return a.Connector == nil && r.Intent == nil && r.Result == nil
	}
	i, v := r.Intent, r.Result
	return i.Schema == 1 && i.ID == "intent-"+a.ContentHash && i.ActionHash == a.ContentHash && i.ConnectorID == trustConnectorID(a.Connector) && i.ConnectorVersion == 1 && i.Resource == a.Resource && i.IdempotencyKey == a.IdempotencyKey && len(i.ApprovalHashes) > 0 && hashMust(i.Parameters) == hashMust(c.parameters(a)) && v.Schema == 1 && v.IntentID == i.ID && v.Receipt == "receipt-"+hashMust(i) && v.Digest == digest
}
