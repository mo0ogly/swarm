//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"testing"
)

func TestMissionSpendingCallLedgerAttributesMissingUsageAndCaches(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	foreign := createTest(t, s)
	cost := 0.0
	cached := int64(40)
	agents := []Agent{{ID: "producer-1", WorkID: w.ID, TaskID: "task1", Attempt: "attempt1", Provider: "codex", Role: "worker", Status: "completed", Started: "2026-10-13T10:00:00Z", Usage: &Usage{Input: 100, Output: 25, CachedInput: &cached, ReportedCost: &cost}}, {ID: "producer-2", WorkID: w.ID, TaskID: "task1", Attempt: "attempt2", Provider: "fault-fixture", Role: "worker", Status: "failed", Started: "2026-10-13T10:00:01Z"}}
	agents = append(agents, Agent{ID: "foreign-agent", WorkID: foreign.ID, TaskID: "other", Usage: &Usage{Input: 900, Output: 900}})
	usage, _ := json.Marshal(Usage{Input: 20, Output: 30})
	for _, row := range []struct {
		id, work, scope, state string
		usage                  []byte
	}{{"planning-1", w.ID, "root", "finished", usage}, {"review-1", w.ID, "reviewer", "estimated", nil}, {"released-1", w.ID, "root", "released", nil}, {"foreign-1", foreign.ID, "root", "finished", usage}} {
		if _, err := s.db.Exec("INSERT INTO planning_calls(id,work_id,scope_id,estimate,usage,state,created_at) VALUES(?,?,?,0,?,?,?)", row.id, row.work, row.scope, row.usage, row.state, "2026-10-13T10:00:02Z"); err != nil {
			t.Fatal(err)
		}
	}
	got, err := s.missionSpending(w, agents)
	if err != nil {
		t.Fatal(err)
	}
	raw, err := json.Marshal(got)
	if err != nil {
		t.Fatal(err)
	}
	var decoded struct {
		Calls []struct {
			ID, Kind, Scope, Task, Attempt, Provider, State string
			Usage                                           *Usage
		}
	}
	if err = json.Unmarshal(raw, &decoded); err != nil {
		t.Fatal(err)
	}
	if len(decoded.Calls) != 4 {
		t.Fatalf("expected four attributable records, got %d: %s", len(decoded.Calls), raw)
	}
	by := map[string]*Usage{}
	seen := map[string]bool{}
	for _, call := range decoded.Calls {
		if seen[call.ID] {
			t.Fatal("duplicate ledger record", call.ID)
		}
		seen[call.ID] = true
		by[call.ID] = call.Usage
		if call.ID == "agent/producer-1" && (call.Task != "task1" || call.Attempt != "attempt1" || call.Provider != "codex" || call.State != "completed") {
			t.Fatal("lost producer attribution", call)
		}
		if call.ID == "planning/review-1" && (call.Kind != "reviewer" || call.Scope != "reviewer" || call.State != "estimated" || call.Usage != nil) {
			t.Fatal("missing usage inferred as measured", call)
		}
	}
	if by["agent/producer-1"] == nil || by["agent/producer-1"].CachedInput == nil || *by["agent/producer-1"].CachedInput != 40 || by["agent/producer-1"].ReportedCost == nil || *by["agent/producer-1"].ReportedCost != 0 {
		t.Fatal("explicit cache/zero cost lost", by)
	}
	if by["agent/producer-2"] != nil {
		t.Fatal("unreported usage became zeros")
	}
	var in, out int64
	for _, row := range got.Rows {
		in += row.Input
		out += row.Output
	}
	if in != 120 || out != 55 {
		t.Fatal("ledger duplicated aggregate or added cache", in, out)
	}
	if _, err = s.db.Exec("UPDATE planning_calls SET usage=? WHERE id='review-1'", []byte(" \n null \t")); err != nil {
		t.Fatal(err)
	}
	nullRecord, err := s.missionSpending(w, agents)
	if err != nil {
		t.Fatal(err)
	}
	nullRaw, _ := json.Marshal(nullRecord)
	var nullable struct {
		Calls []struct {
			ID    string
			Usage *Usage
		}
	}
	if err = json.Unmarshal(nullRaw, &nullable); err != nil {
		t.Fatal(err)
	}
	for _, call := range nullable.Calls {
		if call.ID == "planning/review-1" && call.Usage != nil {
			t.Fatal("JSON null became reported zeros", call.Usage)
		}
	}
	for _, row := range nullRecord.Rows {
		if row.Kind == "reviewer" && row.MissingUsage != 1 {
			t.Fatal("missing usage lost", row)
		}
	}
	for _, invalid := range []string{`{"input_tokens":"invalid"}`, `{}`, `{"input_tokens":-1,"output_tokens":0}`, `{"input_tokens":0,"output_tokens":0,"cached_input_tokens":-1}`, `{"input_tokens":0,"output_tokens":0,"provider_reported_cost_usd":-1}`} {
		if _, err = s.db.Exec("UPDATE planning_calls SET usage=? WHERE id='planning-1'", []byte(invalid)); err != nil {
			t.Fatal(err)
		}
		if _, err = s.missionSpending(w, agents); err == nil {
			t.Fatal("invalid received structure swallowed", invalid)
		}
	}
}

func TestMissionSpendingPreservesCompleteWorkerHistoryAndRetries(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	foreign := createTest(t, s)
	for i := 0; i < 201; i++ {
		a := Agent{ID: fmt.Sprintf("ledger-%03d", i), WorkID: w.ID, TaskID: "same-task", Attempt: fmt.Sprintf("attempt-%03d", i), Status: "completed", Usage: &Usage{Input: 1, Output: 2}}
		raw, _ := json.Marshal(a)
		if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, raw, []byte("{}")); err != nil {
			t.Fatal(err)
		}
	}
	a := Agent{ID: "foreign-worker", WorkID: foreign.ID, TaskID: "same-task", Status: "completed"}
	raw, _ := json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, foreign.ID, a.TaskID, s.root, a.Status, raw, []byte("{}")); err != nil {
		t.Fatal(err)
	}
	recent, err := s.agents(w.ID)
	if err != nil || len(recent) != 200 {
		t.Fatal("recent UI list changed", len(recent), err)
	}
	status, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if len(status.Spending.Calls) != 201 {
		t.Fatalf("silent worker history truncation: %d instead of 201", len(status.Spending.Calls))
	}
	seen := map[string]bool{}
	for _, call := range status.Spending.Calls {
		seen[call.ID] = true
	}
	if !seen["agent/ledger-000"] || seen["agent/foreign-worker"] {
		t.Fatal("oldest record lost or foreign history leaked", seen)
	}
	if status.Spending.Retries != 200 {
		t.Fatalf("ordinary supervisor retries lost: %d", status.Spending.Retries)
	}
	for _, row := range status.Spending.Rows {
		if row.Kind == "worker" && (row.Input != 201 || row.Output != 402 || row.TaskRetries != 200) {
			t.Fatal("incorrect complete aggregate", row)
		}
	}
}
