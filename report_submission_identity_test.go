//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func submissionProducerFixture(t *testing.T) (*Store, Work, Agent, string) {
	t.Helper()
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	a := Agent{ID: "producer", WorkID: w.ID, TaskID: "t1", Attempt: "real-production", Status: "completed", Role: "worker", CWD: s.root, Started: time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano)}
	w.Tasks[0].Attempts = []Attempt{{ID: a.Attempt, Status: "completed"}}
	w.Tasks[0].Status = "blocked"
	w.Tasks[0].Blocker = "review required"
	raw, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	raw, _ = json.Marshal(a)
	if _, e := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, raw, []byte(`{}`)); e != nil {
		t.Fatal(e)
	}
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	report := "docs/t1.md"
	os.WriteFile(filepath.Join(s.root, report), []byte("Actual documentary delivery with its limitations."), 0600)
	return s, w, a, report
}

func TestOperatorSubmissionPreservesProductionIdentity(t *testing.T) {
	s, w, a, report := submissionProducerFixture(t)
	_, err := s.webAction(webRequest{Kind: "submit", Work: w.ID, Task: a.TaskID, Path: report, Revision: w.Revision})
	if err != nil {
		t.Fatal(err)
	}
	got, _ := s.get(w.ID)
	task := got.Tasks[0]
	if task.Status != "submitted" || len(task.Attempts) != 1 || task.Attempts[0].ID != a.Attempt {
		t.Fatalf("synthetic attempt created: %+v", task)
	}
	if task.Gate != nil || task.IndependentReview != nil {
		t.Fatal("submission fabricated validation")
	}
}

func TestOperatorSubmissionRepairsOnlyAuditedSyntheticAttempt(t *testing.T) {
	for _, mode := range []string{"legacy", "real-agent", "no-event", "wrong-report"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, report := submissionProducerFixture(t)
			r := Request{Schema: 1, EventID: "old-submission", Revision: w.Revision, ID: a.TaskID, Status: "submitted", Next: "Évaluer les preuves et la gate delivery ; handoff : " + report}
			raw, _ := json.Marshal(r)
			kind := "task.submit"
			if mode == "no-event" {
				kind = "fixture"
			}
			var err error
			w, err = s.mutate(w.ID, kind, r.EventID, w.Revision, raw, func(w *Work) error {
				if e := s.apply(w, "task.update", Request{ID: a.TaskID, Status: "running"}); e != nil {
					return e
				}
				return s.apply(w, "task.update", Request{ID: a.TaskID, Status: "submitted", Outcome: "completed", Next: r.Next})
			})
			if err != nil {
				t.Fatal(err)
			}
			synthetic := w.Tasks[0].Attempts[1]
			if mode == "real-agent" {
				b := a
				b.ID = "second-producer"
				b.Attempt = synthetic.ID
				raw, _ = json.Marshal(b)
				if _, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", b.ID, w.ID, b.TaskID, s.root, b.Status, raw, []byte(`{}`)); err != nil {
					t.Fatal(err)
				}
			}
			if mode == "wrong-report" {
				report = "docs/other.md"
				os.WriteFile(filepath.Join(s.root, report), []byte("unattributed"), 0600)
			}
			_, err = s.webAction(webRequest{Kind: "submit", Work: w.ID, Task: a.TaskID, Path: report, Revision: w.Revision})
			got, _ := s.get(w.ID)
			task := got.Tasks[0]
			if mode != "legacy" {
				if err == nil || got.Revision != w.Revision || len(task.Attempts) != 2 {
					t.Fatal("unsafe repair accepted", err)
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			if task.Status != "submitted" || len(task.Attempts) != 1 || task.Attempts[0].ID != a.Attempt || len(task.LegacyReportSubmissions) != 1 || task.LegacyReportSubmissions[0].ID != synthetic.ID {
				t.Fatalf("lost evidence: %+v", task)
			}
			events, _ := s.events(w.ID)
			found := false
			for _, event := range events {
				if event.ID == "old-submission" {
					found = true
				}
			}
			if !found {
				t.Fatal("old event lost")
			}
			if _, err = s.webAction(webRequest{Kind: "submit", Work: w.ID, Task: a.TaskID, Path: report, Revision: got.Revision}); err == nil {
				t.Fatal("repair replay created another attempt")
			}
		})
	}
}

func TestCorrectedReportSubmissionPreservesRefusalAndSpend(t *testing.T) {
	for _, changed := range []bool{false, true} {
		t.Run(map[bool]string{false: "unchanged", true: "corrected"}[changed], func(t *testing.T) {
			s, w, a, report := submissionProducerFixture(t)
			old, err := os.ReadFile(filepath.Join(s.root, report))
			if err != nil {
				t.Fatal(err)
			}
			_, configured := planningFixture(t)
			w.Planning = configured.Planning
			w.Tasks[0].ScopeID = "root"
			w.Tasks[0].Requirements = append([]string{}, w.Planning.Scopes[0].Requirements...)
			w.Planning.Reviewer = &ReviewerConfig{Calls: 9, MaxCalls: 40}
			w.Tasks[0].IndependentReview = &IndependentReview{ID: "refusal", Attempt: a.Attempt, Producer: a.ID, State: "changes_requested", Report: report, Digest: hash(old), Finished: now()}
			raw, _ := json.Marshal(w)
			if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			if changed {
				if err = os.WriteFile(filepath.Join(s.root, report), append(old, []byte("\nSupervisor correction and newly executed evidence.")...), 0600); err != nil {
					t.Fatal(err)
				}
			}
			_, err = s.webAction(webRequest{Kind: "submit", Work: w.ID, Task: a.TaskID, Path: report, Revision: w.Revision})
			got, _ := s.get(w.ID)
			task := got.Tasks[0]
			if !changed {
				if err == nil || got.Revision != w.Revision || task.Status != "blocked" {
					t.Fatalf("unchanged failed report retried: %v", err)
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			if task.Status != "submitted" || task.IndependentReview != nil || len(task.PreviousReviews) != 1 || task.PreviousReviews[0].ID != "refusal" || got.Planning.Reviewer.Calls != 9 || len(task.Attempts) != 1 || task.Attempts[0].ID != a.Attempt {
				t.Fatalf("lost refusal, spend or producer: %+v", task)
			}
		})
	}
}
