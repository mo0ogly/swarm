//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Exercise supervisor decision, persistence, subprocess receipt and durable
// effect in an isolated store. This transport double is not a real AI review.
func TestPlanningReviewRetryDeliversSupervisorCorrectionAfterReopen(t *testing.T) {
	s, w, a := planningProtocolReviewFixture(t)
	w, request := planningClaim(t, s, w, "root")
	instruction := "Correction de protocole : regrouper les défauts d'une même pièce dans une seule entrée artifact ; conserver la décision indépendante."
	before, _ := w.task(a.TaskID)
	original := *before.IndependentReview
	request.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: instruction}}
	after, err := s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	after, err = s.mutate(w.ID, "fixture.next", "unrelated-next", after.Revision, []byte(`{}`), func(current *Work) error {
		task, _ := current.task(a.TaskID)
		task.Next = "A later instruction for the producer must not replace the review correction."
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	provider := strings.Replace(fragmentRuntimeProviderFixture, "text=sys.stdin.read()", "text=sys.stdin.read()\nwith open(os.path.join(os.path.dirname(__file__),'received.jsonl'),'a') as f: f.write(json.dumps(text)+'\\n')\nif "+reviewRetryPythonLiteral(instruction)+" not in text: sys.exit(19)", 1)
	if err = os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(provider), 0700); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	task, _ := after.task(a.TaskID)
	active, review, err := reopened.activateManagedFragmentResume(w.ID, a, task.IndependentReview.ID)
	if err != nil {
		t.Fatal(err)
	}
	state, _, err := reopened.runManagedFragmentReview(active, a, review)
	if err != nil || state != "passed" {
		t.Fatalf("supervisor correction did not reach resumed reviewer: state=%s error=%v", state, err)
	}
	raw, err := os.ReadFile(filepath.Join(s.root, "review-fixture/received.jsonl"))
	if err != nil {
		t.Fatal(err)
	}
	for _, line := range strings.Split(strings.TrimSpace(string(raw)), "\n") {
		var prompt string
		if err = json.Unmarshal([]byte(line), &prompt); err != nil {
			t.Fatal(err)
		}
		for _, value := range []string{instruction, request.EventID, original.ID, original.CandidateSHA, original.ResultEvent} {
			if !strings.Contains(prompt, value) {
				t.Fatal("recovery context not attributed in actual received prompt", value)
			}
		}
	}
	current, _ := reopened.get(w.ID)
	task, _ = current.task(a.TaskID)
	plan, journal, err := reopened.readFragmentJournalAnchor(*task.IndependentReview)
	if err != nil {
		t.Fatal(err)
	}
	if len(journal.Entries) == 0 || journal.Entries[0].RetryDirective == nil || journal.Entries[0].RetryDirective.Decision != request.EventID || journal.Entries[0].PromptDigest == "" {
		t.Fatal("actual receipt is missing the decision or prompt digest")
	}
	final, context, prefix, err := reopened.readManagedFragmentFinal(*task.IndependentReview, plan, journal)
	if err != nil || len(final.Calls) < 2 {
		t.Fatal("durable final effect missing", err)
	}
	if _, _, err = validateManagedFragmentFinalJournal(final, journal, context, plan, prefix); err != nil {
		t.Fatal(err)
	}
	altered := *final.Calls[0].RetryDirective
	altered.Instruction = "Replace the supplied correction with a different instruction."
	final.Calls[0].RetryDirective = &altered
	if _, _, err = validateManagedFragmentFinalJournal(final, journal, context, plan, prefix); err == nil {
		t.Fatal("rewritten received instruction accepted with the old prompt digest")
	}
	if task.Status == "accepted" || task.PlanningRetry || len(task.Attempts) != len(before.Attempts) || current.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
		t.Fatal("recovery bypassed acceptance, queued production or changed candidate")
	}
	if current.Planning.Reviewer.Calls <= w.Planning.Reviewer.Calls {
		t.Fatal("actual subprocess calls were not reserved")
	}
}

func TestReviewRetryDirectiveForeignCandidateCannotReserve(t *testing.T) {
	s, w, a := planningProtocolReviewFixture(t)
	w, request := planningClaim(t, s, w, "root")
	request.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "Use one defect per failed artifact with an exact original quote."}}
	after, err := s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := after.task(a.TaskID)
	task.IndependentReview.FragmentJournal.RetryDirective.Candidate = "another-candidate"
	raw, _ := json.Marshal(after)
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	if _, _, err = s.activateManagedFragmentResume(w.ID, a, task.IndependentReview.ID); err == nil {
		t.Fatal("foreign correction activated")
	}
	current, _ := s.get(w.ID)
	if current.Revision != after.Revision || current.Planning.Reviewer.Calls != after.Planning.Reviewer.Calls {
		t.Fatal("rejected correction changed revision or spent a call")
	}
}

func TestFragmentRetryFeedbackDiagnosesDuplicateDefects(t *testing.T) {
	s := storeTest(t)
	packet, reply := inspectionReplyFixture()
	packet.Version = 2
	packet.Artifacts[0].Kind = "diff"
	packetRaw, _ := json.Marshal(packet)
	reply.PacketDigest = hash(packetRaw)
	reply.Findings[0].Verdict = "fail"
	defect := managedFragmentDefect{Artifact: 0, Line: 1, Quote: "original proof", Explanation: "An independently claimed defect in the original artifact content.", Reproduction: "Inspect the isolated original artifact at the claimed line.", Expected: "Keep a single defect entry for this artifact."}
	reply.Defects = []managedFragmentDefect{defect, defect}
	raw, _ := json.Marshal(reply)
	r := IndependentReview{Context: ".swarm/review/context.json"}
	journal := managedFragmentJournal{Entries: []managedFragmentJournalEntry{{Packet: packet.Index, PacketDigest: hash(packetRaw), CallID: "duplicate-defects", State: "interrupted"}}}
	path := filepath.Join(s.root, ".swarm/review", "duplicate-defects-raw-"+hash(raw)+".json")
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		t.Fatal(err)
	}
	if err := atomicWrite(path, raw); err != nil {
		t.Fatal(err)
	}
	before, _ := json.Marshal(journal)
	feedback := s.fragmentRetryFeedback(r, journal, packet)
	if !strings.Contains(feedback, "exactement UNE entrée") || !strings.Contains(feedback, `"defects_per_artifact":{"0":2}`) || !strings.Contains(feedback, `"failed_artifacts":[0]`) {
		t.Fatal("actual duplicate association not diagnosed", feedback)
	}
	if _, _, err := parseManagedFragmentInspection(string(raw), packet); err == nil {
		t.Fatal("malformed opinion accepted as a proof")
	}
	after, _ := json.Marshal(journal)
	if string(before) != string(after) {
		t.Fatal("diagnostic altered historical proof")
	}
}

func reviewRetryPythonLiteral(value string) string {
	raw, _ := json.Marshal(value)
	return string(raw)
}
