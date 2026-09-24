//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"
	"testing"
)

func fragmentStoreFixture(t *testing.T) (*Store, Work, Agent, IndependentReview, managedReviewFragmentPlan, managedFragmentJournal) {
	t.Helper()
	s, w, a := unpaidReviewFixture(t)
	prepared, e := s.readManagedPreflightEvidence(w.ID, a.TaskID)
	if e != nil {
		t.Fatal(e)
	}
	p, e := planManagedReviewFragments(prepared.Context, 12, 2)
	if e != nil {
		t.Fatal(e)
	}
	dir := filepath.Join(w.Planning.Repository.Storage, "proofs", a.ID)
	write := func(name string, data []byte) (string, string) {
		t.Helper()
		path := filepath.Join(dir, name)
		if e := atomicWrite(path, data); e != nil {
			t.Fatal(e)
		}
		rel, e := filepath.Rel(s.root, path)
		if e != nil {
			t.Fatal(e)
		}
		return filepath.ToSlash(rel), hash(data)
	}
	context, _ := json.Marshal(prepared.Context)
	contextPath, contextDigest := write("fragment-context.json", context)
	planRaw, _ := json.Marshal(p)
	planPath, planDigest := write("fragment-plan.json", planRaw)
	j := managedFragmentJournal{Version: 1, PlanDigest: planDigest, Attempt: a.Attempt, ProviderDigest: w.Planning.Reviewer.ProviderDigest}
	journalRaw, _ := json.Marshal(j)
	journalPath, journalDigest := write("fragment-journal.json", journalRaw)
	receipt, e := os.ReadFile(filepath.Join(dir, "receipt.json"))
	if e != nil {
		t.Fatal(e)
	}
	receiptPath, receiptDigest := write("receipt.json", receipt)
	reportPath, reportDigest := write("fragment-report.md", []byte(prepared.Context.Tasks[0].Report))
	task, _ := w.task(a.TaskID)
	r := IndependentReview{ID: "fragment-test-review", State: "running", Attempt: a.Attempt, Producer: a.ID, Contract: reviewContract(task), CandidateSHA: p.Candidate, PreviousCandidate: w.Planning.Repository.Candidate, Context: contextPath, ContextDigest: contextDigest, Receipt: receiptPath, ReceiptDigest: receiptDigest, Report: reportPath, Digest: reportDigest, BatchProviderDigest: w.Planning.Reviewer.ProviderDigest, FragmentJournal: &ManagedFragmentJournalAnchor{Plan: planPath, PlanDigest: planDigest, Journal: journalPath, JournalDigest: journalDigest}}
	workflow, prompt, e := agentWorkflow("reviewer")
	if e != nil {
		t.Fatal(e)
	}
	methodRaw, _ := json.Marshal([]any{workflow, prompt})
	r.FragmentJournal.WorkflowDigest = hash(methodRaw)
	modelRaw, _ := json.Marshal(w.Planning.Reviewer.ModelRoute)
	r.FragmentJournal.ModelConfigDigest = hash(modelRaw)
	task.IndependentReview = &r
	raw, _ := json.Marshal(w)
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	return s, w, a, r, p, j
}
func reserveFragment(p managedReviewFragmentPlan, j managedFragmentJournal) managedFragmentJournal {
	raw, _ := json.Marshal(p.Packets[0])
	j.Entries = append(j.Entries, managedFragmentJournalEntry{Packet: 0, PacketDigest: hash(raw), CallID: "fragment-call-one", State: "reserved"})
	return j
}
func TestManagedFragmentStoreAtomicReservation(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	next := reserveFragment(p, j)
	if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "fragment-reserve-event", next); e != nil {
		t.Fatal(e)
	}
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if current.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls+1 {
		t.Fatal("call not charged")
	}
	task, _ := current.task(a.TaskID)
	_, saved, e := s.readFragmentJournalAnchor(*task.IndependentReview)
	if e != nil || len(saved.Entries) != 1 {
		t.Fatal(saved, e)
	}
	if e = s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "another-event", next); e == nil {
		t.Fatal("stale reservation accepted")
	}
	again, _ := s.get(w.ID)
	if again.Planning.Reviewer.Calls != current.Planning.Reviewer.Calls {
		t.Fatal("double charge")
	}
	task.IndependentReview.State = "passed"
	if s.managedReviewFilesIntact(*task.IndependentReview) == nil {
		t.Fatal("fragment-only acceptance")
	}
}
func TestManagedFragmentStoreBudgetRefusalPreservesAnchor(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls
	raw, _ := json.Marshal(w)
	s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID)
	if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "fragment-budget-refusal", reserveFragment(p, j)); e == nil {
		t.Fatal("budget bypass")
	}
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	if current.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || task.IndependentReview.FragmentJournal.JournalDigest != r.FragmentJournal.JournalDigest {
		t.Fatal("failed transaction mutated state")
	}
}

