//go:build linux

package engine

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

func TestPreparedOrganizationAttachAfterMaterialization(t *testing.T) {
	for _, action := range []string{"create-missions", "authorize-plan"} {
		t.Run(action, func(t *testing.T) {
			s, _, _ := prepDialogueFixture(t, prepReplyScript)
			p := readyPreparation(t, s, "")
			p, err := s.preparationCommand(conversionRequest(t, s, p, "create-missions"))
			if err != nil {
				t.Fatal(err)
			}
			before, err := s.get(p.WorkID)
			if err != nil {
				t.Fatal(err)
			}
			r := conversionRequest(t, s, p, action)
			r.Organization = &PreparationOrganization{Provider: "test", Level: "standard", Workspace: s.root, Validation: "human", MaxTasks: 20, MaxCalls: 40}
			got, err := s.preparationCommand(r)
			if err != nil {
				t.Fatal(err)
			}
			after, err := s.get(got.WorkID)
			if err != nil {
				t.Fatal(err)
			}
			if after.Planning == nil || after.Profile == nil {
				t.Fatal("organization silently ignored after materialization")
			}
			if err := organizationGuard(after); err != nil {
				t.Fatal(err)
			}
			if after.ID != before.ID || len(after.Tasks) != len(before.Tasks) {
				t.Fatal("tasks or work replaced")
			}
			for i, task := range after.Tasks {
				if task.ID != before.Tasks[i].ID || len(task.Attempts) != 0 || task.LaunchHeld != (action == "create-missions") {
					t.Fatal("identity, attempts or launch lock changed", task)
				}
			}
			var count int
			if err := s.db.QueryRow("SELECT count(*) FROM agents").Scan(&count); err != nil || count != 0 {
				t.Fatal("organization launched a provider", count, err)
			}
			replayed, err := s.preparationCommand(r)
			if err != nil || replayed.Revision != got.Revision {
				t.Fatal("receipt replay", err)
			}
			fresh := prepRequest(got, "create-missions")
			fresh.Hash, fresh.WorkRevision, fresh.Organization = r.Hash, &after.Revision, r.Organization
			got, err = s.preparationCommand(fresh)
			if err == nil {
				t.Fatal("new event silently reapplied an existing organization")
			}
			unchanged, _ := s.get(after.ID)
			if unchanged.Revision != after.Revision || len(unchanged.Plans) != len(after.Plans) {
				t.Fatal("duplicate organization")
			}
			if action == "create-missions" {
				f := prepRequest(got, "authorize-plan")
				f.Hash, f.WorkRevision, f.Organization = r.Hash, r.WorkRevision, r.Organization
				if _, err := s.preparationCommand(f); err == nil {
					t.Fatal("authorization silently ignored")
				}
			}
		})
	}
}

func TestPreparedOrganizationAttachExistingStateRefused(t *testing.T) {
	for _, mode := range []string{"stale-work", "planning-removed", "profile-altered", "missing-lock"} {
		t.Run(mode, func(t *testing.T) {
			s, p := preparedTeam(t)
			w, _ := s.get(p.WorkID)
			r := conversionRequest(t, s, p, "create-missions")
			r.Organization = &PreparationOrganization{Provider: "test", Level: "standard", Workspace: s.root, Validation: "human", MaxTasks: 20, MaxCalls: 40}
			switch mode {
			case "stale-work":
				v := w.Revision + 1
				r.WorkRevision = &v
			case "planning-removed":
				w.Planning = nil
			case "profile-altered":
				w.Profile.Provider = "other"
			case "missing-lock":
				if _, err := s.db.Exec("DELETE FROM preparation_launch_locks WHERE work_id=?", w.ID); err != nil {
					t.Fatal(err)
				}
			}
			body, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID); err != nil {
				t.Fatal(err)
			}
			var beforePrep []byte
			if err := s.db.QueryRow("SELECT body FROM preparations WHERE id=?", p.ID).Scan(&beforePrep); err != nil {
				t.Fatal(err)
			}
			if _, err := s.preparationCommand(r); err == nil {
				t.Fatal("new event reported successful replay")
			}
			var afterWork, afterPrep []byte
			if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&afterWork); err != nil {
				t.Fatal(err)
			}
			if err := s.db.QueryRow("SELECT body FROM preparations WHERE id=?", p.ID).Scan(&afterPrep); err != nil {
				t.Fatal(err)
			}
			if string(body) != string(afterWork) || string(beforePrep) != string(afterPrep) {
				t.Fatal("rejection mutated state")
			}
		})
	}
}

