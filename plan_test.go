//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func fixturePlan() ActionPlan {
	return ActionPlan{Version: 1, Objective: "Recette", Assumptions: []string{}, Questions: []PlanQuestion{}, Tasks: []PlanMission{{ID: "T1", Title: "Première mission", Role: "worker", Scope: "docs", Deliverable: "docs/t1.md", Depends: []string{}, Criteria: []string{"preuve"}, Proof: "test", Entry: "prérequis", Validation: "tests", Delivery: "revue", Stop: "OODA sur blocage", MaxAttempts: 1, MaxToolCalls: 5}, {ID: "T2", Title: "Mission dépendante", Role: "worker", Scope: "docs", Deliverable: "docs/t2.md", Depends: []string{"T1"}, Criteria: []string{"preuve"}, Proof: "test", Entry: "T1 acceptée", Validation: "tests", Delivery: "revue", Stop: "OODA sur blocage", MaxAttempts: 2, MaxToolCalls: 10}}}
}
func seedPlan(t *testing.T, s *Store, w Work, p ActionPlan) Work {
	t.Helper()
	b, _ := json.Marshal(p)
	w, e := s.mutate(w.ID, "fixture", newID("e-"), w.Revision, []byte(`{}`), func(w *Work) error {
		w.PlanningBrief = &PlanningBrief{SHA256: "brief", Text: "Brief revu"}
		w.Tasks = append(w.Tasks, Task{ID: "plan-source", Brainstorm: true, PlanBriefHash: "brief", Response: string(b), Status: "blocked"})
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	return w
}
func TestPlanStrictContract(t *testing.T) {
	good := fixturePlan()
	b, _ := json.Marshal(good)
	if _, e := parseActionPlan(string(b)); e != nil {
		t.Fatal(e)
	}
	for _, text := range []string{"```json\n" + string(b) + "\n```", string(b) + " {}", strings.Replace(string(b), `"version":1`, `"version":1,"extra":true`, 1), strings.Replace(string(b), `"questions":[]`, `"questions":[{"question":"Choix"}]`, 1), strings.Replace(string(b), `"questions":[]`, `"questions":[{"question":"Choix","answer":"inventée"}]`, 1)} {
		if _, e := parseActionPlan(text); e == nil {
			t.Fatal("invalid JSON accepted", text)
		}
	}
	for _, edit := range []func(*ActionPlan){func(p *ActionPlan) { p.Tasks[0].Depends = []string{"T2"} }, func(p *ActionPlan) { p.Tasks[0].Depends = []string{"missing"} }, func(p *ActionPlan) { p.Tasks[0].Proof = "" }, func(p *ActionPlan) { p.Tasks[0].MaxAttempts = 0 }, func(p *ActionPlan) { p.Tasks[0].MaxToolCalls = 101 }, func(p *ActionPlan) { p.Tasks[1].ID = "T1" }} {
		p := fixturePlan()
		edit(&p)
		if _, e := validateActionPlan(p, true); e == nil {
			t.Fatal("invalid plan accepted")
		}
	}
	wrongRole := fixturePlan()
	wrongRole.Tasks[0].Role = "planner"
	if _, e := validateActionPlan(wrongRole, true); e == nil || !strings.Contains(e.Error(), "responsables sont des périmètres") {
		t.Fatal("executable planner task accepted", e)
	}
}
func TestPlanAtomicReviewedCommitAndBudgets(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	p := fixturePlan()
	p.Questions = []PlanQuestion{{Question: "Périmètre approuvé ?"}}
	w = seedPlan(t, s, w, p)
	review, e := s.readPlan(w.ID, "plan-source")
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.commitPlan(w.ID, newID("e-"), w.Revision, review); e == nil {
		t.Fatal("unanswered question")
	}
	same, _ := s.get(w.ID)
	if len(same.Tasks) != len(w.Tasks) {
		t.Fatal("partial task creation")
	}
	review.Spec.Questions[0].Answer = "docs uniquement"
	review.Spec.Tasks[0], review.Spec.Tasks[1] = review.Spec.Tasks[1], review.Spec.Tasks[0]
	saved, e := s.commitPlan(w.ID, newID("e-"), w.Revision, review)
	if e != nil {
		t.Fatal(e)
	}
	if len(saved.Plans) != 1 || len(saved.Tasks) != len(w.Tasks)+2 {
		t.Fatal("missing plan")
	}
	ids := saved.Plans[0].TaskIDs
	first, _ := saved.task(ids[0])
	second, _ := saved.task(ids[1])
	if first.Status != "todo" || second.Depends[0] != first.ID {
		t.Fatal("dependencies/order lost")
	}
	if _, e = s.commitPlan(w.ID, newID("e-"), saved.Revision, review); e == nil {
		t.Fatal("duplicate plan")
	}
	r.TaskID = second.ID
	r.Revision = saved.Revision
	r.EventID = newID("a-")
	if _, _, e = s.prepare(w.ID, r); e == nil {
		t.Fatal("dependent launched before acceptance")
	}
	r.TaskID = first.ID
	r.EventID = newID("a-")
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if a.Limits.MaxToolCalls != 5 || a.Role != "worker" || !strings.Contains(a.Prompt, "docs uniquement") {
		t.Fatal("plan constraints lost")
	}
	zero := 0
	if e = s.finishAgent(a, "completed", "test", &zero); e != nil {
		t.Fatal(e)
	}
	saved, _ = s.get(w.ID)
	r.Revision = saved.Revision
	r.Previous = a.ID
	r.EventID = newID("a-")
	if _, _, e = s.prepare(w.ID, r); e == nil || !strings.Contains(e.Error(), "Plafond") {
		t.Fatal("attempt ceiling not enforced", e)
	}
}
func TestPlanSourceDriftAndFormatPrompt(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	w = seedPlan(t, s, w, fixturePlan())
	review, _ := s.readPlan(w.ID, "plan-source")
	w, e := s.mutate(w.ID, "fixture", newID("e-"), w.Revision, []byte(`{}`), func(w *Work) error { w.PlanningBrief.SHA256 = "changed"; return nil })
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.commitPlan(w.ID, newID("e-"), w.Revision, review); e == nil {
		t.Fatal("stale brief committed")
	}
	r.Brainstorm = true
	r.TaskID = ""
	r.Revision = w.Revision
	r.PlanBriefHash = "changed"
	r.Instruction = "Préparer plan"
	a, _, e := s.prepareLaunch(w.ID, r, true)
	if e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(a.Prompt, "UNIQUEMENT par un objet JSON") || strings.Contains(a.Prompt, "Répondre directement en Markdown") {
		t.Fatal("conflicting response contract")
	}
	r.PlanBriefHash = "old"
	if _, _, e = s.prepareLaunch(w.ID, r, true); e == nil {
		t.Fatal("stale generation allowed")
	}
}

func TestPlanCannotReplaceRequiredGates(t *testing.T) {
	task := Task{PlanChecks: map[string]string{"plan-entry": "entry", "plan-criterion-1": "validation"}}
	for _, raw := range []string{`{"checks":[{"id":"other","mandatory":true,"gate":"delivery"}]}`, `{"checks":[{"id":"plan-entry","mandatory":false,"gate":"entry"},{"id":"plan-criterion-1","mandatory":true,"gate":"validation"}]}`} {
		if requiredPlanChecks(&task, []byte(raw)) == nil {
			t.Fatal("weakened required checks accepted")
		}
	}
	if e := requiredPlanChecks(&task, []byte(`{"checks":[{"id":"plan-entry","mandatory":true,"gate":"entry"},{"id":"plan-criterion-1","mandatory":true,"gate":"validation"}]}`)); e != nil {
		t.Fatal(e)
	}
}
