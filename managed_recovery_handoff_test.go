//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func managedRejectedRetry(t *testing.T) (*Store, Work, Agent, Launch) {
	t.Helper()
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "useful unfinished work\n")
	managedReviewMode(t, s, "fail")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[0].IndependentReview == nil || w.Tasks[0].IndependentReview.State != "changes_requested" {
		t.Fatal("fixture did not obtain a real refusal")
	}
	w, request := planningClaim(t, s, w, "root")
	request.Operations = []PlanningOperation{{Kind: "retry", ID: "first", Next: "Corriger le point refusé puis refaire les contrôles de cette tâche"}}
	var e error
	w, e = s.planningChange(w.ID, "decide", request)
	if e != nil {
		t.Fatal(e)
	}
	r := Launch{Schema: 1, EventID: "corrected-first", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Corriger uniquement le résultat de cette tâche", Timeout: 60}
	return s, w, a, r
}

func TestManagedRecoveryHandoffSuppliesRejectedWorkWithoutPublishingIt(t *testing.T) {
	s, w, previous, r := managedRejectedRetry(t)
	old, e := s.managedAttempt(previous.ID)
	if e != nil {
		t.Fatal(e)
	}
	// The packet must use immutable engine evidence, never this mutable copy.
	os.WriteFile(filepath.Join(previous.CWD, "value.txt"), []byte("changed after review\n"), 0600)
	a, created, e := s.prepare(w.ID, r)
	if e != nil || !created {
		t.Fatal("corrective launch", created, e)
	}
	data, e := os.ReadFile(filepath.Join(a.CWD, ".git", "swarm-recovery.json"))
	if e != nil {
		t.Fatal("the corrective worker has no recovery packet", e)
	}
	var packet map[string]any
	if e = json.Unmarshal(data, &packet); e != nil {
		t.Fatal(e)
	}
	if packet["previous_agent"] != previous.ID || packet["previous_attempt"] != previous.Attempt || packet["result_commit"] != old.Result || packet["task"] != "first" {
		t.Fatal("lost recovery attribution", packet)
	}
	if !strings.Contains(string(data), w.Tasks[0].IndependentReview.ID) || !strings.Contains(a.Prompt, "swarm-recovery.json") || !strings.Contains(a.Prompt, hash(data)) || !strings.Contains(a.Prompt, w.Tasks[0].Next) {
		t.Fatal("worker was not told what to correct or where to find its evidence")
	}
	if got := gitTest(t, a.CWD, "show", "refs/swarm/recovery/previous:value.txt"); got != "useful unfinished work" {
		t.Fatal("lost or mutable result", got)
	}
	if got := gitTest(t, a.CWD, "show", "refs/swarm/recovery/previous:docs/first.md"); got != "Rapport nouveau first" {
		t.Fatal("lost handoff", got)
	}
	if got := gitTest(t, a.CWD, "status", "--porcelain"); got != "" {
		t.Fatal("packet polluted candidate", got)
	}
	if got := gitTest(t, a.CWD, "rev-parse", "HEAD"); got != w.Planning.Repository.Candidate {
		t.Fatal("rejected result was adopted as base", got)
	}
	after, _ := s.get(w.ID)
	if after.Planning.Repository.Candidate != w.Planning.Repository.Candidate || after.Tasks[0].Status == "accepted" || len(after.Tasks[0].Attempts) != 2 || after.Tasks[0].PlanMaxAttempts != 2 || managedReviewCalls(t, s) != 1 {
		t.Fatal("handoff changed acceptance, budget or reviewer calls")
	}
}

