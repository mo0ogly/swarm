//go:build linux

package main

import (
	"errors"
	"sync"
	"testing"
	"time"
)

func automationC01Fixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	organizedFixtureStore(t, s)
	w, _ = s.get(w.ID)
	var err error
	w, err = s.planningChange(w.ID, "resume", PlanningRequest{Schema: 1, EventID: newID("automation-resume-"), Revision: w.Revision, Scope: "root"})
	if err != nil {
		t.Fatal(err)
	}
	if err := s.setProfile(w.ID, "", LaunchProfile{Provider: "fixture", Role: "worker", Workspace: s.root}, w.Revision); err != nil {
		t.Fatal(err)
	}
	if err := s.setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	return s, w
}

func automationC01Request(key string, w Work) AutomationResumeRequest {
	r := AutomationResumeRequest{Schema: 1, IdempotencyKey: key, Source: "fixture locale", TargetWorkID: w.ID, Action: "request_resume"}
	r.ContentDigest = automationResumeDigest(r)
	return r
}

func TestAutomationC01Persistence(t *testing.T) {
	s, w := automationC01Fixture(t)
	created, fresh, err := s.submitAutomationResume(automationC01Request("persist-one", w))
	if err != nil || !fresh || created.RequestID == "" || created.OccurrenceID == "" || created.RequestID == created.OccurrenceID {
		t.Fatalf("demande non créée avec identités distinctes : %+v, fresh=%v, err=%v", created, fresh, err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	got, err := reopened.automationRequest(created.RequestID)
	if err != nil || got.ContentDigest != created.ContentDigest || got.State != "received" || got.Revision != 1 {
		t.Fatalf("demande perdue au restart : %+v, %v", got, err)
	}
	var requests, occurrences int
	if err = reopened.db.QueryRow("SELECT count(*) FROM automation_requests WHERE request_id=?", created.RequestID).Scan(&requests); err != nil {
		t.Fatal(err)
	}
	if err = reopened.db.QueryRow("SELECT count(*) FROM automation_occurrences WHERE request_id=?", created.RequestID).Scan(&occurrences); err != nil {
		t.Fatal(err)
	}
	if requests != 1 || occurrences != 1 {
		t.Fatalf("autorité durable incomplète : demandes=%d occurrences=%d", requests, occurrences)
	}

	legacyRoot := t.TempDir()
	legacy, err := openStore(legacyRoot, true)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = legacy.db.Exec(`DROP TABLE automation_effects; DROP TABLE automation_occurrences; DROP TABLE automation_requests; PRAGMA user_version=24`); err != nil {
		t.Fatal(err)
	}
	if err = legacy.db.Close(); err != nil {
		t.Fatal(err)
	}
	migrated, err := openStore(legacyRoot, false)
	if err != nil {
		t.Fatal(err)
	}
	defer migrated.db.Close()
	var version int
	if err = migrated.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != schemaVersion {
		t.Fatalf("migration v24 incompatible : version=%d err=%v", version, err)
	}
	if err = migrated.db.QueryRow("SELECT count(*) FROM automation_requests").Scan(&requests); err != nil || requests != 0 {
		t.Fatalf("table C01 absente après migration : %d, %v", requests, err)
	}
}

func TestAutomationC01Idempotency(t *testing.T) {
	s, w := automationC01Fixture(t)
	r := automationC01Request("same-key", w)
	first, fresh, err := s.submitAutomationResume(r)
	if err != nil || !fresh {
		t.Fatal(fresh, err)
	}
	again, fresh, err := s.submitAutomationResume(r)
	if err != nil || fresh || again.RequestID != first.RequestID || again.Revision != first.Revision {
		t.Fatalf("rejeu identique non idempotent : %+v fresh=%v err=%v", again, fresh, err)
	}
	r.Source = "contenu différent"
	r.ContentDigest = automationResumeDigest(r)
	if _, _, err = s.submitAutomationResume(r); err == nil {
		t.Fatal("conflit de contenu accepté")
	} else {
		var command *CommandError
		if !errors.As(err, &command) || command.Code != "idempotency_conflict" {
			t.Fatalf("mauvais conflit : %v", err)
		}
	}
	var count int
	if err = s.db.QueryRow("SELECT count(*) FROM automation_requests WHERE idempotency_key=?", r.IdempotencyKey).Scan(&count); err != nil || count != 1 {
		t.Fatalf("conflit a dupliqué la demande : %d, %v", count, err)
	}
}

func TestAutomationC01ClaimConcurrency(t *testing.T) {
	s, w := automationC01Fixture(t)
	record, _, err := s.submitAutomationResume(automationC01Request("claim-race", w))
	if err != nil {
		t.Fatal(err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	start := make(chan struct{})
	results := make(chan bool, 2)
	errorsOut := make(chan error, 2)
	var group sync.WaitGroup
	for i, candidate := range []*Store{s, other} {
		group.Add(1)
		go func(index int, store *Store) {
			defer group.Done()
			<-start
			_, claimed, claimErr := store.claimAutomationRequest(record.RequestID, []string{"driver-one", "driver-two"}[index], time.Now(), defaultAutomationRequestConfig())
			results <- claimed
			errorsOut <- claimErr
		}(i, candidate)
	}
	close(start)
	group.Wait()
	close(results)
	close(errorsOut)
	claims := 0
	for claimed := range results {
		if claimed {
			claims++
		}
	}
	for claimErr := range errorsOut {
		if claimErr != nil {
			t.Fatal(claimErr)
		}
	}
	if claims != 1 {
		t.Fatalf("attribution non exclusive : %d claims", claims)
	}
	got, err := s.automationRequest(record.RequestID)
	if err != nil || got.State != "claimed" || got.ClaimedBy == "" || got.LeaseUntil == "" {
		t.Fatalf("bail non durable : %+v, %v", got, err)
	}
}

func TestAutomationC01CrashRecovery(t *testing.T) {
	s, w := automationC01Fixture(t)
	record, _, err := s.submitAutomationResume(automationC01Request("crash-recovery", w))
	if err != nil {
		t.Fatal(err)
	}
	claimed, owned, err := s.claimAutomationRequest(record.RequestID, "crashed-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || !owned || claimed.State != "claimed" {
		t.Fatal(claimed, owned, err)
	}
	effectID, err := s.applyAutomationResumeEffect(record.RequestID, "crashed-driver")
	if err != nil || effectID == "" {
		t.Fatal(effectID, err)
	}
	// Simulate a crash after the durable effect but before request completion.
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	completed, err := reopened.processAutomationRequest(record.RequestID, "recovery-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || completed.State != "executed" || completed.EffectID != effectID {
		t.Fatalf("effet certain non réconcilié : %+v, %v", completed, err)
	}
	var effects, signals int
	_ = reopened.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", record.RequestID).Scan(&effects)
	_ = reopened.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='automation-request'", w.ID).Scan(&signals)
	if effects != 1 || signals != 1 {
		t.Fatalf("effet répété après restart : effects=%d signals=%d", effects, signals)
	}

	uncertain, _, err := reopened.submitAutomationResume(automationC01Request("uncertain-recovery", w))
	if err != nil {
		t.Fatal(err)
	}
	if _, owned, claimErr := reopened.claimAutomationRequest(uncertain.RequestID, "uncertain-driver", time.Now(), defaultAutomationRequestConfig()); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	if err = reopened.markAutomationRequestUncertain(uncertain.RequestID, "uncertain-driver", "confirmation de l’effet indisponible après arrêt"); err != nil {
		t.Fatal(err)
	}
	unchanged, err := reopened.processAutomationRequest(uncertain.RequestID, "another-driver", time.Now().Add(time.Minute), defaultAutomationRequestConfig())
	if err != nil || unchanged.State != "uncertain_effect" || unchanged.NextAction != "operator_reconciliation" {
		t.Fatalf("effet incertain rejoué ou masqué : %+v, %v", unchanged, err)
	}
	_ = reopened.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", uncertain.RequestID).Scan(&effects)
	if effects != 0 {
		t.Fatalf("effet incertain rejoué : %d", effects)
	}
}

func TestAutomationC01Authorization(t *testing.T) {
	s, w := automationC01Fixture(t)
	record, _, err := s.submitAutomationResume(automationC01Request("revoked-auth", w))
	if err != nil {
		t.Fatal(err)
	}
	if err = s.stopMission(w.ID); err != nil {
		t.Fatal(err)
	}
	got, err := s.processAutomationRequest(record.RequestID, "auth-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || got.State != "rejected" || got.Reason != "authorization_required" {
		t.Fatalf("autorisation non revalidée : %+v, %v", got, err)
	}
	var agents, effects int
	_ = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents)
	_ = s.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", record.RequestID).Scan(&effects)
	if agents != 0 || effects != 0 {
		t.Fatalf("rejet a produit un effet : agents=%d effects=%d", agents, effects)
	}

	racingStore, racingWork := automationC01Fixture(t)
	racing, _, err := racingStore.submitAutomationResume(automationC01Request("revoked-after-claim", racingWork))
	if err != nil {
		t.Fatal(err)
	}
	if _, owned, claimErr := racingStore.claimAutomationRequest(racing.RequestID, "effect-driver", time.Now(), defaultAutomationRequestConfig()); claimErr != nil || !owned {
		t.Fatal(owned, claimErr)
	}
	if err = racingStore.stopMission(racingWork.ID); err != nil {
		t.Fatal(err)
	}
	if _, err = racingStore.applyAutomationResumeEffect(racing.RequestID, "effect-driver"); err == nil {
		t.Fatal("autorisation retirée après claim non revalidée avant effet")
	}
	racing, _ = racingStore.automationRequest(racing.RequestID)
	if racing.State != "rejected" || racing.Reason != "authorization_required" {
		t.Fatalf("révocation intercalée mal classée : %+v", racing)
	}
	_ = racingStore.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", racing.RequestID).Scan(&effects)
	if effects != 0 {
		t.Fatalf("révocation intercalée a produit %d effet(s)", effects)
	}
	budgetStore, budgetWork := automationC01Fixture(t)
	budgetRequest, _, err := budgetStore.submitAutomationResume(automationC01Request("budget-recheck", budgetWork))
	if err != nil {
		t.Fatal(err)
	}
	budgetWork, err = budgetStore.mutate(budgetWork.ID, "automation.test-budget", "exhaust-budget-c01", budgetWork.Revision, []byte(`{"budget":"exhausted"}`), func(candidate *Work) error {
		candidate.Planning.Activations = candidate.Planning.MaxActivations
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	budgetResult, err := budgetStore.processAutomationRequest(budgetRequest.RequestID, "budget-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || budgetResult.State != "rejected" || budgetResult.Reason != "budget_or_planning_blocked" {
		t.Fatalf("plafond non revalidé : %+v, %v", budgetResult, err)
	}
	_ = budgetStore.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", budgetRequest.RequestID).Scan(&effects)
	if effects != 0 {
		t.Fatalf("plafond épuisé a produit %d effet(s)", effects)
	}

	terminalStore, terminalWork := automationC01Fixture(t)
	terminalWork, _ = terminalStore.get(terminalWork.ID)
	raw := []byte(`{"close":"root"}`)
	terminalWork, err = terminalStore.mutate(terminalWork.ID, "automation.test-close", "close-for-c01", terminalWork.Revision, raw, func(candidate *Work) error {
		root, rootErr := candidate.Planning.scope("root")
		if rootErr == nil {
			root.State = "closed"
		}
		return rootErr
	})
	if err != nil {
		t.Fatal(err)
	}
	closed, fresh, err := terminalStore.submitAutomationResume(automationC01Request("closed-target", terminalWork))
	if err != nil || !fresh || closed.State != "rejected" || closed.Reason != "terminal_target" {
		t.Fatalf("cible clôturée non refusée durablement : %+v fresh=%v err=%v", closed, fresh, err)
	}
	_ = terminalStore.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", terminalWork.ID).Scan(&agents)
	if agents != 0 {
		t.Fatalf("cible clôturée a créé %d tentative(s)", agents)
	}
}

func TestAutomationC01WorkspaceWait(t *testing.T) {
	s, w := automationC01Fixture(t)
	holder, created, err := s.prepare(w.ID, Launch{Schema: 1, EventID: "workspace-holder", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: s.root, Capture: true})
	if err != nil || !created {
		t.Fatalf("fixture d’occupation absente : %+v created=%v err=%v", holder, created, err)
	}
	before, _ := s.get(w.ID)
	var reservationsBefore int
	_ = s.db.QueryRow("SELECT count(*) FROM reservations WHERE work_id=?", w.ID).Scan(&reservationsBefore)
	record, _, err := s.submitAutomationResume(automationC01Request("workspace-wait", before))
	if err != nil {
		t.Fatal(err)
	}
	got, err := s.processAutomationRequest(record.RequestID, "waiting-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || got.State != "waiting" || got.Reason != "workspace_wait" || got.NextAction != "retry_after_resource_release" {
		t.Fatalf("attente d’espace incorrecte : %+v, %v", got, err)
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) {
		t.Fatalf("attente a consommé une tentative : avant=%d après=%d", len(before.Tasks[0].Attempts), len(after.Tasks[0].Attempts))
	}
	var effects int
	_ = s.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", record.RequestID).Scan(&effects)
	var reservationsAfter int
	_ = s.db.QueryRow("SELECT count(*) FROM reservations WHERE work_id=?", w.ID).Scan(&reservationsAfter)
	if effects != 0 || reservationsAfter != reservationsBefore {
		t.Fatalf("attente a produit un effet ou altéré la consommation : effects=%d réservations=%d→%d", effects, reservationsBefore, reservationsAfter)
	}
}
