//go:build linux

package main

import (
	"errors"
	"reflect"
	"sync"
	"testing"
	"time"
)

func automationC02Time(t *testing.T, value string) time.Time {
	t.Helper()
	parsed, err := time.Parse(time.RFC3339, value)
	if err != nil {
		t.Fatal(err)
	}
	return parsed
}

func automationC02Create(w Work, event, kind, start, until, zone string, count int) AutomationScheduleCreate {
	return AutomationScheduleCreate{
		Schema: 1, EventID: event, Name: "Programme de test", TargetWorkID: w.ID,
		Action: "request_resume", Timezone: zone,
		Schedule:     AutomationScheduleSpec{Kind: kind, LocalTime: start, UntilLocal: until, MaxOccurrences: count},
		MissedPolicy: "skip", ConcurrencyPolicy: "coalesce",
	}
}

func TestAutomationC02SchedulePreview(t *testing.T) {
	s, w := automationC01Fixture(t)
	cfg, err := s.automationConfig()
	if err != nil {
		t.Fatal(err)
	}
	request := automationC02Create(w, "preview-daily", "daily", "2026-01-01T09:00:00", "2026-01-04T09:00:00", "Europe/Paris", 4)
	preview, err := previewAutomationSchedule(request, automationC02Time(t, "2025-12-31T00:00:00Z"), 3, cfg)
	if err != nil {
		t.Fatal(err)
	}
	want := []string{"2026-01-01T08:00:00Z", "2026-01-02T08:00:00Z", "2026-01-03T08:00:00Z"}
	if len(preview.OccurrencesUTC) != len(want) {
		t.Fatalf("preview incomplet : %+v", preview)
	}
	for i := range want {
		if preview.OccurrencesUTC[i] != want[i] {
			t.Fatalf("occurrence %d=%s, attendu %s", i, preview.OccurrencesUTC[i], want[i])
		}
	}
	if preview.DSTPolicy != "reject_ambiguous_or_nonexistent" || preview.ClockBackwardPolicy != "hold_until_last_observed" || preview.CreatesEnabled {
		t.Fatalf("politiques non exposées : %+v", preview)
	}
	once := automationC02Create(w, "preview-once", "once", "2026-02-01T12:00:00", "", "UTC", 1)
	weekly := automationC02Create(w, "preview-weekly", "weekly", "2026-02-02T12:00:00", "2026-02-23T12:00:00", "UTC", 4)
	for _, candidate := range []AutomationScheduleCreate{once, weekly} {
		got, previewErr := previewAutomationSchedule(candidate, automationC02Time(t, "2026-01-01T00:00:00Z"), 1, cfg)
		if previewErr != nil || len(got.OccurrencesUTC) != 1 {
			t.Fatalf("preview %s invalide : %+v, %v", candidate.Schedule.Kind, got, previewErr)
		}
	}
}

func TestAutomationC02DST(t *testing.T) {
	s, w := automationC01Fixture(t)
	cfg, err := s.automationConfig()
	if err != nil {
		t.Fatal(err)
	}
	tests := []struct {
		name  string
		local string
		code  string
	}{
		{"nonexistent", "2026-03-29T02:30:00", "nonexistent_local_time"},
		{"ambiguous", "2026-10-25T02:30:00", "ambiguous_local_time"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			request := automationC02Create(w, "dst-"+tc.name, "once", tc.local, "", "Europe/Paris", 1)
			_, previewErr := previewAutomationSchedule(request, automationC02Time(t, "2026-01-01T00:00:00Z"), 1, cfg)
			if commandFailure(previewErr).Code != tc.code {
				t.Fatalf("heure DST acceptée ou mal classée : %v", previewErr)
			}
		})
	}
	invalidZone := automationC02Create(w, "dst-local-zone", "once", "2026-02-01T12:00:00", "", "Local", 1)
	if _, err = previewAutomationSchedule(invalidZone, automationC02Time(t, "2026-01-01T00:00:00Z"), 1, cfg); commandFailure(err).Code != "invalid_timezone" {
		t.Fatalf("fuseau implicite accepté : %v", err)
	}
}

