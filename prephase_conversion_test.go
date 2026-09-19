//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

func readyPreparation(t *testing.T, s *Store, work string) Preparation {
	t.Helper()
	prepMethods(t, s)
	zero := 0
	p, e := s.preparationCommand(PreparationRequest{Version: 1, Event: newID("e-"), Action: "create", Revision: &zero, WorkID: work, Title: "Conversion test", Text: "Besoin"})
	if e != nil {
		t.Fatal(e)
	}
	p = prepAdopt(t, s, p)
	raw, _ := json.Marshal(fixturePlan())
	p = prepSave(t, s, p, "plan", string(raw))
	r := prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	p, e = s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	return p
}
func conversionRequest(t *testing.T, s *Store, p Preparation, action string) PreparationRequest {
	t.Helper()
	review, e := s.preparationConversionReview(p.ID)
	if e != nil {
		t.Fatal(e)
	}
	r := prepRequest(p, action)
	r.WorkRevision = &review.WorkRevision
	r.Hash = p.Documents["plan"].Hash
	if action == "release-plan" {
		r.Hash = p.Conversion.PlanHash
	}
	return r
}
func TestPreparationConversionAtomicLocksRelease(t *testing.T) {
	s := storeTest(t)
	existing, launch := setupAgent(t, s)
	// Existing work is already automatic, with a valid profile. Its unrelated task is done.
	existing.Tasks[0].Status = "abandoned"
	raw, _ := json.Marshal(existing)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, existing.ID)
	if e := s.setProfile(existing.ID, "", LaunchProfile{Provider: "fixture", Role: "worker", Workspace: s.root}, existing.Revision); e != nil {
		t.Fatal(e)
	}
	if e := s.setAutonomy(existing.ID, autonomyAuto, 2); e != nil {
		t.Fatal(e)
	}
	p := readyPreparation(t, s, existing.ID)
	r := conversionRequest(t, s, p, "create-missions")
	p, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	w, e := s.get(p.WorkID)
	if e != nil {
		t.Fatal(e)
	}
	ids := p.Conversion.TaskIDs
	if len(w.Tasks) != 3 || len(ids) != 2 || w.Tasks[2].Depends[0] != ids[0] {
		t.Fatal(w)
	}
	for _, id := range ids {
		task, _ := w.task(id)
		if !task.LaunchHeld || task.Gate != nil || len(task.Attempts) != 0 || len(task.PlanChecks) != 4 || task.PlanMaxAttempts == 0 {
			t.Fatal(task)
		}
	}
	// Dispatch and both manual preview/launch paths must refuse before reservations.
	decisions, e := s.dispatch(w.ID)
	if e != nil || len(decisions) != 0 {
		t.Fatal(decisions, e)
	}
	launch.TaskID = ids[0]
	launch.Revision = w.Revision
	for _, preview := range []bool{true, false} {
		if _, _, e = s.prepareLaunch(w.ID, launch, preview); e == nil || !strings.Contains(e.Error(), "non autorisé") {
			t.Fatal(e)
		}
	}
	for _, table := range []string{"agents", "reservations"} {
		var n int
		if e = s.db.QueryRow("SELECT count(*) FROM " + table).Scan(&n); e != nil || n != 0 {
			t.Fatal(table, n, e)
		}
	}
	if _, e = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('old-process',?,?,?,'queued','{}','{}')", w.ID, ids[0], s.root); e == nil || !strings.Contains(e.Error(), "verrouille") {
		t.Fatal("database lock bypass", e)
	}
	again, e := s.preparationCommand(r)
	if e != nil || again.Conversion.TaskIDs[0] != ids[0] {
		t.Fatal(e)
	}
	r = conversionRequest(t, s, p, "create-missions")
	again, e = s.preparationCommand(r)
	if e != nil || again.Conversion.TaskIDs[0] != ids[0] {
		t.Fatal(e)
	}
	p = again
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision {
		t.Fatal("duplicate work mutation")
	}
	// Another plan in this same work stays locked when the first is released.
	second := readyPreparation(t, s, w.ID)
	second, e = s.preparationCommand(conversionRequest(t, s, second, "create-missions"))
	if e != nil {
		t.Fatal(e)
	}
	p, e = s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
	if e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	var held int
	s.db.QueryRow("SELECT count(*) FROM preparation_launch_locks WHERE released=0").Scan(&held)
	if held != 2 {
		t.Fatal(held)
	}
	if p.Conversion.ReleasedAt == "" {
		t.Fatal("not released")
	}
	launch.Revision = w.Revision
	if _, _, e = s.prepareLaunch(w.ID, launch, true); e != nil {
		t.Fatal("released task blocked", e)
	}
	launch.TaskID = ids[1]
	if _, _, e = s.prepareLaunch(w.ID, launch, true); e == nil {
		t.Fatal("dependency bypassed")
	}
	// Authorization itself never creates a process or reservation.
	as, _ := s.agents(w.ID)
	if len(as) != 0 {
		t.Fatal(as)
	}
}
func TestPreparationConversionRollbackStaleAndConcurrency(t *testing.T) {
	s := storeTest(t)
	p := readyPreparation(t, s, "")
	r := conversionRequest(t, s, p, "create-missions")
	if _, e := s.db.Exec(`CREATE TRIGGER fail_second_lock BEFORE INSERT ON preparation_launch_locks WHEN NEW.task_id LIKE '%T2' BEGIN SELECT RAISE(ABORT,'fixture failure'); END`); e != nil {
		t.Fatal(e)
	}
	if _, e := s.preparationCommand(r); e == nil {
		t.Fatal("expected rollback")
	}
	for _, table := range []string{"works", "events", "preparation_launch_locks"} {
		var n int
		s.db.QueryRow("SELECT count(*) FROM " + table).Scan(&n)
		if n != 0 {
			t.Fatal(table, n)
		}
	}
	before, _ := s.preparation(p.ID)
	if before.Revision != p.Revision || before.Conversion != nil {
		t.Fatal(before)
	}
	s.db.Exec("DROP TRIGGER fail_second_lock")
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	var wg sync.WaitGroup
	errs := make(chan error, 2)
	for _, store := range []*Store{s, other} {
		wg.Add(1)
		go func(store *Store) { defer wg.Done(); _, e := store.preparationCommand(r); errs <- e }(store)
	}
	wg.Wait()
	close(errs)
	for e := range errs {
		if e != nil && !strings.Contains(e.Error(), "écriture") {
			t.Fatal(e)
		}
	}
	result, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	var n int
	s.db.QueryRow("SELECT count(*) FROM works").Scan(&n)
	if n != 1 {
		t.Fatal(n)
	}
	if len(result.Conversion.TaskIDs) != 2 {
		t.Fatal(result)
	}
	stale := readyPreparation(t, s, "")
	r = conversionRequest(t, s, stale, "create-missions")
	stale = prepSave(t, s, stale, "besoin", "Autre besoin")
	r.Revision = &stale.Revision
	if _, e = s.preparationCommand(r); e == nil {
		t.Fatal("stale verdict used")
	}
}
func TestPreparationV8LockMigrationBackup(t *testing.T) {
	s := storeTest(t)
	if _, e := s.db.Exec("DROP TRIGGER preparation_launch_insert; DROP TRIGGER preparation_launch_update; DROP TABLE preparation_launch_locks; PRAGMA user_version=8"); e != nil {
		t.Fatal(e)
	}
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	backups, _ := filepath.Glob(filepath.Join(s.root, ".swarm/state-pre-v9-*.db"))
	if len(backups) != 1 {
		t.Fatal(backups)
	}
	db, e := sql.Open("sqlite", backups[0])
	if e != nil {
		t.Fatal(e)
	}
	defer db.Close()
	var version int
	if e = db.QueryRow("PRAGMA user_version").Scan(&version); e != nil || version != 8 {
		t.Fatal(version, e)
	}
	if e = other.db.QueryRow("PRAGMA user_version").Scan(&version); e != nil || version != schemaVersion {
		t.Fatal(version, e)
	}
}