func TestPreparedOrganizationAttachRejectsUnsafeState(t *testing.T) {
	for _, mode := range []string{"stale-work", "stale-plan", "released", "task-edited", "attempted", "invalid-provider", "foreign-task", "missing-lock", "work-reframed", "prior-review", "recovery", "prior-policy", "scope", "blocker", "stale-evidence", "work-profile"} {
		t.Run(mode, func(t *testing.T) {
			s, _, _ := prepDialogueFixture(t, prepReplyScript)
			p := readyPreparation(t, s, "")
			p, err := s.preparationCommand(conversionRequest(t, s, p, "create-missions"))
			if err != nil {
				t.Fatal(err)
			}
			w, _ := s.get(p.WorkID)
			r := conversionRequest(t, s, p, "create-missions")
			r.Organization = &PreparationOrganization{Provider: "test", Level: "standard", Workspace: s.root, Validation: "human", MaxTasks: 20, MaxCalls: 40}
			switch mode {
			case "work-profile":
				w.Profile = &LaunchProfile{Provider: "test", Role: "worker", Workspace: s.root}
			case "prior-review":
				w.Tasks[0].PreviousReviews = []IndependentReview{{State: "completed"}}
			case "recovery":
				w.Tasks[0].CorrectiveRecovery = &CorrectiveRecovery{}
			case "prior-policy":
				w.Tasks[0].ValidationPolicy = &ValidationPolicy{Mode: "human"}
			case "scope":
				w.Tasks[0].ScopeID = "previous-root"
				w.Tasks[0].Requirements = []string{"req-1"}
			case "blocker":
				w.Tasks[0].Blocker = "prior issue"
			case "stale-evidence":
				w.Tasks[0].EvidenceStaleReason = "prior evidence stale"
			case "work-reframed":
				w.Criteria = []string{"Additional operator requirement"}
			case "stale-work":
				v := w.Revision + 1
				r.WorkRevision = &v
			case "stale-plan":
				p = prepSave(t, s, p, "plan", p.Documents["plan"].Text+" ")
				r = prepRequest(p, "create-missions")
				r.Hash, r.WorkRevision = p.Conversion.PlanHash, &w.Revision
				r.Organization = &PreparationOrganization{Provider: "test", Level: "standard", Workspace: s.root, Validation: "human", MaxTasks: 20, MaxCalls: 40}
			case "released":
				p, err = s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
				if err != nil {
					t.Fatal(err)
				}
				r = conversionRequest(t, s, p, "create-missions")
				r.Organization = &PreparationOrganization{Provider: "test", Level: "standard", Workspace: s.root, Validation: "human", MaxTasks: 20, MaxCalls: 40}
			case "task-edited":
				w.Tasks[0].Criteria[0] = "Changed operator criterion"
			case "attempted":
				w.Tasks[0].Attempts = []Attempt{{ID: "prior-attempt"}}
			case "invalid-provider":
				r.Organization.Provider = "missing"
			case "foreign-task":
				w.Tasks = append(w.Tasks, Task{ID: "foreign", Status: "todo", LaunchHeld: true})
			case "missing-lock":
				if _, err := s.db.Exec("DELETE FROM preparation_launch_locks WHERE work_id=? AND task_id=?", w.ID, w.Tasks[0].ID); err != nil {
					t.Fatal(err)
				}
			}
			if mode != "stale-work" && mode != "stale-plan" && mode != "released" {
				body, _ := json.Marshal(w)
				if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID); err != nil {
					t.Fatal(err)
				}
			}
			var beforeWork, beforePrep []byte
			if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&beforeWork); err != nil {
				t.Fatal(err)
			}
			if err := s.db.QueryRow("SELECT body FROM preparations WHERE id=?", p.ID).Scan(&beforePrep); err != nil {
				t.Fatal(err)
			}
			if _, err := s.preparationCommand(r); err == nil {
				t.Fatal("unsafe organization accepted")
			}
			var afterWork, afterPrep []byte
			s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&afterWork)
			s.db.QueryRow("SELECT body FROM preparations WHERE id=?", p.ID).Scan(&afterPrep)
			if string(beforeWork) != string(afterWork) || string(beforePrep) != string(afterPrep) {
				t.Fatal("rejection mutated state")
			}
		})
	}
}

