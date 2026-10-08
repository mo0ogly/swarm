//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestFragmentDiagnosticNeverAcceptsOrRepeatsCall(t *testing.T) {
	s, w, a, c, path, receipt := fragmentBeginFixture(t)
	providerPath := filepath.Join(s.root, "review-fixture/claude")
	if e := os.WriteFile(providerPath, []byte(fragmentRuntimeProviderFixture), 0700); e != nil {
		t.Fatal(e)
	}
	managedReviewMode(t, s, "fail")
	r, e := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if e != nil {
		t.Fatal(e)
	}
	state, _, e := s.runManagedFragmentReview(w, a, r)
	if e == nil || state != "changes_requested" {
		t.Fatal(state, e)
	}
	w, e = s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	task, _ := w.task(a.TaskID)
	r = *task.IndependentReview
	r.State = "changes_requested"
	if e = s.saveManagedReview(w.ID, a, r); e != nil {
		t.Fatal(e)
	}
	script := `#!/usr/bin/env python3
import json,sys
p=json.loads(sys.stdin.read().split('SWARM_REVIEW_DIAGNOSTIC\n',1)[1])
r={'candidate_commit':p['candidate_commit'],'review_id':p['review_id'],'findings':[]}
for k in p['refused_artifacts']:
 r['findings'].append({'artifact':int(k),'conclusion':'needs_context','explanation':'The supplied excerpt alone cannot establish the alleged budget defect; the caller validation and concurrent reservation policy must also be examined.','reproduction':'Inspect the caller validation and reproduce competing reservations before claiming a defect.','evidence':''})
print(json.dumps({'type':'result','result':json.dumps(r)}))
`
	if e = os.WriteFile(providerPath, []byte(script), 0700); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	task, _ = w.task(a.TaskID)
	before, _ := json.Marshal(task)
	calls := w.Planning.Reviewer.Calls
	req := PlanningRequest{Schema: 1, EventID: "explain-refusal", Revision: w.Revision, Task: a.TaskID, ReviewID: r.ID, Reason: "Obtain an actionable diagnostic without overriding the refusal"}
	after, e := s.planningChange(w.ID, "diagnose-review", req)
	if e != nil {
		t.Fatal(e)
	}
	task, _ = after.task(a.TaskID)
	now, _ := json.Marshal(task)
	if string(now) != string(before) || after.Planning.Reviewer.Calls != calls+1 {
		t.Fatal("verdict changed or incorrect accounting")
	}
	again, e := s.planningChange(w.ID, "diagnose-review", req)
	if e != nil || again.Planning.Reviewer.Calls != calls+1 {
		t.Fatal("replay charged", e)
	}
	raw, _ := json.Marshal(req)
	rel := filepath.Join(s.root, filepath.Dir(r.Context), "review-diagnostic-"+hash(raw)[:24]+".json")
	saved, e := os.ReadFile(rel)
	if e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(string(saved), `"acceptance":false`) || !strings.Contains(string(saved), `"state":"completed"`) {
		t.Fatal(string(saved))
	}
	req.EventID = "invalid-source"
	req.Revision = again.Revision
	req.Inputs = []string{"../outside"}
	if _, e = s.planningChange(w.ID, "diagnose-review", req); e == nil {
		t.Fatal("invalid source accepted")
	}
	unchanged, _ := s.get(w.ID)
	if unchanged.Planning.Reviewer.Calls != calls+1 {
		t.Fatal("invalid source charged")
	}
	req.Inputs = nil
	_, e = s.mutate(w.ID, "test.exhaust-budget", "exhaust-budget", unchanged.Revision, []byte(`{}`), func(current *Work) error {
		current.Planning.Reviewer.MaxCalls = current.Planning.Reviewer.Calls
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	exhausted, _ := s.get(w.ID)
	req.EventID = "exhausted"
	req.Revision = exhausted.Revision
	if _, e = s.planningChange(w.ID, "diagnose-review", req); e == nil {
		t.Fatal("exhausted budget accepted")
	}
	unchanged, _ = s.get(w.ID)
	if unchanged.Planning.Reviewer.Calls != calls+1 {
		t.Fatal("exhausted budget charged")
	}
	req.EventID = "bad-review"
	req.ReviewID = "missing"
	req.Revision = again.Revision
	if _, e = s.planningChange(w.ID, "diagnose-review", req); e == nil {
		t.Fatal("unbound diagnostic accepted")
	}
}
func TestFragmentDiagnosticRejectsUnprovenDefect(t *testing.T) {
	r := IndependentReview{ID: "review", CandidateSHA: "candidate"}
	a := map[int]managedReviewFragmentArtifact{2: {Content: "original source excerpt"}}
	reply := fragmentDiagnosticReply{Candidate: r.CandidateSHA, Review: r.ID, Findings: []fragmentDiagnosticFinding{{Artifact: 2, Conclusion: "confirmed", Explanation: strings.Repeat("analysis ", 12), Reproduction: "A concrete reproducible scenario is required.", Evidence: "invented quote"}}}
	raw, _ := json.Marshal(reply)
	if parseFragmentDiagnostic(string(raw), r, a) == nil {
		t.Fatal("invented citation")
	}
	reply.Findings[0].Evidence = "original source"
	raw, _ = json.Marshal(reply)
	if e := parseFragmentDiagnostic(string(raw), r, a); e != nil {
		t.Fatal(e)
	}
	reply.Review = "other"
	raw, _ = json.Marshal(reply)
	if parseFragmentDiagnostic(string(raw), r, a) == nil {
		t.Fatal("wrong review")
	}
}
