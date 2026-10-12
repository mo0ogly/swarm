//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedLocalNegativeTransportRecoveryRetainsOpinionAndConsumption(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	provider := strings.Replace(fragmentRuntimeProviderFixture, "if mode=='exit' or", "if (mode=='fail' and len(open(os.path.join(folder,'calls')).readlines())==2) or mode=='exit' or", 1)
	if provider == fragmentRuntimeProviderFixture {
		t.Fatal("one-shot transport injection absent")
	}
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(provider), 0700); err != nil {
		t.Fatal(err)
	}
	managedReviewMode(t, s, "fail")
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	_, _, runErr := s.runManagedFragmentReview(w, a, r)
	if runErr == nil {
		t.Fatal("actual second call did not fail")
	}
	if s.finishManagedFragmentReview(w.ID, a, r.ID, runErr) == nil {
		t.Fatal("transport error accepted")
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := current.task(a.TaskID)
	p, j, err := s.readFragmentJournalAnchor(*task.IndependentReview)
	if err != nil {
		t.Fatal(err)
	}
	if len(j.Entries) != 2 || j.Entries[0].State != "changes_requested" || j.Entries[1].State != "interrupted" || task.IndependentReview.State != "error" {
		t.Fatal("wrong original boundary")
	}
	originalPath := task.IndependentReview.FragmentJournal.Journal
	originalBytes, err := os.ReadFile(filepath.Join(s.root, originalPath))
	if err != nil {
		t.Fatal(err)
	}
	preview, previewErr := s.reviewRecoveryPreview(w.ID, a.TaskID)
	if previewErr != nil {
		t.Errorf("local opinion wrongly prevents actual transport recovery preview: %v", previewErr)
	} else if preview.Reusable != 1 || preview.Required != len(p.Packets)-1+2 {
		t.Fatal("wrong recovery budget", preview)
	}
	beforeCalls := current.Planning.Reviewer.Calls
	after, err := s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "recover-one-shot-transport-after-local-finding", Revision: current.Revision, Task: a.TaskID, Reason: "The observed one-shot second-call transport failure has ended; preserve and reassess the local original defect through the full independent decision."})
	if err != nil {
		t.Fatal("local opinion wrongly prevents public transport recovery", err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	task, _ = after.task(a.TaskID)
	active, review, err := reopened.activateManagedFragmentResume(w.ID, a, task.IndependentReview.ID)
	if err != nil {
		t.Fatal(err)
	}
	state, _, runErr := reopened.runManagedFragmentReview(active, a, review)
	if runErr != nil || state != "changes_requested" {
		t.Fatal("local negative was lost or never independently finalized", state, runErr)
	}
	if reopened.finishManagedFragmentReview(w.ID, a, review.ID, nil) == nil {
		t.Fatal("confirmed final negative accepted")
	}
	final, err := reopened.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = final.task(a.TaskID)
	_, kept, err := reopened.readFragmentJournalAnchor(*task.IndependentReview)
	if err != nil {
		t.Fatal(err)
	}
	left, _ := json.Marshal(kept.Entries[0])
	right, _ := json.Marshal(j.Entries[0])
	if string(left) != string(right) {
		t.Fatal("completed negative observation rewritten")
	}
	originalNow, err := os.ReadFile(filepath.Join(s.root, originalPath))
	if err != nil || string(originalNow) != string(originalBytes) {
		t.Fatal("old journal lost", err)
	}
	want := len(p.Packets) - 1 + 2
	if final.Planning.Reviewer.Calls-beforeCalls != want || managedReviewCalls(t, reopened) != beforeCalls+want || task.Status == "accepted" || len(task.Attempts) != len(w.Tasks[0].Attempts) || task.PlanningRetry || task.IndependentReview.State != "changes_requested" {
		t.Fatal("recovery recharged completed finding, refunded failure, or bypassed final refusal")
	}
}

func TestFragmentRetryFeedbackSeparatesNeedsMismatchFromWrongQuote(t *testing.T) {
	s := storeTest(t)
	packet, reply := inspectionReplyFixture()
	reply.Findings[0].Needs = []string{"another original candidate source is needed"}
	raw, _ := json.Marshal(reply)
	packetRaw, _ := json.Marshal(packet)
	r := IndependentReview{Context: ".swarm/review/context.json"}
	j := managedFragmentJournal{Entries: []managedFragmentJournalEntry{{Packet: packet.Index, PacketDigest: hash(packetRaw), CallID: "needs-mismatch", State: "interrupted"}}}
	target := filepath.Join(s.root, ".swarm/review", "needs-mismatch-raw-"+hash(raw)+".json")
	if err := os.MkdirAll(filepath.Dir(target), 0700); err != nil {
		t.Fatal(err)
	}
	if err := atomicWrite(target, raw); err != nil {
		t.Fatal(err)
	}
	feedback := s.fragmentRetryFeedback(r, j, packet)
	if !strings.Contains(feedback, "n=[]") || !strings.Contains(feedback, "unknown") || !strings.Contains(feedback, "preuves manquantes") {
		t.Fatal("observed needs mismatch misdiagnosed as a quote error", feedback)
	}
	if _, _, err := parseManagedFragmentInspection(string(raw), packet); err == nil {
		t.Fatal("invalid inspected+needs accepted")
	}
}
