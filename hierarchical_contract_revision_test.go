//go:build linux

package main

import (
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func contractRevisionFixture(t *testing.T) (*Store, Work, Request) {
	t.Helper()
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("repair")}
	w = planningDo(t, s, w, "decide", r)
	w = applyTest(t, s, w, "task.update", Request{ID: "repair", Status: "blocked", Blocker: "circular final criterion"})
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task("repair")
	return s, w, Request{Schema: 1, EventID: newID("contract-"), Revision: w.Revision, ID: task.ID, Criteria: []string{"Functional review precedes acceptance; public closure remains a final obligation"}, ConfirmContractRevision: true, ExpectedContract: reviewContract(task), ContractRevisionReason: "Explicit human authorization to repair circular phase order; preserve final obligations"}
}

func TestHierarchicalContractRevisionPublicCLIReplayAndHistory(t *testing.T) {
	s, w, r := contractRevisionFixture(t)
	oldTask := w.Tasks[0]
	oldPlanning := *w.Planning
	input := filepath.Join(s.root, "revision.json")
	raw, _ := json.Marshal(r)
	if err := os.WriteFile(input, raw, 0600); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 2; i++ {
		if code := run([]string{"--root", s.root, "--json", "task", "update", w.ID, "--input", input}, io.Discard, io.Discard); code != 0 {
			t.Fatalf("CLI amendment failed: %d", code)
		}
	}
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := got.task(r.ID)
	if got.Revision != w.Revision+1 || task.Status != "blocked" || !reflect.DeepEqual(task.Criteria, r.Criteria) {
		t.Fatalf("bad amendment: %+v", task)
	}
	if task.ScopeID != oldTask.ScopeID || !reflect.DeepEqual(task.Requirements, oldTask.Requirements) || !reflect.DeepEqual(task.Attempts, oldTask.Attempts) || task.PlanMaxAttempts != oldTask.PlanMaxAttempts || task.PlanToolLimit != oldTask.PlanToolLimit || !reflect.DeepEqual(got.Criteria, w.Criteria) || !reflect.DeepEqual(*got.Planning, oldPlanning) {
		t.Fatal("amendment altered ownership, global requirements or budgets")
	}
	if task.Gate != nil || task.AutoValidation != nil {
		t.Fatal("old acceptance evidence survived amendment")
	}
	events, err := s.events(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	var audit Request
	if err = json.Unmarshal(events[len(events)-1].Payload, &audit); err != nil {
		t.Fatal(err)
	}
	if audit.ExpectedContract != r.ExpectedContract || audit.ContractRevisionReason != r.ContractRevisionReason || !audit.ConfirmContractRevision {
		t.Fatal("explicit amendment audit missing")
	}
}

func TestHierarchicalContractRevisionRejectsUnsafeRequestsAtomically(t *testing.T) {
	for _, name := range []string{"unconfirmed", "old-contract", "budget", "dependencies", "transition", "criteria-removal", "active-agent", "unpaused"} {
		t.Run(name, func(t *testing.T) {
			s, w, r := contractRevisionFixture(t)
			switch name {
			case "unconfirmed":
				r.ConfirmContractRevision = false
			case "old-contract":
				r.ExpectedContract = "stale"
			case "budget":
				r.MaxAttempts = 3
			case "dependencies":
				r.Depends = []string{}
			case "transition":
				r.Status = "todo"
			case "criteria-removal":
				r.Criteria = []string{}
			case "unpaused":
				if err := s.pause(w.ID, false); err != nil {
					t.Fatal(err)
				}
				w, _ = s.get(w.ID)
				r.Revision = w.Revision
			case "active-agent":
				a := Agent{ID: "revision-active", TaskID: r.ID, Status: "queued"}
				raw, _ := json.Marshal(a)
				if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, raw, []byte("{}")); err != nil {
					t.Fatal(err)
				}
			}
			if _, err := s.executeRequest(w.ID, "task.update", r); err == nil {
				t.Fatal("unsafe amendment accepted")
			}
			got, _ := s.get(w.ID)
			if !reflect.DeepEqual(w, got) {
				t.Fatal("refused amendment mutated work")
			}
		})
	}
}

func TestHierarchicalContractRevisionRejectsActiveReviewAndClosedScope(t *testing.T) {
	_, w, r := contractRevisionFixture(t)
	w.Tasks[0].IndependentReview = &IndependentReview{State: "running"}
	if err := validateHierarchicalContractRevision(&w, r); err == nil {
		t.Fatal("active review accepted")
	}
	w.Tasks[0].IndependentReview = nil
	w.Planning.Scopes[0].State = "closed"
	if err := validateHierarchicalContractRevision(&w, r); err == nil {
		t.Fatal("closed scope amended")
	}
}

func TestHierarchicalContractRevisionArchivesRefusalAndInvalidatesEvidence(t *testing.T) {
	_, w, r := contractRevisionFixture(t)
	task, _ := w.task(r.ID)
	old := IndependentReview{ID: "old-refusal", State: "changes_requested", Reason: "circular closure criterion"}
	task.IndependentReview = &old
	task.Gate = &GateRecord{}
	task.AutoValidation = &AutomaticValidation{}
	if err := validateHierarchicalContractRevision(&w, r); err != nil {
		t.Fatal(err)
	}
	if err := updateTaskDefinition(&w, task, r); err != nil {
		t.Fatal(err)
	}
	if task.Status != "blocked" || task.Gate != nil || task.AutoValidation != nil || task.IndependentReview != nil || len(task.PreviousReviews) != 1 || !reflect.DeepEqual(task.PreviousReviews[0], old) {
		t.Fatal("refusal erased or stale acceptance survived")
	}
}

func TestHierarchicalContractRevisionRefusedResultCanResumeAfterAmendment(t *testing.T) {
	s, w, r := contractRevisionFixture(t)
	w, err := s.mutate(w.ID, "test.fixture", newID("fixture-"), w.Revision, []byte("{}"), func(current *Work) error {
		task, _ := current.task(r.ID)
		current.Planning.Reviewer = &ReviewerConfig{Provider: "fixture", MaxCalls: 40}
		task.Attempts = []Attempt{{ID: "completed-attempt", Status: "completed"}}
		task.IndependentReview = &IndependentReview{ID: "refused", State: "changes_requested", Finished: now(), Contract: reviewContract(task), Attempt: "completed-attempt"}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(r.ID)
	prior := reviewContract(task)
	r.Revision = w.Revision
	r.ExpectedContract = prior
	got, err := s.executeRequest(w.ID, "task.update", r)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = got.task(r.ID)
	if !s.refusedReviewEvidenceChanged(&got, task) {
		t.Fatal("recorded contract amendment cannot resume existing refused result")
	}
	if s.recordedContractRevisionMatches(got.ID, task, "invented-contract") {
		t.Fatal("unrecorded prior contract accepted")
	}
	resumed, err := s.retryIndependentReview(got.ID, PlanningRequest{EventID: newID("retry-"), Revision: got.Revision, Task: r.ID, Reason: "Recorded amendment requires fresh independent review"})
	if err != nil {
		t.Fatal(err)
	}
	resumedTask, _ := resumed.task(r.ID)
	if resumedTask.Status != "submitted" {
		t.Fatal("public recovery did not resubmit result")
	}
	task.PreviousReviews[len(task.PreviousReviews)-1].State = "passed"
	if s.boundReviewEvidenceChanged(&got, task, "passed") {
		t.Fatal("old favorable verdict reused across amended contract")
	}
}
