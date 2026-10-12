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

func TestPlanningObservedCapacityAdmitsWholeContextAndFreezesReceipt(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	folder := t.TempDir()
	t.Setenv("CODEX_HOME", folder)
	catalog := `{"models":[{"slug":"gpt-5.6-sol","visibility":"list","supported_reasoning_levels":[{"effort":"medium"}],"context_window":272000,"effective_context_window_percent":95}]}`
	if err := os.WriteFile(filepath.Join(folder, "models_cache.json"), []byte(catalog), 0600); err != nil {
		t.Fatal(err)
	}
	command := filepath.Join(folder, "codex")
	body := "#!/usr/bin/env python3\nimport sys,os,json\n" + tokenMetadataFixture + "\ntext=sys.stdin.read()\nopen(sys.argv[0]+'.prompt','w').write(text)\nc=json.loads(text.rsplit('\\n',1)[-1])\nreply={'input_events':[e['id'] for e in c['events']],'reason':'Observe the supplied complete current input; no acceptance presumed','operations':[]}\nprint(json.dumps({'type':'result','result':json.dumps(reply)}))\n"
	if err := os.WriteFile(command, []byte(body), 0700); err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"test": {Command: command}}})
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "test", Level: "standard", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
	var err error
	w, err = s.mutate(w.ID, "test.large-context", "observed-large-context", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Planning.Inbox = []PlanningEvent{{ID: "whole-current-input", Kind: "operator_resume", Scope: "root", Message: strings.Repeat("Original unchanged evidence. ", 1600)}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.planningStep(w.ID); err != nil {
		t.Fatal(err)
	}
	prompt, err := os.ReadFile(command + ".prompt")
	if err != nil {
		t.Fatal(err)
	}
	if len(prompt) <= planningPromptLimit {
		t.Fatal("fixture did not cross historical boundary", len(prompt))
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	scope, _ := current.Planning.scope("root")
	raw = prompt[bytes.LastIndexByte(prompt, '\n')+1:]
	if scope.Delivery == nil || scope.Delivery.SHA256 != hash(raw) || current.Planning.Activations != 1 || current.Planning.Decisions != 1 {
		t.Fatal("receipt/counters not bound to actual complete prompt")
	}
	var context struct {
		Budget *managedReviewInputBudget `json:"observed_model_input_capacity"`
		Events []PlanningEvent           `json:"events"`
	}
	if err = json.Unmarshal(raw, &context); err != nil {
		t.Fatal(err)
	}
	if context.Budget == nil || context.Events[0].Message != w.Planning.Inbox[0].Message {
		t.Fatal("capacity absent or original event truncated")
	}
	if _, err = context.Budget.measure(string(prompt), planningSchemaForDelivery(*scope.Delivery)); err != nil {
		t.Fatal(err)
	}
}

func TestPlanningCapacityRejectsUnknownAndRetainsLegacyBounds(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
	var err error
	w, err = s.mutate(w.ID, "test.bounded", "large-unresolved-input", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Planning.Inbox = []PlanningEvent{{ID: "whole", Kind: "operator_resume", Scope: "root", Message: strings.Repeat("original evidence ", 4000)}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	before, _ := json.Marshal(w)
	if _, _, err = s.planningCapacityContext(w, "root", "", nil, nil); err == nil {
		t.Fatal("unsupported provider bypassed historic bound")
	}
	for _, mode := range []string{"unknown", "no-room", "bytes"} {
		budget := reviewTokenBudgetFixture()
		switch mode {
		case "unknown":
			budget.Model = "unknown"
		case "no-room":
			budget.ContextTokens = 104000
		case "bytes":
			budget.MaxBytes = 64000
		}
		if _, _, err = s.planningCapacityContext(w, "root", "", &budget, nil); err == nil {
			t.Fatal("invalid or insufficient capacity admitted", mode)
		}
	}
	after, _ := json.Marshal(w)
	if string(before) != string(after) {
		t.Fatal("preflight changed input/counters")
	}
}
