//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

func TestInstructionRevisionPublicDecision(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("future"), {Kind: "revise-instructions", ID: "future", Next: "Conserver le diagnostic et vérifier la réception avant toute reprise."}}
	got, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(got.Tasks[0].Next, "diagnostic") || !strings.Contains(got.Tasks[0].Next, "Vérifier les fichiers") {
		t.Fatal("lost instruction", got.Tasks[0].Next)
	}
	if len(got.Tasks[0].Attempts) != 0 || got.Tasks[0].Status != "todo" {
		t.Fatal("instruction revision launched a task")
	}
}

func revisedInstructionFixture(t *testing.T) (*Store, Work, PlanningRequest) {
	t.Helper()
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("future"), {Kind: "revise-instructions", ID: "future", Next: "Conserver le diagnostic et vérifier la réception avant toute reprise."}}
	got, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	return s, got, r
}
func TestInstructionRevisionRejectPersistenceReplayAndCounters(t *testing.T) {
	s, w, decision := revisedInstructionFixture(t)
	v := w.Tasks[0].InstructionRevisions[0]
	if v.State != "applied" || v.Holder != decision.Holder || v.Decision != decision.EventID || len(v.Inputs) != len(decision.Inputs) || v.Inputs[0] != decision.Inputs[0] || v.BeforeSHA != hash([]byte("Vérifier les fichiers")) {
		t.Fatal("lost provenance", v)
	}
	r := PlanningRequest{Schema: 1, EventID: "reject-instructions-once", Revision: w.Revision, Task: "future", InstructionRevisionID: v.ID, Reason: "Cet ajout ne convient pas au prochain travail"}
	got, err := s.planningChange(w.ID, "reject-instructions", r)
	if err != nil {
		t.Fatal(err)
	}
	if got.Tasks[0].Next != "Vérifier les fichiers" || got.Tasks[0].InstructionRevisions[0].State != "rejected" || got.Planning.Decisions != w.Planning.Decisions || got.Planning.Activations != w.Planning.Activations || reviewContract(&got.Tasks[0]) != reviewContract(&w.Tasks[0]) {
		t.Fatal("rejection changed a contract or lost history", got.Tasks[0])
	}
	if got.Planning.Inbox[len(got.Planning.Inbox)-1].Kind != "instruction_rejected" || got.Planning.Inbox[len(got.Planning.Inbox)-1].Scope != "root" {
		t.Fatal("rejection not routed to owner")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	replay, err := reopened.planningChange(w.ID, "reject-instructions", r)
	if err != nil || replay.Revision != got.Revision || len(replay.Tasks[0].InstructionRevisions) != 1 {
		t.Fatal("replay", err)
	}
	r.EventID = "another-reject"
	r.Revision = got.Revision
	if _, err = reopened.planningChange(w.ID, "reject-instructions", r); err == nil {
		t.Fatal("rejected twice")
	}
}
func TestInstructionRevisionRollbackRightsAndStaleInput(t *testing.T) {
	for _, bad := range []string{"foreign-scope", "contract", "not-received", "duplicate", "active"} {
		t.Run(bad, func(t *testing.T) {
			s, w := planningFixture(t)
			w, r := planningClaim(t, s, w, "root")
			op := PlanningOperation{Kind: "revise-instructions", ID: "future", Next: "Vérifier le diagnostic avant toute reprise, puis préserver la preuve."}
			r.Operations = []PlanningOperation{planningTask("future"), op}
			switch bad {
			case "foreign-scope":
				r.Scope = "other"
			case "contract":
				r.Operations[1].Criteria = []string{"changed"}
			case "not-received":
				r.Inputs = []string{"invented-event"}
			case "duplicate":
				r.Operations = append(r.Operations, op)
			case "active":
				a := Agent{ID: "active-fixture", WorkID: w.ID, TaskID: "future", Status: "queued"}
				b, _ := json.Marshal(a)
				_, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, b, []byte(`{}`))
				if err != nil {
					t.Fatal(err)
				}
			}
			if _, err := s.planningChange(w.ID, "decide", r); err == nil {
				t.Fatal("invalid revision accepted")
			}
			got, _ := s.get(w.ID)
			if got.Revision != w.Revision || len(got.Tasks) != 0 || got.Planning.Decisions != 0 || got.Planning.Inbox[0].Decision != "" {
				t.Fatal("partial decision escaped")
			}
		})
	}
}
func TestInstructionRevisionLaunchFreezesAndRejectRace(t *testing.T) {
	for _, race := range []bool{false, true} {
		t.Run(fmt.Sprint(race), func(t *testing.T) {
			s, w, _ := revisedInstructionFixture(t)
			_, launch := setupAgent(t, s)
			w, err := s.mutate(w.ID, "fixture-human-review", newID("fixture-"), w.Revision, []byte(`{}`), func(w *Work) error {
				directory := filepath.Join(s.root, "fixture-reviewer")
				if err := os.MkdirAll(directory, 0700); err != nil {
					return err
				}
				command := filepath.Join(directory, "claude")
				if err := os.WriteFile(command, []byte("#!/bin/sh\nexit 0\n"), 0700); err != nil {
					return err
				}
				providers, err := s.providers()
				if err != nil {
					return err
				}
				providers.Providers["review-fixture"] = Provider{Command: command}
				body, _ := json.Marshal(providers)
				if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), body, 0600); err != nil {
					return err
				}
				config, err := s.reviewerConfig("review-fixture", "", 10)
				if err != nil {
					return err
				}
				w.Planning.Reviewer = config
				return nil
			})
			if err != nil {
				t.Fatal(err)
			}
			launch.TaskID = "future"
			launch.Revision = w.Revision
			launch.EventID = newID("launch-")
			v := w.Tasks[0].InstructionRevisions[0]
			reject := PlanningRequest{Schema: 1, EventID: newID("reject-"), Revision: w.Revision, Task: "future", InstructionRevisionID: v.ID, Reason: "Rejet explicite avant lancement"}
			var a Agent
			var launchErr, rejectErr error
			if race {
				var wg sync.WaitGroup
				wg.Add(2)
				go func() { defer wg.Done(); a, _, launchErr = s.prepare(w.ID, launch) }()
				go func() { defer wg.Done(); _, rejectErr = s.planningChange(w.ID, "reject-instructions", reject) }()
				wg.Wait()
				if (launchErr == nil) == (rejectErr == nil) {
					t.Fatalf("launch=%v reject=%v", launchErr, rejectErr)
				}
			} else {
				a, _, launchErr = s.prepare(w.ID, launch)
				if launchErr != nil {
					t.Fatal(launchErr)
				}
				current, _ := s.get(w.ID)
				reject.Revision = current.Revision
				_, rejectErr = s.planningChange(w.ID, "reject-instructions", reject)
				if rejectErr == nil {
					t.Fatal("instruction changed after reservation")
				}
			}
			got, _ := s.get(w.ID)
			if launchErr == nil {
				if !strings.Contains(a.Prompt, v.Addition) || !strings.Contains(a.Prompt, "Vérifier les fichiers") || got.Tasks[0].InstructionRevisions[0].UsedByAttempt != a.Attempt {
					t.Fatal("worker missed frozen additions")
				}
			} else if got.Tasks[0].Next != "Vérifier les fichiers" {
				t.Fatal("rejected addition survived")
			}
		})
	}
}
