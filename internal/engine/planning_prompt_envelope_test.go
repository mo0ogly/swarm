//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The provider double records the actual complete request. Enough whole events
// fit the old estimated context budget but overflow the complete wire envelope.
func TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	capture := filepath.Join(t.TempDir(), "received-prompt")
	script := filepath.Join(t.TempDir(), "claude")
	body := "#!/bin/sh\ncat > '" + capture + "'\nprintf '%s\\n' '{\"type\":\"result\",\"structured_output\":{\"input_events\":[\"event-7\"],\"reason\":\"Handle newest actual input and retain the rest\",\"operations\":[]}}'\n"
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
	oldBudget := 64000 - len(methods) - 3000
	w, err = s.mutate(w.ID, "test.envelope", "near-wire-bound", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Planning.Inbox = nil
		for i := 0; i < 8; i++ {
			w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "event-" + string(rune('0'+i)), Scope: "root", Kind: "operator_resume"})
		}
		base, _, e := s.planningDeliveryContext(*w, "root", oldBudget)
		if e != nil {
			return e
		}
		size := (oldBudget - len(base) - 256) / 8
		if size < 1 {
			t.Fatal("fixture cannot saturate envelope")
		}
		for i := range w.Planning.Inbox {
			w.Planning.Inbox[i].Message = strings.Repeat("x", size)
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.planningStep(w.ID); err != nil {
		t.Fatal("fitting whole-event batch failed before provider", err)
	}
	request, err := os.ReadFile(capture)
	if err != nil || len(request) > 64000 {
		t.Fatal("provider did not receive bounded full request", len(request), err)
	}
	got, err := s.get(w.ID)
	if err != nil || got.Planning.Failure != "" || got.Planning.Activations != 1 || got.Planning.Decisions != 1 {
		t.Fatal("bounded activation not applied", got.Planning, err)
	}
	owner, _ := got.Planning.scope("root")
	raw := request[bytes.LastIndexByte(request, '\n')+1:]
	if hash(raw) != owner.Delivery.SHA256 {
		t.Fatal("provider request differs from frozen receipt")
	}
	var ctx struct {
		Events []PlanningEvent `json:"events"`
	}
	if err = json.Unmarshal(raw, &ctx); err != nil {
		t.Fatal(err)
	}
	if len(ctx.Events) == 0 || len(ctx.Events) >= 8 || ctx.Events[0].ID != "event-7" {
		t.Fatal("batch not reduced or latest input missing", len(ctx.Events))
	}
	t.Logf("Complete request=%d bytes; methods=%d, context=%d; whole events=%d/8", len(request), len(methods), len(raw), len(ctx.Events))
	for _, e := range ctx.Events {
		if e.Message != w.Planning.Inbox[int(e.ID[len(e.ID)-1]-'0')].Message {
			t.Fatal("event truncated", e.ID)
		}
	}
	pending := 0
	for _, e := range got.Planning.Inbox {
		if e.Decision == "" {
			pending++
		}
	}
	if pending != 7 {
		t.Fatal("projection consumed unacknowledged events", pending)
	}
}