func TestManagedRecoveryHandoffCorrectedResultNeedsNewChecksAndReview(t *testing.T) {
	s, w, previous, r := managedRejectedRetry(t)
	a, created, e := s.prepare(w.ID, r)
	if e != nil || !created {
		t.Fatal(e)
	}
	// Worker fixture reads the local, attributed result and corrects it. The
	// real integration/check/review pipeline below is not mocked.
	value := gitTest(t, a.CWD, "show", managedRecoveryRef+":value.txt")
	if e = os.WriteFile(filepath.Join(a.CWD, "value.txt"), []byte(value+"; corrected\n"), 0600); e != nil {
		t.Fatal(e)
	}
	if e = os.MkdirAll(filepath.Join(a.CWD, "docs"), 0700); e != nil {
		t.Fatal(e)
	}
	os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("Correction depuis le résultat précédent, avec nouveaux contrôles."), 0600)
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	delivery, _ := json.Marshal(completeDelivery(task, a))
	os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), delivery, 0600)
	a.Status = "completed"
	if e = s.saveAgent(a); e != nil {
		t.Fatal(e)
	}
	managedReviewMode(t, s, "pass")
	if e = s.settleAgentTask(a); e != nil {
		t.Fatal(e)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	task, _ = after.task(a.TaskID)
	if task.Status != "accepted" || !s.acceptedFresh(&after, task, map[string]bool{}) || task.IndependentReview.Attempt != a.Attempt || task.IndependentReview.CandidateSHA != task.AutoValidation.CandidateSHA || managedReviewCalls(t, s) != 2 {
		t.Fatal("corrected result did not get fresh checks and review", task.Status, task.Blocker, managedReviewCalls(t, s))
	}
	if len(task.Attempts) != 2 || task.Attempts[0].ID != previous.Attempt || task.PlanMaxAttempts != 2 {
		t.Fatal("history or limits changed")
	}
	if _, e = os.Stat(filepath.Join(previous.CWD, "docs/first.md")); e != nil {
		t.Fatal("old copy removed")
	}
	bare := filepath.Join(after.Planning.Repository.Storage, "repository.git")
	if got := gitTest(t, bare, "show", after.Planning.Repository.Candidate+":value.txt"); got != "useful unfinished work; corrected" {
		t.Fatal("wrong published content", got)
	}
}

func TestManagedRecoveryHandoffResumeAndTampering(t *testing.T) {
	for _, change := range []string{"none", "packet", "packet-link", "local-ref", "source-ref", "receipt"} {
		t.Run(change, func(t *testing.T) {
			s, w, previous, r := managedRejectedRetry(t)
			path, e := s.ensureManagedAttempt(w, r)
			if e != nil {
				t.Fatal(e)
			}
			packet := filepath.Join(path, managedRecoveryFile)
			switch change {
			case "packet":
				os.WriteFile(packet, []byte(`{"fake":true}`), 0600)
			case "packet-link":
				os.Remove(packet)
				os.Symlink(filepath.Join(previous.CWD, "value.txt"), packet)
			case "local-ref":
				gitTest(t, path, "update-ref", managedRecoveryRef, w.Planning.Repository.Candidate)
			case "source-ref":
				gitTest(t, filepath.Join(w.Planning.Repository.Storage, "repository.git"), "update-ref", "refs/swarm/attempts/"+previous.ID, w.Planning.Repository.Candidate)
			case "receipt":
				os.WriteFile(filepath.Join(s.root, w.Tasks[0].IndependentReview.Receipt), []byte(`{"changed":true}`), 0600)
			}
			reopened, e := openStore(s.root, false)
			if e != nil {
				t.Fatal(e)
			}
			defer reopened.db.Close()
			a, created, e := reopened.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
			if change == "none" {
				if e != nil || !created || a.ID != r.EventID {
					t.Fatal("durable recovery lost", e)
				}
				_, duplicate, e := reopened.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
				if e != nil || duplicate {
					t.Fatal("duplicate recovery", e)
				}
			} else {
				if e == nil || created {
					t.Fatal("tampered evidence allowed a launch")
				}
				after, _ := reopened.get(w.ID)
				if len(after.Tasks[0].Attempts) != 1 || after.Revision != w.Revision {
					t.Fatal("rejected packet consumed an attempt")
				}
			}
			if _, e := os.Stat(path); e != nil {
				t.Fatal("retained copy lost")
			}
			if managedReviewCalls(t, s) != 1 {
				t.Fatal("recovery called reviewer")
			}
		})
	}
}

func TestManagedRecoveryHandoffMissingResultIsExplicit(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "uncommitted partial work\n")
	if e := s.managedFailure(a, "Livraison interrompue avant enregistrement du résultat"); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	h, e := s.managedRecoveryHandoff(w, &w.Tasks[0])
	if e != nil || h.Result != "" || h.Limit == "" {
		t.Fatal("invented recovery snapshot", h, e)
	}
	if h.PreviousAgent != a.ID || !strings.Contains(h.Failure, "interrompue") {
		t.Fatal("failure identity lost")
	}
}
