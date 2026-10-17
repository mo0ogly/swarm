//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func citationDirectiveFixture(t *testing.T) *ManagedReviewRetryDirective {
	t.Helper()
	d := ManagedReviewRetryDirective{Decision: "decision", Scope: "root", ResultEvent: "result", Review: "review", Candidate: "candidate", Attempt: "attempt", FailureCode: reviewFragmentProtocolError, Cause: "A literal citation was altered", Instruction: "Re-examine the supplied originals and preserve an independent decision."}
	raw, _ := json.Marshal(d)
	var wire map[string]any
	if err := json.Unmarshal(raw, &wire); err != nil {
		t.Fatal(err)
	}
	original := "+if enabled:\n+    actualOperation()\n"
	wire["citation_diagnostic"] = map[string]any{"version": 1, "review": "review", "candidate_commit": "candidate", "call_id": "previous-final", "reply_sha256": strings.Repeat("a", 64), "issues": []any{map[string]any{"packet": 0, "artifact": 0, "need_index": 0, "rejected_quote": "if enabled:\n actualOperation()", "quote_sha256": strings.Repeat("b", 64), "originals": []any{map[string]any{"name": "selected/0", "sha256": hash([]byte(original)), "line": 1, "original_text": original}}}}}
	raw, _ = json.Marshal(wire)
	if err := json.Unmarshal(raw, &d); err != nil {
		t.Fatal(err)
	}
	return &d
}

func TestRecoveryPrefixCarriesLiteralCitationConventions(t *testing.T) {
	d := citationDirectiveFixture(t)
	prompt, err := managedReviewRecoveryPrefix("prefix", d)
	if err != nil {
		t.Fatal(err)
	}
	for _, value := range []string{"CONVENTIONS DE CITATION", "SWARM_REVIEW_CITATION_DIAGNOSTIC", "actualOperation", "indentation", "unknown"} {
		if !strings.Contains(prompt, value) {
			t.Fatal("literal original or exchange convention absent", value)
		}
	}
	if strings.Count(prompt, `"call_id":"previous-final"`) != 1 {
		t.Fatal("diagnostic serialized more than once")
	}
}

