//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// Real structured-provider transport with a deterministic local subprocess.
// This proves receipt -> leased decision -> persistence -> next received context,
// not the quality or autonomy of a real model's judgment.
func TestPlanningJudgeProviderReceivesCycleAndAppliesRenewal(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	script := filepath.Join(t.TempDir(), "claude")
	body := `#!/bin/sh
cat > "$0.prompt"
printf '%s\n' "$@" > "$0.args"
cat "$0.reply"
`
	if err := os.WriteFile(script, []byte(body), 0700); err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"cycle-fixture": {Command: script}}})
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "cycle-fixture", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 10})
	for index, kind := range []string{"judge", "renew", "acknowledge"} {
		pending := []string{}
		for _, e := range w.Planning.Inbox {
			if e.Scope == "root" && e.Decision == "" {
				pending = append(pending, e.ID)
			}
		}
		ops := []map[string]any{}
		if kind == "judge" {
			ops = append(ops, map[string]any{"kind": "judge", "judge_cycle": 1, "judge_verdict": "continue", "expected_candidate": ""})
		}
		if kind == "renew" {
			ops = append(ops, map[string]any{"kind": "renew", "judge_cycle": 1})
		}
		response := map[string]any{"input_events": pending, "reason": "Deterministic cycle protocol check", "operations": ops}
		envelope, _ := json.Marshal(map[string]any{"type": "result", "structured_output": response})
		if err := os.WriteFile(script+".reply", envelope, 0600); err != nil {
			t.Fatal(err)
		}
		if err := s.planningStep(w.ID); err != nil {
			t.Fatal(kind, err)
		}
		var err error
		w, err = s.get(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		prompt, err := os.ReadFile(script + ".prompt")
		if err != nil {
			t.Fatal(err)
		}
		args, err := os.ReadFile(script + ".args")
		if err != nil {
			t.Fatal(err)
		}
		parts := strings.Split(strings.TrimSpace(string(args)), "\n")
		kinds := planningOperationSchemaKinds(t, parts[len(parts)-1])
		if kind != "acknowledge" && !containsString(kinds, kind) {
			t.Fatal("provider never received cycle operation", kind, kinds)
		}
		if index == 2 && !strings.Contains(string(prompt), `"cycle":2`) {
			t.Fatal("renewed provider context did not contain actual cycle 2")
		}
		if w.Planning.Activations != index+1 || w.Planning.Decisions != index+1 {
			t.Fatal("cycle transport duplicated reservations", w.Planning)
		}
	}
	if w.Planning.Judge.Cycle != 2 || len(w.Planning.Judge.History) != 1 || w.Planning.Reviewer.Calls != 0 || len(w.Tasks) != 0 {
		t.Fatal("cycle operations changed production or candidate review", w.Planning)
	}
}

func judgeOperation(t *testing.T, raw string) PlanningOperation {
	t.Helper()
	var op PlanningOperation
	if err := json.Unmarshal([]byte(raw), &op); err != nil {
		t.Fatal(err)
	}
	return op
}

func TestPlanningJudgeStructuredSchemaMatchesCurrentRootCycle(t *testing.T) {
	s, w := planningFixture(t)
	_, receipt, err := s.planningDeliveryContext(w, "root", 64000)
	if err != nil {
		t.Fatal(err)
	}
	kinds := planningOperationSchemaKinds(t, planningSchemaForDelivery(receipt))
	if !containsString(kinds, "judge") || containsString(kinds, "renew") || receipt.JudgeEndAvailable || receipt.JudgeCycle != 1 {
		t.Fatal("root cycle proposal unavailable or end offered without proofs", receipt, kinds)
	}
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Child owner", Requirements: []string{"req-1"}, Next: "Analyze", MaxTasks: 1, MaxActivations: 2}}
	w, err = s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	_, child, err := s.planningDeliveryContext(w, "child", 64000)
	if err != nil {
		t.Fatal(err)
	}
	kinds = planningOperationSchemaKinds(t, planningSchemaForDelivery(child))
	if containsString(kinds, "judge") || containsString(kinds, "renew") {
		t.Fatal("child granted global cycle operations", kinds)
	}
}

