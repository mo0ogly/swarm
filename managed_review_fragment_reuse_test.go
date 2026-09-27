//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func fragmentHistoricalFixture(t *testing.T) (managedReviewContext, managedReviewContext, IndependentReview, managedReviewFragmentPlan, managedFragmentJournal, string) {
	t.Helper()
	old, p, replies := finalBundleFixture(t)
	pr, _ := json.Marshal(p)
	j := managedFragmentJournal{Version: 1, PlanDigest: hash(pr), Attempt: "original-attempt", ProviderDigest: "original-provider"}
	for i, reply := range replies {
		raw, _ := json.Marshal(p.Packets[i])
		j.Entries = append(j.Entries, managedFragmentJournalEntry{Packet: i, PacketDigest: hash(raw), CallID: fmt.Sprint("original-call-", i), State: "inspected", Reply: reply, ReplyDigest: hash([]byte(reply))})
	}
	r := IndependentReview{ID: "original-review", CandidateSHA: old.Candidate, FragmentJournal: &ManagedFragmentJournalAnchor{}}
	// Deep copy: changes to current task contracts must not mutate the old proof.
	raw, _ := json.Marshal(old)
	var current managedReviewContext
	json.Unmarshal(raw, &current)
	current.Candidate = "new-candidate"
	current.Diff = strings.Replace(current.Diff, "+évidence\n", "+correction verified\n", 1)
	delta := "diff --git a/f0 b/f0\n@@ -1 +1 @@\n-évidence\n+correction verified\n"
	return current, old, r, p, j, delta
}
func fragmentHistoricalReplies(t *testing.T, p managedReviewFragmentPlan) []string {
	t.Helper()
	replies := make([]string, len(p.Packets))
	for i, packet := range p.Packets {
		if r := fragmentReuseAt(p, i); r != nil {
			replies[i] = r.Reply
			continue
		}
		raw, _ := json.Marshal(packet)
		inspection := managedFragmentInspection{Candidate: packet.Candidate, ContextDigest: packet.ContextDigest, PacketDigest: hash(raw)}
		for k, a := range packet.Artifacts {
			evidence := a.Content
			if len(evidence) > 40 {
				evidence = evidence[:40]
			}
			inspection.Findings = append(inspection.Findings, managedFragmentFinding{Artifact: k, Digest: a.Digest, Verdict: "inspected", Reason: "Current original evidence inspected", Evidence: evidence, Needs: []string{}})
		}
		raw, _ = json.Marshal(inspection)
		replies[i] = string(raw)
	}
	return replies
}
func TestManagedFragmentHistoricalSavingsAndIdentity(t *testing.T) {
	c, old, r, op, j, delta := fragmentHistoricalFixture(t)
	before, _ := json.Marshal(j)
	p, err := differentialFragmentPlan(c, old, r, op, j, delta, 100)
	if err != nil {
		t.Fatal(err)
	}
	full, err := planManagedReviewFragments(c, 100, 2)
	if err != nil {
		t.Fatal(err)
	}
	if p.Version != 3 || len(p.Reused) == 0 || fragmentPaidInspections(p) >= len(full.Packets) {
		t.Fatal("no reduction", len(p.Packets), len(p.Reused), len(full.Packets))
	}
	t.Logf("full=%d calls, differential=%d calls, historical groups=%d", len(full.Packets)+2, fragmentPaidInspections(p)+2, len(p.Reused))
	bundle, err := managedFragmentFinalBundle(c, p, fragmentHistoricalReplies(t, p))
	if err != nil {
		t.Fatal(err)
	}
	if len(bundle.Historical) != len(p.Reused) || len(bundle.Questions) != len(p.Reused) || bundle.ChangeDiff != delta {
		t.Fatal("lost impact obligations")
	}
	for _, ref := range p.Reused {
		if ref.Original.Candidate != old.Candidate || ref.Reply != j.Entries[ref.Original.Index].Reply {
			t.Fatal("rewritten old proof")
		}
		// Old raw replies must never parse as fresh current-candidate inspections.
		if _, _, err = parseManagedFragmentInspection(ref.Reply, p.Packets[ref.Packet]); err == nil {
			t.Fatal("old reply considered fresh")
		}
	}
	after, _ := json.Marshal(j)
	if string(before) != string(after) {
		t.Fatal("old reservations changed")
	}
}
func TestManagedFragmentHistoricalInvalidation(t *testing.T) {
	for _, mode := range []string{"dependency", "contract", "changed-all", "no-delta", "nested"} {
		t.Run(mode, func(t *testing.T) {
			c, old, r, op, j, delta := fragmentHistoricalFixture(t)
			switch mode {
			case "dependency":
				// A second artifact read in the same original packet is a dependency.
				c.Diff = strings.Replace(c.Diff, "diff --git a/f5 b/f5\n", "diff --git a/f5 b/f5\n+dependency changed\n", 1)
			case "contract":
				c.Tasks[0].Title += " new scope"
			case "changed-all":
				c.Diff = strings.ReplaceAll(c.Diff, "+évidence\n", "+changed evidence\n")
			case "no-delta":
				delta = ""
			case "nested":
				op.Version = 3
			}
			p, err := differentialFragmentPlan(c, old, r, op, j, delta, 100)
			if err != nil {
				t.Fatal(err)
			}
			if mode == "changed-all" {
				for _, ref := range p.Reused {
					for _, a := range ref.Original.Artifacts {
						if a.Kind == "diff" {
							t.Fatal("changed diff reused")
						}
					}
				}
				return
			}
			if mode != "dependency" {
				if len(p.Reused) != 0 {
					t.Fatal("invalid history reused")
				}
				return
			}
			for _, ref := range p.Reused {
				for _, a := range ref.Original.Artifacts {
					if a.Name == "diff --git a/f5 b/f5" {
						t.Fatal("changed dependency reused")
					}
				}
			}
		})
	}
}
func TestManagedFragmentHistoricalTampering(t *testing.T) {
	for _, mode := range []string{"duplicate", "omitted", "reply", "origin", "delta", "charged"} {
		t.Run(mode, func(t *testing.T) {
			c, old, r, op, j, delta := fragmentHistoricalFixture(t)
			p, err := differentialFragmentPlan(c, old, r, op, j, delta, 100)
			if err != nil {
				t.Fatal(err)
			}
			switch mode {
			case "duplicate":
				p.Reused = append(p.Reused, p.Reused[0])
			case "omitted":
				p.Packets[0].Artifacts = p.Packets[0].Artifacts[1:]
			case "reply":
				p.Reused[0].Reply = "{}"
			case "origin":
				p.Reused[0].Original.Candidate = c.Candidate
			case "delta":
				p.ChangeDiff = ""
			case "charged":
				raw, _ := json.Marshal(p)
				packet, _ := json.Marshal(p.Packets[p.Reused[0].Packet])
				jj := managedFragmentJournal{Version: 1, PlanDigest: hash(raw), Attempt: j.Attempt, ProviderDigest: j.ProviderDigest, Entries: []managedFragmentJournalEntry{{Packet: p.Reused[0].Packet, PacketDigest: hash(packet), CallID: "fake-charge", State: "reserved"}}}
				if _, err = validateManagedFragmentJournal(jj, p, j.Attempt, j.ProviderDigest); err == nil {
					t.Fatal("paid for historical observation")
				}
				return
			}
			if validateManagedReviewFragments(c, p) == nil {
				t.Fatal("tamper accepted")
			}
		})
	}
}
func TestManagedFragmentHistoricalUnknownRemainsOpen(t *testing.T) {
	c, old, r, op, j, delta := fragmentHistoricalFixture(t)
	index := 1
	var reply managedFragmentInspection
	json.Unmarshal([]byte(j.Entries[index].Reply), &reply)
	reply.Findings[0].Verdict = "unknown"
	reply.Findings[0].Needs = []string{"Examine the related caller"}
	raw, _ := json.Marshal(reply)
	j.Entries[index].Reply = string(raw)
	j.Entries[index].ReplyDigest = hash(raw)
	j.Entries[index].State = "unknown"
	p, err := differentialFragmentPlan(c, old, r, op, j, delta, 100)
	if err != nil {
		t.Fatal(err)
	}
	b, err := managedFragmentFinalBundle(c, p, fragmentHistoricalReplies(t, p))
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, q := range b.Questions {
		if q.Text == "Examine the related caller" {
			found = true
		}
	}
	if !found || len(b.Questions) != len(p.Reused)+1 {
		t.Fatal("original unknown disappeared")
	}
	// All original calls remain spent: the pure planner never mutates the journal.
	if !reflect.DeepEqual(j.Entries[index].State, "unknown") {
		t.Fatal("history changed")
	}
}