func TestAutomationC02RestartSkip(t *testing.T) {
	s, w := automationC01Fixture(t)
	createdAt := automationC02Time(t, "2026-01-01T07:00:00Z")
	request := automationC02Create(w, "restart-skip", "daily", "2026-01-01T09:00:00", "2026-01-03T09:00:00", "UTC", 3)
	record, err := s.createAutomationSchedule(request, func() time.Time { return createdAt })
	if err != nil || record.State != "disabled" {
		t.Fatalf("création non désactivée : %+v, %v", record, err)
	}
	record, err = s.setAutomationScheduleState(record.ScheduleID, "enabled", record.Revision, func() time.Time { return createdAt })
	if err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	restartAt := automationC02Time(t, "2026-01-02T10:00:00Z")
	if err = reopened.tickAutomationSchedules(func() time.Time { return restartAt }); err != nil {
		t.Fatal(err)
	}
	got, err := reopened.automationSchedule(record.ScheduleID)
	if err != nil || got.NextAt != "2026-01-03T09:00:00Z" || got.EmittedCount != 2 {
		t.Fatalf("rattrapage non sauté : %+v, %v", got, err)
	}
	var skipped, requests int
	_ = reopened.db.QueryRow("SELECT count(*) FROM automation_schedule_journal WHERE schedule_id=? AND state='skipped'", record.ScheduleID).Scan(&skipped)
	_ = reopened.db.QueryRow("SELECT count(*) FROM automation_requests WHERE target_work_id=?", w.ID).Scan(&requests)
	if skipped != 2 || requests != 0 {
		t.Fatalf("restart a produit une rafale : skipped=%d requests=%d", skipped, requests)
	}
	if err = reopened.tickAutomationSchedules(func() time.Time { return automationC02Time(t, "2026-01-02T09:00:00Z") }); err != nil {
		t.Fatal(err)
	}
	var held int
	_ = reopened.db.QueryRow("SELECT count(*) FROM automation_schedule_journal WHERE schedule_id=? AND state='held' AND reason='clock_moved_backward'", record.ScheduleID).Scan(&held)
	if held != 1 {
		t.Fatalf("recul d’horloge non retenu et journalisé : held=%d", held)
	}
}

func TestAutomationC02PauseArchive(t *testing.T) {
	s, w := automationC01Fixture(t)
	base := automationC02Time(t, "2026-01-01T08:00:00Z")
	record, err := s.createAutomationSchedule(automationC02Create(w, "pause-archive", "daily", "2026-01-01T09:00:00", "2026-01-02T09:00:00", "UTC", 2), func() time.Time { return base })
	if err != nil {
		t.Fatal(err)
	}
	record, err = s.setAutomationScheduleState(record.ScheduleID, "enabled", record.Revision, func() time.Time { return base })
	if err != nil {
		t.Fatal(err)
	}
	record, err = s.setAutomationScheduleState(record.ScheduleID, "paused", record.Revision, func() time.Time { return base })
	if err != nil || record.State != "paused" {
		t.Fatal(record, err)
	}
	if err = s.tickAutomationSchedules(func() time.Time { return automationC02Time(t, "2026-01-01T09:00:10Z") }); err != nil {
		t.Fatal(err)
	}
	var requests int
	_ = s.db.QueryRow("SELECT count(*) FROM automation_requests WHERE target_work_id=?", w.ID).Scan(&requests)
	if requests != 0 {
		t.Fatalf("pause a laissé partir %d demande(s)", requests)
	}
	record, err = s.setAutomationScheduleState(record.ScheduleID, "archived", record.Revision, func() time.Time { return base })
	if err != nil || record.State != "archived" || record.NextAt != "" {
		t.Fatal(record, err)
	}
	if _, err = s.setAutomationScheduleState(record.ScheduleID, "enabled", record.Revision, func() time.Time { return base }); commandFailure(err).Code != "schedule_archived" {
		t.Fatalf("archive réactivable : %v", err)
	}
}

