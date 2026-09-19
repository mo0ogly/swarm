//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestPlanningReportRelayIsAtomicScopedAndIdempotent(t *testing.T) {
	s, p := preparedTeam(t)
	w, _ := s.get(p.WorkID)
	cwd := filepath.Join(s.root, "isolated")
	os.MkdirAll(filepath.Join(cwd, "docs"), 0700)
	id := w.Tasks[0].ID
	a := Agent{ID: "producer", WorkID: w.ID, TaskID: id, Attempt: "attempt-relay", CWD: cwd, Role: "worker", Status: "completed", Started: time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano)}
	w.Tasks[0].Status = "blocked"
	w.Tasks[0].Attempts = []Attempt{{ID: a.Attempt, Status: "completed"}}
	raw, _ := json.Marshal(w)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	report := filepath.Join(cwd, "docs", id+".md")
	os.WriteFile(report, []byte("Observed result; evidence and limitations."), 0600)
	// Another root report must not replace this attempt's workspace report.
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	os.WriteFile(filepath.Join(s.root, "docs", id+".md"), []byte("Unrelated report"), 0600)
	rel := "isolated/docs/" + id + ".md"
	if err := s.submitReportVerified(w.ID, id, rel, w.Revision, conductorAuthor, "wrong-digest", hash([]byte("different content"))); err == nil {
		t.Fatal("modified evidence accepted")
	}
	if err := s.submitReportVerified(w.ID, id, rel, w.Revision-1, conductorAuthor, "stale-revision", ""); err == nil {
		t.Fatal("stale revision accepted")
	}
	previous := a
	previous.Attempt = "replaced-attempt"
	if report, _ := s.relayHandoff(previous, "completed"); report != "" {
		t.Fatal("replaced attempt relayed")
	}
	unchanged, _ := s.get(w.ID)
	if unchanged.Revision != w.Revision || len(unchanged.Planning.Inbox) != len(w.Planning.Inbox) {
		t.Fatal("refusal partially mutated work or manager inbox")
	}
	path, reason := s.relayHandoff(a, "completed")
	if path != "isolated/docs/"+id+".md" {
		t.Fatal(path, reason)
	}
	got, _ := s.get(w.ID)
	task, _ := got.task(id)
	if task.Status != "submitted" || task.Gate != nil {
		t.Fatal("relay accepted result", task)
	}
	var count int
	for _, event := range got.Planning.Inbox {
		if event.Kind == "handoff" {
			count++
			if event.Attempt != a.Attempt || len(event.Artifacts) != 1 || event.Artifacts[0].Path != path || event.Artifacts[0].SHA256 != hash([]byte("Observed result; evidence and limitations.")) {
				t.Fatal(event)
			}
		}
	}
	if count != 1 {
		t.Fatal("missing atomic planning event", count)
	}
	revision := got.Revision
	if path, _ := s.relayHandoff(a, "completed"); path != "" {
		t.Fatal("duplicate relay")
	}
	got, _ = s.get(w.ID)
	if got.Revision != revision {
		t.Fatal("duplicate mutation")
	}
}
func TestAttemptReportRefusesStaleAmbiguousOutsideAndInterrupted(t *testing.T) {
	s := storeTest(t)
	cwd := filepath.Join(s.root, "sandbox")
	os.MkdirAll(filepath.Join(cwd, "docs"), 0700)
	a := Agent{TaskID: "task", CWD: cwd, Started: time.Now().UTC().Format(time.RFC3339Nano)}
	path := filepath.Join(cwd, "docs/task.md")
	os.WriteFile(path, []byte("old"), 0600)
	old := time.Now().Add(-time.Hour)
	os.Chtimes(path, old, old)
	if p, _ := s.provenAttemptReport(a); p != "" {
		t.Fatal("stale report accepted")
	}
	os.WriteFile(path, []byte("fresh"), 0600)
	os.WriteFile(filepath.Join(cwd, "docs/task-handoff-other.md"), []byte("ambiguous"), 0600)
	if p, _ := s.provenAttemptReport(a); p != "" {
		t.Fatal("ambiguous report accepted")
	}
	a.CWD = t.TempDir()
	if p, _ := s.provenAttemptReport(a); p != "" {
		t.Fatal("outside workspace accepted")
	}
	if p, _ := s.relayHandoff(a, "interrupted"); p != "" {
		t.Fatal("interrupted result relayed")
	}
}
