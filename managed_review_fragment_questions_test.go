//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestManagedFragmentQuestionsCannotDisappear(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	var inspection managedFragmentInspection
	json.Unmarshal([]byte(replies[0]), &inspection)
	inspection.Findings[0].Verdict = "unknown"
	inspection.Findings[0].Needs = []string{"Show the original dependency", "Show the relevant test"}
	raw, _ := json.Marshal(inspection)
	replies[0] = string(raw)
	b, e := managedFragmentFinalBundle(c, p, replies)
	if e != nil || len(b.Questions) != 2 {
		t.Fatal(b, e)
	}
	prompt, visible, e := managedFragmentDecisionPrompt("", c, p, replies, nil)
	if e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(prompt, "Show the relevant test") || !strings.Contains(prompt, "resolutions") {
		t.Fatal("question lost")
	}
	tasks := []any{}
	for _, task := range visible.Tasks {
		criteria := []ReviewCriterion{}
		for i := range task.Criteria {
			criteria = append(criteria, ReviewCriterion{Index: i + 1, Verdict: "pass", Evidence: task.Report})
		}
		tasks = append(tasks, map[string]any{"task": task.Task, "reason": "Original evidence reviewed", "criteria": criteria})
	}
	review := map[string]any{"candidate_commit": visible.Candidate, "tasks": tasks}
	for _, mode := range []string{"resolved", "missing", "duplicate", "invented", "unknown", "bare"} {
		t.Run(mode, func(t *testing.T) {
			resolutions := []map[string]any{}
			for _, q := range b.Questions {
				resolutions = append(resolutions, map[string]any{"packet": q.Packet, "artifact": q.Artifact, "need_index": q.Need, "verdict": "resolved", "reason": "Original evidence answers this question", "evidence": visible.Tasks[0].Report})
			}
			switch mode {
			case "missing":
				resolutions = resolutions[:1]
			case "duplicate":
				resolutions[1] = resolutions[0]
			case "invented":
				resolutions[0]["evidence"] = "Nonexistent fabricated evidence"
			case "unknown":
				resolutions[0]["verdict"] = "unknown"
			}
			response := any(map[string]any{"review": review, "resolutions": resolutions})
			if mode == "bare" {
				response = review
			}
			encoded, _ := json.Marshal(response)
			state, _, err := parseManagedFragmentDecision(string(encoded), visible, c, p, replies)
			if mode == "resolved" {
				if err != nil || state != "passed" {
					t.Fatal(state, err)
				}
			} else if mode == "unknown" {
				if err != nil || state != "unknown" {
					t.Fatal(state, err)
				}
			} else if err == nil {
				t.Fatal("unproved resolution accepted", mode)
			}
		})
	}
}
func TestManagedFragmentUnknownJournalReusableWithoutRefund(t *testing.T) {
	p, j := journalFixture()
	var reply managedFragmentInspection
	json.Unmarshal([]byte(j.Entries[0].Reply), &reply)
	reply.Findings[0].Verdict = "unknown"
	reply.Findings[0].Needs = []string{"Read the related caller"}
	raw, _ := json.Marshal(reply)
	j.Entries[0].Reply = string(raw)
	j.Entries[0].ReplyDigest = hash(raw)
	j.Entries[0].State = "unknown"
	reused, e := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
	if e != nil || len(reused) != 1 {
		t.Fatal(reused, e)
	}
	budget, e := fragmentRecoveryBudget(p, j, 22, 34)
	if e != nil || budget.Required != p.ReservedFinalCalls {
		t.Fatal(budget, e)
	}
	if j.Entries[0].State != "unknown" || len(j.Entries) != 1 {
		t.Fatal("history rewritten")
	}
}

func TestManagedFragmentResumePreservesUnknownQuestions(t *testing.T) {
	s, w, a, r, p, j := fragmentStoreFixture(t)
	packet := p.Packets[0]
	pr, _ := json.Marshal(packet)
	inspection := managedFragmentInspection{Candidate: packet.Candidate, ContextDigest: packet.ContextDigest, PacketDigest: hash(pr)}
	for i, artifact := range packet.Artifacts {
		excerpt := artifact.Content
		if len(excerpt) > 16 {
			excerpt = excerpt[:16]
		}
		inspection.Findings = append(inspection.Findings, managedFragmentFinding{Artifact: i, Digest: artifact.Digest, Verdict: "unknown", Reason: "Related context is still required", Evidence: excerpt, Needs: []string{"Read the caller before deciding"}})
	}
	rr, _ := json.Marshal(inspection)
	j.Entries = append(j.Entries, managedFragmentJournalEntry{Packet: 0, PacketDigest: hash(pr), CallID: "existing-unknown", State: "unknown", Reply: string(rr), ReplyDigest: hash(rr)})
	raw, _ := json.Marshal(j)
	if e := atomicWrite(s.root+"/"+r.FragmentJournal.Journal, raw); e != nil {
		t.Fatal(e)
	}
	r.FragmentJournal.JournalDigest = hash(raw)
	r.State = "error"
	task, _ := w.task(a.TaskID)
	task.IndependentReview = &r
	w.Planning.Reviewer.Calls = 22
	w.Planning.Reviewer.MaxCalls = 22 + len(p.Packets) - 1 + p.ReservedFinalCalls
	if e := s.queueManagedFragmentResume(w, task, "explicit-resume"); e != nil {
		t.Fatal(e)
	}
	_, saved, e := s.readFragmentJournalAnchor(*task.IndependentReview)
	if e != nil || len(saved.Entries) != 1 || saved.Entries[0].Reply != string(rr) || saved.Entries[0].State != "unknown" || w.Planning.Reviewer.Calls != 22 {
		t.Fatal("unknown evidence altered or charged", saved, e)
	}
}
