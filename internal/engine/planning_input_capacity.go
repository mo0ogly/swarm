//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
)

func (s *Store) planningInputCapacity(w Work, id string) *managedReviewInputBudget {
	if w.Planning == nil {
		return nil
	}
	scope, err := w.Planning.scope(id)
	if err != nil {
		return nil
	}
	p := effectivePlanningScope(w.Planning, scope)
	b, err := s.reviewInputBudget(&ReviewerConfig{Provider: p.Provider, ProviderDigest: p.ProviderDigest, ModelRoute: p.ModelRoute})
	if err != nil {
		return nil
	}
	return b
}
func (s *Store) planningCapacityContext(w Work, scope, workflow string, budget *managedReviewInputBudget, inputs map[string]any) ([]byte, PlanningDelivery, error) {
	limit := planningPromptContextLimit(workflow)
	if budget != nil {
		room, err := budget.inputTokens()
		if err != nil {
			return nil, PlanningDelivery{}, err
		}
		limit = min(budget.MaxBytes, room) - len(workflow) - len(planningInstructions) - len(planningProposalSchema)
	}
	raw, delivery, err := s.planningDeliveryContext(w, scope, limit, inputs)
	if err != nil {
		return nil, delivery, err
	}
	if budget != nil {
		var context map[string]json.RawMessage
		if err = json.Unmarshal(raw, &context); err != nil {
			return nil, delivery, err
		}
		context["observed_model_input_capacity"], _ = json.Marshal(budget)
		raw, _ = json.Marshal(context)
		delivery.SHA256 = hash(raw)
		if _, err = budget.measure(workflow+planningInstructions+string(raw), planningSchemaForDelivery(delivery)); err != nil {
			return nil, delivery, fmt.Errorf("capacité réelle de planification : %w", err)
		}
	}
	return raw, delivery, nil
}
