package engine

import (
	"encoding/json"
	"testing"
)

func TestPlanningSchemaAdmitsOnlyCurrentEventBatch(t *testing.T) {
	for _, events := range [][]string{{"current-child-closed"}, {"current-a", "current-b"}, nil} {
		var schema map[string]any
		if err := json.Unmarshal([]byte(planningSchemaForEvents(events)), &schema); err != nil {
			t.Fatal(err)
		}
		inputs := schema["properties"].(map[string]any)["input_events"].(map[string]any)
		if len(events) == 0 {
			if inputs["maxItems"] != float64(0) {
				t.Fatal(inputs)
			}
			continue
		}
		allowed := inputs["items"].(map[string]any)["enum"].([]any)
		if len(allowed) != len(events) {
			t.Fatal(allowed)
		}
		for i, event := range events {
			if allowed[i] != event {
				t.Fatal(allowed)
			}
		}
		for _, event := range allowed {
			if event == "historical-root-event" {
				t.Fatal("historical input admitted")
			}
		}
	}
}
