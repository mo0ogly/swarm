//go:build linux

package main

import (
	"testing"
	"time"
)

// R08: authorizing a new plan must leave an unrelated live attempt alone.
func TestPreparationReleasePreservesUnrelatedLiveAttempt(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	if err := s.setAutonomy(w.ID, autonomyAssisted, 2); err != nil {
		t.Fatal(err)
	}
	launch.Instruction = "TEST_SLEEP"
	a, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { done <- s.supervise(a.ID) }()
	t.Cleanup(func() {
		if err := s.stopAgent(a.ID); err != nil {
			t.Error(err)
		}
		select {
		case err := <-done:
			if err != nil {
				t.Error(err)
			}
		case <-time.After(10 * time.Second):
			t.Error("unrelated provider did not stop during cleanup")
		}
	})
	live := waitAgent(t, s, a.ID, func(a Agent) bool { return a.Status == "running" && a.Child > 0 })
	stamp := processStamp(live.Child)
	if stamp == "" {
		t.Fatal("fixture provider is not alive")
	}
	before, _ := s.get(w.ID)
	var initialReservations int
	if err := s.db.QueryRow("SELECT count(*) FROM reservations").Scan(&initialReservations); err != nil {
		t.Fatal(err)
	}
	p := readyPreparation(t, s, w.ID)
	p, err = s.preparationCommand(conversionRequest(t, s, p, "create-missions"))
	if err != nil {
		t.Fatal(err)
	}
	p, err = s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
	if err != nil {
		t.Fatal(err)
	}
	if p.Conversion.ReleasedAt == "" || s.autonomy(w.ID) != autonomyAssisted {
		t.Fatal("release changed autonomy or failed")
	}
	agents, err := s.agents(w.ID)
	if err != nil || len(agents) != 1 || agents[0].ID != live.ID || agents[0].Child != live.Child || agents[0].Status != "running" || processStamp(live.Child) != stamp {
		t.Fatalf("release changed the unrelated live process: %+v, %v", agents, err)
	}
	after, _ := s.get(w.ID)
	oldTask, _ := before.task(launch.TaskID)
	currentTask, _ := after.task(launch.TaskID)
	if oldTask.Status != currentTask.Status || len(oldTask.Attempts) != len(currentTask.Attempts) {
		t.Fatal("release changed unrelated task or attempts")
	}
	var reservations int
	if err := s.db.QueryRow("SELECT count(*) FROM reservations").Scan(&reservations); err != nil || reservations != initialReservations {
		t.Fatalf("release duplicated reservations: %d, %v", reservations, err)
	}
}
