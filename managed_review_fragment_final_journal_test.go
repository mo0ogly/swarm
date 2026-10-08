//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"testing"
)

func finalJournalFixture(t *testing.T) (managedReviewContext, managedReviewFragmentPlan, managedFragmentJournal, managedFragmentFinalJournal) {
	t.Helper()
	c, p, replies := finalBundleFixture(t)
	raw, _ := json.Marshal(p)
	j := managedFragmentJournal{Version: 1, PlanDigest: hash(raw), Attempt: "attempt", ProviderDigest: "provider"}
	for i, packet := range p.Packets {
		raw, _ = json.Marshal(packet)
		j.Entries = append(j.Entries, managedFragmentJournalEntry{Packet: i, PacketDigest: hash(raw), CallID: fmt.Sprintf("inspection-%d", i), State: "inspected", Reply: replies[i], ReplyDigest: hash([]byte(replies[i]))})
	}
	raw, _ = json.Marshal(j)
	f := managedFragmentFinalJournal{Version: 1, InspectionDigest: hash(raw)}
	raw, _ = json.Marshal(p)
	request := managedFragmentFinalRequest{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: hash(raw), State: "ready", Reason: "Existing original evidence supports final assessment", References: []managedFragmentEvidenceRef{}}
	raw, _ = json.Marshal(request)
	prompt, e := managedFragmentRequestPrompt("", c, p, replies)
	if e != nil {
		t.Fatal(e)
	}
	f.Calls = append(f.Calls, managedFragmentFinalCall{Phase: "selection", CallID: "select", PromptDigest: hash([]byte(prompt)), State: "ready", Reply: string(raw), ReplyDigest: hash(raw)})
	tasks := []map[string]any{}
	for _, tc := range c.Tasks {
		criteria := []ReviewCriterion{}
		for i := range tc.Criteria {
			criteria = append(criteria, ReviewCriterion{Index: i + 1, Verdict: "pass", Evidence: tc.Report})
		}
		tasks = append(tasks, map[string]any{"task": tc.Task, "reason": "Supported by original report and controls", "criteria": criteria})
	}
	raw, _ = json.Marshal(map[string]any{"candidate_commit": c.Candidate, "tasks": tasks})
	prompt, _, e = managedFragmentDecisionPrompt("", c, p, replies, nil)
	if e != nil {
		t.Fatal(e)
	}
	f.Calls = append(f.Calls, managedFragmentFinalCall{Phase: "decision", CallID: "decide", PromptDigest: hash([]byte(prompt)), State: "passed", Reply: string(raw), ReplyDigest: hash(raw)})
	return c, p, j, f
}
func TestManagedFragmentFinalJournalRechecksProofs(t *testing.T) {
	for _, mode := range []string{"valid", "inspection", "reply", "prompt", "call", "phase", "state", "budget", "reserved"} {
		t.Run(mode, func(t *testing.T) {
			c, p, j, f := finalJournalFixture(t)
			switch mode {
			case "inspection":
				f.InspectionDigest = "other"
			case "reply":
				f.Calls[1].Reply = "{}"
			case "prompt":
				f.Calls[1].PromptDigest = "other"
			case "call":
				f.Calls[1].CallID = j.Entries[0].CallID
			case "phase":
				f.Calls[0].Phase = "decision"
			case "state":
				f.Calls[1].State = "unknown"
			case "budget":
				f.Calls = append(f.Calls, f.Calls[1])
			case "reserved":
				f.Calls = f.Calls[:1]
				f.Calls[0].State = "reserved"
				f.Calls[0].Reply = ""
				f.Calls[0].ReplyDigest = ""
			}
			state, records, e := validateManagedFragmentFinalJournal(f, j, c, p, "")
			if mode == "valid" {
				if e != nil || state != "passed" || len(records) != len(c.Tasks) {
					t.Fatal(state, records, e)
				}
			} else if mode == "reserved" {
				if e != nil || state != "reserved" || len(records) != 0 {
					t.Fatal(state, e)
				}
			} else if e == nil {
				t.Fatal("invalid journal accepted")
			}
		})
	}
}
