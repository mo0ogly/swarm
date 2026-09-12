//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestWebGateWriteFailureIsNotReportedAsSuccess(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	raw := fixture(t, s.root)
	os.Mkdir(filepath.Join(s.root, "docs"), 0700)
	if err := os.WriteFile(filepath.Join(s.root, "docs/t1.evidence.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("CREATE TRIGGER reject_gate BEFORE UPDATE ON works BEGIN SELECT RAISE(ABORT, 'write refused by regression fixture'); END"); err != nil {
		t.Fatal(err)
	}
	_, err := s.webAction(webRequest{Kind: "gate", Work: w.ID, Task: "t1", Path: "docs/t1.evidence.json", Name: "Gate de recette", Revision: w.Revision, Event: newID("test-")})
	if err == nil || !strings.Contains(err.Error(), "write refused") {
		t.Fatal("gate write error swallowed", err)
	}
	got, _ := s.get(w.ID)
	if got.Tasks[0].Gate != nil {
		t.Fatal("failed write persisted")
	}
}

func TestObservedClaudeTerminalUsagePreservesCacheFields(t *testing.T) {
	raw := []byte(`{"subtype":"success","terminal_reason":"completed","session_id":"fixture","total_cost_usd":0.9,"usage":{"input_tokens":12,"output_tokens":14782,"cache_read_input_tokens":179357,"cache_creation_input_tokens":47452}}`)
	var event map[string]any
	json.Unmarshal(raw, &event)
	u := providerUsage(event)
	if u == nil || u.Input != 12 || u.CacheRead == nil || *u.CacheRead != 179357 || u.CacheCreation == nil || *u.CacheCreation != 47452 || u.ReportedCost == nil {
		t.Fatal(u)
	}
	delete(event, "session_id")
	if providerUsage(event) != nil {
		t.Fatal("unidentified result recognized")
	}
}

func TestVersionTwoBackupRestoresContextBeforeTrackingMigration(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if _, err := s.db.Exec("DROP TABLE assist_previews; DROP TABLE assist_reservations; DROP TABLE assist_turns; DROP TABLE decisions; DROP TABLE session_visits; DROP TABLE budgets; DROP TABLE reservations; PRAGMA user_version=2;"); err != nil {
		t.Fatal(err)
	}
	s.db.Close()
	migrated, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer migrated.db.Close()
	backups, err := filepath.Glob(filepath.Join(s.root, ".swarm/state-pre-v3-*.db"))
	if err != nil || len(backups) != 1 {
		t.Fatal(backups, err)
	}
	info, _ := os.Stat(backups[0])
	if info.Mode().Perm() != 0600 {
		t.Fatal("backup permissions")
	}
	destination := t.TempDir()
	os.Mkdir(filepath.Join(destination, ".swarm"), 0700)
	raw, err := os.ReadFile(backups[0])
	if err != nil {
		t.Fatal(err)
	}
	os.WriteFile(filepath.Join(destination, ".swarm/state.db"), raw, 0600)
	restored, err := openStore(destination, false)
	if err != nil {
		t.Fatal(err)
	}
	defer restored.db.Close()
	got, err := restored.get(w.ID)
	if err != nil || got.Revision != w.Revision || len(got.Tasks) != 1 {
		t.Fatal(got, err)
	}
	agents, err := restored.agents(w.ID)
	if err != nil || len(agents) != 0 {
		t.Fatal("restore activated processes", err)
	}
}

func TestSilenceRecursAfterNewHeartbeat(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Heartbeat = time.Now().Add(-time.Hour).UTC().Format(time.RFC3339Nano)
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	ds, err := s.decisions(w.ID)
	if err != nil || len(ds) != 1 || ds[0].Kind != "silence" {
		t.Fatal(ds, err)
	}
	if err = s.resolveDecision(w.ID, ds[0].ID, "reviewer", "Silence examiné"); err != nil {
		t.Fatal(err)
	}
	again, _ := s.decisions(w.ID)
	if len(again) != 1 {
		t.Fatal("same silence duplicated")
	}
	a.Heartbeat = time.Now().UTC().Format(time.RFC3339Nano)
	s.saveAgent(a)
	s.decisions(w.ID)
	a.Heartbeat = time.Now().Add(-30 * time.Minute).UTC().Format(time.RFC3339Nano)
	s.saveAgent(a)
	ds, err = s.decisions(w.ID)
	if err != nil || len(ds) != 2 || ds[1].ResolvedAt != "" {
		t.Fatal("new silence hidden", ds, err)
	}
}

func TestTaskGraphRejectsCyclesNotOrdering(t *testing.T) {
	for _, tasks := range [][]Task{{{ID: "a", Depends: []string{"b"}}, {ID: "b"}}, {{ID: "b"}, {ID: "a", Depends: []string{"b"}}}} {
		if err := validateTaskGraph(tasks); err != nil {
			t.Fatal(err)
		}
	}
	for _, tasks := range [][]Task{{{ID: "a", Depends: []string{"a"}}}, {{ID: "a", Depends: []string{"b"}}, {ID: "b", Depends: []string{"a"}}}, {{ID: "a", Depends: []string{"missing"}}}} {
		if validateTaskGraph(tasks) == nil {
			t.Fatal("invalid graph accepted")
		}
	}
}

func TestExportChecksEveryGateSharingEvidence(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = gateTest(t, s, w, fixture(t, s.root))
	copyTask := w.Tasks[0]
	copyTask.ID = "t2"
	b, _ := json.Marshal(copyTask.Gate)
	copyTask.Gate = nil
	json.Unmarshal(b, &copyTask.Gate)
	for file := range copyTask.Gate.Evaluation.Artifacts {
		copyTask.Gate.Evaluation.Artifacts[file] = "stale"
	}
	w.Tasks = append(w.Tasks, copyTask)
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	err := s.export(w.ID, filepath.Join(t.TempDir(), "shared.zip"))
	if err == nil || !strings.Contains(err.Error(), "partagée") {
		t.Fatal("conflicting shared evidence exported", err)
	}
}
