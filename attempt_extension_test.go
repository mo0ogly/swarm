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

func exhaustedTaskFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s, w := managedFixture(t)
	task := &w.Tasks[0]
	task.Status = "blocked"
	task.PlanMaxAttempts = 2
	task.PlanToolLimit = 100
	task.Attempts = []Attempt{{ID: "old-1", Status: "failed"}, {ID: "old-2", Status: "completed"}}
	task.IndependentReview = &IndependentReview{ID: "old-opinion", State: "changes_requested", Attempt: "old-2", Reason: "Missing evidence"}
	task.Profile = &LaunchProfile{Provider: "fixture", Role: "worker", Workspace: s.root, Instruction: "old instruction"}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	return s, w
}

func TestAttemptExtensionPublicDecisionPreservesEvidenceAndDoesNotLaunch(t *testing.T) {
	s, w := exhaustedTaskFixture(t)
	r := PlanningRequest{Schema: 1, EventID: "operator-extension", Revision: w.Revision, Task: w.Tasks[0].ID, Reason: "Operator authorizes one correction after reviewing evidence", RecoveryInstruction: "Supply missing source evidence and rerun the same checks."}
	file := filepath.Join(t.TempDir(), "extension.json")
	raw, _ := json.Marshal(r)
	os.WriteFile(file, raw, 0600)
	var out bytes.Buffer
	if err := s.planningCLI([]string{"planning", "extend-attempt", w.ID}, file, &out); err != nil {
		t.Fatal(err)
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	want := w.Tasks[0]
	want.PlanMaxAttempts = 3
	want.Next = r.RecoveryInstruction
	profile := *want.Profile
	profile.Instruction = r.RecoveryInstruction
	profile.Actor = after.Tasks[0].Profile.Actor
	profile.Updated = after.Tasks[0].Profile.Updated
	want.Profile = &profile
	if !reflect.DeepEqual(want, after.Tasks[0]) || !reflect.DeepEqual(w.Planning, after.Planning) {
		t.Fatal("extension changed evidence, task contract or reviewer budget")
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 0 || after.Tasks[0].Status != "blocked" {
		t.Fatal("extension started production")
	}
	replay, err := s.planningChange(w.ID, "extend-attempt", r)
	if err != nil || replay.Revision != after.Revision {
		t.Fatal("extension replay not idempotent", err)
	}
	r.EventID = "another-decision"
	if _, err = s.planningChange(w.ID, "extend-attempt", r); err == nil {
		t.Fatal("stale revision accepted")
	}
	r.Revision = after.Revision
	if _, err = s.planningChange(w.ID, "extend-attempt", r); err == nil {
		t.Fatal("fourth attempt authorized")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	persisted, _ := reopened.get(w.ID)
	if !reflect.DeepEqual(after, persisted) {
		t.Fatal("authorization not durable")
	}
	events, _ := s.events(w.ID)
	last := events[len(events)-1]
	if !bytes.Contains(last.Payload, []byte(r.RecoveryInstruction)) || !bytes.Contains(last.Payload, []byte(r.Reason)) {
		t.Fatal("reason or changed precondition absent from audit")
	}
}

func TestAttemptExtensionRejectsActiveUnexhaustedAndUnexplained(t *testing.T) {
	for _, mode := range []string{"queued", "review", "available", "reason", "instruction", "accepted"} {
		t.Run(mode, func(t *testing.T) {
			s, w := exhaustedTaskFixture(t)
			r := PlanningRequest{Schema: 1, EventID: "extension", Revision: w.Revision, Task: w.Tasks[0].ID, Reason: "Explicit operator decision", RecoveryInstruction: "Correct the missing evidence before repeating the controls."}
			switch mode {
			case "queued":
				a := Agent{ID: "queued-agent", WorkID: w.ID, TaskID: r.Task, Status: "queued"}
				raw, _ := json.Marshal(a)
				s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, r.Task, s.root, a.Status, raw, []byte("{}"))
			case "review":
				w.Tasks[0].IndependentReview.State = "running"
			case "available":
				w.Tasks[0].Attempts = w.Tasks[0].Attempts[:1]
			case "reason":
				r.Reason = " "
			case "instruction":
				r.RecoveryInstruction = "retry"
			case "accepted":
				w.Tasks[0].Status = "accepted"
			}
			raw, _ := json.Marshal(w)
			s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
			if _, err := s.planningChange(w.ID, "extend-attempt", r); err == nil {
				t.Fatal("invalid authorization accepted")
			}
			after, _ := s.get(w.ID)
			if !reflect.DeepEqual(w, after) {
				t.Fatal("rejected authorization changed state")
			}
		})
	}
}
