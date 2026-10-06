//go:build linux

package main

import (
	"testing"
	"time"
)

func TestAutomationC02Lifecycle(t *testing.T) {
	for _, action := range []string{"archive", "delete"} {
		t.Run(action, func(t *testing.T) {
			s, w := automationC01Fixture(t)
			clock := func() time.Time { return automationC02Time(t, "2026-01-01T00:00:00Z") }
			request := automationC02Create(w, "lifecycle-schedule", "once", "2026-02-01T12:00:00", "", "UTC", 1)
			schedule, err := s.createAutomationSchedule(request, clock)
			if err != nil {
				t.Fatal(err)
			}
			schedule, err = s.setAutomationScheduleState(schedule.ScheduleID, "enabled", schedule.Revision, clock)
			if err != nil {
				t.Fatal(err)
			}
			if err = s.journalAutomationSchedule(schedule.ScheduleID, schedule.NextAt, clock(), "held", "", "lifecycle-test", "clock_moved_backward"); err != nil {
				t.Fatal(err)
			}
			if err = s.stopMission(w.ID); err != nil {
				t.Fatal(err)
			}
			w, err = s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			preview, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: action})
			if err != nil {
				t.Fatal(err)
			}
			receipt, err := s.lifecycleApply(w.ID, lifecycleRequest(preview, "c02-lifecycle-"+action))
			if err != nil {
				t.Fatalf("%s avec programme: %v", action, err)
			}
			var restored *Store
			if action == "archive" {
				restored = storeTest(t)
				if _, err = restored.importBundle(receipt.ArchivePath); err != nil {
					t.Fatal(err)
				}
			} else {
				preview, err = s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "restore"})
				if err != nil {
					t.Fatal(err)
				}
				if _, err = s.lifecycleApply(w.ID, lifecycleRequest(preview, "c02-lifecycle-restore")); err != nil {
					t.Fatal(err)
				}
				restored = s
			}
			got, err := restored.automationSchedule(schedule.ScheduleID)
			wantedState := "enabled"
			if action == "archive" {
				wantedState = "disabled"
			}
			if err != nil || got.TargetWorkID != w.ID || got.State != wantedState {
				t.Fatalf("programme non restauré: %+v %v", got, err)
			}
			var count int
			if err = restored.db.QueryRow("SELECT count(*) FROM automation_schedule_journal WHERE schedule_id=?", schedule.ScheduleID).Scan(&count); err != nil || count != 1 {
				t.Fatalf("journal non restauré: %d %v", count, err)
			}
		})
	}
}

func TestAutomationC02ArchiveGuards(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	cases := []struct {
		name   string
		tables []lifecycleTable
	}{
		{"foreign-target", []lifecycleTable{{Name: "automation_schedules", Columns: []string{"schedule_id", "target_work_id"}, Rows: [][]lifecycleCell{{{Kind: "text", Value: "schedule"}, {Kind: "text", Value: "other-work"}}}}}},
		{"foreign-request", []lifecycleTable{{Name: "automation_occurrences", Columns: []string{"request_id"}, Rows: [][]lifecycleCell{{{Kind: "text", Value: "other-request"}}}}}},
		{"foreign-schedule", []lifecycleTable{{Name: "automation_schedule_journal", Columns: []string{"schedule_id"}, Rows: [][]lifecycleCell{{{Kind: "text", Value: "other-schedule"}}}}}},
		{"unexpected-table", []lifecycleTable{{Name: "works"}}},
		{"duplicate-table", []lifecycleTable{{Name: "automation_schedules"}, {Name: "automation_schedules"}}},
		{"duplicate-column", []lifecycleTable{{Name: "automation_schedules", Columns: []string{"state", "state"}}}},
	}
	for _, candidate := range cases {
		t.Run(candidate.name, func(t *testing.T) {
			tx, err := s.db.Begin()
			if err != nil {
				t.Fatal(err)
			}
			defer tx.Rollback()
			if err = importAutomationTables(tx, w.ID, candidate.tables); err == nil {
				t.Fatal("archive étrangère ou malformée acceptée")
			}
		})
	}
	if _, err := s.get(w.ID); err != nil {
		t.Fatal("mission existante altérée", err)
	}
}