func TestCitationDiagnosticPreservesLiteralBoundaries(t *testing.T) {
	for _, tc := range []struct{ name, quote, literal string }{
		{"indentation", "if enabled:\n actualOperation()", "if enabled:\n    actualOperation()\n"},
		{"encoded output", "PASS exact output\n", `{"stdout":"PASS exact output\n"}`},
	} {
		t.Run(tc.name, func(t *testing.T) {
			c, p, replies, _, _ := fragmentContractQuestionFixture(t, "Check exact current originals", ReviewSource{Path: "proof.txt", Content: tc.literal, Bytes: len(tc.literal), Digest: hash([]byte(tc.literal))})
			c.Tasks[0].Controls = []ValidationControlResult{{ID: "captured", ReviewOutput: tc.literal}}
			c, p, replies, _, _ = fragmentContractQuestionFromContext(t, c)
			refs := []managedFragmentEvidenceRef{}
			for i, packet := range p.Packets {
				for j, a := range packet.Artifacts {
					if a.Kind == "source" {
						refs = append(refs, managedFragmentEvidenceRef{i, j, a.Digest})
					}
				}
			}
			_, visible, err := managedFragmentDecisionPrompt("", c, p, replies, refs)
			if err != nil {
				t.Fatal(err)
			}
			raw := []byte(fragmentContractDecision(t, c, tc.quote, c.Tasks[0].Report))
			if _, _, err := parseManagedFragmentDecision(string(raw), visible, c, p, replies); err == nil {
				t.Fatal("nonliteral evidence admitted before feedback")
			}
			bundle, err := managedFragmentFinalBundle(c, p, replies)
			if err != nil {
				t.Fatal(err)
			}
			r := IndependentReview{ID: "review", CandidateSHA: c.Candidate}
			d, err := buildCitationDiagnostic(raw, r, "old-final", visible, bundle.Questions)
			if err != nil || d == nil || len(d.Issues) != 1 || len(d.Issues[0].Originals) == 0 {
				t.Fatal("mismatch lacks original context", err)
			}
			var multiple map[string]any
			if err = json.Unmarshal(raw, &multiple); err != nil {
				t.Fatal(err)
			}
			multiple["resolutions"] = append(multiple["resolutions"].([]any), map[string]any{"packet": 0, "artifact": 0, "need_index": 1, "verdict": "resolved", "evidence": "Another unavailable literal original"})
			more, _ := json.Marshal(multiple)
			questions := append(append([]managedFragmentQuestion(nil), bundle.Questions...), managedFragmentQuestion{0, 0, 1, "A second unresolved question"})
			first, e := buildCitationDiagnostic(more, r, "old-final", visible, questions)
			if e != nil || first == nil || len(first.Issues) != 1 || first.Issues[0].Need != 0 {
				t.Fatal("diagnostic must identify the first rejected citation without dropping other review questions", e)
			}
			originals, err := managedFragmentResolutionOriginals(visible)
			if err != nil {
				t.Fatal(err)
			}
			for _, hint := range d.Issues[0].Originals {
				found := false
				for _, original := range originals {
					if hash([]byte(original.Content)) == hint.Digest && hint.Offset+len(hint.Text) <= len(original.Content) && original.Content[hint.Offset:hint.Offset+len(hint.Text)] == hint.Text {
						found = true
					}
				}
				if !found {
					t.Fatal("diagnostic normalized, invented or crossed originals", hint.Name)
				}
			}
			if _, _, err := parseManagedFragmentDecision(string(raw), visible, c, p, replies); err == nil {
				t.Fatal("diagnostic repaired the old opinion")
			}
			if _, err := buildCitationDiagnostic(raw, IndependentReview{ID: "review", CandidateSHA: "foreign"}, "old-final", visible, bundle.Questions); err == nil {
				t.Fatal("foreign reply accepted")
			}
		})
	}
}

func TestCitationDiagnosticRejectsInvalidWireAndKeepsHistoricalPrefix(t *testing.T) {
	d := citationDirectiveFixture(t)
	before := d.CitationDiagnostic
	d.CitationDiagnostic = nil
	legacy, err := managedReviewRecoveryPrefix("prefix", d)
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(legacy, "CONVENTIONS DE CITATION") || strings.Contains(legacy, "citation_diagnostic") {
		t.Fatal("historical prompt changed")
	}
	d.CitationDiagnostic = before
	for _, mode := range []string{"candidate", "hash", "limit", "invalid UTF8"} {
		raw, _ := json.Marshal(d)
		var copy ManagedReviewRetryDirective
		if err := json.Unmarshal(raw, &copy); err != nil {
			t.Fatal(err)
		}
		switch mode {
		case "candidate":
			copy.CitationDiagnostic.Candidate = "foreign"
		case "hash":
			copy.CitationDiagnostic.ReplyDigest = "invalid"
		case "limit":
			copy.CitationDiagnostic.Issues[0].Originals[0].Text = strings.Repeat("x", managedReviewCitationTextLimit+1)
		case "invalid UTF8":
			copy.CitationDiagnostic.Issues[0].Originals[0].Text = "invalid \xff"
		}
		if _, err := managedReviewRecoveryPrefix("prefix", &copy); err == nil {
			t.Fatal("invalid diagnostic reached prompt", mode)
		}
	}
}

func finalCitationProtocolFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	provider := strings.Replace(fragmentRuntimeProviderFixture, "'evidence':c['tasks'][0]['report'][:24]", "'evidence':c['tasks'][0]['report'][:24]+'\\nWRONG literal tail'", 1)
	if provider == fragmentRuntimeProviderFixture {
		t.Fatal("provider counterexample unchanged")
	}
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(provider), 0700); err != nil {
		t.Fatal(err)
	}
	managedReviewMode(t, s, "questions-resolved")
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	_, _, callErr := s.runManagedFragmentReview(w, a, r)
	if callErr == nil || !strings.Contains(callErr.Error(), "résolution sans preuve originale") {
		t.Fatal("literal counterexample not observed", callErr)
	}
	if err := s.finishManagedFragmentReview(w.ID, a, r.ID, callErr); err == nil {
		t.Fatal("malformed final opinion accepted")
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "fixture.block", "blocked-final", w.Revision, []byte(`{}`), func(current *Work) error {
		task, _ := current.task(a.TaskID)
		task.Status = "blocked"
		for _, event := range current.Planning.Inbox {
			if event.ID == task.IndependentReview.ResultEvent {
				current.Planning.Inbox = []PlanningEvent{event}
				return nil
			}
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return s, w, a
}

func TestFinalCitationRecoveryReplacesLegacyContractAfterReopen(t *testing.T) {
	for _, mode := range []string{"operator", "supervisor"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := finalCitationProtocolFixture(t)
			original := *w.Tasks[0].IndependentReview
			calls := managedReviewCalls(t, s)
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			var after Work
			if mode == "operator" {
				after, err = reopened.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "operator-new-citation-contract", Revision: w.Revision, Task: a.TaskID, Reason: "The reference contract replaces literal copying; rerun a complete independent review of the same candidate.", ConfirmReviewErrorRepair: true})
			} else {
				var request PlanningRequest
				w, request = planningClaim(t, reopened, w, "root")
				request.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "The reference contract is available; review the complete candidate under the corrected method."}}
				after, err = reopened.planningChange(w.ID, "decide", request)
			}
			if err != nil {
				t.Fatal(err)
			}
			task, _ := after.task(a.TaskID)
			if task.IndependentReview != nil || len(task.PreviousReviews) == 0 || task.PreviousReviews[len(task.PreviousReviews)-1].ID != original.ID || task.PreviousReviews[len(task.PreviousReviews)-1].FragmentJournal.FinalJournalDigest != original.FragmentJournal.FinalJournalDigest || managedReviewCalls(t, reopened) != calls || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
				t.Fatal("new contract lost history, reused an opinion or spent a call")
			}
			for name, digest := range map[string]string{original.Context: original.ContextDigest, original.FragmentJournal.Plan: original.FragmentJournal.PlanDigest, original.FragmentJournal.Journal: original.FragmentJournal.JournalDigest, original.FragmentJournal.FinalJournal: original.FragmentJournal.FinalJournalDigest} {
				raw, err := os.ReadFile(filepath.Join(s.root, name))
				if err != nil || hash(raw) != digest {
					t.Fatal("historical original changed", name, err)
				}
			}
			if err = os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(managedReviewerFixture), 0700); err != nil {
				t.Fatal(err)
			}
			if err = reopened.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			finished, err := reopened.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			task, _ = finished.task(a.TaskID)
			if task.Status != "accepted" || task.IndependentReview == nil || task.IndependentReview.ID == original.ID || task.IndependentReview.CandidateSHA != original.CandidateSHA || managedReviewCalls(t, reopened) != calls+1 {
				t.Fatal("new contract did not reach acceptance on the unchanged candidate", task.Status, task.Blocker)
			}
			if err = reopened.independentReviewGuard(&finished, task); err != nil {
				t.Fatal(err)
			}

		})
	}
}