func TestManagedFragmentHistoricalPublicRecovery(t *testing.T) {
	for _, mode := range []string{"pass", "fail", "exit"} {
		t.Run(mode, func(t *testing.T) { fragmentHistoricalPublicRecovery(t, mode) })
	}
}
func fragmentHistoricalPublicRecovery(t *testing.T, mode string) {
	s, w, a, request := recoveredResultFixture(t)
	// Exercise actual candidate creation, controls, durable review, public recovery
	// and final publication. Only the external model is a deterministic subprocess.
	for i := 0; i < 8; i++ {
		if err := os.WriteFile(filepath.Join(a.CWD, fmt.Sprintf("source-%02d.txt", i)), []byte(strings.Repeat(fmt.Sprintf("source %02d original evidence\n", i), 2200)), 0600); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fragmentRuntimeProviderFixture), 0700); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	request.ResultTree = gitTest(t, a.CWD, "write-tree")
	managedReviewMode(t, s, "decision-fail")
	first, err := s.planningChange(w.ID, "submit-recovered-result", request)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := first.task(a.TaskID)
	if task.IndependentReview == nil || task.IndependentReview.State != "changes_requested" || task.IndependentReview.FragmentJournal == nil {
		t.Fatal("first review", task.Status, task.Blocker, task.IndependentReview)
	}
	origin := *task.IndependentReview
	oldJournal, err := os.ReadFile(filepath.Join(s.root, origin.FragmentJournal.Journal))
	if err != nil {
		t.Fatal(err)
	}
	oldPlan, _, err := s.readFragmentJournalAnchor(origin)
	if err != nil {
		t.Fatal(err)
	}
	initialCalls := managedReviewCalls(t, s)
	if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("Corrected external report with complete current evidence"), 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	next := request
	next.EventID = "differential-repair"
	next.Revision = first.Revision
	next.ReviewID = origin.ID
	next.ResultTree = gitTest(t, a.CWD, "write-tree")
	managedReviewMode(t, s, mode)
	after, err := s.planningChange(w.ID, "revise-recovered-result", next)
	if err != nil {
		t.Fatal(err)
	}
	current, _ := after.task(a.TaskID)
	if mode == "fail" || mode == "exit" {
		expectedState := "changes_requested"
		if mode == "exit" {
			expectedState = "error"
		}
		if current.Status != "blocked" || current.IndependentReview == nil || current.IndependentReview.State != expectedState || managedReviewCalls(t, s) != initialCalls+1 {
			t.Fatal("new defect bypassed", current.Status, current.Blocker)
		}
		if current.AutoValidation != nil && current.AutoValidation.CandidateSHA == current.IndependentReview.CandidateSHA {
			t.Fatal("failed current review validated")
		}
		// A second correction must return to the original evidence, not discard
		// it merely because the previous plan already contained historical refs.
		if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte("Second corrected report; original observations remain historical"), 0600); err != nil {
			t.Fatal(err)
		}
		gitTest(t, a.CWD, "add", "-A")
		third := next
		third.ConfirmReviewErrorRepair = mode == "exit"
		third.EventID, third.Revision, third.ReviewID = "second-differential-repair", after.Revision, current.IndependentReview.ID
		third.ResultTree = gitTest(t, a.CWD, "write-tree")
		managedReviewMode(t, s, "pass")
		last, e := s.planningChange(w.ID, "revise-recovered-result", third)
		if e != nil {
			t.Fatal(e)
		}
		final, _ := last.task(a.TaskID)
		if final.Status != "accepted" || final.IndependentReview == nil {
			t.Fatal("second correction", final.Blocker)
		}
		plan, _, e := s.readFragmentJournalAnchor(*final.IndependentReview)
		if e != nil || len(plan.Reused) == 0 {
			t.Fatal("historical observations lost after second correction", e)
		}
		for _, ref := range plan.Reused {
			if ref.Review.ID != origin.ID {
				t.Fatal("intermediate identity substituted")
			}
		}
		if managedReviewCalls(t, s)-initialCalls-1 != fragmentPaidInspections(plan)+2 {
			t.Fatal("wrong cumulative budget")
		}
		kept, _ := os.ReadFile(filepath.Join(s.root, origin.FragmentJournal.Journal))
		if string(kept) != string(oldJournal) {
			t.Fatal("original journal changed")
		}
		return
	}
	if current.Status != "accepted" || current.IndependentReview == nil {
		t.Fatal("repair", current.Status, current.Blocker, current.IndependentReview)
	}
	p, j, err := s.readFragmentJournalAnchor(*current.IndependentReview)
	if err != nil {
		t.Fatal(err)
	}
	if p.Version != 3 || len(p.Reused) == 0 {
		t.Fatal("no historical references")
	}
	charged := managedReviewCalls(t, s) - initialCalls
	if charged != fragmentPaidInspections(p)+2 || charged >= len(oldPlan.Packets)+2 || len(j.Entries) != fragmentPaidInspections(p) {
		t.Fatal("wrong real reservations", charged, len(j.Entries), len(oldPlan.Packets), len(p.Packets), len(p.Reused))
	}
	if after.Planning.Reviewer.Calls != first.Planning.Reviewer.Calls+charged {
		t.Fatal("budget refunded")
	}
	if current.AutoValidation.CandidateSHA != current.IndependentReview.CandidateSHA || current.IndependentReview.CandidateSHA == origin.CandidateSHA {
		t.Fatal("wrong final candidate")
	}
	kept, _ := os.ReadFile(filepath.Join(s.root, origin.FragmentJournal.Journal))
	if string(kept) != string(oldJournal) {
		t.Fatal("old journal overwritten")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	if err = reopened.managedReviewFilesIntact(*current.IndependentReview); err != nil {
		t.Fatal("reopen", err)
	}
	replay, err := s.planningChange(w.ID, "revise-recovered-result", next)
	if err != nil || replay.Revision != after.Revision || managedReviewCalls(t, s) != initialCalls+charged {
		t.Fatal("replay charged", err)
	}
	if err = os.WriteFile(filepath.Join(s.root, origin.FragmentJournal.Journal), append(oldJournal, ' '), 0600); err != nil {
		t.Fatal(err)
	}
	if reopened.managedReviewFilesIntact(*current.IndependentReview) == nil {
		t.Fatal("historical corruption accepted")
	}
	t.Logf("Actual subprocess calls: original=%d differential=%d; reused=%d; exact candidate, reopen, corruption and idempotence checked", initialCalls, charged, len(p.Reused))
}