func TestManagedFragmentStorePauseRollsBackReservation(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	if e := s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "paused-reservation", reserveFragment(p, j)); e == nil {
		t.Fatal("paused mission spent a call")
	}
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	if current.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || task.IndependentReview.FragmentJournal.JournalDigest != r.FragmentJournal.JournalDigest {
		t.Fatal("rollback failed")
	}
}

func TestManagedFragmentStoreConcurrentReservationOnce(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	next := reserveFragment(p, j)
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	stores := []*Store{s, other}
	start := make(chan struct{})
	results := make(chan error, 2)
	var wg sync.WaitGroup
	for i, event := range []string{"conductor-one", "conductor-two"} {
		wg.Add(1)
		go func(store *Store, event string) {
			defer wg.Done()
			<-start
			results <- store.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, event, next)
		}(stores[i], event)
	}
	close(start)
	wg.Wait()
	close(results)
	success := 0
	for e := range results {
		if e == nil {
			success++
		}
	}
	current, _ := s.get(w.ID)
	if success != 1 || current.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls+1 {
		t.Fatal("duplicate reservation", success, current.Planning.Reviewer.Calls)
	}
}

func completedFragmentJournal(p managedReviewFragmentPlan, j managedFragmentJournal) managedFragmentJournal {
	packet := p.Packets[0]
	raw, _ := json.Marshal(packet)
	reply := managedFragmentInspection{Candidate: packet.Candidate, ContextDigest: packet.ContextDigest, PacketDigest: hash(raw)}
	for i, a := range packet.Artifacts {
		excerpt := []rune(a.Content)
		if len(excerpt) > 32 {
			excerpt = excerpt[:32]
		}
		reply.Findings = append(reply.Findings, managedFragmentFinding{Artifact: i, Digest: a.Digest, Verdict: "inspected", Reason: "Fixture independent inspection", Evidence: string(excerpt), Needs: []string{}})
	}
	raw, _ = json.Marshal(reply)
	j.Entries = append([]managedFragmentJournalEntry(nil), j.Entries...)
	entry := &j.Entries[len(j.Entries)-1]
	entry.State = "inspected"
	entry.Reply = string(raw)
	entry.ReplyDigest = hash(raw)
	return j
}
func TestManagedFragmentStoreReplySurvivesReopen(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	reserved := reserveFragment(p, j)
	if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "reserve-before-reopen", reserved); e != nil {
		t.Fatal(e)
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	current, _ := reopened.get(w.ID)
	task, _ := current.task(a.TaskID)
	anchor := task.IndependentReview.FragmentJournal.JournalDigest
	if e = reopened.commitFragmentJournal(w.ID, a, r.ID, anchor, "reply-after-reopen", completedFragmentJournal(p, reserved)); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	task, _ = after.task(a.TaskID)
	plan, journal, e := s.readFragmentJournalAnchor(*task.IndependentReview)
	if e != nil {
		t.Fatal(e)
	}
	reusable, e := validateManagedFragmentJournal(journal, plan, a.Attempt, r.BatchProviderDigest)
	if e != nil || len(reusable) != 1 || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls+1 {
		t.Fatal("reply lost or paid twice", e)
	}
}
func TestManagedFragmentStoreModelChangeRejectsReturn(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	reserved := reserveFragment(p, j)
	if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "reserve-before-model-change", reserved); e != nil {
		t.Fatal(e)
	}
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	anchor := task.IndependentReview.FragmentJournal.JournalDigest
	current.Planning.Reviewer.ModelRoute = &ModelRoute{Level: "standard", Model: "different"}
	raw, _ := json.Marshal(current)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	if e := s.commitFragmentJournal(w.ID, a, r.ID, anchor, "return-after-model-change", completedFragmentJournal(p, reserved)); e == nil {
		t.Fatal("changed model accepted")
	}
	after, _ := s.get(w.ID)
	task, _ = after.task(a.TaskID)
	if task.IndependentReview.FragmentJournal.JournalDigest != anchor || after.Planning.Reviewer.Calls != current.Planning.Reviewer.Calls {
		t.Fatal("refusal mutated state")
	}
}
