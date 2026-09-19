//go:build linux

package main

import (
	"strings"
	"testing"
	"time"
)

func failedRecoveryAgent(id, task, category, detail string, ended time.Time) Agent {
	a := Agent{ID: id, TaskID: task, Status: "failed", Origin: originConductor,
		Activity: detail, Ended: ended.UTC().Format(time.RFC3339Nano),
		Recovery: RecoveryState{OperationID: "operation-" + task, AutomaticMax: maxAutomaticAttempts, AutomaticUsed: 1, BudgetRemaining: 1}}
	if category != "" {
		a.Diagnostic = AttemptDiagnostic{AgentID: id, Items: []DiagnosticItem{{Category: category, Cause: detail, Traces: []string{detail}}}}
	}
	return a
}

func TestRecoveryEnvironmentSameCauseDoesNotRetryAndIndependentBranchContinues(t *testing.T) {
	at := time.Date(2026, 9, 18, 12, 0, 0, 0, time.UTC)
	tasks := []Task{
		{ID: "environment", Status: "blocked", Profile: &LaunchProfile{Provider: "fixture", Workspace: "/tmp/recovery-environment"}},
		{ID: "independent", Status: "todo", Profile: &LaunchProfile{Provider: "fixture", Workspace: "/tmp/recovery-independent"}},
	}
	in := dispatchTest(tasks, []Agent{failedRecoveryAgent("a-env", "environment", "environment", "mount failed: permission denied", at.Add(-time.Minute))})
	in.at = at
	list, reason := planDispatch(in)
	if got := dispatchedTasks(list); len(got) != 1 || got[0] != "independent" {
		t.Fatalf("branche indépendante arrêtée ou défaut environnemental relancé : %v", got)
	}
	if !strings.Contains(reason, "nouvelle vérification technique") {
		t.Fatalf("retenue environnementale non expliquée : %q", reason)
	}
}

func TestRecoveryTransientKeepsOperationIdentityAndStopsCommonCauseLoop(t *testing.T) {
	at := time.Date(2026, 9, 18, 12, 0, 0, 0, time.UTC)
	delayed := failedRecoveryAgent("a-delayed", "t1", "tool", "connection reset by peer", at.Add(-500*time.Millisecond))
	delayedInput := dispatchTest([]Task{{ID: "t1", Status: "blocked"}}, []Agent{delayed})
	delayedInput.at = at
	if list, reason := planDispatch(delayedInput); len(list) != 0 || !strings.Contains(reason, "différée") {
		t.Fatalf("délai de reprise ignoré : %+v %q", list, reason)
	}
	a := failedRecoveryAgent("a-one", "t1", "tool", "connection reset by peer", at.Add(-2*time.Second))
	in := dispatchTest([]Task{{ID: "t1", Status: "blocked"}}, []Agent{a})
	in.at = at
	list, reason := planDispatch(in)
	if len(list) != 1 || reason != "" {
		t.Fatalf("incident transitoire non repris : %+v %q", list, reason)
	}
	d := list[0]
	if d.Previous != a.ID || d.OperationID != a.Recovery.OperationID || d.RecoveryCategory != recoveryTransient || d.CauseFingerprint == "" {
		t.Fatalf("identité ou cause perdue : %+v", d)
	}

	repeated := failedRecoveryAgent("a-two", "t1", "tool", "connection reset by peer", at.Add(-time.Second))
	repeated.Previous = a.ID
	repeated.Recovery = RecoveryState{OperationID: d.OperationID, Category: d.RecoveryCategory, CauseFingerprint: d.CauseFingerprint,
		AutomaticMax: maxAutomaticAttempts, AutomaticUsed: 2, BudgetRemaining: 0}
	in.agents = []Agent{repeated, a}
	list, reason = planDispatch(in)
	if len(list) != 0 || (!strings.Contains(reason, "cause commune") && !strings.Contains(reason, "budget")) {
		t.Fatalf("boucle de cause commune non arrêtée : %+v %q", list, reason)
	}
}

func TestRecoveryMissingSignalIsNotSuccessOrAutomaticRetry(t *testing.T) {
	a := Agent{ID: "a-silent", TaskID: "t1", Status: "failed", Origin: originConductor,
		Recovery: RecoveryState{OperationID: "operation-t1", AutomaticMax: maxAutomaticAttempts, AutomaticUsed: 1, BudgetRemaining: 1}}
	in := dispatchTest([]Task{{ID: "t1", Status: "blocked"}}, []Agent{a})
	list, reason := planDispatch(in)
	if len(list) != 0 || !strings.Contains(reason, "signal absent") {
		t.Fatalf("absence de signal transformée en reprise ou succès : %+v %q", list, reason)
	}
}

