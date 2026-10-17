//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strings"
	"testing"
)

// Decode the public prompt representation, independently of its producer's types.
func recoveryProjectionJSON(t *testing.T, prompt string) map[string]json.RawMessage {
	t.Helper()
	const marker = "MÉMOIRE DE REPRISE FOURNIE PAR LE MOTEUR :\n"
	_, rest, ok := strings.Cut(prompt, marker)
	if !ok {
		t.Fatal("recovery projection absent")
	}
	line, _, _ := strings.Cut(rest, "\n")
	var value map[string]json.RawMessage
	if err := json.Unmarshal([]byte(line), &value); err != nil {
		t.Fatal(err)
	}
	return value
}

func recoveryProjectionForFixture(t *testing.T, s *Store, w Work, a Agent) map[string]json.RawMessage {
	t.Helper()
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	prompt, err := workerRecoveryContext(tx, a, w.ID, a.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	return recoveryProjectionJSON(t, prompt)
}

func TestRecoveryProjectionSignalsTruncatedOperation(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	if err := s.log(a.ID, "activity", "Read · "+strings.Repeat("long-path/", 35)); err != nil {
		t.Fatal(err)
	}
	v := recoveryProjectionForFixture(t, s, w, a)
	if string(v["bounded_excerpt"]) != "true" {
		t.Fatal("shortened activity presented as unbounded", string(v["observed_operations"]))
	}
}

func TestRecoveryProjectionSignalsEveryTruncatedFocusField(t *testing.T) {
	for _, field := range []string{"next_action", "blocker", "criteria.1.expected", "historical_report"} {
		t.Run(field, func(t *testing.T) {
			s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
			task, _ := w.task(a.TaskID)
			long := strings.Repeat("界", 320)
			switch field {
			case "next_action":
				task.Next = long
			case "blocker":
				task.Blocker = long
			case "criteria.1.expected":
				task.Criteria = []string{long}
			case "historical_report":
				task.IndependentReview = &IndependentReview{ID: "review-projection", Producer: a.ID, Attempt: a.Attempt, Report: long}
			}
			raw, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			v := recoveryProjectionForFixture(t, s, w, a)
			var focus struct {
				Bounded   bool     `json:"bounded_excerpt"`
				Truncated []string `json:"truncated_fields"`
			}
			if err := json.Unmarshal(v["task_focus"], &focus); err != nil {
				t.Fatal(err)
			}
			if !focus.Bounded || string(v["bounded_excerpt"]) != "true" {
				t.Fatalf("shortened %s presented as unbounded: focus=%t whole=%s", field, focus.Bounded, v["bounded_excerpt"])
			}
			if len(focus.Truncated) != 1 || focus.Truncated[0] != field {
				t.Fatalf("missing field attribution: %+v", focus.Truncated)
			}
		})
	}
}

func TestRecoveryProjectionRetainsDistinctLongOperations(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	for _, suffix := range []string{"first.go", "second.go"} {
		if err := s.log(a.ID, "activity", "Read · "+strings.Repeat("common/", 40)+suffix); err != nil {
			t.Fatal(err)
		}
	}
	v := recoveryProjectionForFixture(t, s, w, a)
	var operations []string
	if err := json.Unmarshal(v["observed_operations"], &operations); err != nil {
		t.Fatal(err)
	}
	if len(operations) != 2 {
		t.Fatalf("different long operations collapsed after shortening: %d retained", len(operations))
	}
}

func TestRecoveryProjectionPropagatesCriterionOmission(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	task, _ := w.task(a.TaskID)
	// Retained legacy/corrupt fixture: public task updates currently admit at most 12.
	task.Criteria = make([]string, 13)
	for i := range task.Criteria {
		task.Criteria[i] = "Still requires verification"
	}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	v := recoveryProjectionForFixture(t, s, w, a)
	if string(v["bounded_excerpt"]) != "true" {
		t.Fatal("criterion omission absent from overall marker")
	}
	var focus struct {
		Omitted int `json:"omitted_criteria_count"`
	}
	if err := json.Unmarshal(v["task_focus"], &focus); err != nil || focus.Omitted != 1 {
		t.Fatal("criterion omission not counted", focus, err)
	}
}

func TestRecoveryProjectionAccountsForEveryRetainedActivityRow(t *testing.T) {
	for _, scenario := range []string{"empty", "complete", "filtered", "row-cap", "byte-cap", "scan-cap"} {
		t.Run(scenario, func(t *testing.T) {
			s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
			var lines []string
			switch scenario {
			case "complete":
				lines = []string{"Read · first.go", "Test · second.go"}
			case "filtered":
				lines = []string{"Read · first.go", "Read · first.go", "Résultat d'outil reçu · hidden", "Bash · token=private-first", "Bash · password=private-second"}
			case "row-cap", "byte-cap", "scan-cap":
				n := 26
				if scenario == "scan-cap" {
					n = 165
				}
				for i := 0; i < n; i++ {
					line := fmt.Sprintf("Read · %03d.go", i)
					if scenario == "byte-cap" {
						line += strings.Repeat("界", 100)
					}
					lines = append(lines, line)
				}
			}
			for _, line := range lines {
				if err := s.log(a.ID, "activity", line); err != nil {
					t.Fatal(err)
				}
			}
			if err := s.log(a.ID, "output", "raw private tool result"); err != nil {
				t.Fatal(err)
			}
			logs, err := s.logs(a.ID, -1)
			if err != nil {
				t.Fatal(err)
			}
			before, _ := json.Marshal(logs)
			v := recoveryProjectionForFixture(t, s, w, a)
			for field, expected := range map[string]string{"work": w.ID, "task": a.TaskID, "agent": a.ID, "attempt": a.Attempt} {
				var got string
				if err := json.Unmarshal(v[field], &got); err != nil || got != expected {
					t.Fatalf("wrong %s source: %s %v", field, got, err)
				}
			}
			var p struct {
				Retained  int `json:"retained_activity_rows"`
				Scanned   int `json:"scanned_activity_rows"`
				Unscanned int `json:"unscanned_activity_rows"`
				Excluded  int `json:"excluded_tool_result_rows"`
				Duplicate int `json:"duplicate_rows"`
				Omitted   int `json:"omitted_budget_rows"`
				Sources   []struct {
					Seq       int64 `json:"seq"`
					Truncated bool  `json:"truncated"`
				} `json:"sources"`
			}
			if err := json.Unmarshal(v["activity_projection"], &p); err != nil {
				t.Fatal(err)
			}
			var operations []string
			if err := json.Unmarshal(v["observed_operations"], &operations); err != nil {
				t.Fatal(err)
			}
			if p.Retained != len(lines) || p.Scanned != min(160, len(lines)) || p.Unscanned != p.Retained-p.Scanned || p.Scanned != p.Excluded+p.Duplicate+p.Omitted+len(operations) || len(p.Sources) != len(operations) || len(operations) > 24 {
				t.Fatalf("activity rows unaccounted for: %+v / %d operations", p, len(operations))
			}
			if scenario == "filtered" && (p.Excluded != 1 || p.Duplicate != 2 || len(operations) != 2) {
				t.Fatalf("filters not attributed: %+v", p)
			}
			if scenario == "row-cap" && (len(operations) != 24 || p.Omitted != 2) {
				t.Fatalf("row limit changed: %+v", p)
			}
			if scenario == "byte-cap" && (len(operations) >= 24 || p.Omitted == 0) {
				t.Fatalf("byte limit not exercised: %+v", p)
			}
			if scenario == "scan-cap" && p.Unscanned != 5 {
				t.Fatalf("scan omission missing: %+v", p)
			}
			used := 0
			for i, source := range p.Sources {
				if i > 0 && source.Seq <= p.Sources[i-1].Seq {
					t.Fatal("source order differs from chronology")
				}
				found := false
				for _, log := range logs {
					if log.Seq != source.Seq {
						continue
					}
					found = true
					complete, projected := recoveryText(log.Message, 240)
					if log.AgentID != a.ID || log.Kind != "activity" || operations[i] != projected || source.Truncated != (projected != complete) {
						t.Fatal("operation source does not explain the projected row")
					}
				}
				if !found {
					t.Fatal("source sequence missing from retained logs")
				}
				used += len(operations[i])
			}
			if used > 4500 {
				t.Fatal("activity byte bound changed", used)
			}
			wantBounded := scenario == "row-cap" || scenario == "byte-cap" || scenario == "scan-cap"
			if (string(v["bounded_excerpt"]) == "true") != wantBounded {
				t.Fatal("misleading bounded marker", scenario)
			}
			raw, _ := json.Marshal(v)
			for _, forbidden := range []string{"private-first", "private-second", "raw private tool result", "hidden"} {
				if strings.Contains(string(raw), forbidden) {
					t.Fatal("untrusted raw data copied into projection")
				}
			}
			logs, err = s.logs(a.ID, -1)
			after, _ := json.Marshal(logs)
			if err != nil || string(before) != string(after) {
				t.Fatal("projection changed retained logs", err)
			}
		})
	}
}

func TestRecoveryProjectionSanitizationIsNotReportedAsWidthTruncation(t *testing.T) {
	for _, value := range []string{"", "small\npath\tname\x00", "token=" + strings.Repeat("private", 80), strings.Repeat("e\u0301", 10)} {
		complete, projected := recoveryText(value, 240)
		if complete != projected || strings.Contains(projected, "private") || strings.ContainsAny(projected, "\n\t\x00") {
			t.Fatalf("sanitization incorrectly treated as width loss: %q", projected)
		}
	}
}

func TestRecoveryProjectionRejectsMissingTask(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	w.Tasks = nil
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	if context, err := workerRecoveryContext(tx, a, w.ID, a.TaskID); err == nil || context != "" {
		t.Fatal("missing current task became a successful projection")
	}
}
