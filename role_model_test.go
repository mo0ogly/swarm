//go:build linux

package main

import (
	"encoding/json"
	"os"
	"reflect"
	"strings"
	"testing"
)

func TestRoleModelIndependentRoutesPreserveCounters(t *testing.T) {
	s, _, p := taskModelFixture(t)
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "claude", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15, MaxReviewCalls: 7})
	var e error
	w, e = s.mutate(w.ID, "fixture", "role-fixture", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Planning.Scopes = append(w.Planning.Scopes, PlanningScope{ID: "child", Parent: "root", State: "ready", Revision: 1})
		w.Planning.Activations = 3
		w.Planning.Reviewer.Calls = 2
		w.Planning.Reviewer.Failure = "retained failure"
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	_, route, _ := resolveModel(p, "exigeant", "planning")
	r := RoleModelRequest{Schema: 1, EventID: "root-model", Revision: w.Revision, Scope: "root", Provider: "claude", Level: "exigeant", PolicyHash: route.PolicyHash}
	if _, e = s.configureRoleModel(w.ID, r); e == nil {
		t.Fatal("unpaused edit allowed")
	}
	if e = s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	r.Revision = w.Revision
	if _, e = s.previewRoleModel(w.ID, r); e != nil {
		t.Fatal(e)
	}
	before, _ := s.get(w.ID)
	if before.Revision != w.Revision {
		t.Fatal("preview wrote")
	}
	w, e = s.configureRoleModel(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	root, _ := w.Planning.scope("root")
	child, _ := w.Planning.scope("child")
	if effectivePlanningScope(w.Planning, root).ModelRoute.Model != "opus" || effectivePlanningScope(w.Planning, child).ModelRoute.Model != "sonnet" || w.Planning.Reviewer.ModelRoute.Model != "sonnet" {
		t.Fatal("routes coupled")
	}
	r.EventID = "reviewer-model"
	r.Revision = w.Revision
	r.Scope = ""
	r.Reviewer = true
	w, e = s.configureRoleModel(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if w.Planning.Reviewer.ModelRoute.Model != "opus" || w.Planning.Reviewer.Calls != 2 || w.Planning.Reviewer.MaxCalls != 7 || w.Planning.Reviewer.Failure != "retained failure" || w.Planning.Activations != 3 {
		t.Fatal("counters or failure changed")
	}
	if !reflect.DeepEqual(before.Tasks, w.Tasks) {
		t.Fatal("task history changed")
	}
	r.EventID = "scope-inherit"
	r.Revision = w.Revision
	r.Scope = "root"
	r.Reviewer = false
	r.Inherit = true
	w, e = s.configureRoleModel(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	root, _ = w.Planning.scope("root")
	if effectivePlanningScope(w.Planning, root).ModelRoute.Model != "sonnet" || w.Planning.Reviewer.ModelRoute.Model != "opus" {
		t.Fatal("inherit changed reviewer")
	}
}
func TestRoleModelRejectsHeldScope(t *testing.T) {
	s, _, p := taskModelFixture(t)
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "claude", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
	w, _ = planningClaim(t, s, w, "root")
	s.pause(w.ID, true)
	w, _ = s.get(w.ID)
	w, _ = s.mutate(w.ID, "fixture", "held-fixture", w.Revision, []byte(`{}`), func(w *Work) error { w.Planning.Scopes[0].Holder = "active-holder"; return nil })
	_, route, _ := resolveModel(p, "exigeant", "planning")
	if _, e := s.configureRoleModel(w.ID, RoleModelRequest{Schema: 1, EventID: "held", Revision: w.Revision, Scope: "root", Provider: "claude", Level: "exigeant", PolicyHash: route.PolicyHash}); e == nil {
		t.Fatal("held planner model changed")
	}
}

func TestRoleModelRunnerUsesSelectedModel(t *testing.T) {
	s, _, provider := taskModelFixture(t)
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "claude", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
	response, _ := json.Marshal(map[string]any{"input_events": []string{w.Planning.Inbox[0].ID}, "reason": "Fixture acknowledgement", "operations": []any{}})
	envelope, _ := json.Marshal(map[string]any{"type": "result", "result": string(response)})
	script := "#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' \"$@\" >\"$0.args\"\nprintf '%s\\n' '" + string(envelope) + "'\n"
	if e := os.WriteFile(provider.Command, []byte(script), 0700); e != nil {
		t.Fatal(e)
	}
	s.pause(w.ID, true)
	w, _ = s.get(w.ID)
	_, route, _ := resolveModel(provider, "exigeant", "planning")
	var e error
	w, e = s.configureRoleModel(w.ID, RoleModelRequest{Schema: 1, EventID: "runner-model", Revision: w.Revision, Scope: "root", Provider: "claude", Level: "exigeant", PolicyHash: route.PolicyHash})
	if e != nil {
		t.Fatal(e)
	}
	s.pause(w.ID, false)
	if e = s.planningStep(w.ID); e != nil {
		t.Fatal(e)
	}
	args, e := os.ReadFile(provider.Command + ".args")
	if e != nil || !strings.Contains(string(args), "opus") {
		t.Fatal(string(args), e)
	}
	w, _ = s.get(w.ID)
	scope, _ := w.Planning.scope("root")
	if scope.LastModel == nil || scope.LastModel.Route.Model != "opus" || w.Planning.Activations != 1 {
		t.Fatal("used model or charge missing")
	}
}
