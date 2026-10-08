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

// E3 — independent review of the managed Git candidate. These tests cover the
// two gaps left open by managed_review_test.go: the generic runStructuredProvider
// kill-on-invalidation mechanism was never exercised end-to-end on the managed
// review path, and no test asserted that the commands/dates/exit codes handed
// to the reviewer are the real ones observed by the engine, not placeholders.

// A slow reviewer fixture: signals it has started, then sleeps long enough for
// a concurrent contract mutation to be observed by the poller in
// runStructuredProvider before it would otherwise reply.
const slowManagedReviewerFixture = `#!/usr/bin/env python3
import json, os, sys, time
text=sys.stdin.read()
ctx=json.loads(text.split('\nSWARM_MANAGED_REVIEW_CONTEXT\n',1)[1])
folder=os.path.dirname(__file__)
with open(os.path.join(folder,'calls'),'a') as f: f.write('call\n')
with open(os.path.join(folder,'started'),'w') as f: f.write('1')
time.sleep(2)
reply={'candidate_commit':ctx['candidate_commit'],'tasks':[{'task':t['task'],'reason':'Examen lent','criteria':[{'index':i+1,'verdict':'pass','evidence':t['report'][:80]} for i,_ in enumerate(t['criteria'])]} for t in ctx['tasks']]}
print(json.dumps({'type':'result','result':json.dumps(reply)}))
`

func TestEngineContractRevisionKillsInferenceOnContractMutation(t *testing.T) {
	s, w := managedFixture(t)
	base := w.Planning.Repository.Candidate
	a := managedCompleted(t, s, w, "first", "reviewed\n")

	folder := filepath.Join(s.root, "review-fixture")
	if e := os.WriteFile(filepath.Join(folder, "claude"), []byte(slowManagedReviewerFixture), 0700); e != nil {
		t.Fatal(e)
	}
	startedMarker := filepath.Join(folder, "started")

	start := time.Now()
	done := make(chan error, 1)
	go func() { done <- s.integrateManagedAttempt(a) }()

	deadline := time.Now().Add(2 * time.Second)
	for {
		if _, e := os.Stat(startedMarker); e == nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("reviewer subprocess never signalled its start")
		}
		time.Sleep(10 * time.Millisecond)
	}

	// Mutate the task's reviewed contract (a new criterion, as a reopened scope
	// would add) while the reviewer subprocess is still running on the old one.
	mutated, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	for i := range mutated.Tasks {
		if mutated.Tasks[i].ID == "first" {
			mutated.Tasks[i].Criteria = append(mutated.Tasks[i].Criteria, "critère ajouté pendant l'inférence")
		}
	}
	raw, _ := json.Marshal(mutated)
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, mutated.ID); e != nil {
		t.Fatal(e)
	}

	select {
	case e := <-done:
		if e != nil {
			t.Fatal(e)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("integration did not return after the contract mutated mid-inference")
	}
	if elapsed := time.Since(start); elapsed > 1800*time.Millisecond {
		t.Fatalf("reviewer process was not killed promptly on contract mutation: %s", elapsed)
	}

	final, _ := s.get(w.ID)
	task, _ := final.task("first")
	if task.Status != "blocked" || final.Planning.Repository.Candidate != base {
		t.Fatalf("candidate published despite contract mutated mid-inference: %+v", task)
	}
	// The kill-on-invalidate poller in runStructuredProvider is one layer of
	// defense (it stops the paid call quickly, evidenced by elapsed above being
	// far under the fixture's 2s sleep). saveManagedReview's own freshness
	// re-check at persistence time is the second, independent layer: whatever
	// verdict the reviewer call produced, it is forced "stale" here because the
	// contract on disk no longer matches the one that was reviewed.
	if task.IndependentReview == nil || task.IndependentReview.State == "passed" || !strings.Contains(task.IndependentReview.Reason, "contrat") {
		t.Fatalf("contract mutation mid-inference not reflected as a stale verdict: %+v", task.IndependentReview)
	}
	if managedReviewCalls(t, s) != 1 {
		t.Fatal("interrupted call was repeated automatically instead of requiring an explicit bounded retry")
	}
}

func TestEngineContractRevisionTransmitsRealCommandDatesAndExitCodesToReviewer(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")

	before := time.Now()
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after := time.Now()

	data, e := os.ReadFile(filepath.Join(s.root, "review-fixture/observed.json"))
	if e != nil {
		t.Fatal(e)
	}
	var observed struct {
		Context managedReviewContext `json:"context"`
	}
	if e = json.Unmarshal(data, &observed); e != nil {
		t.Fatal(e)
	}
	if len(observed.Context.Tasks) != 1 || len(observed.Context.Tasks[0].Controls) != 1 {
		t.Fatalf("unexpected task/control shape sent to reviewer: %+v", observed.Context)
	}
	c := observed.Context.Tasks[0].Controls[0]
	if len(c.Command) != 3 || c.Command[0] != "git" || c.Command[1] != "diff" || c.Command[2] != "--exit-code" {
		t.Fatalf("command transmitted to reviewer does not match the real control: %+v", c.Command)
	}
	if !c.Executed || !c.Passed || c.ExitCode != 0 {
		t.Fatalf("execution outcome transmitted to reviewer is not the real one: %+v", c)
	}
	started, e1 := time.Parse(time.RFC3339Nano, c.Started)
	finished, e2 := time.Parse(time.RFC3339Nano, c.Finished)
	if e1 != nil || e2 != nil {
		t.Fatalf("dates transmitted to reviewer are not real timestamps: started=%q finished=%q", c.Started, c.Finished)
	}
	if started.Before(before.Add(-time.Second)) || finished.After(after.Add(time.Second)) || finished.Before(started) {
		t.Fatalf("dates transmitted to reviewer fall outside the real execution window: started=%s finished=%s window=[%s,%s]", started, finished, before, after)
	}
}

func TestEngineContractRevisionRejectsReceiptChangedDuringInference(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	base := w.Planning.Repository.Candidate
	folder := filepath.Join(s.root, "review-fixture")
	if err := os.WriteFile(filepath.Join(folder, "claude"), []byte(slowManagedReviewerFixture), 0700); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { done <- s.integrateManagedAttempt(a) }()
	deadline := time.Now().Add(3 * time.Second)
	for {
		if _, err := os.Stat(filepath.Join(folder, "started")); err == nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("review did not start")
		}
		time.Sleep(10 * time.Millisecond)
	}
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	if err := os.WriteFile(filepath.Join(s.root, task.IndependentReview.Receipt), []byte("altered receipt"), 0600); err != nil {
		t.Fatal(err)
	}
	select {
	case <-done:
	case <-time.After(4 * time.Second):
		t.Fatal("review did not stop")
	}
	after, _ := s.get(w.ID)
	task, _ = after.task(a.TaskID)
	if task.Status == "accepted" || after.Planning.Repository.Candidate != base || task.IndependentReview.State == "passed" {
		t.Fatal("altered receipt published", task.Status, task.IndependentReview.State)
	}
	if managedReviewCalls(t, s) != 1 {
		t.Fatal("review repeated")
	}
}
