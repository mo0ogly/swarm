package engine

import "encoding/json"

// All fields are required for structured-output adapters; unused fields remain
// empty. This is a wire format, not authority to execute an operation.
const planningProposalSchema = `{
 "type":"object","additionalProperties":false,
 "required":["input_events","reason","operations"],
 "properties":{
  "input_events":{"type":"array","items":{"type":"string"}},
  "reason":{"type":"string"},
  "operations":{"type":"array","items":{
   "type":"object","additionalProperties":false,
   "required":["kind","id","title","requirements","deliverable","criteria","depends","next","max_tasks","max_activations"],
   "properties":{
    "kind":{"type":"string","enum":["task","delegate","retry","close","review-plan","review-retry","continue-run","stop-run","revise-instructions","explore","revise-dependencies"]},
    "id":{"type":"string"},"title":{"type":"string","maxLength":500},
    "requirements":{"type":"array","items":{"type":"string"}},
    "deliverable":{"type":"string"},
    "criteria":{"type":"array","items":{"type":"string"}},
    "depends":{"type":"array","items":{"type":"string"}},
    "next":{"type":"string","maxLength":2000},"max_tasks":{"type":"integer","minimum":0,"maximum":100},"max_activations":{"type":"integer","minimum":0,"maximum":200}
   }
  }}
 }
}`

// Restrict structured output to the exact event batch admitted by the claim.
// Historical IDs appearing inside event payloads are context, not new inputs.
func planningSchemaForEvents(events []string) string {
	return planningSchemaForDelivery(PlanningDelivery{Events: events})
}

func planningSchemaForDelivery(delivery PlanningDelivery) string {
	events := delivery.Events
	var schema map[string]any
	if err := json.Unmarshal([]byte(planningProposalSchema), &schema); err != nil {
		panic(err)
	}
	properties := schema["properties"].(map[string]any)
	inputs := properties["input_events"].(map[string]any)
	if len(events) == 0 {
		inputs["maxItems"] = 0
	} else {
		inputs["items"] = map[string]any{"type": "string", "enum": events}
	}
	if delivery.HistoricalOnly {
		properties["operations"].(map[string]any)["maxItems"] = 0
	}
	if delivery.DelegationUnavailable || delivery.ClosureUnavailable || delivery.ProductionRetryUnavailable || delivery.ReviewRetryUnavailable || !delivery.ExplorationAvailable {
		operations := properties["operations"].(map[string]any)
		kinds := operations["items"].(map[string]any)["properties"].(map[string]any)["kind"].(map[string]any)
		available := []any{}
		for _, kind := range kinds["enum"].([]any) {
			if !(kind == "delegate" && delivery.DelegationUnavailable) && !(kind == "close" && delivery.ClosureUnavailable) && !((kind == "explore" || kind == "revise-dependencies") && !delivery.ExplorationAvailable) && !(kind == "retry" && delivery.ProductionRetryUnavailable) && !(kind == "review-retry" && delivery.ReviewRetryUnavailable) {
				available = append(available, kind)
			}
		}
		kinds["enum"] = available
	}
	// Match the operation-specific contract before generation. The historical
	// full wire schema remains available for reading old representations.
	operations := properties["operations"].(map[string]any)
	legacyItem := operations["items"].(map[string]any)
	legacyProperties := legacyItem["properties"].(map[string]any)
	// Fresh receipts offer root-owned cycle operations. The historical flat
	// schema is unchanged; absence of these receipt flags grants no new action.
	offered := legacyProperties["kind"].(map[string]any)["enum"].([]any)
	if delivery.JudgeAvailable {
		offered = append(offered, "judge")
	}
	if delivery.RenewAvailable {
		offered = append(offered, "renew")
	}
	legacyProperties["kind"].(map[string]any)["enum"] = offered
	verdicts := []string{"continue"}
	if delivery.JudgeEndAvailable {
		verdicts = append(verdicts, "end")
	}
	legacyProperties["judge_verdict"] = map[string]any{"type": "string", "enum": verdicts}
	legacyProperties["judge_cycle"] = map[string]any{"type": "integer", "minimum": 1}
	if delivery.JudgeCycle > 0 {
		legacyProperties["judge_cycle"].(map[string]any)["enum"] = []int{delivery.JudgeCycle}
	}
	legacyProperties["expected_candidate"] = map[string]any{"type": "string"}
	// Scope allocations are constrained by the actual persisted parent budget,
	// rather than a second fixed ceiling in the provider's output schema.
	if delivery.TaskBudget > 0 {
		legacyProperties["max_tasks"].(map[string]any)["maximum"] = delivery.TaskBudget
	}
	if delivery.ActivationBudget > 0 {
		legacyProperties["max_activations"].(map[string]any)["maximum"] = delivery.ActivationBudget
	}
	kinds := legacyProperties["kind"].(map[string]any)["enum"].([]any)
	variants := []any{}
	for _, rawKind := range kinds {
		kind := rawKind.(string)
		fields := planningOperationSchemaFields(kind)
		props := map[string]any{"kind": map[string]any{"type": "string", "enum": []string{kind}}}
		required := []string{"kind"}
		for _, field := range fields {
			props[field] = legacyProperties[field]
			required = append(required, field)
		}
		variants = append(variants, map[string]any{"type": "object", "additionalProperties": false, "required": required, "properties": props})
	}
	operations["items"] = map[string]any{"anyOf": variants}
	raw, err := json.Marshal(schema)
	if err != nil {
		panic(err)
	}
	return string(raw)
}

// Only meaningful fields of each existing engine operation are supplied. This
// changes output shape, not scope, evidence, rights or execution authorization.
func planningOperationSchemaFields(kind string) []string {
	switch kind {
	case "judge":
		return []string{"judge_cycle", "judge_verdict", "expected_candidate"}
	case "renew":
		return []string{"judge_cycle"}
	case "revise-dependencies":
		return []string{"id", "depends"}
	case "task":
		return []string{"id", "title", "requirements", "deliverable", "criteria", "depends", "next"}
	case "delegate":
		return []string{"id", "title", "requirements", "next", "max_tasks", "max_activations"}
	case "review-plan", "explore":
		return []string{"id", "deliverable"}
	case "close":
		return []string{"title"}
	default:
		return []string{"id", "next"}
	}
}
