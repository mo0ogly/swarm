//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestFragmentPartialNegativeRequiresWholeCandidateDecision(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	provider := fragmentRuntimeProviderFixture
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(provider), 0700); err != nil {
		t.Fatal(err)
	}
	managedReviewMode(t, s, "fail")
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	p, _, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		t.Fatal(err)
	}
	state, records, runErr := s.runManagedFragmentReview(w, a, r)
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := current.task(a.TaskID)
	_, journal, err := s.readFragmentJournalAnchor(*task.IndependentReview)
	if err != nil {
		t.Fatal(err)
	}
	if len(journal.Entries) != len(p.Packets) || task.IndependentReview.FragmentJournal.FinalJournalDigest == "" {
		t.Fatalf("partial finding closed review before whole-candidate decision: inspected %d/%d final=%q state=%s err=%v", len(journal.Entries), len(p.Packets), task.IndependentReview.FragmentJournal.FinalJournalDigest, state, runErr)
	}
	for _, entry := range journal.Entries {
		_, reply, parseErr := parseManagedFragmentInspection(entry.Reply, p.Packets[entry.Packet])
		if parseErr != nil {
			t.Fatal(parseErr)
		}
		if len(reply.Defects) == 0 {
			t.Fatal("original partial defect lost")
		}
	}
	if runErr != nil || state != "changes_requested" || len(records) != len(c.Tasks) {
		t.Fatal("final negative opinion missing", state, runErr)
	}
	if err = s.finishManagedFragmentReview(w.ID, a, r.ID, runErr); err == nil {
		t.Fatal("negative decision was accepted")
	}
	current, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = current.task(a.TaskID)
	if task.IndependentReview.State != "changes_requested" || !strings.Contains(task.IndependentReview.Reason, "Fixture") {
		t.Fatal("durable final opinion lost")
	}
	if task.Status == "accepted" {
		t.Fatal("partial inspection accepted task")
	}
}

func TestFragmentLocalNegativeCannotReplaceLaterTransportError(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	provider := strings.Replace(fragmentRuntimeProviderFixture, "if mode=='exit' or", "if (mode=='fail' and len(open(os.path.join(folder,'calls')).readlines())==2) or mode=='exit' or", 1)
	if provider == fragmentRuntimeProviderFixture {
		t.Fatal("transport failure injection missing")
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
		t.Fatal("transport failure missing")
	}
	if s.finishManagedFragmentReview(w.ID, a, r.ID, runErr) == nil {
		t.Fatal("transport failure accepted")
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := current.task(a.TaskID)
	if task.IndependentReview.State != "error" || !strings.Contains(task.IndependentReview.Reason, "PARTIEL") {
		t.Fatal("local defect replaced actual transport failure", task.IndependentReview.State, task.IndependentReview.Reason)
	}
	_, j, err := s.readFragmentJournalAnchor(*task.IndependentReview)
	if err != nil {
		t.Fatal(err)
	}
	if len(j.Entries) != 2 || j.Entries[0].State != "changes_requested" || j.Entries[1].State != "interrupted" {
		t.Fatal("original partial/transport boundaries lost", fmt.Sprint(j))
	}
}

func TestFragmentFinalReviewerCanRefuteLocalFindingWithOriginalEvidence(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	provider := strings.Replace(fragmentRuntimeProviderFixture, "'fail' if mode in ('decision-fail','fail') else 'pass'", "'fail' if mode=='decision-fail' else 'pass'", 1)
	provider = strings.Replace(provider, "'fail' if mode=='fail' else 'unknown' if mode=='questions-open' else 'resolved'", "'unknown' if mode=='questions-open' else 'resolved'", 1)
	provider = strings.Replace(provider, "text=sys.stdin.read()", `text=sys.stdin.read()
if '\nSWARM_FRAGMENT_FINAL_EVIDENCE\n' in text:
 with open(os.path.join(os.path.dirname(__file__),'received-final'),'w') as f: f.write(text)`, 1)
	if err := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(provider), 0700); err != nil {
		t.Fatal(err)
	}
	managedReviewMode(t, s, "fail")
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	state, records, runErr := s.runManagedFragmentReview(w, a, r)
	if runErr != nil || state != "passed" || len(records) != len(c.Tasks) {
		t.Fatal("engine replaced independent final reassessment", state, runErr)
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
	raw, err := os.ReadFile(filepath.Join(s.root, "review-fixture/received-final"))
	if err != nil {
		t.Fatal(err)
	}
	parts := strings.SplitN(string(raw), "\nSWARM_FRAGMENT_FINAL_EVIDENCE\n", 2)
	if len(parts) != 2 {
		t.Fatal("final payload absent")
	}
	var payload struct {
		Inspections managedFragmentFinalTransport `json:"partial_inspections"`
	}
	if err = json.Unmarshal([]byte(parts[1]), &payload); err != nil {
		t.Fatal(err)
	}
	if len(payload.Inspections.Defects) != len(p.Packets) || len(payload.Inspections.Questions) != len(p.Packets) {
		t.Fatal("actual final input lost adverse claims or mandatory resolutions")
	}
	for _, claim := range payload.Inspections.Defects {
		_, original, parseErr := parseManagedFragmentInspection(j.Entries[claim.Packet].Reply, p.Packets[claim.Packet])
		if parseErr != nil {
			t.Fatal(parseErr)
		}
		if len(original.Defects) != 1 || original.Defects[0] != claim.Defect {
			t.Fatal("original defect rewritten in final input")
		}
	}
	f, _, _, err := s.readManagedFragmentFinal(*task.IndependentReview, p, j)
	if err != nil {
		t.Fatal(err)
	}
	if hash(raw) != f.Calls[len(f.Calls)-1].PromptDigest {
		t.Fatal("received final input is not durable prompt")
	}
	calls := current.Planning.Reviewer.Calls
	if err = s.finishManagedFragmentReview(w.ID, a, r.ID, nil); err != nil {
		t.Fatal(err)
	}
	current, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = current.task(a.TaskID)
	if task.IndependentReview.State != "passed" || task.Status == "accepted" || current.Planning.Reviewer.Calls != calls {
		t.Fatal("finalization bypassed publication or changed consumption")
	}
}
