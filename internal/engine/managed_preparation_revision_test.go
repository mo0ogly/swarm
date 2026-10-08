//go:build linux

package engine

import "testing"

func TestManagedPreparedStaleLaunchIsRevisionConflict(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-stale-conflict", Revision: w.Revision,
		TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete", Timeout: 60}
	if _, e := s.ensureManagedAttempt(w, r); e != nil {
		t.Fatal(e)
	}
	a, made, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
	if e != nil || !made {
		t.Fatalf("first reservation: %v %v", made, e)
	}
	before, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	// A different stale click must be rejected as stale, rather than inspecting
	// the now-active agent as a historical recovery. It must reserve nothing.
	stale := r
	stale.EventID = "other-stale-click"
	got, created, e := s.prepare(w.ID, stale)
	conflict, ok := e.(*CommandError)
	if !ok || conflict.Code != "revision_conflict" || !conflict.Retryable || created || got.ID != "" {
		t.Fatalf("stale concurrent view: %+v %v %v", got, created, e)
	}
	after, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	agents, e := s.agents(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if after.Revision != before.Revision || len(agents) != 1 || agents[0].ID != a.ID || len(after.Tasks[0].Attempts) != 1 {
		t.Fatalf("stale click changed reservation: %d -> %d, agents %d", before.Revision, after.Revision, len(agents))
	}
	replay, created, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
	if e != nil || created || replay.ID != a.ID {
		t.Fatalf("original replay: %+v %v %v", replay, created, e)
	}
}
