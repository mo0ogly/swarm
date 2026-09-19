//go:build linux

package main

import (
	"context"
	"errors"
	"os"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestDurableCoordinatorDeduplicatesConcurrentLoops(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	if err := s.setProfile(w.ID, "", LaunchProfile{Provider: request.Provider, Workspace: s.root, Role: "worker"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	if err := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()

	start := make(chan struct{})
	var group sync.WaitGroup
	for i, store := range []*Store{s, other} {
		group.Add(1)
		go func(index int, candidate *Store) {
			defer group.Done()
			<-start
			candidate.missionOwnedCycle(map[string]string{}, map[string]bool{},
				[]string{"conductor-one", "conductor-two"}[index], "recette", w.ID)
		}(i, store)
	}
	close(start)
	group.Wait()
	agents, err := s.agents(w.ID)
	if err != nil || len(agents) != 1 {
		t.Fatalf("deux conducteurs ont créé %d tentatives : %v", len(agents), err)
	}
	var leases int
	if err = s.db.QueryRow("SELECT count(*) FROM mission_coordination_events WHERE work_id=? AND kind='lease-acquired'", w.ID).Scan(&leases); err != nil || leases != 1 {
		t.Fatalf("possession non dédupliquée : %d, %v", leases, err)
	}
	_ = s.reconcile(agents[0].ID)
}

func TestDurableCoordinatorFencesExpiredOwnerAtLaunchTransaction(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	if err := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	at := time.Now().Add(-missionConductorStaleAfter - time.Second)
	if owned, err := s.claimMissionSupervision(w.ID, "expired-owner", "recette", at); err != nil || !owned {
		t.Fatal(owned, err)
	}
	if owned, err := s.claimMissionSupervision(w.ID, "current-owner", "recette", time.Now()); err != nil || !owned {
		t.Fatal(owned, err)
	}
	request.Origin = originConductor
	request.ConductorID = "expired-owner"
	if _, _, err := s.prepare(w.ID, request); err == nil || !strings.Contains(err.Error(), "bail du conducteur perdu") {
		t.Fatalf("ancien propriétaire non neutralisé : %v", err)
	}
	agents, err := s.agents(w.ID)
	if err != nil || len(agents) != 0 {
		t.Fatalf("le propriétaire expiré a réservé : %+v, %v", agents, err)
	}
}

func TestDurableCoordinatorRestartResumesSamePersistedAttempt(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	agent, created, err := s.prepare(w.ID, request)
	if err != nil || !created {
		t.Fatal(agent, created, err)
	}
	restarted, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer restarted.db.Close()
	if owned, claimErr := restarted.claimMissionSupervision(w.ID, "after-restart", "recette", time.Now()); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	launched := ""
	err = restarted.reconcileMissionAttempts(w.ID, "after-restart", func(candidate Agent) error {
		launched = candidate.ID
		return restarted.supervise(candidate.ID)
	})
	if err != nil {
		t.Fatal(err)
	}
	if launched != agent.ID {
		t.Fatalf("la reprise a changé d’identité : %s au lieu de %s", launched, agent.ID)
	}
	agents, err := restarted.agents(w.ID)
	if err != nil || len(agents) != 1 || agents[0].Status != "completed" {
		t.Fatalf("tentative perdue ou doublée : %+v, %v", agents, err)
	}
	if err = restarted.reconcileMissionAttempts(w.ID, "after-restart", func(Agent) error {
		return errors.New("une tentative terminale ne doit pas être relancée")
	}); err != nil {
		t.Fatal(err)
	}
	var resumed, known int
	_ = restarted.db.QueryRow("SELECT count(*) FROM mission_coordination_events WHERE event_key=?", "resume-intent:"+agent.ID).Scan(&resumed)
	_ = restarted.db.QueryRow("SELECT count(*) FROM mission_coordination_events WHERE event_key=?", "known-result:"+agent.ID).Scan(&known)
	if resumed != 1 || known != 1 {
		t.Fatalf("reçus idempotents absents : reprise=%d résultat=%d", resumed, known)
	}
}

func TestDurableCoordinatorClassifiesGoneProcessWithoutInventingResult(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	agent, _, err := s.prepare(w.ID, request)
	if err != nil {
		t.Fatal(err)
	}
	agent.Status = "running"
	agent.Supervisor = 2147483000
	agent.SupervisorStamp = "absent"
	agent.Child = 2147483001
	agent.ChildStamp = "absent"
	agent.Heartbeat = time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano)
	if err = s.saveAgent(agent); err != nil {
		t.Fatal(err)
	}
	if owned, claimErr := s.claimMissionSupervision(w.ID, "recovery", "recette", time.Now()); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	if err = s.reconcileMissionAttempts(w.ID, "recovery", func(Agent) error {
		return errors.New("aucune intention queued")
	}); err != nil {
		t.Fatal(err)
	}
	got, err := s.agent(agent.ID)
	if err != nil || got.Status != "interrupted" || strings.Contains(strings.ToLower(got.Activity), "réussi") {
		t.Fatalf("processus disparu confondu avec un résultat : %+v, %v", got, err)
	}
	var gone int
	if err = s.db.QueryRow("SELECT count(*) FROM mission_coordination_events WHERE event_key=?", "process-gone:"+agent.ID).Scan(&gone); err != nil || gone != 1 {
		t.Fatalf("classification non persistée : %d, %v", gone, err)
	}
}

func TestDurableCoordinatorWorkspaceReleaseWakesWaitingMission(t *testing.T) {
	s := storeTest(t)
	holderWork, holderRequest := setupAgent(t, s)
	holderRequest.Workspace = s.root
	holder, _, err := s.prepare(holderWork.ID, holderRequest)
	if err != nil {
		t.Fatal(err)
	}
	waitingWork, _ := setupAgent(t, s)
	if err = s.setProfile(waitingWork.ID, "", LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}, waitingWork.Revision); err != nil {
		t.Fatal(err)
	}
	if err = organizedFixtureStore(t, s).setAutonomy(waitingWork.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err = s.setMission(waitingWork.ID, true); err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	done := make(chan struct{})
	go func() {
		s.missionLoop(ctx, waitingWork.ID)
		close(done)
	}()
	defer func() {
		cancel()
		select {
		case <-done:
		case <-time.After(2 * time.Second):
			t.Error("le conducteur ne s’est pas arrêté")
		}
	}()

	deadline := time.Now().Add(2 * time.Second)
	for {
		var message string
		err = s.db.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='dispatch' ORDER BY rowid DESC LIMIT 1", waitingWork.ID).Scan(&message)
		if err == nil && strings.Contains(message, "espace de travail déjà occupé") {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("attente de ressource non persistée : %q, %v", message, err)
		}
		time.Sleep(20 * time.Millisecond)
	}
	if err = s.finishAgent(holder, "interrupted", "libération de la recette isolée", nil); err != nil {
		t.Fatal(err)
	}
	releasedAt := time.Now()
	deadline = releasedAt.Add(2*missionPollInterval + time.Second)
	for {
		agents, listErr := s.agents(waitingWork.ID)
		if listErr != nil {
			t.Fatal(listErr)
		}
		if len(agents) == 1 {
			if time.Since(releasedAt) > 2*missionPollInterval+time.Second {
				t.Fatal("réveil hors borne")
			}
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("le dossier libéré n’a pas réveillé la mission autorisée")
		}
		time.Sleep(20 * time.Millisecond)
	}
}

// Simulates a crash after the terminal agent row was persisted but before
// finishAgent updated its budget reservation.
func TestDurableCoordinatorRecoversTerminalBudgetAfterCrash(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("INSERT INTO reservations(agent_id,work_id,amount,state) VALUES(?,?,1,'reserved')", a.ID, w.ID); err != nil {
		t.Fatal(err)
	}
	a.Status = "completed"
	a.Child = 42
	a.Ended = "2026-09-18T10:00:00Z"
	reported := 0.42
	a.Usage = &Usage{ReportedCost: &reported, Source: "recette fournisseur"}
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if owned, claimErr := s.claimMissionSupervision(w.ID, "restart", "recette", time.Now()); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	if err = s.reconcileMissionAttempts(w.ID, "restart", func(Agent) error { t.Fatal("terminal agent relaunched"); return nil }); err != nil {
		t.Fatal(err)
	}
	var state string
	var amount float64
	if err = s.db.QueryRow("SELECT state,amount FROM reservations WHERE agent_id=?", a.ID).Scan(&state, &amount); err != nil {
		t.Fatal(err)
	}
	if state != "estimated" || amount != 1 {
		t.Fatalf("terminal budget not settled exactly once: state=%s amount=%v", state, amount)
	}
	budget, err := s.budget(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if budget.Reserved != 0 || budget.Estimated != 1 || budget.ActualCost == nil || *budget.ActualCost != reported {
		t.Fatalf("recovered budget lost reservation or reported cost: %+v", budget)
	}
	stored, err := s.agent(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	if stored.Ended != a.Ended || stored.Usage == nil || stored.Usage.ReportedCost == nil || *stored.Usage.ReportedCost != reported {
		t.Fatalf("reconciliation rewrote terminal evidence: %+v", stored)
	}
	if err = s.reconcileMissionAttempts(w.ID, "restart", func(Agent) error { t.Fatal("terminal agent relaunched on replay"); return nil }); err != nil {
		t.Fatal(err)
	}
	if err = s.db.QueryRow("SELECT state,amount FROM reservations WHERE agent_id=?", a.ID).Scan(&state, &amount); err != nil || state != "estimated" || amount != 1 {
		t.Fatalf("budget settlement replay changed durable cost: state=%s amount=%v err=%v", state, amount, err)
	}
}

func TestDurableCoordinatorResumesBlockedHandoffAndValidationOnce(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	if owned, err := s.claimMissionSupervision(w.ID, "restart", "recette", time.Now()); err != nil || !owned {
		t.Fatal(owned, err)
	}
	if err := s.reconcileMissionAttempts(w.ID, "restart", func(Agent) error {
		t.Fatal("terminal agent relaunched")
		return nil
	}); err != nil {
		t.Fatal(err)
	}
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := got.task("t1")
	if task.Status != "accepted" || task.AutoValidation == nil || task.AutoValidation.Attempt != a.Attempt {
		t.Fatalf("blocked handoff not resumed through authorized validation: %+v", task)
	}
	if err = s.reconcileMissionAttempts(w.ID, "restart", func(Agent) error {
		t.Fatal("terminal agent relaunched on replay")
		return nil
	}); err != nil {
		t.Fatal(err)
	}
	var submissions, validations, decisions int
	if err = s.db.QueryRow("SELECT count(*) FROM events WHERE work_id=? AND kind='task.submit'", w.ID).Scan(&submissions); err != nil {
		t.Fatal(err)
	}
	if err = s.db.QueryRow("SELECT count(*) FROM events WHERE work_id=? AND kind='task.auto-validation'", w.ID).Scan(&validations); err != nil {
		t.Fatal(err)
	}
	if err = s.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='validation-accepted'", w.ID).Scan(&decisions); err != nil {
		t.Fatal(err)
	}
	if submissions != 1 || validations != 1 || decisions != 1 {
		t.Fatalf("replay duplicated recovery: submissions=%d validations=%d decisions=%d", submissions, validations, decisions)
	}
}

func TestDurableCoordinatorDoesNotRelayOldReportForRecoveredAttempt(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	report := writeReport(t, s, "t1.md", "rapport de la tentative précédente")
	old := time.Now().Add(-time.Hour)
	if err := os.Chtimes(report, old, old); err != nil {
		t.Fatal(err)
	}
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Status = "completed"
	a.Ended = now()
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	applyTest(t, s, current, "task.update", Request{ID: "t1", Status: "blocked", Outcome: "completed", Blocker: "crash avant relais"})
	if owned, claimErr := s.claimMissionSupervision(w.ID, "restart", "recette", time.Now()); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	for i := 0; i < 2; i++ {
		if err = s.reconcileMissionAttempts(w.ID, "restart", func(Agent) error {
			t.Fatal("terminal agent relaunched")
			return nil
		}); err != nil {
			t.Fatal(err)
		}
	}
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if got.Tasks[0].Status != "blocked" {
		t.Fatalf("old report was relayed to the recovered attempt: %+v", got.Tasks[0])
	}
	stored, err := s.agent(a.ID)
	if err != nil || !strings.Contains(stored.Relay, "antérieur") {
		t.Fatalf("stale report refusal not persisted: relay=%q err=%v", stored.Relay, err)
	}
	var submissions int
	if err = s.db.QueryRow("SELECT count(*) FROM events WHERE work_id=? AND kind='task.submit'", w.ID).Scan(&submissions); err != nil || submissions != 0 {
		t.Fatalf("stale report produced a submission: count=%d err=%v", submissions, err)
	}
}

func TestDurableCoordinatorRejectsReconciliationByReplacedOwner(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("INSERT INTO reservations(agent_id,work_id,amount,state) VALUES(?,?,1,'reserved')", a.ID, w.ID); err != nil {
		t.Fatal(err)
	}
	a.Status = "completed"
	a.Ended = now()
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	stale := time.Now().Add(-missionConductorStaleAfter - time.Second)
	if owned, claimErr := s.claimMissionSupervision(w.ID, "old-owner", "recette", stale); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	if owned, claimErr := s.claimMissionSupervision(w.ID, "new-owner", "recette", time.Now()); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	if err = s.reconcileMissionAttempts(w.ID, "old-owner", func(Agent) error { return nil }); err == nil || !strings.Contains(err.Error(), "bail de supervision remplacé") {
		t.Fatalf("replaced owner reconciled without fencing: %v", err)
	}
	var state string
	if err = s.db.QueryRow("SELECT state FROM reservations WHERE agent_id=?", a.ID).Scan(&state); err != nil || state != "reserved" {
		t.Fatalf("replaced owner mutated the reservation: state=%s err=%v", state, err)
	}
	before, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if before.Tasks[0].Status != "running" {
		t.Fatalf("replaced owner mutated the task: %+v", before.Tasks[0])
	}
	if err = s.reconcileMissionAttempts(w.ID, "new-owner", func(Agent) error { return nil }); err != nil {
		t.Fatal(err)
	}
	if err = s.db.QueryRow("SELECT state FROM reservations WHERE agent_id=?", a.ID).Scan(&state); err != nil || state != "released" {
		t.Fatalf("current owner did not recover reservation: state=%s err=%v", state, err)
	}
}
