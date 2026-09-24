//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"sync"
	"testing"
)

func finalStoreFixture(t *testing.T) (*Store, Work, Agent, IndependentReview, managedReviewFragmentPlan, managedFragmentJournal, managedReviewContext, []string, string) {
	t.Helper()
	s, w, a, r, p, j := fragmentStoreFixture(t)
	replies := []string{}
	for i, packet := range p.Packets {
		raw, _ := json.Marshal(packet)
		j.Entries = append(j.Entries, managedFragmentJournalEntry{Packet: i, PacketDigest: hash(raw), CallID: fmt.Sprintf("inspect-%d", i), State: "reserved"})
		if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, fmt.Sprintf("reserve-%d", i), j); e != nil {
			t.Fatal(e)
		}
		current, _ := s.get(w.ID)
		task, _ := current.task(a.TaskID)
		r = *task.IndependentReview
		response := managedFragmentInspection{Candidate: p.Candidate, ContextDigest: p.ContextDigest, PacketDigest: hash(raw)}
		for k, artifact := range packet.Artifacts {
			excerpt := []rune(artifact.Content)
			if len(excerpt) > 16 {
				excerpt = excerpt[:16]
			}
			response.Findings = append(response.Findings, managedFragmentFinding{Artifact: k, Digest: artifact.Digest, Verdict: "inspected", Reason: "Independent inspection", Evidence: string(excerpt), Needs: []string{}})
		}
		raw, _ = json.Marshal(response)
		replies = append(replies, string(raw))
		entry := &j.Entries[len(j.Entries)-1]
		entry.State = "inspected"
		entry.Reply = string(raw)
		entry.ReplyDigest = hash(raw)
		if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, fmt.Sprintf("result-%d", i), j); e != nil {
			t.Fatal(e)
		}
		current, _ = s.get(w.ID)
		task, _ = current.task(a.TaskID)
		r = *task.IndependentReview
	}
	_, c, prefix, e := s.readManagedFragmentFinal(r, p, j)
	if e != nil {
		t.Fatal(e)
	}
	return s, w, a, r, p, j, c, replies, prefix
}
func finalReservation(t *testing.T, c managedReviewContext, p managedReviewFragmentPlan, j managedFragmentJournal, replies []string, prefix string) managedFragmentFinalJournal {
	t.Helper()
	raw, _ := json.Marshal(j)
	prompt, e := managedFragmentRequestPrompt(prefix, c, p, replies)
	if e != nil {
		t.Fatal(e)
	}
	return managedFragmentFinalJournal{Version: 1, InspectionDigest: hash(raw), Calls: []managedFragmentFinalCall{{Phase: "selection", CallID: "final-select", PromptDigest: hash([]byte(prompt)), State: "reserved"}}}
}
func TestManagedFragmentFinalStoreReservationAndReply(t *testing.T) {
	s, w, a, r, p, j, c, replies, prefix := finalStoreFixture(t)
	before, _ := s.get(w.ID)
	f := finalReservation(t, c, p, j, replies, prefix)
	if e := s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "", "reserve-final", f); e != nil {
		t.Fatal(e)
	}
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	saved := *task.IndependentReview
	if current.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls+1 {
		t.Fatal("reservation not charged")
	}
	if e := s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "", "duplicate-final", f); e == nil {
		t.Fatal("duplicate reservation")
	}
	if e := s.commitFragmentJournal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "rewrite-inspections", j); e == nil {
		t.Fatal("inspection journal not frozen")
	}
	raw, _ := json.Marshal(p)
	request := managedFragmentFinalRequest{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw), State: "ready", Reason: "Original evidence sufficient for decision", References: []managedFragmentEvidenceRef{}}
	raw, _ = json.Marshal(request)
	f.Calls[0].State = "ready"
	f.Calls[0].Reply = string(raw)
	f.Calls[0].ReplyDigest = hash(raw)
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	if e = reopened.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, saved.FragmentJournal.FinalJournalDigest, "reply-final", f); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	task, _ = after.task(a.TaskID)
	got, _, _, e := s.readManagedFragmentFinal(*task.IndependentReview, p, j)
	if e != nil || got.Calls[0].State != "ready" {
		t.Fatal(got, e)
	}
	if after.Planning.Reviewer.Calls != current.Planning.Reviewer.Calls || task.Status == "accepted" || task.IndependentReview.State != "running" {
		t.Fatal("reply charged or accepted task")
	}
	saved = *task.IndependentReview
	prompt, _, e := managedFragmentDecisionPrompt(prefix, c, p, replies, nil)
	if e != nil {
		t.Fatal(e)
	}
	f.Calls = append(f.Calls, managedFragmentFinalCall{Phase: "decision", CallID: "final-decision", PromptDigest: hash([]byte(prompt)), State: "reserved"})
	if e = s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, saved.FragmentJournal.FinalJournalDigest, "reserve-decision", f); e != nil {
		t.Fatal(e)
	}
	after, _ = s.get(w.ID)
	task, _ = after.task(a.TaskID)
	saved = *task.IndependentReview
	tasks := []map[string]any{}
	for _, tc := range c.Tasks {
		excerpt := []rune(tc.Report)
		if len(excerpt) > 24 {
			excerpt = excerpt[:24]
		}
		criteria := []ReviewCriterion{}
		for i := range tc.Criteria {
			criteria = append(criteria, ReviewCriterion{Index: i + 1, Verdict: "pass", Evidence: string(excerpt)})
		}
		tasks = append(tasks, map[string]any{"task": tc.Task, "reason": "Original report supports the criterion", "criteria": criteria})
	}
	raw, _ = json.Marshal(map[string]any{"candidate_commit": c.Candidate, "tasks": tasks})
	f.Calls[1].State = "passed"
	f.Calls[1].Reply = string(raw)
	f.Calls[1].ReplyDigest = hash(raw)
	if e = s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, saved.FragmentJournal.FinalJournalDigest, "decision-result", f); e != nil {
		t.Fatal(e)
	}
	after, _ = s.get(w.ID)
	task, _ = after.task(a.TaskID)
	got, _, _, e = s.readManagedFragmentFinal(*task.IndependentReview, p, j)
	if e != nil {
		t.Fatal(e)
	}
	state, records, e := validateManagedFragmentFinalJournal(got, j, c, p, prefix)
	if e != nil || state != "passed" || len(records) != len(c.Tasks) {
		t.Fatal(state, e)
	}
	if after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls+2 || task.Status == "accepted" {
		t.Fatal("wrong total or implicit publication")
	}
}
func TestManagedFragmentFinalStorePauseRollsBack(t *testing.T) {
	s, w, a, r, p, j, c, replies, prefix := finalStoreFixture(t)
	before, _ := s.get(w.ID)
	if e := s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	if e := s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "", "paused-final", finalReservation(t, c, p, j, replies, prefix)); e == nil {
		t.Fatal("paused reservation")
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls || task.IndependentReview.FragmentJournal.FinalJournalDigest != "" {
		t.Fatal("rollback failed")
	}
}