func TestPreparedOrganizationManagedRepository(t *testing.T) {
	for _, validation := range []string{"automatic", "human"} {
		t.Run(validation, func(t *testing.T) {
			s, _, _ := prepDialogueFixture(t, prepReplyScript)
			p := readyPreparation(t, s, "")
			source := filepath.Join(s.root, "source")
			if err := os.MkdirAll(source, 0700); err != nil {
				t.Fatal(err)
			}
			gitTest(t, source, "init")
			if err := os.WriteFile(filepath.Join(source, "value.txt"), []byte("base\n"), 0600); err != nil {
				t.Fatal(err)
			}
			gitTest(t, source, "add", ".")
			gitTest(t, source, "commit", "-m", "base")
			original := gitTest(t, source, "rev-parse", "HEAD")
			if err := os.WriteFile(filepath.Join(source, "value.txt"), []byte("selected dirty value\n"), 0600); err != nil {
				t.Fatal(err)
			}
			r := conversionRequest(t, s, p, "create-missions")
			controls := map[string][]ValidationControl{}
			for _, task := range fixturePlan().Tasks {
				for _, id := range []string{"plan-entry", "plan-validation", "plan-delivery", "plan-criterion-1"} {
					c := automaticPolicy("git", "diff", "--exit-code").Controls[0]
					c.ID = id
					controls[task.ID] = append(controls[task.ID], c)
				}
			}
			r.Organization = &PreparationOrganization{Provider: "test", Workspace: source, Validation: validation, Controls: controls, MaxTasks: 10, MaxCalls: 30, Repository: &ManagedRepositoryRequest{Path: source, Include: []string{"value.txt"}}}
			got, err := s.preparationCommand(r)
			if validation == "human" {
				if err == nil || !strings.Contains(err.Error(), "contrôles automatiques") {
					t.Fatal(err)
				}
				fresh, _ := s.preparation(p.ID)
				if fresh.Revision != p.Revision || fresh.Conversion != nil {
					t.Fatal("rejected conversion changed preparation")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			w, err := s.get(got.WorkID)
			if err != nil {
				t.Fatal(err)
			}
			if w.Planning.Repository == nil || w.Planning.Repository.Base == original {
				t.Fatal("selected snapshot was not attached")
			}
			copy, err := s.ensureManagedAttempt(w, Launch{EventID: "snapshot-attempt", TaskID: w.Tasks[0].ID})
			if err != nil {
				t.Fatal(err)
			}
			b, err := os.ReadFile(filepath.Join(copy, "value.txt"))
			if err != nil || string(b) != "selected dirty value\n" {
				t.Fatal(string(b), err)
			}
			if gitTest(t, source, "rev-parse", "HEAD") != original {
				t.Fatal("source HEAD changed")
			}
			replayed, err := s.preparationCommand(r)
			if err != nil || replayed.WorkID != got.WorkID || replayed.Revision != got.Revision {
				t.Fatal("conversion replay changed snapshot", err)
			}
		})
	}
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
	if _, e = s.preparationCommand(conversionRequest(t, s, p, "revise-missions")); e == nil {
		t.Fatal("fresh identical revision silently succeeded")
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

func TestPreparedRevisionPreservesRecoveryAndArchivesReviews(t *testing.T) {
	s, p := preparedTeam(t)
	w, e := s.get(p.WorkID)
	if e != nil {
		t.Fatal(e)
	}
	// Isolated fixture: a produced task and a dependent both have historical verdicts.
	for i := range w.Tasks {
		task := &w.Tasks[i]
		task.Status = "blocked"
		task.IndependentReview = &IndependentReview{ID: "old-" + task.ID, State: "changes_requested"}
		task.Attempts = []Attempt{{ID: "produced-" + task.ID, Status: "completed"}}
	}
	w.Tasks[0].PlanMaxAttempts = 4
	w.Tasks[0].CorrectiveRecovery = &CorrectiveRecovery{Event: "operator-grant"}
	w.Planning.Reviewer.Calls = 4
	raw, _ := json.Marshal(w)
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	spec := p.Conversion.Spec
	spec.Tasks[0].Title = "Clarified documentary scope"
	raw, _ = json.Marshal(spec)
	p = prepSave(t, s, p, "plan", string(raw))
	r := prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	p, e = s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	p, e = s.preparationCommand(conversionRequest(t, s, p, "revise-missions"))
	if e != nil {
		t.Fatal(e)
	}
	got, e := s.get(p.WorkID)
	if e != nil {
		t.Fatal(e)
	}
	if got.Tasks[0].PlanMaxAttempts != 4 || got.Tasks[0].CorrectiveRecovery.Event != "operator-grant" || got.Planning.Reviewer.Calls != 4 {
		t.Fatal("lost grant or refunded review", got.Tasks[0], got.Planning.Reviewer)
	}
	for _, task := range got.Tasks {
		if task.IndependentReview != nil || len(task.PreviousReviews) != 1 || task.PreviousReviews[0].ID != "old-"+task.ID || len(task.Attempts) != 1 || task.Attempts[0].ID != "produced-"+task.ID {
			t.Fatalf("stale review or lost evidence: %+v", task)
		}
	}
}

func TestPreparedOrganizationRepositoryRollback(t *testing.T) {
	for _, late := range []bool{false, true} {
		t.Run(map[bool]string{false: "initial", true: "late"}[late], func(t *testing.T) {
			s, _, _ := prepDialogueFixture(t, prepReplyScript)
			p := readyPreparation(t, s, "")
			if late {
				var err error
				p, err = s.preparationCommand(conversionRequest(t, s, p, "create-missions"))
				if err != nil {
					t.Fatal(err)
				}
			}
			source := filepath.Join(s.root, "source")
			if err := os.MkdirAll(source, 0700); err != nil {
				t.Fatal(err)
			}
			gitTest(t, source, "init")
			if err := os.WriteFile(filepath.Join(source, "value.txt"), []byte("initial"), 0600); err != nil {
				t.Fatal(err)
			}
			gitTest(t, source, "add", ".")
			gitTest(t, source, "commit", "-m", "fixture")
			var beforePrep, beforeWork []byte
			if err := s.db.QueryRow("SELECT body FROM preparations WHERE id=?", p.ID).Scan(&beforePrep); err != nil {
				t.Fatal(err)
			}
			if late {
				if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", p.WorkID).Scan(&beforeWork); err != nil {
					t.Fatal(err)
				}
			}
			r := conversionRequest(t, s, p, "create-missions")
			controls := map[string][]ValidationControl{}
			var spec ActionPlan
			if err := json.Unmarshal([]byte(p.Documents["plan"].Text), &spec); err != nil {
				t.Fatal(err)
			}
			for _, task := range spec.Tasks {
				for _, id := range []string{"plan-entry", "plan-validation", "plan-delivery", "plan-criterion-1"} {
					controls[task.ID] = append(controls[task.ID], ValidationControl{ID: id, Command: []string{"git", "status", "--short"}, Criteria: []int{1}, Justification: "fixture de refus atomique"})
				}
			}
			r.Organization = &PreparationOrganization{Provider: "test", Level: "standard", Workspace: s.root, Validation: "automatic", MaxTasks: 20, MaxCalls: 40, Controls: controls, Repository: &ManagedRepositoryRequest{Path: source, Include: []string{"value.txt"}}}
			if _, err := s.db.Exec("CREATE TRIGGER refuse_fixture_receipt BEFORE INSERT ON preparation_commands BEGIN SELECT RAISE(ABORT, 'injected post-snapshot failure'); END"); err != nil {
				t.Fatal(err)
			}
			if _, err := s.preparationCommand(r); err == nil || !strings.Contains(err.Error(), "injected post-snapshot") {
				t.Fatal("failure not injected after snapshot", err)
			}
			id := "w-" + hash([]byte(p.ID))[:24]
			for _, name := range []string{"repository.git", "repository.json", "repository.json.pending"} {
				if _, err := os.Stat(filepath.Join(s.root, ".swarm", "managed", id, name)); !os.IsNotExist(err) {
					t.Fatal("rejected operation retained a frozen snapshot", name, err)
				}
			}
			var afterPrep, afterWork []byte
			if err := s.db.QueryRow("SELECT body FROM preparations WHERE id=?", p.ID).Scan(&afterPrep); err != nil {
				t.Fatal(err)
			}
			if string(beforePrep) != string(afterPrep) {
				t.Fatal("preparation changed on rollback")
			}
			if late {
				if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", id).Scan(&afterWork); err != nil {
					t.Fatal(err)
				}
				if string(beforeWork) != string(afterWork) {
					t.Fatal("work changed on rollback")
				}
			} else {
				var count int
				if err := s.db.QueryRow("SELECT count(*) FROM works WHERE id=?", id).Scan(&count); err != nil || count != 0 {
					t.Fatal("work retained on rollback", count, err)
				}
			}
			if _, err := s.db.Exec("DROP TRIGGER refuse_fixture_receipt"); err != nil {
				t.Fatal(err)
			}
			r.Organization.Repository = &ManagedRepositoryRequest{Path: source, CommittedOnly: true}
			if _, err := s.preparationCommand(r); err != nil {
				t.Fatal("corrected selection cannot resume", err)
			}
		})
	}
}

func TestPreparedRepositoryRetainsIncompletePendingEvidence(t *testing.T) {
	for _, mode := range []string{"pending-only", "different-request", "malformed"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			repo := w.Planning.Repository
			pending := filepath.Join(repo.Storage, "repository.json.pending")
			raw, err := os.ReadFile(pending)
			if err != nil {
				t.Fatal(err)
			}
			if err := os.Remove(filepath.Join(repo.Storage, "repository.json")); err != nil {
				t.Fatal(err)
			}
			if err := os.RemoveAll(filepath.Join(repo.Storage, "repository.git")); err != nil {
				t.Fatal(err)
			}
			request := ManagedRepositoryRequest{Path: repo.Source, CommittedOnly: true}
			if mode == "different-request" {
				request = ManagedRepositoryRequest{Path: repo.Source, Include: []string{"value.txt"}}
			}
			if mode == "malformed" {
				raw = []byte("{invalid retained original")
				if err := os.WriteFile(pending, raw, 0600); err != nil {
					t.Fatal(err)
				}
			}
			if _, err := s.configureManagedRepository(w.ID, request); err == nil {
				t.Fatal("incomplete retained state overwritten")
			}
			after, err := os.ReadFile(pending)
			if err != nil || string(after) != string(raw) {
				t.Fatal("retained evidence changed", err)
			}
			for _, name := range []string{"repository.git", "repository.json"} {
				if _, err := os.Stat(filepath.Join(repo.Storage, name)); !os.IsNotExist(err) {
					t.Fatal("incomplete recovery mutated artifacts", name, err)
				}
			}
		})
	}
}

