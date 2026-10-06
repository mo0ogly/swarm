//go:build linux

package main

import (
	"errors"
	"reflect"
	"sync"
	"testing"
	"time"
)

// D01 deliberately composes the public service boundaries used by the CLI and
// HTTP handlers.  The stores are all backed by t.TempDir through storeTest; no
// running mission or provider is touched.
func TestGraphDeliveryD01PublicLifecycle(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)

	draft, preview := createGraphDraftTest(t, s, w,
		GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	applied := applyGraphDraftTest(t, s, w, draft, preview, "d01-public-apply")
	current, err := s.get(w.ID)
	if err != nil || applied.Revision != current.Revision || !taskDependsOn(t, current, "t2", "t1") {
		t.Fatalf("public graph apply was not observable: result=%+v work=%+v err=%v", applied, current, err)
	}

	cycle, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{
		Schema: 1, WorkID: current.ID, ExpectedRevision: current.Revision,
		Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t2", Dependent: "t1"}},
	})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: current.ID, DraftID: cycle.ID, ExpectedRevision: current.Revision}); graphCode(err) != "dependency_cycle" {
		t.Fatalf("cycle was not rejected at preview: %v", err)
	}
	afterCycle, _ := s.get(w.ID)
	if afterCycle.Revision != current.Revision || !taskDependsOn(t, afterCycle, "t2", "t1") {
		t.Fatal("cycle rejection changed the accepted graph")
	}

	request := automationC02Create(current, "d01-program", "once", "2099-01-01T09:00:00", "", "UTC", 1)
	service := s.automationService()
	programPreview, err := service.Preview(request, 1, func() time.Time { return automationC02Time(t, "2098-01-01T00:00:00Z") })
	if err != nil || programPreview.PreviewToken == "" || programPreview.CreatesEnabled {
		t.Fatalf("program preview did not remain inert: %+v err=%v", programPreview, err)
	}
	program, err := service.Create(AutomationCreateCommand{Schedule: request, PreviewToken: programPreview.PreviewToken}, func() time.Time { return automationC02Time(t, "2098-01-01T00:00:00Z") })
	if err != nil || program.Schedule.State != "disabled" || len(program.Occurrences) != 0 {
		t.Fatalf("program was not created disabled: %+v err=%v", program, err)
	}
	listed, err := service.List()
	if err != nil || len(listed) != 1 || listed[0].Schedule.ScheduleID != program.Schedule.ScheduleID {
		t.Fatalf("program journal/list did not expose the created program: %+v err=%v", listed, err)
	}
	var agents int
	if err = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents); err != nil || agents != 0 {
		t.Fatalf("draft/program preparation started an agent: count=%d err=%v", agents, err)
	}
}

func TestGraphDeliveryD01RequestReplay(t *testing.T) {
	s, w := automationC01Fixture(t)
	request := automationC01Request("d01-request-replay", w)
	first, fresh, err := s.submitAutomationResume(request)
	if err != nil || !fresh {
		t.Fatalf("initial request missing: fresh=%v err=%v", fresh, err)
	}
	replayed, fresh, err := s.submitAutomationResume(request)
	if err != nil || fresh || replayed.RequestID != first.RequestID || replayed.OccurrenceID != first.OccurrenceID || replayed.Revision != first.Revision {
		t.Fatalf("same request was not replayed: first=%+v replay=%+v fresh=%v err=%v", first, replayed, fresh, err)
	}
	request.Source = "different-content"
	request.ContentDigest = automationResumeDigest(request)
	_, _, err = s.submitAutomationResume(request)
	var command *CommandError
	if !errors.As(err, &command) || command.Code != "idempotency_conflict" {
		t.Fatalf("same key with different content was not rejected: %v", err)
	}
	var count int
	if err = s.db.QueryRow("SELECT count(*) FROM automation_requests WHERE idempotency_key=?", request.IdempotencyKey).Scan(&count); err != nil || count != 1 {
		t.Fatalf("request replay produced duplicate durable rows: count=%d err=%v", count, err)
	}
}

func TestGraphDeliveryD01Conflict(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)
	draftA, previewA := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	draftB, previewB := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t3"})
	requests := []GraphDraftApplyRequest{
		{Schema: 1, WorkID: w.ID, DraftID: draftA.ID, EventID: "d01-conflict-a", ExpectedRevision: w.Revision, PreviewToken: previewA.PreviewToken, ContentDigest: previewA.ContentDigest},
		{Schema: 1, WorkID: w.ID, DraftID: draftB.ID, EventID: "d01-conflict-b", ExpectedRevision: w.Revision, PreviewToken: previewB.PreviewToken, ContentDigest: previewB.ContentDigest},
	}
	errs := make([]error, len(requests))
	start := make(chan struct{})
	var group sync.WaitGroup
	for index := range requests {
		group.Add(1)
		go func(index int) {
			defer group.Done()
			<-start
			_, errs[index] = s.applyGraphDraft(operatorIdentity(), requests[index])
		}(index)
	}
	close(start)
	group.Wait()
	passes, conflicts := 0, 0
	for _, applyErr := range errs {
		if applyErr == nil {
			passes++
		} else if graphCode(applyErr) == "revision_conflict" {
			conflicts++
		} else {
			t.Fatalf("unexpected concurrent result: %v", applyErr)
		}
	}
	current, err := s.get(w.ID)
	if err != nil || passes != 1 || conflicts != 1 || current.Revision != w.Revision+1 {
		t.Fatalf("conflict did not preserve one winner: passes=%d conflicts=%d revision=%d err=%v", passes, conflicts, current.Revision, err)
	}
}

