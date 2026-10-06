package main

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func blockingB04Control(t *testing.T, name string) (policy *ValidationPolicy, started, release string) {
	t.Helper()
	root := t.TempDir()
	started, release = filepath.Join(root, name+".started"), filepath.Join(root, name+".release")
	body := "import os,time\nopen(" + strconvQuote(started) + ", 'w').close()\nwhile not os.path.exists(" + strconvQuote(release) + "): time.sleep(0.01)\n"
	return automaticPolicy("python3", "-c", body), started, release
}

func strconvQuote(value string) string {
	quoted := "'"
	for _, r := range value {
		if r == '\'' {
			quoted += "\\'"
		} else {
			quoted += string(r)
		}
	}
	return quoted + "'"
}

func waitB04Control(t *testing.T, marker string) {
	t.Helper()
	deadline := time.Now().Add(3 * time.Second)
	for time.Now().Before(deadline) {
		if _, err := os.Stat(marker); err == nil {
			return
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatal("long control did not start")
}

func TestGraphDraftB04RelevantChange(t *testing.T) {
	before := Work{Planning: &PlanningState{Repository: &ManagedRepository{Candidate: "candidate-before"}}, Tasks: []Task{{ID: "source", Status: "accepted"}, {ID: "checked", Status: "accepted", Depends: []string{"source"}}}}
	after := before
	after.Tasks = append([]Task(nil), before.Tasks...)
	after.Planning = &PlanningState{Repository: &ManagedRepository{Candidate: "candidate-after"}}
	p := graphDraftProofProjection(&before, &after, []string{"checked"})
	if !p.Relevant || p.Kind != "relevant" || p.BeforeDigest == p.AfterDigest {
		t.Fatalf("candidate-relevant change preserved stale proof: %+v", p)
	}

	t.Run("long-control-receipt-becomes-stale", func(t *testing.T) {
		policy, started, release := blockingB04Control(t, "relevant")
		s, w, agent, _ := automaticValidationFixture(t, policy, true)
		current, _ := s.get(w.ID)
		done := make(chan struct{})
		go func() { s.conduct(agent, "completed"); close(done) }()
		waitB04Control(t, started)
		current, _ = s.get(w.ID)
		draft, preview := createGraphDraftTest(t, s, current, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t3", Dependent: "t1"})
		applyGraphDraftTest(t, s, current, draft, preview, "relevant-during-control")
		if err := os.WriteFile(release, []byte("release"), 0600); err != nil {
			t.Fatal(err)
		}
		<-done
		got, _ := s.get(w.ID)
		task, _ := got.task("t1")
		if task.Status == "accepted" || task.AutoValidation != nil || task.Gate != nil {
			t.Fatalf("stale long control accepted changed inputs: %+v", task)
		}
		receipts, _ := filepath.Glob(filepath.Join(s.root, ".swarm", "validation", w.ID, "t1", "*.json"))
		if len(receipts) == 0 {
			t.Fatal("stale receipt was not preserved")
		}
	})

	t.Run("attached-receipt-is-kept-but-stale", func(t *testing.T) {
		s := storeTest(t)
		w := graphDraftWork(t, s)
		w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
		w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
		w = gateTest(t, s, w, gateDocument(t, s, "t1"))
		w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
		beforeGate := w.Tasks[0].Gate
		draft, preview := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t3", Dependent: "t1"})
		applyGraphDraftTest(t, s, w, draft, preview, "stale-attached-receipt")
		got, _ := s.get(w.ID)
		task, _ := got.task("t1")
		if task.Gate == nil || task.Gate.At != beforeGate.At || task.EvidenceStaleReason == "" || s.validGate(task) || s.acceptedFresh(&got, task, map[string]bool{}) {
			t.Fatalf("attached receipt was lost or remained current: %+v", task)
		}
	})
}

func TestGraphDraftB04AdministrativeChange(t *testing.T) {
	before := Work{Revision: 7, Tasks: []Task{{ID: "checked", Title: "Avant", Owner: "worker", Next: "attendre", Deliverable: "docs/result.md", Criteria: []string{"tests"}}}}
	after := before
	after.Revision++
	after.Tasks = append([]Task(nil), before.Tasks...)
	after.Tasks[0].Title, after.Tasks[0].Owner, after.Tasks[0].Next = "Nom affiché", "autre acteur", "consulter le journal"
	p := graphDraftProofProjection(&before, &after, []string{"checked"})
	if p.Relevant || p.Kind != "administrative" || p.BeforeDigest != p.AfterDigest {
		t.Fatalf("presentation-only change invalidated unchanged inputs: %+v", p)
	}

	t.Run("long-control-survives-administrative-revision", func(t *testing.T) {
		policy, started, release := blockingB04Control(t, "administrative")
		s, w, agent, _ := automaticValidationFixture(t, policy, false)
		current, _ := s.get(w.ID)
		done := make(chan struct{})
		go func() { s.conduct(agent, "completed"); close(done) }()
		waitB04Control(t, started)
		current, _ = s.get(w.ID)
		applyTest(t, s, current, "checkpoint", Request{Summary: "Présentation opérateur actualisée", Next: "Consulter le journal"})
		if err := os.WriteFile(release, []byte("release"), 0600); err != nil {
			t.Fatal(err)
		}
		<-done
		got, _ := s.get(w.ID)
		task, _ := got.task("t1")
		if task.Status != "accepted" || task.AutoValidation == nil || !s.validGate(task) {
			t.Fatalf("administrative revision discarded unchanged proof inputs: %+v", task)
		}
	})
}

func TestGraphDraftB04UnknownCost(t *testing.T) {
	w := Work{Tasks: []Task{{ID: "checked", Title: "Contrôle", Status: "accepted", Attempts: []Attempt{{ID: "attempt-1"}, {ID: "attempt-2"}}}}}
	agents := []Agent{{ID: "agent-2", TaskID: "checked", Attempt: "attempt-2", Role: "worker", Status: "completed", Activity: "Processus terminé"}, {ID: "agent-1", TaskID: "checked", Attempt: "attempt-1", Role: "worker", Status: "failed"}}
	p := projectMissionAttempt(&w.Tasks[0], agents)
	if p.AgentID != "agent-2" || p.AttemptID != "attempt-2" || p.ProcessState != "completed" || p.Validation != "accepted" || p.CostState != "unknown" {
		t.Fatalf("attempt identity, process, validation or unknown cost conflated: %+v", p)
	}
	ledgers := attemptLedgers(w, agents)
	if len(ledgers) != 2 || ledgers[0].Attempt != "attempt-1" || ledgers[1].Attempt != "attempt-2" || ledgers[1].Role != "worker" || !ledgers[0].MissingUsage || !ledgers[1].MissingUsage || ledgers[0].Cost.Reported != 0 || ledgers[0].Cost.Silent != 1 {
		t.Fatalf("unknown provider consumption rendered as measured zero: %+v", ledgers)
	}
}
