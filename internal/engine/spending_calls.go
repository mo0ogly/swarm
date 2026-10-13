//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
)

// SpendingCall is a recorded attempt/reservation, not evidence that inference
// was sent. Unknown usage remains nil; reported caches keep provider semantics.
// Only a worker agent persists its own requested/observed model attribution.
// Historical planner routes are deliberately not inferred from current settings.
type SpendingCall struct {
	ID             string `json:"id"`
	RecordType     string `json:"record_type"`
	Kind           string `json:"kind"`
	Role           string `json:"role"`
	Scope          string `json:"scope,omitempty"`
	Task           string `json:"task,omitempty"`
	Attempt        string `json:"attempt,omitempty"`
	Provider       string `json:"provider,omitempty"`
	RequestedModel string `json:"requested_model,omitempty"`
	ObservedModel  string `json:"observed_model,omitempty"`
	State          string `json:"state"`
	RegisteredAt   string `json:"registered_at,omitempty"`
	Usage          *Usage `json:"usage"`
}

func spendingAgentCall(w Work, a Agent) SpendingCall {
	call := SpendingCall{ID: "agent/" + a.ID, RecordType: "producer_attempt", Kind: "worker", Role: a.Role, Task: a.TaskID, Attempt: a.Attempt, Provider: a.Provider, State: a.Status, RegisteredAt: a.Started, Usage: a.Usage}
	if task, err := w.task(a.TaskID); err == nil {
		call.Scope = task.ScopeID
	}
	if a.ModelRoute != nil {
		call.RequestedModel = a.ModelRoute.Model
	}
	if a.ReportedModel != nil {
		call.ObservedModel = a.ReportedModel.Model
	}
	return call
}

// Decode JSON null as absence regardless of whitespace. A partial/malformed
// object must not manufacture measured zero token counts.
func decodeSpendingUsage(raw []byte) (*Usage, error) {
	if len(raw) == 0 {
		return nil, nil
	}
	var usage *Usage
	if err := json.Unmarshal(raw, &usage); err != nil {
		return nil, err
	}
	if usage == nil {
		return nil, nil
	}
	var required struct {
		Input  *int64 `json:"input_tokens"`
		Output *int64 `json:"output_tokens"`
	}
	if err := json.Unmarshal(raw, &required); err != nil {
		return nil, err
	}
	if required.Input == nil || required.Output == nil || usage.Input < 0 || usage.Output < 0 {
		return nil, fmt.Errorf("usage enregistré invalide : nombres de jetons requis et non négatifs")
	}
	for _, cache := range []*int64{usage.CacheRead, usage.CacheCreation, usage.CachedInput} {
		if cache != nil && *cache < 0 {
			return nil, fmt.Errorf("usage enregistré invalide : cache négatif")
		}
	}
	if usage.ReportedCost != nil && *usage.ReportedCost < 0 {
		return nil, fmt.Errorf("usage enregistré invalide : coût négatif")
	}
	return usage, nil
}
