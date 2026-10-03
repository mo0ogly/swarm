package main

import "testing"

func TestMissionGuidancePrioritizesActionableBlockOverConfiguration(t *testing.T) {
	d := MissionStatus{Total: 3, Organization: Organization{Ready: true}, Understanding: MissionUnderstanding{What: "An execution needs attention", NextStep: "Inspect evidence", Actor: "You"}, Tasks: []MissionTask{{ID: "setup", State: "configure", Action: "configure"}, {ID: "low", State: "intervention", Action: "inspect", Impact: 1}, {ID: "high", State: "intervention", Action: "inspect", Impact: 4}}}
	g := missionGuidance(Work{}, d)
	if g.Primary.Task != "high" || g.Primary.Kind != "task" || g.What != d.Understanding.What || g.Next != d.Understanding.NextStep {
		t.Fatalf("wrong authoritative action: %+v", g)
	}
	d.Tasks[2].AttemptLimitReached = true
	d.Tasks[2].Action = "recovery"
	g = missionGuidance(Work{}, d)
	if g.Primary.Task != "high" || g.Primary.Label != "Examiner les tentatives et les refus" {
		t.Fatalf("exhausted task offered a launch: %+v", g)
	}
}
func TestMissionGuidanceKeepsInfrastructureAndPlanningPriority(t *testing.T) {
	d := MissionStatus{Total: 1, Organization: Organization{Ready: true}, Tasks: []MissionTask{{ID: "blocked", State: "intervention", Action: "inspect"}}}
	d.Runtime.State = "blocked"
	if missionGuidance(Work{}, d).Primary.Kind != "runtime" {
		t.Fatal("storage incident hidden by task")
	}
	d.Runtime.State = ""
	d.Organization.Ready = false
	if missionGuidance(Work{}, d).Primary.Kind != "organization" {
		t.Fatal("missing organization bypassed")
	}
	d.Organization.Ready = true
	d.Tasks[0].State = "waiting"
	failed := missionGuidance(Work{Planning: &PlanningState{Failure: "planning provider failed"}}, d)
	if failed.Primary.Kind != "planning" || failed.What == d.Understanding.What || len(failed.What) > 160 {
		t.Fatal("planning incident hidden by task")
	}
	if missionGuidance(Work{Planning: &PlanningState{Paused: true}}, d).Primary.Kind != "planning-resume" {
		t.Fatal("paused planning bypassed")
	}
}
func TestMissionGuidanceAutomaticReviewDoesNotRequestHumanValidation(t *testing.T) {
	d := MissionStatus{Organization: Organization{Ready: true}, Tasks: []MissionTask{{ID: "review", State: "review", ValidationMode: "automatic", Action: "inspect"}}}
	if got := missionGuidance(Work{}, d); got.Primary.Label != "Voir les contrôles en cours" || got.Primary.Tone != "info" {
		t.Fatalf("automatic checks treated as human acceptance: %+v", got)
	}
	d.Total = 1
	d.Validated = 1
	d.Tasks[0].State = "validated"
	if got := missionGuidance(Work{}, d); got.Primary.Kind != "results" || got.Primary.Tone != "succes" {
		t.Fatalf("validated result absent: %+v", got)
	}
	d.Tasks[0].State = "waived"
	d.Validated = 0
	if missionGuidance(Work{}, d).Primary.Tone == "succes" {
		t.Fatal("waiver presented as validated success")
	}
}

func TestPlanningFailureDoesNotHideExistingTaskActions(t *testing.T) {
	w := Work{Planning: &PlanningState{Failure: "planner timed out"}}
	for _, tc := range []struct{ state, kind string }{
		{"running", "follow"}, {"review", "task"}, {"intervention", "task"}, {"configure", "task"},
	} {
		t.Run(tc.state, func(t *testing.T) {
			d := MissionStatus{Total: 1, Organization: Organization{Ready: true},
				Tasks: []MissionTask{{ID: "existing", State: tc.state, Action: "inspect", ValidationMode: "human"}}}
			if tc.state == "running" {
				d.Running, d.ActiveAgents = 1, 1
			}
			if tc.state == "review" {
				d.Review = 1
			}
			d.Understanding = missionUnderstanding(d)
			g := missionGuidance(w, d)
			if missionPlanningOwnsNextStep(d) || g.Primary.Kind != tc.kind || g.Primary.Task != "existing" || g.What != d.Understanding.What || g.Next != d.Understanding.NextStep {
				t.Fatalf("planning error replaced actual task action: %+v", g)
			}
		})
	}
	for _, state := range []string{"ready", "manual"} {
		if missionPlanningOwnsNextStep(MissionStatus{Tasks: []MissionTask{{State: state}}}) {
			t.Fatalf("existing %s task treated as awaiting a new planning decision", state)
		}
	}
}

func TestMissionStatusKeepsExistingAttemptVisibleAfterPlanningFailure(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	// Prepare a real engine reservation in an isolated root, without launching
	// any provider. It remains an observable queued attempt.
	a, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	organizedFixtureStore(t, s)
	w, _ = s.get(w.ID)
	_, err = s.mutate(w.ID, "planning.test-failure", newID("e-"), w.Revision, []byte("{}"), func(w *Work) error {
		w.Planning.Paused = false
		w.Planning.Failure = "planner timed out"
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	d, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if d.Running != 1 || d.Guidance.Primary.Kind != "follow" || d.Guidance.Primary.Task != launch.TaskID || d.Understanding.ActorKind == "user" || d.Understanding.What == "planner timed out" {
		t.Fatalf("planning failure hid reserved attempt: %+v", d)
	}
	current, _ := s.get(w.ID)
	stored, _ := s.agent(a.ID)
	if current.Planning.Failure != "planner timed out" || stored.Status != a.Status {
		t.Fatal("presentation changed the planning diagnostic or attempt")
	}
}