func TestCurrentReviewCitationConventionsBeforeFirstCall(t *testing.T) {
	c := fragmentPlanFixture()
	c.CitationContractVersion = 1
	if !strings.Contains(incrementalReviewPrefix("prefix", c), "CONVENTIONS DE CITATION") {
		t.Fatal("first review lacks exchange conventions")
	}
	c, p, replies, _, _ := fragmentContractQuestionFromContext(t, c)
	selection, err := managedFragmentRequestPrompt("", c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	decision, _, err := managedFragmentDecisionPrompt("", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	for _, prompt := range []string{selection, decision} {
		if !strings.Contains(prompt, "CONVENTIONS DE CITATION") {
			t.Fatal("first final exchange lacks literal contract")
		}
	}
	c.CitationContractVersion = 999
	if _, err := planManagedReviewFragments(c, 12, 2); err == nil {
		t.Fatal("unknown citation contract admitted")
	}
}

func TestCitationDiagnosticCapturedReplyIntegrityBlocksRecovery(t *testing.T) {
	s, w, a := finalCitationProtocolFixture(t)
	r := *w.Tasks[0].IndependentReview
	_, j, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		t.Fatal(err)
	}
	p, _, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		t.Fatal(err)
	}
	f, _, _, err := s.readManagedFragmentFinal(r, p, j)
	if err != nil {
		t.Fatal(err)
	}
	last := f.Calls[len(f.Calls)-1]
	matches, err := filepath.Glob(filepath.Join(s.root, filepath.Dir(r.Context), last.CallID+"-raw-*.json"))
	if err != nil || len(matches) != 1 {
		t.Fatal("raw original missing", err)
	}
	original, err := os.ReadFile(matches[0])
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(matches[0], append(append([]byte(nil), original...), byte(' ')), 0600); err != nil {
		t.Fatal(err)
	}
	before := managedReviewCalls(t, s)
	req := PlanningRequest{Schema: 1, EventID: "reject-altered-original", Revision: w.Revision, Task: a.TaskID, Reason: "Provide exact original context without changing candidate or acceptance."}
	if _, err := s.planningChange(w.ID, "retry-review", req); err == nil {
		t.Fatal("altered raw original admitted")
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if after.Revision != w.Revision || managedReviewCalls(t, s) != before {
		t.Fatal("failed diagnostic changed durable state or spending")
	}
}

func TestFinalCitationRetryMeasuresRemainingMessage(t *testing.T) {
	s, _, _, saved, _, _ := fragmentStoreFixture(t)
	c, p, j, f := finalJournalFixture(t)
	_, method, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		t.Fatal(err)
	}
	prefix := managedReviewPrefix(method)
	replies := make([]string, len(p.Packets))
	for _, entry := range j.Entries {
		replies[entry.Packet] = entry.Reply
	}
	selection, err := managedFragmentRequestPrompt(prefix, c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	f.Calls[0].PromptDigest = hash([]byte(selection))
	decision, _, err := managedFragmentDecisionPrompt(prefix, c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	f.Calls[1].PromptDigest = hash([]byte(decision))
	f.Calls[1].State, f.Calls[1].Reply, f.Calls[1].ReplyDigest = "interrupted", "", ""
	if err = os.MkdirAll(filepath.Join(s.root, "citation-capacity"), 0700); err != nil {
		t.Fatal(err)
	}
	write := func(name string, value any) (string, string) {
		t.Helper()
		raw, e := json.Marshal(value)
		if e != nil {
			t.Fatal(e)
		}
		path := filepath.Join("citation-capacity", name)
		if e = atomicWrite(filepath.Join(s.root, path), raw); e != nil {
			t.Fatal(e)
		}
		return filepath.ToSlash(path), hash(raw)
	}
	r := IndependentReview{ID: "capacity-review", Attempt: j.Attempt, CandidateSHA: c.Candidate, BatchProviderDigest: j.ProviderDigest, FragmentJournal: &ManagedFragmentJournalAnchor{WorkflowDigest: saved.FragmentJournal.WorkflowDigest}}
	r.Context, r.ContextDigest = write("context.json", c)
	r.FragmentJournal.Plan, r.FragmentJournal.PlanDigest = write("plan.json", p)
	r.FragmentJournal.Journal, r.FragmentJournal.JournalDigest = write("journal.json", j)
	r.FragmentJournal.FinalJournal, r.FragmentJournal.FinalJournalDigest = write("final.json", f)
	d := citationDirectiveFixture(t)
	d.Review, d.Candidate, d.Attempt = r.ID, c.Candidate, j.Attempt
	d.CitationDiagnostic.Review, d.CitationDiagnostic.Candidate = r.ID, c.Candidate
	d.Cause, d.Instruction = strings.Repeat("a", 3900), strings.Repeat("b", 1900)
	issue := d.CitationDiagnostic.Issues[0]
	issue.Originals = nil
	for i := 0; i < managedFragmentDefectLimit; i++ {
		issue.Originals = append(issue.Originals, managedReviewCitationOriginal{Name: strings.Repeat("😀", managedReviewCitationTextLimit), Digest: strings.Repeat("c", 64), Line: 1, Text: strings.Repeat("x", managedReviewCitationTextLimit)})
	}
	d.CitationDiagnostic.Issues = nil
	for i := 0; i < managedFragmentDefectLimit; i++ {
		d.CitationDiagnostic.Issues = append(d.CitationDiagnostic.Issues, issue)
	}
	callPrefix, err := managedReviewRecoveryPrefix(prefix, d)
	if err != nil {
		t.Fatal(err)
	}
	if err = preflightManagedFragmentCalls(callPrefix, c, p); err == nil {
		t.Fatal("counterexample must exceed a completed inspection")
	}
	if _, _, err = managedFragmentDecisionPrompt(callPrefix, c, p, replies, nil); err != nil {
		t.Fatal("remaining actual decision must fit", err)
	}
	if err = s.preflightReviewRetryDirective(r, d); err != nil {
		t.Fatal("completed inspections blocked a fitting final retry", err)
	}
	raw, err := os.ReadFile(filepath.Join(s.root, r.FragmentJournal.FinalJournal))
	if err != nil || hash(raw) != r.FragmentJournal.FinalJournalDigest {
		t.Fatal("preflight changed historical final journal", err)
	}
	for _, stage := range []string{"ready", "selection"} {
		next := f
		next.Calls = append([]managedFragmentFinalCall(nil), f.Calls[:1]...)
		if stage == "selection" {
			next.Calls[0].State, next.Calls[0].Reply, next.Calls[0].ReplyDigest = "interrupted", "", ""
		}
		r.FragmentJournal.FinalJournal, r.FragmentJournal.FinalJournalDigest = write("final-"+stage+".json", next)
		if err = s.preflightReviewRetryDirective(r, d); err != nil {
			t.Fatal("fitting remaining stage rejected", stage, err)
		}
	}
	refs := []managedFragmentEvidenceRef{}
	limited := false
	for i, packet := range p.Packets {
		for k, artifact := range packet.Artifacts {
			if artifact.Kind != "diff" {
				continue
			}
			refs = append(refs, managedFragmentEvidenceRef{i, k, artifact.Digest})
			if _, _, err = managedFragmentDecisionPrompt(callPrefix, c, p, replies, refs); err == nil {
				continue
			}
			oldPrompt, _, e := managedFragmentDecisionPrompt(prefix, c, p, replies, refs)
			if e != nil {
				t.Fatal("overflow fixture did not fit before recovery", e)
			}
			var request managedFragmentFinalRequest
			if e = json.Unmarshal([]byte(f.Calls[0].Reply), &request); e != nil {
				t.Fatal(e)
			}
			request.References = refs
			reply, _ := json.Marshal(request)
			next := f
			next.Calls = append([]managedFragmentFinalCall(nil), f.Calls...)
			next.Calls[0].Reply, next.Calls[0].ReplyDigest = string(reply), hash(reply)
			next.Calls[1].PromptDigest = hash([]byte(oldPrompt))
			r.FragmentJournal.FinalJournal, r.FragmentJournal.FinalJournalDigest = write("final-overflow.json", next)
			if e = s.preflightReviewRetryDirective(r, d); e == nil {
				t.Fatal("oversized actual decision admitted")
			}
			limited = true
			break
		}
		if limited {
			break
		}
	}
	if !limited {
		t.Fatal("actual decision overflow was not exercised")
	}
	r.FragmentJournal.FinalJournal, r.FragmentJournal.FinalJournalDigest = write("final.json", f)
	if err = os.WriteFile(filepath.Join(s.root, r.FragmentJournal.Journal), []byte("{}"), 0600); err != nil {
		t.Fatal(err)
	}
	if err = s.preflightReviewRetryDirective(r, d); err == nil {
		t.Fatal("completed inspection integrity was bypassed")
	}
}
