package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestPlanningOperationSchemaSeparatesMutationKinds(t *testing.T) {
	var schema map[string]any
	if err := json.Unmarshal([]byte(planningSchemaForDelivery(PlanningDelivery{})), &schema); err != nil {
		t.Fatal(err)
	}
	items := schema["properties"].(map[string]any)["operations"].(map[string]any)["items"].(map[string]any)
	variants := items["anyOf"].([]any)
	if len(variants) != 9 {
		t.Fatalf("operation kinds lost: %d", len(variants))
	}
	for _, value := range variants {
		object := value.(map[string]any)
		props := object["properties"].(map[string]any)
		kind := props["kind"].(map[string]any)["enum"].([]any)[0].(string)
		if kind == "review-retry" || kind == "revise-instructions" || kind == "continue-run" || kind == "stop-run" {
			if len(props) != 3 || props["id"] == nil || props["next"] == nil || object["additionalProperties"] != false {
				t.Fatalf("%s can still contain unrelated plan fields", kind)
			}
		}
	}
	output := os.Getenv("SWARM_FINDING_SCHEMA_ORACLE_DIR")
	if output != "" {
		if err := os.MkdirAll(output, 0700); err != nil {
			t.Fatal(err)
		}
		cases := map[string]any{
			"planner-current-schema.json":    json.RawMessage(planningSchemaForDelivery(PlanningDelivery{})),
			"planner-no-retries-schema.json": json.RawMessage(planningSchemaForDelivery(PlanningDelivery{ProductionRetryUnavailable: true, ReviewRetryUnavailable: true})),
			"planner-legacy-schema.json":     json.RawMessage(planningProposalSchema),
		}
		for name, value := range cases {
			raw, err := json.MarshalIndent(value, "", "  ")
			if err != nil {
				t.Fatal(err)
			}
			if err = os.WriteFile(filepath.Join(output, name), raw, 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
}

// Inspect the offered operations, without coupling availability assertions to
// the old flat object representation. Every current branch carries one kind.
func planningOperationSchemaKinds(t *testing.T, raw string) []string {
	t.Helper()
	var schema struct {
		Properties map[string]struct {
			Items struct {
				AnyOf []struct {
					Properties map[string]json.RawMessage `json:"properties"`
				} `json:"anyOf"`
			} `json:"items"`
		} `json:"properties"`
	}
	if err := json.Unmarshal([]byte(raw), &schema); err != nil {
		t.Fatal(err)
	}
	kinds := []string{}
	for _, branch := range schema.Properties["operations"].Items.AnyOf {
		var kind struct {
			Enum []string `json:"enum"`
		}
		if err := json.Unmarshal(branch.Properties["kind"], &kind); err != nil {
			t.Fatal(err)
		}
		values := kind.Enum
		if len(values) != 1 {
			t.Fatalf("operation branch has %d kinds", len(values))
		}
		kinds = append(kinds, values[0])
	}
	if len(kinds) == 0 {
		t.Fatal("no operation branches")
	}
	return kinds
}

// Constrained output follows property order. The common discriminator must be
// chosen before branch-specific required payload (criteria vs id, for example).
func TestPlanningSchemaChoosesKindBeforePayload(t *testing.T) {
	for _, delivery := range []PlanningDelivery{{}, {JudgeAvailable: true, JudgeCycle: 2, ExplorationAvailable: true}, {HistoricalOnly: true, DelegationUnavailable: true}} {
		var schema struct {
			Properties map[string]struct {
				Items struct {
					AnyOf []struct {
						Properties json.RawMessage `json:"properties"`
					} `json:"anyOf"`
				} `json:"items"`
			} `json:"properties"`
		}
		if err := json.Unmarshal([]byte(planningSchemaForDelivery(delivery)), &schema); err != nil {
			t.Fatal(err)
		}
		for _, branch := range schema.Properties["operations"].Items.AnyOf {
			decoder := json.NewDecoder(bytes.NewReader(branch.Properties))
			if token, err := decoder.Token(); err != nil || token != json.Delim('{') {
				t.Fatal("invalid properties object", token, err)
			}
			if key, err := decoder.Token(); err != nil || key != "kind" {
				t.Fatalf("payload determines branch before discriminator: first=%v err=%v", key, err)
			}
		}
	}
}