func TestManagedFragmentHistoricalDecisionRequiresCurrentImpact(t *testing.T) {
	c, old, r, op, j, delta := fragmentHistoricalFixture(t)
	p, err := differentialFragmentPlan(c, old, r, op, j, delta, 100)
	if err != nil {
		t.Fatal(err)
	}
	replies := fragmentHistoricalReplies(t, p)
	b, err := managedFragmentFinalBundle(c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	_, visible, err := managedFragmentDecisionPrompt("", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	tasks := []any{}
	for _, task := range visible.Tasks {
		criteria := []ReviewCriterion{}
		for i := range task.Criteria {
			criteria = append(criteria, ReviewCriterion{Index: i + 1, Verdict: "pass", Evidence: task.Report})
		}
		tasks = append(tasks, map[string]any{"task": task.Task, "reason": "Original evidence reviewed", "criteria": criteria})
	}
	review := map[string]any{"candidate_commit": c.Candidate, "tasks": tasks}
	for _, mode := range []string{"resolved", "missing", "unknown", "old-excerpt", "report", "path-only", "invented"} {
		t.Run(mode, func(t *testing.T) {
			resolutions := []map[string]any{}
			for _, q := range b.Questions {
				resolutions = append(resolutions, map[string]any{"packet": q.Packet, "artifact": q.Artifact, "need_index": q.Need, "verdict": "resolved", "reason": "The current correction has been examined for effects on this historical group", "evidence": "correction verified"})
			}
			switch mode {
			case "missing":
				resolutions = resolutions[1:]
			case "unknown":
				resolutions[0]["verdict"] = "unknown"
			case "old-excerpt":
				resolutions[0]["evidence"] = b.Evidence[len(b.Evidence)-1].Excerpt
			case "report":
				resolutions[0]["evidence"] = visible.Tasks[0].Report
			case "path-only":
				resolutions[0]["evidence"] = "diff --git a/f0 b/f0"
			case "invented":
				resolutions[0]["evidence"] = "imaginary current correction"
			}
			raw, _ := json.Marshal(map[string]any{"review": review, "resolutions": resolutions})
			state, _, err := parseManagedFragmentDecision(string(raw), visible, c, p, replies)
			if mode == "resolved" {
				if err != nil || state != "passed" {
					t.Fatal(state, err)
				}
			} else if mode == "unknown" {
				if err != nil || state != "unknown" {
					t.Fatal(state, err)
				}
			} else if err == nil {
				t.Fatal("unproven current impact accepted", mode, state)
			}
		})
	}
}

func TestManagedFragmentHistoricalTinyAndRenameEvidence(t *testing.T) {
	for _, tc := range []struct {
		delta, evidence string
		want            bool
	}{
		{"diff --git a/f b/f\n@@ -1 +1 @@\n-x\n+y\n", "@@ -1 +1 @@\n-x\n+y", true},
		{"diff --git a/a b/b\nsimilarity index 100%\nrename from a\nrename to b\n", "rename from a\nrename to b", true},
		{"diff --git a/a b/a\nold mode 100644\nnew mode 100755\n", "new mode 100755", true},
		{"diff --git a/f b/f\n@@ -1 +1 @@\n-x\n+y\n", "diff --git a/f b/f", false},
	} {
		if got := fragmentCurrentChangeEvidence(tc.delta, tc.evidence); got != tc.want {
			t.Fatal(tc, got)
		}
	}
}

func TestManagedFragmentHistoricalCapacityPreservesKnownInputs(t *testing.T) {
	c, old, r, op, j, delta := fragmentHistoricalFixture(t)
	var inspection managedFragmentInspection
	json.Unmarshal([]byte(j.Entries[1].Reply), &inspection)
	inspection.Findings[0].Verdict = "unknown"
	inspection.Findings[0].Needs = []string{"Current dependency must be examined before reuse"}
	raw, _ := json.Marshal(inspection)
	j.Entries[1].Reply = string(raw)
	j.Entries[1].ReplyDigest = hash(raw)
	j.Entries[1].State = "unknown"
	p, err := differentialFragmentPlan(c, old, r, op, j, delta, 100)
	if err != nil {
		t.Fatal(err)
	}
	b := maximalManagedFragmentBundle(c, p)
	found := false
	for _, q := range b.Questions {
		if q.Text == inspection.Findings[0].Needs[0] {
			found = true
		}
	}
	if !found {
		t.Fatal("known unanswered question omitted from capacity")
	}
	actual, err := managedFragmentFinalBundle(c, p, fragmentHistoricalReplies(t, p))
	if err != nil {
		t.Fatal(err)
	}
	for _, ref := range p.Reused {
		for i, e := range b.Evidence {
			if e.Packet == ref.Packet && !reflect.DeepEqual(e, actual.Evidence[i]) {
				t.Fatal("historical content size invented")
			}
		}
	}
	if err = preflightManagedFragmentCalls("", c, p); err != nil {
		t.Fatal(err)
	}
	p.ChangeDiff = "diff --git a/f b/f\n+" + strings.Repeat("large", managedReviewPromptLimit)
	if preflightManagedFragmentCalls("", c, p) == nil {
		t.Fatal("oversized final delta allowed to consume inspection calls")
	}
}
