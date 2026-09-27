//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestTaskRestartPublicPreservesHistoryAndIsIdempotent(t *testing.T) {
	s, w := exhaustedTaskFixture(t)
	w = managedReviewFixture(t, s, w)
	task := w.Tasks[0]
	r := PlanningRequest{Schema: 1, EventID: "explicit-restart", Revision: w.Revision, Task: task.ID, Attempt: task.Attempts[len(task.Attempts)-1].ID, ExpectedCandidate: w.Planning.Repository.Candidate, ConfirmRecovery: true, Reason: "Operator requests a fresh bounded production", RecoveryInstruction: "Start a fresh production in a newly attributed copy; preserve historical evidence and rerun acceptance checks."}
	raw, _ := json.Marshal(r)
	path := filepath.Join(t.TempDir(), "request.json")
	if e := os.WriteFile(path, raw, 0600); e != nil {
		t.Fatal(e)
	}
	var out bytes.Buffer
	if e := s.planningCLI([]string{"planning", "restart-task", w.ID}, path, &out); e != nil {
		t.Fatal(e)
	}
	after, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	got := after.Tasks[0]
	if got.Status != "todo" || got.PlanMaxAttempts != task.PlanMaxAttempts+1 || !got.PlanningRetry || len(got.Restarts) != 1 || got.IndependentReview != nil || got.Gate != nil {
		t.Fatalf("restart not prepared: %+v", got)
	}
	if !reflect.DeepEqual(got.Attempts, task.Attempts) || !reflect.DeepEqual(got.Criteria, task.Criteria) || !reflect.DeepEqual(after.Planning, w.Planning) || len(got.PreviousReviews) != len(task.PreviousReviews)+1 {
		t.Fatal("history, contract or review budget changed")
	}
	replay, e := s.planningChange(w.ID, "restart-task", r)
	if e != nil || replay.Revision != after.Revision {
		t.Fatal("idempotence", e)
	}
	r.EventID = "duplicate-restart"
	r.Revision = after.Revision
	if _, e = s.planningChange(w.ID, "restart-task", r); e == nil {
		t.Fatal("duplicate grant accepted")
	}
}

func TestTaskRestartRejectsStaleOrUnsafeRequests(t *testing.T) {
	for _, mode := range []string{"unconfirmed", "attempt", "candidate", "revision", "running-review", "active-agent"} {
		t.Run(mode, func(t *testing.T) {
			s, w := exhaustedTaskFixture(t)
			w = managedReviewFixture(t, s, w)
			task := w.Tasks[0]
			r := PlanningRequest{Schema: 1, EventID: "restart", Revision: w.Revision, Task: task.ID, Attempt: task.Attempts[len(task.Attempts)-1].ID, ExpectedCandidate: w.Planning.Repository.Candidate, ConfirmRecovery: true, Reason: "Explicit operator restart", RecoveryInstruction: "Fresh bounded production with all mandatory acceptance checks."}
			switch mode {
			case "unconfirmed":
				r.ConfirmRecovery = false
			case "attempt":
				r.Attempt = "foreign"
			case "candidate":
				r.ExpectedCandidate = "foreign"
			case "revision":
				r.Revision--
			case "running-review":
				w.Tasks[0].IndependentReview.State = "running"
				raw, _ := json.Marshal(w)
				s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
			case "active-agent":
				a := Agent{ID: "active", WorkID: w.ID, TaskID: task.ID, Status: "running"}
				body, _ := json.Marshal(a)
				if _, e := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, task.ID, s.root, a.Status, body, []byte("{}")); e != nil {
					t.Fatal(e)
				}
			}
			if _, e := s.planningChange(w.ID, "restart-task", r); e == nil {
				t.Fatal("unsafe restart accepted")
			}
			after, _ := s.get(w.ID)
			if after.Revision != w.Revision || len(after.Tasks[0].Restarts) != 0 {
				t.Fatal("failed request mutated task")
			}
		})
	}
}