func TestGraphDeliveryD01WorkspaceWait(t *testing.T) {
	s, w := automationC01Fixture(t)
	holder, created, err := s.prepare(w.ID, Launch{Schema: 1, EventID: "d01-workspace-holder", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: s.root, Capture: true})
	if err != nil || !created || holder.ID == "" {
		t.Fatalf("workspace holder missing: %+v created=%v err=%v", holder, created, err)
	}
	before, _ := s.get(w.ID)
	request, _, err := s.submitAutomationResume(automationC01Request("d01-workspace-wait", before))
	if err != nil {
		t.Fatal(err)
	}
	waiting, err := s.processAutomationRequest(request.RequestID, "d01-wait-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || waiting.State != "waiting" || waiting.Reason != "workspace_wait" || waiting.NextAction != "retry_after_resource_release" {
		t.Fatalf("occupied workspace did not yield durable wait: %+v err=%v", waiting, err)
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) {
		t.Fatalf("workspace wait consumed an attempt: before=%d after=%d", len(before.Tasks[0].Attempts), len(after.Tasks[0].Attempts))
	}
	var effects int
	if err = s.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", request.RequestID).Scan(&effects); err != nil || effects != 0 {
		t.Fatalf("workspace wait produced an effect: count=%d err=%v", effects, err)
	}
}

func TestGraphDeliveryD01RestartRetention(t *testing.T) {
	s, w := automationC01Fixture(t)
	request, _, err := s.submitAutomationResume(automationC01Request("d01-restart-request", w))
	if err != nil {
		t.Fatal(err)
	}
	base := automationC02Time(t, "2026-01-01T07:00:00Z")
	schedule, err := s.createAutomationSchedule(automationC02Create(w, "d01-restart-program", "daily", "2026-01-01T09:00:00", "2026-01-03T09:00:00", "UTC", 3), func() time.Time { return base })
	if err != nil {
		t.Fatal(err)
	}
	schedule, err = s.setAutomationScheduleState(schedule.ScheduleID, "enabled", schedule.Revision, func() time.Time { return base })
	if err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	retained, err := reopened.automationRequest(request.RequestID)
	if err != nil || retained.State != "received" || retained.OccurrenceID != request.OccurrenceID {
		t.Fatalf("request identity lost on reopen: %+v err=%v", retained, err)
	}
	if err = reopened.tickAutomationSchedules(func() time.Time { return automationC02Time(t, "2026-01-02T10:00:00Z") }); err != nil {
		t.Fatal(err)
	}
	program, err := reopened.automationSchedule(schedule.ScheduleID)
	if err != nil || program.NextAt != "2026-01-03T09:00:00Z" || program.EmittedCount != 2 {
		t.Fatalf("restart skip policy was not retained: %+v err=%v", program, err)
	}
	var submitted int
	if err = reopened.db.QueryRow("SELECT count(*) FROM automation_schedule_journal WHERE schedule_id=? AND state='submitted'", schedule.ScheduleID).Scan(&submitted); err != nil || submitted != 0 {
		t.Fatalf("restart emitted a hidden catch-up request: count=%d err=%v", submitted, err)
	}
}

func TestGraphDeliveryD01ProofFreshness(t *testing.T) {
	before := Work{Planning: &PlanningState{Repository: &ManagedRepository{Candidate: "candidate-before"}}, Tasks: []Task{{ID: "source", Status: "accepted"}, {ID: "checked", Status: "accepted", Depends: []string{"source"}}}}
	after := before
	after.Tasks = append([]Task(nil), before.Tasks...)
	after.Planning = &PlanningState{Repository: &ManagedRepository{Candidate: "candidate-after"}}
	projection := graphDraftProofProjection(&before, &after, []string{"checked"})
	if !projection.Relevant || projection.Kind != "relevant" || projection.BeforeDigest == projection.AfterDigest {
		t.Fatalf("changed candidate did not stale the proof projection: %+v", projection)
	}

	s, w := automationC01Fixture(t)
	request, _, err := s.submitAutomationResume(automationC01Request("d01-causal-recovery", w))
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
	rejected, err := s.processAutomationRequest(request.RequestID, "d01-causal-driver", time.Now(), defaultAutomationRequestConfig())
	if err != nil || rejected.State != "rejected" || rejected.Reason != "authorization_required" {
		t.Fatalf("causal prerequisite was not observable: %+v err=%v", rejected, err)
	}
	if err = s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	if err = s.pause(w.ID, false); err != nil {
		t.Fatal(err)
	}
	evidence := AutomationCausalRecovery{Schema: 1, RequestID: request.RequestID, PreviousReason: rejected.Reason, EvidenceKind: "authorization_revision", EvidenceDigest: automationEvidenceDigest("d01-mission-policy-reenabled")}
	recovered, err := s.recoverAutomationRequestCausally(evidence, func() time.Time { return time.Now() })
	if err != nil || recovered.State != "received" || recovered.NextAction != "claim" {
		t.Fatalf("changed causal evidence did not recover the request: %+v err=%v", recovered, err)
	}
	if _, err = s.recoverAutomationRequestCausally(evidence, func() time.Time { return time.Now() }); commandFailure(err).Code != "causal_change_required" {
		t.Fatalf("same causal evidence was reusable: %v", err)
	}
	budgetAfter, err := s.budget(w.ID)
	if err != nil || !reflect.DeepEqual(budgetBefore, budgetAfter) || budgetAfter.ActualCost != nil {
		t.Fatalf("recovery changed limits or invented provider cost: before=%+v after=%+v err=%v", budgetBefore, budgetAfter, err)
	}
}
