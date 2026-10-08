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
    "kind":{"type":"string","enum":["task","delegate","retry","close","review-plan"]},
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
	raw, err := json.Marshal(schema)
	if err != nil {
		panic(err)
	}
	return string(raw)
}