func TestPreparedConversionFreshEventsRefusedWithoutMutation(t *testing.T) {
	for _, action := range []string{"create-missions", "authorize-plan"} {
		for _, mode := range []string{"pristine", "task-edited", "missing-lock", "stale-work"} {
			t.Run(action+"/"+mode, func(t *testing.T) {
				s := storeTest(t)
				p := readyPreparation(t, s, "")
				var err error
				p, err = s.preparationCommand(conversionRequest(t, s, p, "create-missions"))
				if err != nil {
					t.Fatal(err)
				}
				w, _ := s.get(p.WorkID)
				r := conversionRequest(t, s, p, action)
				if mode == "task-edited" {
					w.Tasks[0].Title = "altered"
				}
				if mode == "missing-lock" {
					if _, err := s.db.Exec("DELETE FROM preparation_launch_locks WHERE work_id=?", w.ID); err != nil {
						t.Fatal(err)
					}
				}
				if mode == "stale-work" {
					stale := w.Revision + 1
					r.WorkRevision = &stale
				}
				body, _ := json.Marshal(w)
				if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID); err != nil {
					t.Fatal(err)
				}
				var beforePrep []byte
				if err := s.db.QueryRow("SELECT body FROM preparations WHERE id=?", p.ID).Scan(&beforePrep); err != nil {
					t.Fatal(err)
				}
				if _, err := s.preparationCommand(r); err == nil {
					t.Fatal("fresh organization-less event silently succeeded")
				}
				var afterWork, afterPrep []byte
				s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&afterWork)
				s.db.QueryRow("SELECT body FROM preparations WHERE id=?", p.ID).Scan(&afterPrep)
				if string(body) != string(afterWork) || string(beforePrep) != string(afterPrep) {
					t.Fatal("refusal mutated state")
				}
			})
		}
	}
}

