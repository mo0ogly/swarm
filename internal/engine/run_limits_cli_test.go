//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func writeTempInput(t *testing.T, raw []byte) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "run-limits.json")
	if err := os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	return path
}

// TestRunLimitsCLIRefusesSameInvalidValuesAsEngine mirrors, through the CLI
// entry point, every rejection TestRunLimitsConfigRejectsInvalidValue
// exercises directly against the store (run_limits_admin_test.go): an
// out-of-bounds max_tool_calls, a negative silence_seconds, an unknown
// scope and a mission scope without a mission id. Same cases, same refusal;
// the CLI adds no bound of its own.
func TestRunLimitsCLIRefusesSameInvalidValuesAsEngine(t *testing.T) {
	s := storeTest(t)

	run := func(scope, mission, key string, body map[string]any) error {
		raw, err := json.Marshal(body)
		if err != nil {
			t.Fatal(err)
		}
		path := writeTempInput(t, raw)
		var out bytes.Buffer
		return s.runLimitsCLI([]string{"run-limits", "apply", scope, mission, key}, path, &out)
	}

	if err := run(ScopeProject, "-", "-", map[string]any{"schema_version": 1, "event_id": "e1", "values": map[string]any{"max_tool_calls": 999999}, "reason": "test"}); err == nil {
		t.Fatal("expected rejection of out-of-bounds max_tool_calls via CLI")
	}
	if err := run(ScopeProject, "-", "-", map[string]any{"schema_version": 1, "event_id": "e2", "values": map[string]any{"silence_seconds": -1}, "reason": "test"}); err == nil {
		t.Fatal("expected rejection of negative silence_seconds via CLI")
	}
	if err := run("invalid-scope", "-", "-", map[string]any{"schema_version": 1, "event_id": "e3", "values": map[string]any{}, "reason": "test"}); err == nil {
		t.Fatal("expected rejection of unknown scope via CLI")
	}
	if err := run(ScopeMission, "-", "-", map[string]any{"schema_version": 1, "event_id": "e4", "values": map[string]any{}, "reason": "test"}); err == nil {
		t.Fatal("expected rejection of mission scope without a mission id via CLI")
	}
	// A valid all-zero override (pure inheritance) must be accepted, exactly
	// as it is directly against the store.
	if err := run(ScopeProject, "-", "-", map[string]any{"schema_version": 1, "event_id": "e5", "values": map[string]any{}, "reason": "no-op"}); err != nil {
		t.Fatalf("all-zero override should be accepted via CLI: %v", err)
	}
}

// TestRunLimitsCLIAppliesShowsHistoryAndRollsBack exercises the full admin
// cycle (apply, show, history, rollback) through the CLI, delegating to the
// same Store methods and asserting the same observable effects T2's tests
// assert directly, plus event_id replay idempotency and revision conflict.
func TestRunLimitsCLIAppliesShowsHistoryAndRollsBack(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)

	apply := func(eventID string, rev int, maxCalls int, reason string) (RunLimitsConfigEntry, error) {
		body, _ := json.Marshal(map[string]any{"schema_version": 1, "event_id": eventID, "expected_revision": rev, "values": map[string]any{"max_tool_calls": maxCalls}, "reason": reason})
		path := writeTempInput(t, body)
		var out bytes.Buffer
		if err := s.runLimitsCLI([]string{"run-limits", "apply", ScopeTask, w.ID, "t1"}, path, &out); err != nil {
			return RunLimitsConfigEntry{}, err
		}
		var entry RunLimitsConfigEntry
		if err := json.Unmarshal(out.Bytes(), &entry); err != nil {
			t.Fatal(err)
		}
		return entry, nil
	}

	first, err := apply("task1", 0, 30, "tâche T1 plus stricte")
	if err != nil {
		t.Fatal(err)
	}
	if first.Revision != 1 || first.Values.MaxToolCalls != 30 {
		t.Fatalf("unexpected apply result: %+v", first)
	}

	var showOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "show", ScopeTask, w.ID, "t1"}, "-", &showOut); err != nil {
		t.Fatal(err)
	}
	var shown RunLimitsConfigEntry
	if err = json.Unmarshal(showOut.Bytes(), &shown); err != nil {
		t.Fatal(err)
	}
	if shown.Revision != 1 || shown.Values.MaxToolCalls != 30 {
		t.Fatalf("show mismatch with apply: %+v", shown)
	}

	var effOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "effective", w.ID, "worker", "t1"}, "-", &effOut); err != nil {
		t.Fatal(err)
	}
	var eff RunLimits
	if err = json.Unmarshal(effOut.Bytes(), &eff); err != nil {
		t.Fatal(err)
	}
	if eff.MaxToolCalls != 30 {
		t.Fatalf("effective resolution via CLI ignores task override: %+v", eff)
	}

	// Stale expected_revision conflicts, exactly like the engine test.
	if _, err = apply("task2", 0, 10, "encore plus strict"); err == nil {
		t.Fatal("expected a revision conflict via CLI")
	}
	second, err := apply("task2", 1, 10, "encore plus strict")
	if err != nil {
		t.Fatal(err)
	}
	if second.Revision != 2 || second.Values.MaxToolCalls != 10 {
		t.Fatalf("second apply unexpected: %+v", second)
	}

	// Replaying the exact same event_id with the same content is idempotent.
	replay, err := apply("task2", 1, 10, "encore plus strict")
	if err != nil {
		t.Fatal(err)
	}
	if replay.Revision != second.Revision {
		t.Fatalf("identical replay via CLI must not create a new revision: got %d want %d", replay.Revision, second.Revision)
	}

	var histOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "history", ScopeTask, w.ID, "t1"}, "-", &histOut); err != nil {
		t.Fatal(err)
	}
	var hist []RunLimitsHistoryEntry
	if err = json.Unmarshal(histOut.Bytes(), &hist); err != nil {
		t.Fatal(err)
	}
	if len(hist) != 2 {
		t.Fatalf("unexpected history length via CLI: %d", len(hist))
	}

	rollbackBody, _ := json.Marshal(map[string]any{"event_id": "task-rollback", "expected_revision": 2, "reason": "task-rollback"})
	rollbackPath := writeTempInput(t, rollbackBody)
	var rollbackOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "rollback", ScopeTask, w.ID, "t1", "1"}, rollbackPath, &rollbackOut); err != nil {
		t.Fatal(err)
	}
	var rolled RunLimitsConfigEntry
	if err = json.Unmarshal(rollbackOut.Bytes(), &rolled); err != nil {
		t.Fatal(err)
	}
	if rolled.Revision != 3 || rolled.Values.MaxToolCalls != 30 {
		t.Fatalf("rollback via CLI unexpected result: %+v", rolled)
	}

	// Rolling back to a non-existent revision fails, same as the engine test.
	badRollbackPath := writeTempInput(t, rollbackBody)
	var badOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "rollback", ScopeTask, w.ID, "t1", "99"}, badRollbackPath, &badOut); err == nil {
		t.Fatal("expected rollback to a non-existent revision to fail via CLI")
	}
}
