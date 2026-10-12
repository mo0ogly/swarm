//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPlanningUnrelatedNextPreviewsCannotStarveWholeCurrentEvent(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	script := filepath.Join(t.TempDir(), "claude")
	body := "#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' '{\"type\":\"result\",\"structured_output\":{\"input_events\":[\"current-refusal\"],\"reason\":\"Observe the complete refusal; acceptance remains unproven\",\"operations\":[]}}'\n"
	if err := os.WriteFile(script, []byte(body), 0700); err != nil {
		t.Fatal(err)
	}
	config, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"test": {Command: script}}})
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), config, 0600); err != nil {
		t.Fatal(err)
	}
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "test", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
	_, methods, err := s.projectAgentWorkflow("planner")
	if err != nil {
		t.Fatal(err)
	}
	limit := planningPromptContextLimit(methods)
	w, err = s.mutate(w.ID, "test.preview", "large-current-refusal", w.Revision, []byte(`{}`), func(w *Work) error {
		for i := len(w.Criteria); i < 29; i++ {
			w.Criteria = append(w.Criteria, strings.Repeat("Requirement remains owned and fully described. ", 7))
			w.Planning.Scopes[0].Requirements = append(w.Planning.Scopes[0].Requirements, fmt.Sprintf("req-%d", i+1))
		}
		for i := 0; i < 8; i++ {
			w.Tasks = append(w.Tasks, Task{ID: fmt.Sprintf("task-%d", i), Title: "Existing task", ScopeID: "root", Status: "todo", Next: strings.Repeat("future instruction ", 80), Requirements: []string{"req-1"}, PlanMaxAttempts: 2})
		}
		w.Tasks[1].Status = "blocked"
		w.Planning.Inbox = []PlanningEvent{{ID: "current-refusal", Scope: "root", Kind: "integration_failed", Task: "task-1"}}
		base, err := planningContextLimit(*w, "root", 1)
		if err != nil {
			return err
		}
		size := limit - len(base) + 256
		if size < 1 {
			t.Fatal("fixture state alone exceeds envelope")
		}
		w.Planning.Inbox[0].Message = strings.Repeat("r", size)
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.planningStep(w.ID); err != nil {
		t.Fatal("unrelated instruction previews prevented owner activation", err)
	}
	request, err := os.ReadFile(script + ".prompt")
	if err != nil || len(request) > 64000 {
		t.Fatal("complete provider envelope missing or enlarged", err, len(request))
	}
	var ctx struct {
		Events  []PlanningEvent  `json:"events"`
		Tasks   []map[string]any `json:"tasks"`
		IDs     []string         `json:"existing_task_ids"`
		Omitted bool             `json:"inactive_task_next_omitted"`
	}
	raw := request[bytes.LastIndexByte(request, '\n')+1:]
	if err = json.Unmarshal(raw, &ctx); err != nil || !ctx.Omitted || len(ctx.IDs) != 8 || len(ctx.Tasks) != 8 || len(ctx.Events) != 1 || ctx.Events[0].Message != w.Planning.Inbox[0].Message {
		t.Fatal("projection lost inventory or truncated current evidence", err)
	}
	for _, task := range ctx.Tasks {
		if task["id"] == "task-1" && task["next"] != guardBlock(w.Tasks[1].Next, 1000) {
			t.Fatal("current task instruction preview was removed")
		}
	}
	got, err := s.get(w.ID)
	if err != nil || got.Planning.Activations != 1 || got.Planning.Decisions != 1 || got.Tasks[1].Status != "blocked" || got.Planning.Scopes[0].Delivery.SHA256 != hash(raw) {
		t.Fatal("receipt, counters or acceptance changed incorrectly", err)
	}
}
