//go:build linux

package engine

// These describe representation loss over retained activity rows, not a policy
// for execution or log retention. Sources align with observed_operations.
type RecoveryOperationSource struct {
	Seq       int64 `json:"seq"`
	Truncated bool  `json:"truncated"`
}

type RecoveryActivityProjection struct {
	RetainedRows       int                       `json:"retained_activity_rows"`
	ScannedRows        int                       `json:"scanned_activity_rows"`
	UnscannedRows      int                       `json:"unscanned_activity_rows"`
	ExcludedResultRows int                       `json:"excluded_tool_result_rows"`
	DuplicateRows      int                       `json:"duplicate_rows"`
	OmittedBudgetRows  int                       `json:"omitted_budget_rows"`
	Sources            []RecoveryOperationSource `json:"sources"`
}

// Use the existing control-character and sensitive-text filter before comparing
// or deduplicating. A masked secret is never copied into a source key or hash.
// Widths are display cells, while the activity excerpt's total bound is bytes.
func recoveryText(value string, width int) (complete, projected string) {
	complete = operationText(value, max(1, textWidth(terminalText(value))))
	projected = operationText(value, width)
	return complete, projected
}
