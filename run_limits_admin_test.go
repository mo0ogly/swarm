//go:build linux

package main

import "testing"

func TestRunLimitsConfigRejectsInvalidValue(t *testing.T) {
	s := storeTest(t)
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 999999}, Reason: "test"}); err == nil {
		t.Fatal("expected rejection of out-of-bounds max_tool_calls")
	}
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e2", Scope: ScopeProject, Values: RunLimits{SilenceSeconds: -1}, Reason: "test"}); err == nil {
		t.Fatal("expected rejection of negative silence_seconds")
	}
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e3", Scope: "invalid-scope", Values: RunLimits{}, Reason: "test"}); err == nil {
		t.Fatal("expected rejection of unknown scope")
	}
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e4", Scope: ScopeMission, Mission: "", Values: RunLimits{}, Reason: "test"}); err == nil {
		t.Fatal("expected rejection of mission scope without a mission id")
	}
	// A valid all-zero override (pure inheritance) must be accepted.
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e5", Scope: ScopeProject, Values: RunLimits{}, Reason: "no-op"}); err != nil {
		t.Fatalf("all-zero override should be accepted: %v", err)
	}
}

func TestRunLimitsConfigPersistsAfterRealRestart(t *testing.T) {
	s := storeTest(t)
	entry, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "set1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 42}, Reason: "abaisser le plafond global"})
	if err != nil {
		t.Fatal(err)
	}
	if entry.Revision != 1 {
		t.Fatalf("revision=%d", entry.Revision)
	}
	root := s.root
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	got, err := reopened.currentRunLimitsConfig(ScopeProject, "", "")
	if err != nil {
		t.Fatal(err)
	}
	if got.Values.MaxToolCalls != 42 || got.Revision != 1 {
		t.Fatalf("not persisted across a real reopen of the store: %+v", got)
	}
	eff, err := reopened.effectiveRunLimits("", "", "")
	if err != nil {
		t.Fatal(err)
	}
	if eff.MaxToolCalls != 42 {
		t.Fatalf("effective resolution after restart ignores persisted override: %+v", eff)
	}
}

func TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)

	base, err := s.effectiveRunLimits(w.ID, "worker", "t1")
	if err != nil {
		t.Fatal(err)
	}
	if base.MaxToolCalls != 100 {
		t.Fatalf("built-in default changed underneath the test: %d", base.MaxToolCalls)
	}
	// An attempt frozen right now, before any admin change (Agent.Limits is a
	// value copy in the running engine, never a pointer into live config).
	frozen := base

	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "proj1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 60}, Reason: "réduire le défaut global"}); err != nil {
		t.Fatal(err)
	}
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "mission1", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{MaxToolCalls: 50}, Reason: "mission plus stricte"}); err != nil {
		t.Fatal(err)
	}
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "role1", Scope: ScopeRole, Mission: w.ID, Key: "worker", Values: RunLimits{MaxToolCalls: 40}, Reason: "rôle worker plus strict"}); err != nil {
		t.Fatal(err)
	}
	taskEntry, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "task1", Scope: ScopeTask, Mission: w.ID, Key: "t1", Values: RunLimits{MaxToolCalls: 30}, Reason: "tâche T1 plus stricte"})
	if err != nil {
		t.Fatal(err)
	}

	future, err := s.effectiveRunLimits(w.ID, "worker", "t1")
	if err != nil {
		t.Fatal(err)
	}
	if future.MaxToolCalls != 30 {
		t.Fatalf("most specific (task) override not applied, got %d", future.MaxToolCalls)
	}
	sibling, err := s.effectiveRunLimits(w.ID, "worker", "t2")
	if err != nil {
		t.Fatal(err)
	}
	if sibling.MaxToolCalls != 40 {
		t.Fatalf("sibling task should fall back to role scope, got %d", sibling.MaxToolCalls)
	}
	otherRole, err := s.effectiveRunLimits(w.ID, "planner", "t3")
	if err != nil {
		t.Fatal(err)
	}
	if otherRole.MaxToolCalls != 50 {
		t.Fatalf("other role should fall back to mission scope, got %d", otherRole.MaxToolCalls)
	}

	// REQ-ADM-05: the value frozen before any of these changes stays untouched.
	if frozen.MaxToolCalls != 100 {
		t.Fatalf("changing configuration mutated an already-frozen attempt: %+v", frozen)
	}

	hist, err := s.runLimitsHistory(ScopeTask, w.ID, "t1")
	if err != nil {
		t.Fatal(err)
	}
	if len(hist) != 1 || hist[0].Revision != 1 || hist[0].Values.MaxToolCalls != 30 {
		t.Fatalf("unexpected history after first write: %+v", hist)
	}

	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "task2", Scope: ScopeTask, Mission: w.ID, Key: "t1", Revision: taskEntry.Revision, Values: RunLimits{MaxToolCalls: 10}, Reason: "encore plus strict"}); err != nil {
		t.Fatal(err)
	}
	tightened, err := s.effectiveRunLimits(w.ID, "worker", "t1")
	if err != nil {
		t.Fatal(err)
	}
	if tightened.MaxToolCalls != 10 {
		t.Fatalf("second override not applied: %d", tightened.MaxToolCalls)
	}

	rolled, err := s.rollbackRunLimits(ScopeTask, w.ID, "t1", 1, "task-rollback", "", 2)
	if err != nil {
		t.Fatal(err)
	}
	if rolled.Values.MaxToolCalls != 30 {
		t.Fatalf("rollback restored wrong value: %+v", rolled)
	}
	if rolled.Revision != 3 {
		t.Fatalf("rollback must create a new revision, not rewrite history: got %d", rolled.Revision)
	}
	after, err := s.effectiveRunLimits(w.ID, "worker", "t1")
	if err != nil {
		t.Fatal(err)
	}
	if after.MaxToolCalls != 30 {
		t.Fatalf("effective limits not rolled back: %d", after.MaxToolCalls)
	}
	hist2, err := s.runLimitsHistory(ScopeTask, w.ID, "t1")
	if err != nil {
		t.Fatal(err)
	}
	if len(hist2) != 3 {
		t.Fatalf("rollback must keep every prior revision in history, got %d entries", len(hist2))
	}
	if hist2[0].RollbackOf != 1 {
		t.Fatalf("latest history entry should record which revision it rolled back to: %+v", hist2[0])
	}

	// Still frozen: unaffected by the tighten-then-rollback sequence too.
	if frozen.MaxToolCalls != 100 {
		t.Fatalf("frozen attempt mutated after rollback: %+v", frozen)
	}

	if _, err = s.rollbackRunLimits(ScopeTask, w.ID, "t1", 99, "task-rollback-bad", "", 3); err == nil {
		t.Fatal("expected rollback to a non-existent revision to fail")
	}
}

func TestRunLimitsConfigConcurrencyAndReplay(t *testing.T) {
	s := storeTest(t)
	first, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "c1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 50}, Reason: "r"})
	if err != nil {
		t.Fatal(err)
	}
	// Stale expected revision (still 0) now conflicts with the current revision (1).
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "c2", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 60}, Reason: "r"}); err == nil {
		t.Fatal("expected a revision conflict")
	}
	// Replaying the exact same event is idempotent: no new revision.
	replay, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "c1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 50}, Reason: "r"})
	if err != nil {
		t.Fatal(err)
	}
	if replay.Revision != first.Revision {
		t.Fatalf("identical replay must not create a new revision: got %d want %d", replay.Revision, first.Revision)
	}
	// Same event id, different content, is refused rather than silently applied.
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "c1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 51}, Reason: "r"}); err == nil {
		t.Fatal("expected event_id reuse with different content to be refused")
	}
}

func TestRunLimitsMigrationKeepsExistingConfiguration(t *testing.T) {
	s := storeTest(t)
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "before-upgrade", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 42}}); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("PRAGMA user_version=22"); err != nil {
		t.Fatal(err)
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}
	next, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer next.db.Close()
	config, err := next.currentRunLimitsConfig(ScopeProject, "", "")
	if err != nil || config.Values.MaxToolCalls != 42 || config.Revision != 1 {
		t.Fatalf("configuration lost: %+v %v", config, err)
	}
	history, err := next.runLimitsHistory(ScopeProject, "", "")
	if err != nil || len(history) != 1 || history[0].Values.MaxToolCalls != 42 {
		t.Fatalf("history lost: %+v %v", history, err)
	}
}