func TestRecoveryConflictRecalculatesAndBusinessNeedsAuthorizedCorrection(t *testing.T) {
	at := time.Date(2026, 9, 18, 12, 0, 0, 0, time.UTC)
	conflict := failedRecoveryAgent("a-conflict", "conflict", "tool", "revision conflict on canonical record", at.Add(-3*time.Second))
	business := failedRecoveryAgent("a-business", "business", "check", "go test failed", at.Add(-3*time.Second))
	tasks := []Task{{ID: "conflict", Status: "blocked", Profile: &LaunchProfile{Provider: "fixture", Workspace: "/tmp/recovery-conflict"}},
		{ID: "business", Status: "blocked", Profile: &LaunchProfile{Provider: "fixture", Workspace: "/tmp/recovery-business"}}}
	in := dispatchTest(tasks, []Agent{conflict, business})
	in.at = at
	list, reason := planDispatch(in)
	if len(list) != 1 || list[0].TaskID != "conflict" || list[0].RecoveryCategory != recoveryConflict {
		t.Fatalf("le conflit doit recalculer seul : %+v / %q", list, reason)
	}
	if !strings.Contains(recoveryInstruction(list[0].RecoveryCategory, list[0].OperationID, list[0].CauseFingerprint), "recalculer") {
		t.Fatal("consigne de recalcul absente")
	}
	if !strings.Contains(reason, "aucun cycle de correction") {
		t.Fatalf("correction métier non autorisée mal expliquée : %q", reason)
	}
	tasks[1].PlanMaxAttempts = 3
	in = dispatchTest([]Task{tasks[1]}, []Agent{business})
	in.at = at
	list, _ = planDispatch(in)
	if len(list) != 1 || list[0].TaskID != "business" || list[0].RecoveryCategory != recoveryBusiness {
		t.Fatalf("correction explicitement bornée non planifiée : %+v", list)
	}
}

func TestRecoveryBudgetAndBoundPersistAcrossStoreRestart(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	first, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	first.Diagnostic = AttemptDiagnostic{AgentID: first.ID, Items: []DiagnosticItem{{Category: "tool", Cause: "temporary failure", Traces: []string{"temporary failure"}}}}
	if err = s.finishAgent(first, "failed", "temporary failure", nil); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	cause := recoveryCauseFingerprint(first, recoveryTransient)
	retry := launch
	retry.EventID = "recovery-persisted"
	retry.Revision = w.Revision
	retry.Previous = first.ID
	retry.Origin = originConductor
	if err = organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err = s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	if owned, claimErr := s.claimMissionSupervision(w.ID, "recovery-test", "recette", time.Now()); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	retry.ConductorID = "recovery-test"
	retry.recoveryCategory = recoveryTransient
	retry.recoveryCause = cause
	retry.recoveryOperation = first.Recovery.OperationID
	retry.recoveryNext = time.Now().UTC().Format(time.RFC3339Nano)
	next, created, err := s.prepare(w.ID, retry)
	if err != nil || !created {
		t.Fatalf("reprise non créée : %+v %t %v", next, created, err)
	}
	if next.Recovery.AutomaticUsed != 1 || next.Recovery.BudgetRemaining != 1 || next.Recovery.AutomaticMax != maxAutomaticAttempts {
		t.Fatalf("borne initiale incorrecte : %+v", next.Recovery)
	}
	restarted, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer restarted.db.Close()
	stored, err := restarted.agent(next.ID)
	if err != nil {
		t.Fatal(err)
	}
	if stored.Recovery != next.Recovery || stored.Recovery.OperationID != first.ID || stored.Recovery.CauseFingerprint != cause || stored.Recovery.NextEligibleAt == "" {
		t.Fatalf("budget ou borne perdu après redémarrage : avant=%+v après=%+v", next.Recovery, stored.Recovery)
	}
	// Rejouer exactement le même event_id relit l'intention sans créer un
	// second effet ni consommer une unité de budget supplémentaire.
	replayed, created, err := restarted.prepare(w.ID, retry)
	if err != nil || created || replayed.ID != next.ID || replayed.Recovery.BudgetRemaining != 1 {
		t.Fatalf("rejeu non idempotent : %+v created=%t err=%v", replayed, created, err)
	}
}
