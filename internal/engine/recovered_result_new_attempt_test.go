//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestRecoveredResultNewAttemptPreservesPreviousRepair(t *testing.T) {
	s, w, previous := managedRepairedRefusal(t)
	priorTask, _ := w.task(previous.TaskID)
	priorRepair := *priorTask.RecoveredResult
	priorReview := *priorTask.IndependentReview
	priorReceipt, err := os.ReadFile(filepath.Join(s.root, priorReview.Receipt))
	if err != nil {
		t.Fatal(err)
	}
	bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
	priorOriginal := gitTest(t, bare, "rev-parse", "refs/swarm/attempts/"+previous.ID)
	a, _, err := s.prepare(w.ID, Launch{Schema: 1, EventID: "second-interrupted-repair", Revision: w.Revision, TaskID: previous.TaskID, Provider: "managed-review-fixture", Role: "worker", Instruction: "Correct the rejected result", Timeout: 60})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.finishAgent(a, "interrupted", "Explicitly stopped fixture", nil); err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	if err = os.MkdirAll(filepath.Join(a.CWD, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("External correction of second interrupted attempt; original first repair retained."), 0600); err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(a.CWD, "value.txt"), []byte("Second repaired value\n"), 0600); err != nil {
		t.Fatal(err)
	}
	body, _ := json.Marshal(completeDelivery(task, a))
	if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), body, 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	r := PlanningRequest{Schema: 1, EventID: "repair-current-second-attempt", Revision: w.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ConfirmRecovery: true, ResultTree: gitTest(t, a.CWD, "write-tree"), Reason: "Explicit external correction of the current stopped attempt; preserve every old repair and review"}
	managedReviewMode(t, s, "pass")
	after, err := s.planningChange(w.ID, "submit-recovered-result", r)
	if err != nil {
		t.Fatal("a retained repair of a previous attempt must not block the current stopped attempt", err)
	}
	current, _ := after.task(a.TaskID)
	if current.Status != "accepted" || !s.acceptedFresh(&after, current, map[string]bool{}) || current.RecoveredResult.Agent != a.ID || current.RecoveredResult.Attempt != a.Attempt || current.IndependentReview.ID == priorReview.ID || current.IndependentReview.Attempt != a.Attempt {
		t.Fatal("current candidate was not freshly checked and reviewed", current.Status, current.Blocker)
	}
	if len(current.Attempts) != 2 || current.Attempts[1].Status != "interrupted" || current.PlanMaxAttempts != task.PlanMaxAttempts || managedReviewCalls(t, s) != 3 {
		t.Fatal("attempts, allowance or review accounting changed")
	}
	if gitTest(t, bare, "rev-parse", "refs/swarm/attempts/"+previous.ID) != priorOriginal || gitTest(t, bare, "rev-parse", "refs/swarm/repairs/"+priorRepair.ProofKey) != priorRepair.Result {
		t.Fatal("historical Git references changed")
	}
	preserved, err := os.ReadFile(filepath.Join(s.root, priorReview.Receipt))
	if err != nil || string(preserved) != string(priorReceipt) {
		t.Fatal("previous review receipt changed", err)
	}
	var request []byte
	if err = s.db.QueryRow("SELECT request FROM events WHERE id=? AND work_id=?", r.EventID, w.ID).Scan(&request); err != nil {
		t.Fatal(err)
	}
	var history struct {
		Repair RecoveredResult   `json:"previous_repair"`
		Review IndependentReview `json:"previous_review"`
	}
	if err = json.Unmarshal(request, &history); err != nil || history.Repair.Event != priorRepair.Event || history.Repair.Result != priorRepair.Result || history.Review.ID != priorReview.ID {
		t.Fatal("previous repair/review attribution missing from durable event", err)
	}
	if _, err = s.planningChange(w.ID, "submit-recovered-result", r); err != nil || managedReviewCalls(t, s) != 3 {
		t.Fatal("replaying current submission repeated review", err)
	}
	r.EventID = "duplicate-current-repair"
	r.Revision = after.Revision
	if _, err = s.planningChange(w.ID, "submit-recovered-result", r); err == nil || managedReviewCalls(t, s) != 3 {
		t.Fatal("duplicate current-attempt repair must still be refused")
	}
}