func TestPreparedRepositoryRejectsNonCanonicalManifest(t *testing.T) {
	for _, state := range []string{"pending", "final", "both"} {
		for _, field := range []string{"storage", "source", "project_subdir", "candidate_commit", "base_commit", "snapshot_digest", "included"} {
			t.Run(state+"/"+field, func(t *testing.T) {
				s, w := managedFixture(t)
				repo := w.Planning.Repository
				name := "repository.json"
				if state == "pending" || state == "both" {
					name += ".pending"
				}
				path := filepath.Join(repo.Storage, name)
				original, err := os.ReadFile(path)
				if err != nil {
					t.Fatal(err)
				}
				var raw map[string]any
				if err := json.Unmarshal(original, &raw); err != nil {
					t.Fatal(err)
				}
				if field == "included" {
					raw[field] = []string{"foreign.txt"}
				} else if field == "storage" || field == "source" || field == "project_subdir" {
					raw[field] = "../redirected"
				} else {
					delete(raw, field)
				}
				changed, _ := json.Marshal(raw)
				if err := os.WriteFile(path, changed, 0600); err != nil {
					t.Fatal(err)
				}
				if state == "pending" {
					if err := os.Remove(filepath.Join(repo.Storage, "repository.json")); err != nil {
						t.Fatal(err)
					}
				}
				if _, err := s.configureManagedRepository(w.ID, ManagedRepositoryRequest{Path: repo.Source, CommittedOnly: true}); err == nil {
					t.Fatal("noncanonical manifest adopted")
				}
				after, err := os.ReadFile(path)
				if err != nil || string(after) != string(changed) {
					t.Fatal("retained evidence rewritten", err)
				}
				if state == "pending" {
					if _, err := os.Stat(filepath.Join(repo.Storage, "repository.json")); !os.IsNotExist(err) {
						t.Fatal("invalid pending manifest promoted", err)
					}
				}
			})
		}
	}
}

func TestPreparedRepositoryRejectsDivergentManifests(t *testing.T) {
	s, w := managedFixture(t)
	repo := w.Planning.Repository
	pending := filepath.Join(repo.Storage, "repository.json.pending")
	final := filepath.Join(repo.Storage, "repository.json")
	original, err := os.ReadFile(final)
	if err != nil {
		t.Fatal(err)
	}
	var prior ManagedRepository
	if err := json.Unmarshal(original, &prior); err != nil {
		t.Fatal(err)
	}
	prior.Snapshot = strings.Repeat("0", 64)
	changed, _ := json.Marshal(prior)
	if err := os.WriteFile(pending, changed, 0600); err != nil {
		t.Fatal(err)
	}
	if _, err := s.configureManagedRepository(w.ID, ManagedRepositoryRequest{Path: repo.Source, CommittedOnly: true}); err == nil {
		t.Fatal("divergent canonical-shaped manifests silently accepted")
	}
	for path, expected := range map[string][]byte{pending: changed, final: original} {
		actual, err := os.ReadFile(path)
		if err != nil || string(actual) != string(expected) {
			t.Fatal("manifest changed on refusal", path, err)
		}
	}
}
