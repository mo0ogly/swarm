//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestSharedRequestReplayAndConflict(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	r := Request{Schema: 1, EventID: newID("test-"), Revision: w.Revision, ID: "t1", Status: "running"}
	first, e := s.executeRequest(w.ID, "task.update", r)
	if e != nil {
		t.Fatal(e)
	}
	again, e := s.executeRequest(w.ID, "task.update", r)
	if e != nil || again.Revision != first.Revision || len(again.Tasks[0].Attempts) != 1 {
		t.Fatal("replay has another effect", e)
	}
	r.EventID = newID("test-")
	_, e = s.executeRequest(w.ID, "task.update", r)
	if e == nil || commandFailure(e).Code != "revision_conflict" {
		t.Fatal("missing structured conflict", e)
	}
}
func TestCLIAndTerminalShareLiveTaskGuard(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if _, _, e := s.prepare(w.ID, r); e != nil {
		t.Fatal(e)
	}
	e := s.operatorTask(w.ID, Request{ID: "t1", Status: "todo"})
	if e == nil || commandFailure(e).Code != "active_agent" {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	request := Request{Schema: 1, EventID: newID("test-"), Revision: w.Revision, ID: "t1", Status: "todo"}
	raw, _ := json.Marshal(request)
	path := filepath.Join(s.root, "request.json")
	os.WriteFile(path, raw, 0600)
	var out, errs bytes.Buffer
	code := run([]string{"--root", s.root, "--json", "task", "update", w.ID, "--input", path}, &out, &errs)
	var result struct{ Failure CommandError }
	if code != 2 || json.Unmarshal(errs.Bytes(), &result) != nil || result.Failure.Code != "active_agent" {
		t.Fatal(code, errs.String())
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision || current.Tasks[0].Status != "running" {
		t.Fatal("rejected CLI mutation persisted")
	}
}
func TestMigrationBackupCanRestoreAndResume(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	_, e := s.db.Exec("DROP TABLE assist_previews; DROP TABLE assist_reservations; DROP TABLE assist_turns; DROP TABLE decisions; DROP TABLE session_visits; DROP TABLE budgets; DROP TABLE reservations; DROP TABLE agent_logs; DROP TABLE agents; DROP TABLE cockpit_events; DROP TABLE cockpit_controls; DROP TABLE cockpit_tasks; PRAGMA user_version=1;")
	if e != nil {
		t.Fatal(e)
	}
	s.db.Close()
	migrated, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer migrated.db.Close()
	backups, e := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v2-*.db"))
	if e != nil || len(backups) != 1 {
		t.Fatal("backup absent", backups, e)
	}
	info, _ := os.Stat(backups[0])
	if info.Mode().Perm() != 0600 {
		t.Fatal("backup permissions")
	}
	raw, e := os.ReadFile(backups[0])
	if e != nil {
		t.Fatal(e)
	}
	root := t.TempDir()
	os.Mkdir(filepath.Join(root, ".swarm"), 0700)
	os.WriteFile(filepath.Join(root, ".swarm/state.db"), raw, 0600)
	restored, e := openStore(root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer restored.db.Close()
	got, e := restored.get(w.ID)
	if e != nil || got.Revision != w.Revision || got.Title != w.Title {
		t.Fatal("restore lost work", e)
	}
	agents, e := restored.agents(w.ID)
	if e != nil || len(agents) != 0 {
		t.Fatal("restore launched agent", e)
	}
}