func TestAutomationC02Coalescing(t *testing.T) {
	s, w := automationC01Fixture(t)
	base := automationC02Time(t, "2026-01-01T08:00:00Z")
	for _, event := range []string{"coalesce-one", "coalesce-two"} {
		record, err := s.createAutomationSchedule(automationC02Create(w, event, "once", "2026-01-01T09:00:00", "", "UTC", 1), func() time.Time { return base })
		if err != nil {
			t.Fatal(err)
		}
		if _, err = s.setAutomationScheduleState(record.ScheduleID, "enabled", record.Revision, func() time.Time { return base }); err != nil {
			t.Fatal(err)
		}
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	start := make(chan struct{})
	errorsOut := make(chan error, 2)
	var group sync.WaitGroup
	for _, store := range []*Store{s, other} {
		group.Add(1)
		go func(candidate *Store) {
			defer group.Done()
			<-start
			errorsOut <- candidate.tickAutomationSchedules(func() time.Time { return automationC02Time(t, "2026-01-01T09:00:10Z") })
		}(store)
	}
	close(start)
	group.Wait()
	close(errorsOut)
	for tickErr := range errorsOut {
		if tickErr != nil {
			t.Fatal(tickErr)
		}
	}
	var requests, origins, submitted, coalesced int
	_ = s.db.QueryRow("SELECT count(*) FROM automation_requests WHERE target_work_id=?", w.ID).Scan(&requests)
	_ = s.db.QueryRow("SELECT count(*) FROM automation_request_origins").Scan(&origins)
	_ = s.db.QueryRow("SELECT count(*) FROM automation_schedule_journal WHERE state='submitted'").Scan(&submitted)
	_ = s.db.QueryRow("SELECT count(*) FROM automation_schedule_journal WHERE state='coalesced'").Scan(&coalesced)
	if requests != 1 || origins != 2 || submitted != 1 || coalesced != 1 {
		t.Fatalf("coalescence incorrecte : requests=%d origins=%d submitted=%d coalesced=%d", requests, origins, submitted, coalesced)
	}
}

func TestAutomationC02CausalRecovery(t *testing.T) {
	s, w := automationC01Fixture(t)
	request, _, err := s.submitAutomationResume(automationC01Request("causal-auth", w))
	if err != nil {
		t.Fatal(err)
	}
	before, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	budgetBefore, err := s.budget(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.stopMission(w.ID); err != nil {
		t.Fatal(err)
	}
	rejected, err := s.processAutomationRequest(request.RequestID, "causal-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || rejected.State != "rejected" || rejected.Reason != "authorization_required" {
		t.Fatalf("précondition de reprise absente : %+v, %v", rejected, err)
	}
	if err = s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	if err = s.pause(w.ID, false); err != nil {
		t.Fatal(err)
	}
	recovery := AutomationCausalRecovery{Schema: 1, RequestID: request.RequestID, PreviousReason: rejected.Reason, EvidenceKind: "authorization_revision", EvidenceDigest: automationEvidenceDigest("mission-policy-enabled-revision-2")}
	recovered, err := s.recoverAutomationRequestCausally(recovery, func() time.Time { return time.Now() })
	if err != nil || recovered.State != "received" || recovered.NextAction != "claim" {
		t.Fatalf("reprise causale refusée : %+v, %v", recovered, err)
	}
	if _, err = s.recoverAutomationRequestCausally(recovery, func() time.Time { return time.Now() }); commandFailure(err).Code != "causal_change_required" {
		t.Fatalf("même preuve réutilisable : %v", err)
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if before.Planning.Activations != after.Planning.Activations || before.Planning.Decisions != after.Planning.Decisions || len(before.Tasks) != len(after.Tasks) {
		t.Fatalf("reprise a relevé des budgets ou recréé le plan : before=%+v after=%+v", before.Planning, after.Planning)
	}
	for i := range before.Tasks {
		if !reflect.DeepEqual(before.Tasks[i].Attempts, after.Tasks[i].Attempts) || before.Tasks[i].PlanMaxAttempts != after.Tasks[i].PlanMaxAttempts {
			t.Fatal("reprise a modifié les tentatives ou leur plafond")
		}
	}
	if before.Planning.MaxActivations != after.Planning.MaxActivations || before.Planning.MaxDecisions != after.Planning.MaxDecisions {
		t.Fatal("reprise a modifié les plafonds de planification")
	}
	budgetAfter, err := s.budget(w.ID)
	if err != nil || !reflect.DeepEqual(budgetBefore, budgetAfter) {
		t.Fatalf("reprise a modifié budgets ou coûts : before=%+v after=%+v err=%v", budgetBefore, budgetAfter, err)
	}
	if budgetBefore.ActualCost != nil || budgetAfter.ActualCost != nil {
		t.Fatal("coût inconnu inventé pendant la reprise")
	}

	blocked, _, err := s.submitAutomationResume(automationC01Request("causal-cooldown", w))
	if err != nil {
		t.Fatal(err)
	}
	if err = s.stopMission(w.ID); err != nil {
		t.Fatal(err)
	}
	blocked, err = s.processAutomationRequest(blocked.RequestID, "cooldown-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil {
		t.Fatal(err)
	}
	if err = s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	if err = s.pause(w.ID, false); err != nil {
		t.Fatal(err)
	}
	provider := w.Planning.Provider
	cooldown := &ProviderCooldown{Provider: provider, ObservedAt: now(), ResetAt: time.Now().Add(time.Hour).Unix(), Source: "fixture", Signal: "result.api_error_status.429"}
	if err = s.recordProviderCooldown(provider, "c02-test", cooldown); err != nil {
		t.Fatal(err)
	}
	_, err = s.recoverAutomationRequestCausally(AutomationCausalRecovery{Schema: 1, RequestID: blocked.RequestID, PreviousReason: blocked.Reason, EvidenceKind: "provider_check", EvidenceDigest: automationEvidenceDigest("provider-still-unavailable")}, func() time.Time { return time.Now() })
	var command *CommandError
	if !errors.As(err, &command) || command.Code != "provider_cooldown" {
		t.Fatalf("cooldown effacé par reprise : %v", err)
	}
	unchanged, readErr := s.automationRequest(blocked.RequestID)
	if readErr != nil || unchanged.State != "rejected" {
		t.Fatalf("blocage fournisseur a muté la demande : %+v, %v", unchanged, readErr)
	}
}