func TestManagedFragmentFinalStoreConcurrentReservationOnce(t *testing.T) {
	s, w, a, r, p, j, c, replies, prefix := finalStoreFixture(t)
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	before, _ := s.get(w.ID)
	f := finalReservation(t, c, p, j, replies, prefix)
	start := make(chan struct{})
	results := make(chan error, 2)
	var wg sync.WaitGroup
	for i, store := range []*Store{s, other} {
		wg.Add(1)
		go func(i int, store *Store) {
			defer wg.Done()
			<-start
			results <- store.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "", fmt.Sprintf("race-final-%d", i), f)
		}(i, store)
	}
	close(start)
	wg.Wait()
	close(results)
	success := 0
	for err := range results {
		if err == nil {
			success++
		}
	}
	after, _ := s.get(w.ID)
	if success != 1 || after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls+1 {
		t.Fatal("duplicate charge", success)
	}
}
func TestManagedFragmentFinalStoreBudgetAndModelGuards(t *testing.T) {
	for _, mode := range []string{"budget", "model"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, r, p, j, c, replies, prefix := finalStoreFixture(t)
			before, _ := s.get(w.ID)
			altered := before
			if mode == "budget" {
				altered.Planning.Reviewer.MaxCalls = altered.Planning.Reviewer.Calls + 1
			} else {
				altered.Planning.Reviewer.ModelRoute = &ModelRoute{Level: "standard", Model: "changed"}
			}
			raw, _ := json.Marshal(altered)
			if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
				t.Fatal(e)
			}
			if e := s.commitManagedFragmentFinal(w.ID, a, r.ID, r.FragmentJournal.JournalDigest, "", "invalid-final", finalReservation(t, c, p, j, replies, prefix)); e == nil {
				t.Fatal("guard bypass")
			}
			after, _ := s.get(w.ID)
			task, _ := after.task(a.TaskID)
			if after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls || task.IndependentReview.FragmentJournal.FinalJournalDigest != "" {
				t.Fatal("refusal changed state")
			}
		})
	}
}
