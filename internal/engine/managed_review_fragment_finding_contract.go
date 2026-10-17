//go:build linux

package engine

import "strconv"

// Independent from plan/reuse/input-budget versions. Zero preserves the exact
// historical prompt and schema; new packets bind their finding shape explicitly.
const managedFragmentFindingContractVersion = 2

func supportedFragmentFindingContract(version int) bool {
	return version == 0 || version == 1 || version == managedFragmentFindingContractVersion
}

func managedFragmentNestedFindingSchema(defs map[string]any, quoteCount int) map[string]any {
	indexes := []int{0}
	if quoteCount > 1 {
		indexes = append(indexes, 1)
	}
	suffix := strconv.Itoa(quoteCount)
	choice := "finding-nested-" + suffix
	if _, exists := defs[choice]; !exists {
		states := []any{}
		for _, verdict := range []string{"inspected", "unknown", "fail"} {
			if quoteCount == 0 && verdict == "inspected" {
				continue
			}
			props := map[string]any{"v": map[string]any{"type": "string", "enum": []string{verdict}}, "r": map[string]any{"$ref": "#/$defs/t"}, "e": map[string]any{"type": "integer", "enum": indexes}}
			required := []string{"v", "r", "e"}
			if verdict == "unknown" {
				props["n"] = map[string]any{"type": "array", "minItems": 1, "maxItems": 16, "items": map[string]any{"type": "string", "minLength": 3, "maxLength": 240}}
				required = append(required, "n")
			}
			if verdict == "fail" {
				props["d"] = map[string]any{"$ref": "#/$defs/finding-defects"}
				required = append(required, "d")
			}
			states = append(states, map[string]any{"type": "object", "additionalProperties": false, "properties": props, "required": required})
		}
		defs[choice] = map[string]any{"anyOf": states}
		defects := managedFragmentDefectQuoteOnlySchema()
		defects["minItems"] = 1
		item := defects["items"].(map[string]any)
		delete(item["properties"].(map[string]any), "artifact")
		item["required"] = []string{"quote", "explanation", "reproduction", "expected"}
		defs["finding-defects"] = defects
	}
	return map[string]any{"$ref": "#/$defs/" + choice}
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
