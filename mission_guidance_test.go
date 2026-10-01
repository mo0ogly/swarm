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