func TestPlanningJudgeStructuredInvalidDecisionsHaveNoEffect(t *testing.T) {
	for _, mode := range []string{"stale-cycle", "mixed-operations", "foreign-fields", "end-without-proofs", "renew-without-verdict", "cycle-fields-on-task", "global-pause", "child"} {
		t.Run(mode, func(t *testing.T) {
			s, w := planningFixture(t)
			scope := "root"
			if mode == "child" {
				current, request := planningClaim(t, s, w, "root")
				request.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Child owner", Requirements: []string{"req-1"}, Next: "Analyze", MaxTasks: 1, MaxActivations: 2}}
				var err error
				w, err = s.planningChange(current.ID, "decide", request)
				if err != nil {
					t.Fatal(err)
				}
				scope = "child"
			}
			w, r := planningClaim(t, s, w, scope)
			op := PlanningOperation{Kind: "judge", JudgeCycle: 1, JudgeVerdict: "continue"}
			switch mode {
			case "stale-cycle":
				op.JudgeCycle = 2
			case "foreign-fields":
				op.ID = "unrelated"
			case "end-without-proofs":
				op.JudgeVerdict = "end"
				op.ExpectedCandidate = "missing-proof"
			case "renew-without-verdict":
				op.Kind = "renew"
				op.JudgeVerdict = ""
			case "cycle-fields-on-task":
				op = planningTask("first")
				op.JudgeCycle = 1
			case "global-pause":
				if err := s.pause(w.ID, true); err != nil {
					t.Fatal(err)
				}
			}
			r.Operations = []PlanningOperation{op}
			if mode == "mixed-operations" {
				r.Operations = append(r.Operations, planningTask("first"))
			}
			before, _ := s.get(w.ID)
			if _, err := s.planningChange(w.ID, "decide", r); err == nil {
				t.Fatal("invalid cycle decision accepted", mode)
			}
			after, _ := s.get(w.ID)
			if !reflect.DeepEqual(before, after) {
				t.Fatal("refused cycle decision changed authoritative work", mode)
			}
		})
	}
}

func TestPlanningJudgeStructuredEndRequiresNoActiveProcess(t *testing.T) {
	for _, active := range []bool{false, true} {
		t.Run(map[bool]string{false: "fresh-end", true: "active-process"}[active], func(t *testing.T) {
			s, w := judgeAcceptedFixture(t)
			w, r := planningClaim(t, s, w, "root")
			r.Operations = []PlanningOperation{{Kind: "judge", JudgeCycle: 1, JudgeVerdict: "end", ExpectedCandidate: w.Planning.Repository.Candidate}}
			if active {
				if _, err := s.db.Exec("UPDATE agents SET status='running' WHERE id=?", "agent-first"); err != nil {
					t.Fatal(err)
				}
			}
			before, readErr := s.get(w.ID)
			if readErr != nil {
				t.Fatal(readErr)
			}
			after, err := s.planningChange(w.ID, "decide", r)
			if active {
				if err == nil {
					t.Fatal("active process allowed global end")
				}
				current, _ := s.get(w.ID)
				if !reflect.DeepEqual(before, current) {
					t.Fatal("rejected end changed state")
				}
			} else if err != nil || !after.Planning.Judge.Closed || after.Planning.Scopes[0].State != "closed" {
				t.Fatal("explicit proven end not applied", err, after.Planning)
			} else if !s.postDelivery(&after).Ready {
				t.Fatal("proven root end did not expose post-recipe RETEX")
			}
		})
	}
}

func TestPlanningJudgeOperationIsReceivedAndRenewsFreshContext(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{judgeOperation(t, `{"kind":"judge","judge_cycle":1,"judge_verdict":"continue","expected_candidate":""}`)}
	first, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal("root cannot submit a structured global judgment", err)
	}
	if first.Planning.Judge.Verdict != "continue" || first.Planning.Decisions != 1 || first.Planning.Scopes[0].State != "ready" {
		t.Fatal("judgment did not wake root for renewal", first.Planning)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	replay, err := reopened.planningChange(w.ID, "decide", r)
	if err != nil || replay.Revision != first.Revision {
		t.Fatal("judgment replay changed state", err)
	}
	current, renew := planningClaim(t, reopened, first, "root")
	renew.Operations = []PlanningOperation{judgeOperation(t, `{"kind":"renew","judge_cycle":1}`)}
	next, err := reopened.planningChange(w.ID, "decide", renew)
	if err != nil {
		t.Fatal(err)
	}
	if next.Planning.Judge.Cycle != 2 || next.Planning.Judge.Verdict != "" || len(next.Planning.Judge.History) != 1 || next.Planning.Judge.History[0].RenewedAt == "" || next.Planning.Activations != current.Planning.Activations || next.Planning.Decisions != 2 || next.Planning.Scopes[0].State != "ready" {
		t.Fatal("renewal lost history, counters or wakeup", next.Planning)
	}
	pending := 0
	for _, e := range next.Planning.Inbox {
		if e.Scope == "root" && e.Decision == "" {
			pending++
			if e.Kind != "cycle_renewed" {
				t.Fatal("wrong cycle event", e)
			}
		}
	}
	if pending != 1 {
		t.Fatal("fresh cycle activation missing", pending)
	}
	_, fresh := planningClaim(t, reopened, next, "root")
	if len(fresh.Inputs) != 1 || fresh.Inputs[0] == r.Inputs[0] {
		t.Fatal("new cycle reused original delivery", fresh.Inputs)
	}
}
