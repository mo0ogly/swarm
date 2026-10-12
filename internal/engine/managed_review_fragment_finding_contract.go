//go:build linux

package engine

import "strconv"

// Independent from plan/reuse/input-budget versions. Zero preserves the exact
// historical prompt and schema; new packets bind their finding shape explicitly.
const managedFragmentFindingContractVersion = 1

func supportedFragmentFindingContract(version int) bool {
	return version == 0 || version == managedFragmentFindingContractVersion
}

// Use reusable union definitions rather than a distinct union per artifact.
// An inspected result has no needs field; uncertainty and demonstrated defects
// carry needs. Omission is structural and does not depend on unsupported maxItems0
// or conditional-schema keywords. The engine still checks every original and
// rejects contradictory legacy representations after transport.
func managedFragmentStatefulFindingSchema(defs map[string]any, quoteCount int) map[string]any {
	count := quoteCount
	if count == 0 {
		count = 1
	}
	suffix := strconv.Itoa(count)
	complete, pending, choice := "finding-complete-"+suffix, "finding-pending-"+suffix, "finding-choice-"+suffix
	if _, exists := defs[pending]; !exists {
		indexes := []int{0}
		if count > 1 {
			indexes = append(indexes, 1)
		}
		properties := func(verdicts []string) map[string]any {
			return map[string]any{
				"v": map[string]any{"type": "string", "enum": verdicts},
				"r": map[string]any{"$ref": "#/$defs/t"},
				"e": map[string]any{"type": "integer", "enum": indexes},
			}
		}
		inspected := properties([]string{"inspected"})
		uncertain := properties([]string{"unknown", "fail"})
		uncertain["n"] = map[string]any{"$ref": "#/$defs/n"}
		defs[complete] = map[string]any{"type": "object", "additionalProperties": false, "required": []string{"v", "r", "e"}, "properties": inspected}
		defs[pending] = map[string]any{"type": "object", "additionalProperties": false, "required": []string{"v", "r", "e", "n"}, "properties": uncertain}
		defs[choice] = map[string]any{"anyOf": []any{
			map[string]any{"$ref": "#/$defs/" + complete},
			map[string]any{"$ref": "#/$defs/" + pending},
		}}
	}
	if quoteCount == 0 {
		return map[string]any{"$ref": "#/$defs/" + pending}
	}
	return map[string]any{"$ref": "#/$defs/" + choice}
}
