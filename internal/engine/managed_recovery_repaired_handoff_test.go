//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func managedRepairedRefusal(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "initial work\n")
	managedReviewMode(t, s, "fail")
	if err := s.finishAgent(a, "completed", "", nil); err != nil {
		t.Fatal(err)
	}
	a, _ = s.agent(a.ID)
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("Externally corrected proof, retained original execution"), 0600); err != nil {
		t.Fatal(err)
	}
	body, _ := json.Marshal(completeDelivery(task, a))
	if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), body, 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	req := PlanningRequest{Schema: 1, EventID: "repair-before-worker-retry", Revision: w.Revision, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, ConfirmRecovery: true, ReviewID: task.IndependentReview.ID, ResultTree: gitTest(t, a.CWD, "write-tree"), Reason: "Repair evidence before an independent review; preserve the original producer"}
	w, err := s.planningChange(w.ID, "revise-recovered-result", req)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = w.task(a.TaskID)
	if task.IndependentReview.State != "changes_requested" || task.RecoveredResult.ProofKey == "" {
		t.Fatal("fixture did not create a repaired, rejected result")
	}
	w, decision := planningClaim(t, s, w, "root")
	decision.Operations = []PlanningOperation{{Kind: "retry", ID: a.TaskID, Next: "Correct the freshly rejected repair in a new owned worker attempt"}}
	w, err = s.planningChange(w.ID, "decide", decision)
	if err != nil {
		t.Fatal(err)
	}
	return s, w, a
}

func TestManagedRecoveryHandoffAfterExternalRepair(t *testing.T) {
	s, w, previous := managedRepairedRefusal(t)
	task, _ := w.task(previous.TaskID)
	bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
	original := gitTest(t, bare, "rev-parse", "refs/swarm/attempts/"+previous.ID)
	if original == task.RecoveredResult.Result {
		t.Fatal("fixture must retain distinct original and repaired results")
	}
	r := Launch{Schema: 1, EventID: "worker-after-repair", Revision: w.Revision, TaskID: previous.TaskID, Provider: "managed-review-fixture", Role: "worker", Instruction: "Correct only the attributed repaired result", Timeout: 60}
	a, created, err := s.prepare(w.ID, r)
	if err != nil || !created {
		t.Fatal("repaired-result handoff refused", created, err)
	}
	raw, err := os.ReadFile(filepath.Join(a.CWD, managedRecoveryFile))
	if err != nil {
		t.Fatal(err)
	}
	var packet ManagedRecoveryHandoff
	if err = json.Unmarshal(raw, &packet); err != nil {
		t.Fatal(err)
	}
	if packet.Result != task.RecoveredResult.Result || packet.PreviousAttempt != previous.Attempt || packet.Review.ID != task.IndependentReview.ID || gitTest(t, a.CWD, "rev-parse", managedRecoveryRef) != packet.Result {
		t.Fatal("wrong repaired-result attribution", packet)
	}
	if got := gitTest(t, a.CWD, "show", managedRecoveryRef+":docs/first.md"); got != "Externally corrected proof, retained original execution" {
		t.Fatal("original rather than repaired work was supplied", got)
	}
	after, _ := s.get(w.ID)
	if after.Planning.Repository.Candidate != w.Planning.Repository.Candidate || gitTest(t, a.CWD, "rev-parse", "HEAD") != w.Planning.Repository.Candidate || len(after.Tasks[0].Attempts) != 2 || managedReviewCalls(t, s) != 2 || gitTest(t, bare, "rev-parse", "refs/swarm/attempts/"+previous.ID) != original {
		t.Fatal("handoff changed canonical candidate, historical result or consumed review calls")
	}
}

func TestManagedRecoveryHandoffRejectsAlteredRepair(t *testing.T) {
	for _, change := range []string{"repair-ref", "agent", "attempt", "result", "process", "tree", "proof-key", "base", "receipt"} {
		t.Run(change, func(t *testing.T) {
			s, w, previous := managedRepairedRefusal(t)
			task, _ := w.task(previous.TaskID)
			bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
			switch change {
			case "repair-ref":
				gitTest(t, bare, "update-ref", "refs/swarm/repairs/"+task.RecoveredResult.ProofKey, w.Planning.Repository.Candidate)
			case "agent":
				task.RecoveredResult.Agent = "another-agent"
			case "attempt":
				task.RecoveredResult.Attempt = "another-attempt"
			case "result":
				task.RecoveredResult.Result = w.Planning.Repository.Candidate
			case "process":
				task.RecoveredResult.ProcessStatus = "interrupted"
			case "tree":
				task.RecoveredResult.Tree = w.Planning.Repository.Candidate
			case "proof-key":
				task.RecoveredResult.ProofKey = "another-repair"
			case "base":
				if _, err := s.db.Exec("UPDATE managed_attempts SET base_commit=? WHERE agent_id=?", "absent-base", previous.ID); err != nil {
					t.Fatal(err)
				}
			case "receipt":
				if err := os.WriteFile(filepath.Join(s.root, task.IndependentReview.Receipt), []byte(`{}`), 0600); err != nil {
					t.Fatal(err)
				}
			}
			if _, err := s.managedRecoveryHandoff(w, task); err == nil {
				t.Fatal("altered repair accepted")
			}
			after, _ := s.get(w.ID)
			if after.Revision != w.Revision || len(after.Tasks[0].Attempts) != 1 || managedReviewCalls(t, s) != 2 {
				t.Fatal("invalid handoff changed revision, attempts or calls")
			}
		})
	}
}
