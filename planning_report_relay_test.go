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
	reportBody := strings.Repeat("Observed result; evidence and limitations.\n", 140) + "IMPORTANT_FINDING_AFTER_4000_BYTES"
	os.WriteFile(report, []byte(reportBody), 0600)
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
			if event.Attempt != a.Attempt || event.Handoff == nil || len(event.Artifacts) != 1 || event.Artifacts[0].Path != path || event.Artifacts[0].SHA256 != hash([]byte(reportBody)) {
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
	// A real child process receives the complete report, including a finding
	// beyond the old 4000-byte excerpt, and uses it to propose a scoped task.
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	provider := ps.Providers[got.Planning.Provider]
	script := `#!/usr/bin/env python3
import json,sys
text=sys.stdin.read()
ctx=json.loads(text[text.rfind('\n{')+1:])
reports=ctx['handoff_contents']
assert len(reports)==1 and reports[0]['text'].endswith('IMPORTANT_FINDING_AFTER_4000_BYTES')
assert reports[0]['scope']==ctx['scope']['id']
with open(__file__+'.context','w') as f: json.dump(ctx,f)
op={'kind':'task','id':'followup-finding','title':'Traiter le constat remonté','requirements':list(ctx['requirements']), 'deliverable':'docs/followup-finding.md','criteria':['Le constat est vérifié'], 'depends':[], 'next':'Traiter IMPORTANT_FINDING_AFTER_4000_BYTES sans modifier les autres tâches'}
reply={'input_events':[reports[0]['event_id']],'reason':'Le retour complet révèle une action dans mon périmètre','operations':[op]}
print(json.dumps({'type':'result','result':json.dumps(reply)}))
`
	if err = os.WriteFile(provider.Command, []byte(script), 0700); err != nil {
		t.Fatal(err)
	}
	got.Planning.Paused = false // fixture release; no real mission is touched
	raw, _ = json.Marshal(got)
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, got.ID); err != nil {
		t.Fatal(err)
	}
	if err = s.planningStep(got.ID); err != nil {
		t.Fatal(err)
	}
	final, err := s.get(got.ID)
	if err != nil {
		t.Fatal(err)
	}
	followup, err := final.task("followup-finding")
	if err != nil || !strings.Contains(followup.Next, "IMPORTANT_FINDING_AFTER_4000_BYTES") {
		t.Fatal("planner did not act on the delivered finding", err)
	}
	owner, _ := final.Planning.scope(followup.ScopeID)
	if owner.Delivery == nil || owner.Delivery.Decision == "" || len(owner.Delivery.Reports) != 1 {
		t.Fatal("missing durable context/decision receipt")
	}
	for _, event := range final.Planning.Inbox {
		if event.Kind == "handoff" && event.Decision != owner.Delivery.Decision {
			t.Fatal("decision not linked to the delivered handoff")
		}
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
