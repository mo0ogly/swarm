//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func preparedTeam(t *testing.T) (*Store, Preparation) {
	t.Helper()
	s, _, _ := prepDialogueFixture(t, prepReplyScript)
	p := readyPreparation(t, s, "")
	r := conversionRequest(t, s, p, "create-missions")
	r.Organization = &PreparationOrganization{Provider: "test", Level: "standard", Workspace: s.root, Validation: "human", MaxTasks: 20, MaxCalls: 40}
	p, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	return s, p
}
func TestPreparedOrganizationAndRevision(t *testing.T) {
	s, p := preparedTeam(t)
	w, e := s.get(p.WorkID)
	if e != nil {
		t.Fatal(e)
	}
	if e = organizationGuard(w); e != nil {
		t.Fatal(e)
	}
	if w.Planning == nil || w.Planning.Reviewer == nil || !w.Planning.ReviewerRequired || w.Planning.ModelRoute == nil || w.Profile == nil || w.Planning.HumanReviewAuthorized == "" {
		t.Fatal("missing organization", w.Planning)
	}
	for _, task := range w.Tasks {
		if task.ScopeID != "root" || task.ValidationPolicy.Mode != "human" || task.ValidationPolicy.Authorized == "" || !task.LaunchHeld {
			t.Fatal(task)
		}
	}
	p, e = s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
	if e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(p.WorkID)
	for i := range w.Tasks {
		w.Tasks[i].Status = "accepted"
		w.Tasks[i].Response = "historical response"
	}
	raw, _ := json.Marshal(w)
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	spec := p.Conversion.Spec
	spec.Tasks[0].Title = "Mission révisée"
	raw, _ = json.Marshal(spec)
	p = prepSave(t, s, p, "plan", string(raw))
	r := prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	p, e = s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	review, e := s.preparationConversionReview(p.ID)
	if e != nil || review.Action != "revise-missions" {
		t.Fatal(review, e)
	}
	if review.Changes[0].Kind != "modifiée" || review.Changes[1].Kind != "à revérifier (dépendance modifiée)" {
		t.Fatal("incorrect impact preview", review.Changes)
	}
	r = conversionRequest(t, s, p, "revise-missions")
	p, e = s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(p.WorkID)
	if len(w.Plans) != 2 || !w.Planning.Paused || w.Tasks[0].Title != "Mission révisée" {
		t.Fatal(w)
	}
	for _, task := range w.Tasks {
		if task.Status != "todo" || task.Response != "historical response" || !task.LaunchHeld {
			t.Fatal(task)
		}
	}
	var paused int
	s.db.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", w.ID).Scan(&paused)
	if paused != 1 {
		t.Fatal("revision did not pause")
	}
	rev := w.Revision
	if _, e = s.preparationCommand(conversionRequest(t, s, p, "revise-missions")); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Revision != rev {
		t.Fatal("semantic replay mutated work")
	}
	p, _ = s.preparation(p.ID)
	p, e = s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
	if e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Planning.Paused || w.Tasks[0].LaunchHeld {
		t.Fatal("release did not unlock")
	}
	// Release does not resume mission execution without the separate mission action.
	s.db.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", w.ID).Scan(&paused)
	if paused != 1 {
		t.Fatal("release resumed execution")
	}
}
func TestPreparedOrganizationRequiresControlsAtomic(t *testing.T) {
	s, _, _ := prepDialogueFixture(t, prepReplyScript)
	p := readyPreparation(t, s, "")
	r := conversionRequest(t, s, p, "create-missions")
	r.Organization = &PreparationOrganization{Provider: "test", Workspace: s.root, Validation: "automatic", MaxTasks: 20, MaxCalls: 40}
	if _, e := s.preparationCommand(r); e == nil {
		t.Fatal("missing controls accepted")
	}
	fresh, _ := s.preparation(p.ID)
	if fresh.Conversion != nil || fresh.Revision != p.Revision {
		t.Fatal("partial conversion")
	}
	var count int
	s.db.QueryRow("SELECT count(*) FROM works").Scan(&count)
	if count != 0 {
		t.Fatal("partial work", count)
	}
	r.Organization.Controls = map[string][]ValidationControl{}
	for _, id := range []string{"T1", "T2"} {
		for _, check := range []string{"plan-entry", "plan-validation", "plan-delivery", "plan-criterion-1"} {
			r.Organization.Controls[id] = append(r.Organization.Controls[id], ValidationControl{ID: check, Command: []string{"git", "diff", "--check"}, Criteria: []int{1}, Justification: "Contrôle explicite de la preuve", Dir: ".", Timeout: 30})
		}
	}
	p, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	w, _ := s.get(p.WorkID)
	if e = organizationGuard(w); e != nil {
		t.Fatal(e)
	}
	if w.Planning.HumanReviewAuthorized != "" || len(w.Planning.Checks) != 2 {
		t.Fatal(w.Planning)
	}
}
func TestPreparedRevisionRefusesActiveAgent(t *testing.T) {
	s, p := preparedTeam(t)
	var e error
	p, e = s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
	if e != nil {
		t.Fatal(e)
	}
	w, _ := s.get(p.WorkID)
	_, e = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('active',?,?,?,'running','{}','{}')", w.ID, w.Tasks[0].ID, s.root)
	if e != nil {
		t.Fatal(e)
	}
	spec := p.Conversion.Spec
	spec.Tasks[0].Title = "Changed"
	raw, _ := json.Marshal(spec)
	p = prepSave(t, s, p, "plan", string(raw))
	r := prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	p, e = s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.preparationCommand(conversionRequest(t, s, p, "revise-missions")); e == nil || !strings.Contains(e.Error(), "active") {
		t.Fatal(e)
	}
	fresh, _ := s.get(w.ID)
	if fresh.Revision != w.Revision || fresh.Tasks[0].Title != w.Tasks[0].Title {
		t.Fatal("active work mutated")
	}
}

