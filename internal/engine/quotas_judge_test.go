//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"testing"
)

func TestJudgeCycleQuotaPublicCLIAndPersistence(t *testing.T) {
	s, w := planningFixture(t)
	before, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(t.TempDir(), "quota.json")
	raw, _ := json.Marshal(map[string]any{"schema_version": 1, "event_id": "judge-cycle-quota", "expected_revision": before.Revision, "limits": map[string]any{"planning_activations": before.Planning.MaxActivations, "planning_decisions": before.Planning.MaxDecisions, "review_calls": nil, "judge_cycles": 7}, "reason": "Explicit isolated judge cycle policy"})
	if err = os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	var out bytes.Buffer
	if err = s.quotasCLI([]string{"quotas", "preview", w.ID}, path, &out); err != nil {
		t.Fatal(err)
	}
	unchanged, _ := s.get(w.ID)
	if unchanged.Revision != before.Revision || unchanged.Planning.Judge.MaxCycles != before.Planning.Judge.MaxCycles {
		t.Fatal("preview changed judge policy")
	}
	for range 2 {
		out.Reset()
		if err = s.quotasCLI([]string{"quotas", "apply", w.ID}, path, &out); err != nil {
			t.Fatal(err)
		}
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	after, _ := reopened.get(w.ID)
	if after.Revision != before.Revision+1 || after.Planning.Judge.MaxCycles != 7 || after.Planning.Judge.Cycle != before.Planning.Judge.Cycle || after.Planning.Activations != before.Planning.Activations || after.Planning.Decisions != before.Planning.Decisions {
		t.Fatal("cycle policy lost or consumed history reset")
	}
}

func TestJudgeCycleQuotaRefusalsPreserveHistory(t *testing.T) {
	for _, variant := range []string{"below-consumed", "negative", "missing-judge", "active-planner", "stale"} {
		t.Run(variant, func(t *testing.T) {
			s, w := planningFixture(t)
			for cycle := 1; cycle < 3; cycle++ {
				var err error
				w, err = judgeDo(t, s, w, "judge", cycle, PlanningRequest{JudgeVerdict: "continue", Reason: "Advance with real history before the quota check"})
				if err != nil {
					t.Fatal(err)
				}
				w, err = judgeDo(t, s, w, "renew", cycle, PlanningRequest{})
				if err != nil {
					t.Fatal(err)
				}
			}
			w, err := s.mutate(w.ID, "test.judge-quota", "prepare-cycle", w.Revision, []byte(`{}`), func(current *Work) error {
				if variant == "missing-judge" {
					current.Planning.Judge = nil
				}
				if variant == "active-planner" {
					current.Planning.Scopes[0].Holder = "reserved-owner"
				}
				return nil
			})
			if err != nil {
				t.Fatal(err)
			}
			limit := 7
			switch variant {
			case "below-consumed":
				limit = 2
			case "negative":
				limit = -1
			}
			r := QuotaChange{Schema: 1, EventID: "refused-judge-policy", Revision: w.Revision, Reason: "Verify policy refusal retains all history", Limits: QuotaValues{Planning: w.Planning.MaxActivations, Decisions: w.Planning.MaxDecisions, JudgeCycles: &limit}}
			if variant == "stale" {
				r.Revision--
			}
			if _, err := s.configureQuotas(w.ID, r); err == nil {
				t.Fatal("invalid judge quota accepted")
			}
			after, _ := s.get(w.ID)
			if !reflect.DeepEqual(w, after) {
				t.Fatal("refused quota changed consumption, history or authority")
			}
		})
	}
}

func TestJudgeCycleWithoutImplicitCeilingAndExplicitPolicy(t *testing.T) {
	s, w := planningFixture(t)
	if w.Planning.Judge.MaxCycles != 0 {
		t.Fatal("new mission received an implicit policy ceiling")
	}
	advance := func(action string, cycle int, r PlanningRequest) {
		t.Helper()
		updated, err := judgeDo(t, s, w, action, cycle, r)
		if err != nil {
			t.Fatal(err)
		}
		w = updated
	}
	for cycle := 1; cycle <= 21; cycle++ {
		advance("judge", cycle, PlanningRequest{JudgeVerdict: "continue", Reason: "Continue the unfinished objective with its existing budgets"})
		advance("renew", cycle, PlanningRequest{})
	}
	if w.Planning.Judge.Cycle != 22 || len(w.Planning.Judge.History) != 21 {
		t.Fatal("implicit historical 20-cycle cutoff remains")
	}
	limit := 22
	request := QuotaChange{Schema: 1, EventID: "explicit-cycle-ceiling", Revision: w.Revision, Reason: "Set the operator cycle policy explicitly", Limits: QuotaValues{Planning: w.Planning.MaxActivations, Decisions: w.Planning.MaxDecisions, JudgeCycles: &limit}}
	var err error
	w, err = s.configureQuotas(w.ID, request)
	if err != nil {
		t.Fatal(err)
	}
	advance("judge", 22, PlanningRequest{JudgeVerdict: "continue", Reason: "The objective remains unfinished"})
	root, _ := w.Planning.scope("root")
	if _, err = s.planningChange(w.ID, "renew", PlanningRequest{Schema: 1, EventID: "cap-" + strconv.Itoa(limit), Revision: w.Revision, Scope: "root", ScopeRevision: root.Revision, JudgeCycle: 22}); err == nil {
		t.Fatal("explicit cycle policy ignored")
	}
	zero := 0
	request.EventID, request.Revision, request.Limits.JudgeCycles = "remove-cycle-ceiling", w.Revision, &zero
	w, err = s.configureQuotas(w.ID, request)
	if err != nil {
		t.Fatal(err)
	}
	advance("renew", 22, PlanningRequest{})
	if w.Planning.Judge.Cycle != 23 || len(w.Planning.Judge.History) != 22 {
		t.Fatal("operator policy change reset or blocked the cycle history")
	}
}

func TestJudgeCyclePolicyStorageFencePreservesOriginalBody(t *testing.T) {
	s, w := planningFixture(t)
	var original []byte
	if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&original); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("PRAGMA user_version=31"); err != nil {
		t.Fatal(err)
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}
	if old, err := openStoreWithMigration(s.root, false, false); err == nil {
		old.db.Close()
		t.Fatal("old storage policy accepted without explicit migration")
	}
	up, err := openStoreWithMigration(s.root, false, true)
	if err != nil {
		t.Fatal(err)
	}
	defer up.db.Close()
	var after []byte
	if err = up.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&after); err != nil || !bytes.Equal(original, after) {
		t.Fatal("migration altered original mission or configured ceilings", err)
	}
}
