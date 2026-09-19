//go:build linux

package main

import (
	"strings"
	"testing"
)

func environmentFailure(id, task, origin string) Agent {
	return Agent{
		ID: id, TaskID: task, Status: "failed", Origin: origin,
		Diagnostic: AttemptDiagnostic{AgentID: id, Items: []DiagnosticItem{{Category: "environment"}}},
	}
}

func TestDispatchHoldsEnvironmentFailureAndContinuesIndependentBranch(t *testing.T) {
	tasks := []Task{
		{ID: "blocked-env", Status: "blocked", Profile: &LaunchProfile{Provider: "fixture", Role: "worker", Workspace: "/tmp/ws-env"}},
		{ID: "independent", Status: "todo", Profile: &LaunchProfile{Provider: "fixture", Role: "worker", Workspace: "/tmp/ws-independent"}},
	}
	in := dispatchTest(tasks, []Agent{environmentFailure("a-env", "blocked-env", originConductor)})
	list, reason := planDispatch(in)
	if got := dispatchedTasks(list); len(got) != 1 || got[0] != "independent" {
		t.Fatalf("la branche indépendante doit continuer, obtenu %v", got)
	}
	if !strings.Contains(reason, "aucune relance automatique") || !strings.Contains(reason, "blocked-env") {
		t.Fatalf("retenue environnementale non expliquée : %q", reason)
	}
	state, explanation := missionDispatchState(in, tasks[0])
	if state != "intervention" || !strings.Contains(explanation, "relance automatique retenue") {
		t.Fatalf("état utilisateur imprécis : %s / %s", state, explanation)
	}
}

func TestEnvironmentRetryConsoleShowsVerificationField(t *testing.T) {
	a := environmentFailure("a-env", "t1", originOperator)
	d := &taskDialog{
		task: Task{ID: "t1", Title: "Montage"}, agent: &a, mode: "retry",
		providers: []string{"fixture"}, workspace: "/tmp/ws", instruction: "reprendre",
	}
	rendered := renderTaskDialog(strings.Repeat("\r\n", 40), d, 120, 40)
	if !strings.Contains(rendered, "Vérification nouvelle") || !strings.Contains(rendered, "obligatoire") {
		t.Fatalf("champ de reprise environnementale absent de la console : %q", rendered)
	}
}

func TestEnvironmentRetryRequiresNewEvidenceAndKeepsLimits(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	launch.Limits = &RunLimits{MaxToolCalls: 12, MaxRepeatedCalls: 2}
	previous, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.finishAgent(previous, "failed", "permission denied sur le montage de recette", nil); err != nil {
		t.Fatal(err)
	}
	previous, err = s.agent(previous.ID)
	if err != nil {
		t.Fatal(err)
	}
	previous.Diagnostic = buildAttemptDiagnostic(previous.ID, previous.Attempt, []toolFailure{{name: "command_execution", technical: "mount failed: permission denied"}}, "", previous.Limits.MaxConsecutiveErrors)
	if err = s.saveAgent(previous); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	bypass := launch
	bypass.EventID = newID("start-")
	bypass.Revision = w.Revision
	if _, _, err = s.prepare(w.ID, bypass); err == nil || !strings.Contains(err.Error(), "reprise explicite") {
		t.Fatalf("nouveau départ accepté en contournement de la reprise : %v", err)
	}
	retry := launch
	retry.EventID = newID("retry-")
	retry.Revision = w.Revision
	retry.Previous = previous.ID
	retry.Limits = nil
	if _, _, err = s.prepare(w.ID, retry); err == nil || !strings.Contains(err.Error(), "vérification nouvelle") {
		t.Fatalf("reprise sans preuve acceptée ou mal expliquée : %v", err)
	}

	retry.PreconditionEvidence = "Lecture hôte réussie du montage de recette en écriture"
	next, created, err := s.prepare(w.ID, retry)
	if err != nil || !created {
		t.Fatalf("reprise vérifiée refusée : created=%t err=%v", created, err)
	}
	if next.Limits.MaxToolCalls != 12 || next.Limits.MaxRepeatedCalls != 2 {
		t.Fatalf("protections relâchées pendant la reprise : %+v", next.Limits)
	}
	if next.PreconditionEvidence != retry.PreconditionEvidence || !strings.Contains(next.Prompt, retry.PreconditionEvidence) {
		t.Fatalf("preuve de reprise non conservée : %+v", next)
	}
	if err = s.finishAgent(next, "failed", "mount failed: permission denied", nil); err != nil {
		t.Fatal(err)
	}
	next, _ = s.agent(next.ID)
	next.Diagnostic = buildAttemptDiagnostic(next.ID, next.Attempt, []toolFailure{{technical: "mount failed: permission denied"}}, "", next.Limits.MaxConsecutiveErrors)
	if err = s.saveAgent(next); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	repeated := retry
	repeated.EventID = newID("retry-")
	repeated.Revision = w.Revision
	repeated.Previous = next.ID
	repeated.PreconditionEvidence = "  LECTURE hôte réussie   du montage de recette en écriture  "
	if _, _, err = s.prepare(w.ID, repeated); err == nil || !strings.Contains(err.Error(), "identique") {
		t.Fatalf("preuve identique acceptée après le même échec : %v", err)
	}
}