func TestPreparationPersistsAndUsesModelRoute(t *testing.T) {
	path := filepath.Join(t.TempDir(), "arguments.txt")
	s, _, r := prepDialogueFixture(t, "printf '%s\\n' \"$@\" > '"+path+"'\n"+prepReplyScript)
	r.Level = "exigeant"
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	if turn.ModelRoute == nil || turn.ModelRoute.Model != "opus" {
		t.Fatal(turn.ModelRoute)
	}
	s.runPreparationTurn(turn)
	got, e := s.preparationTurn(turn.ID)
	if e != nil || got.Status != "answered" {
		t.Fatal(got, e)
	}
	b, e := os.ReadFile(path)
	if e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(string(b), "--model\nopus\n") {
		t.Fatal("model not passed to executable", string(b))
	}
}

func TestPreparationRetainsOnlyExactOperatorDecisions(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	p = prepAdopt(t, s, p)
	prior := fixturePlan()
	prior.Questions = []PlanQuestion{{Question: "Quel périmètre ?", Answer: "Documentation seulement"}}
	raw, _ := json.Marshal(prior)
	p = prepSave(t, s, p, "plan", string(raw))
	r.Revision = p.Revision
	r.Target = "plan"
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	proposal := fixturePlan()
	proposal.Questions = []PlanQuestion{{Question: "Quel périmètre ?"}, {Question: "Peut-on publier ?"}}
	raw, _ = json.Marshal(proposal)
	answer, _ := json.Marshal(PreparationAnswer{Message: "Plan révisé", Plan: string(raw)})
	if e = s.finishPreparationTurn(turn, string(answer), nil, ""); e != nil {
		t.Fatal(e)
	}
	request := prepRequest(p, "use-proposal")
	request.Turn = turn.ID
	p, e = s.preparationCommand(request)
	if e != nil {
		t.Fatal(e)
	}
	var saved ActionPlan
	if e = json.Unmarshal([]byte(p.Documents["plan"].Text), &saved); e != nil {
		t.Fatal(e)
	}
	if saved.Questions[0].Answer != "Documentation seulement" || saved.Questions[1].Answer != "" || p.PlanReady {
		t.Fatal("decision invented or lost", saved.Questions)
	}
}
