//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestControlInfrastructureStorageUpgradePreservesBindingsAndRefusesOldContract(t *testing.T) {
	s, w, a := runSupervisionFixture(t, true)
	var err error
	w, err = s.mutate(w.ID, "fixture", "persist-authority", w.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task(a.TaskID)
		task.IntegrationRetries = []IntegrationRetry{{Agent: a.ID, Attempt: a.Attempt, RuntimeBudgetDigest: "explicit-budget-hash", Reason: "Retain public reservation across upgrade"}}
		d := buildAttemptDiagnostic(a.ID, a.Attempt, []toolFailure{{technical: "mount failed: permission denied"}}, "", 3)
		w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "infra-contract", Scope: task.ScopeID, Task: task.ID, Attempt: a.Attempt, Kind: "run_observation", RunObservation: &RunObservation{Agent: a.ID, Trigger: "infrastructure", Diagnostic: &d}})
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	var before []byte
	if err = s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&before); err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("PRAGMA user_version=29"); err != nil {
		t.Fatal(err)
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reader, err := openStoreWithMigration(s.root, false, false)
	if err == nil {
		reader.db.Close()
		t.Fatal("legacy Work contract allowed to lose authority fields")
	}
	if commandFailure(err).Code != "storage_upgrade_required" {
		t.Fatal(err)
	}
	up, err := openStoreWithMigration(s.root, false, true)
	if err != nil {
		t.Fatal(err)
	}
	defer up.db.Close()
	var after []byte
	if err = up.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&after); err != nil {
		t.Fatal(err)
	}
	if string(before) != string(after) {
		t.Fatal("migration rewrote original Work")
	}
	got, err := up.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := got.task(a.TaskID)
	if got.Revision != w.Revision || len(task.IntegrationRetries) != 1 || task.IntegrationRetries[0].RuntimeBudgetDigest != "explicit-budget-hash" {
		t.Fatal("budget authority or revision lost")
	}
	raw, _ := json.Marshal(got.Planning.Inbox)
	if !containsInfrastructureContract(raw) {
		t.Fatal("live incident binding lost")
	}
	paths, err := filepath.Glob(filepath.Join(s.root, ".swarm/state-pre-v30-*.db"))
	if err != nil || len(paths) != 1 {
		t.Fatal("private upgrade backup absent", err)
	}
	info, err := os.Stat(paths[0])
	if err != nil || info.Mode().Perm() != 0600 {
		t.Fatal("backup not private", err)
	}
}
func containsInfrastructureContract(raw []byte) bool {
	var events []PlanningEvent
	if json.Unmarshal(raw, &events) != nil {
		return false
	}
	for _, e := range events {
		if e.ID == "infra-contract" && e.RunObservation != nil && e.RunObservation.Diagnostic != nil && e.RunObservation.Diagnostic.AttemptID == e.Attempt {
			return true
		}
	}
	return false
}
