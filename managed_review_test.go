//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Real child-process protocol fixture. It validates the input supplied to the
// reviewer but is deterministic; it does not measure a model's review quality.
const managedReviewerFixture = `#!/usr/bin/env python3
import json, os, sys
text=sys.stdin.read()
if '\nSWARM_MANAGED_REVIEW_TEXT_CONTEXT\n' in text:
 import hashlib, io, re
 stream=io.BytesIO(text.split('\nSWARM_MANAGED_REVIEW_TEXT_CONTEXT\n',1)[1].encode())
 stream.readline()
 ctx=json.loads(stream.readline())
 while True:
  line=stream.readline()
  if not line: break
  h=json.loads(line); raw=stream.read(h['bytes'])
  assert len(raw)==h['bytes'] and hashlib.sha256(raw).hexdigest()==h['sha256']
  assert stream.read(1)==b'\n'
  if h['field']=='diff': ctx['diff']=raw.decode()
  else:
   m=re.fullmatch(r'(sources|tasks)\[(\d+)\]\.(content|report)',h['field']); assert m
   ctx[m[1]][int(m[2])][m[3]]=raw.decode()
else:
 ctx=json.loads(text.split('\nSWARM_MANAGED_REVIEW_CONTEXT\n',1)[1])
folder=os.path.dirname(__file__)
with open(os.path.join(folder,'calls'),'a') as f: f.write('call\n')
with open(os.path.join(folder,'observed.json'),'w') as f: json.dump({'context':ctx,'prompt':text,'args':sys.argv[1:],'cwd':os.getcwd(),'pid':os.getpid()},f)
mode='pass'
try:
 with open(os.path.join(folder,'mode')) as f: mode=f.read().strip()
except FileNotFoundError: pass
if mode=='exit': sys.exit(9)
reply={'candidate_commit':ctx['candidate_commit'],'tasks':[]}
for task in ctx['tasks']:
 assert task['controls'] and all(c['executed'] and c['passed'] and c['exit_code']==0 for c in task['controls'])
 assert ctx['candidate_commit'] == ctx['receipt']['candidate_commit']
 reply['tasks'].append({'task':task['task'],'reason':'Examen fixture du rapport et du reçu Git fourni','criteria':[{'index':i+1,'verdict':mode if mode in ('fail','unknown') else 'pass','evidence':task['report'][:80]} for i,_ in enumerate(task['criteria'])]})
if mode=='wrong-sha': reply['candidate_commit']='0'*40
if mode=='missing-task': reply['tasks']=[]
print(json.dumps({'type':'result','result':json.dumps(reply)}))
`

func managedReviewFixture(t *testing.T, s *Store, w Work) Work {
	t.Helper()
	folder := filepath.Join(s.root, "review-fixture")
	if e := os.MkdirAll(folder, 0700); e != nil {
		t.Fatal(e)
	}
	cmd := filepath.Join(folder, "claude")
	if e := os.WriteFile(cmd, []byte(managedReviewerFixture), 0700); e != nil {
		t.Fatal(e)
	}
	ps, e := s.providers()
	if e != nil {
		ps = Providers{Schema: 1, Providers: map[string]Provider{}}
	}
	ps.Providers["managed-review-fixture"] = Provider{Command: cmd}
	data, _ := json.Marshal(ps)
	if e = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), data, 0600); e != nil {
		t.Fatal(e)
	}
	cfg, e := s.reviewerConfig("managed-review-fixture", "auto", 20)
	if e != nil {
		t.Fatal(e)
	}
	w, e = s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	w.Planning.Reviewer = cfg
	w.Planning.ReviewerRequired = true
	data, _ = json.Marshal(w)
	if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", data, w.ID); e != nil {
		t.Fatal(e)
	}
	return w
}
func managedReviewMode(t *testing.T, s *Store, mode string) {
	t.Helper()
	if e := os.WriteFile(filepath.Join(s.root, "review-fixture/mode"), []byte(mode), 0600); e != nil {
		t.Fatal(e)
	}
}
func managedReviewCalls(t *testing.T, s *Store) int {
	t.Helper()
	b, _ := os.ReadFile(filepath.Join(s.root, "review-fixture/calls"))
	return strings.Count(string(b), "call\n")
}

func TestManagedIndependentReviewPublishesSameSHAAndSeparateProcess(t *testing.T) {
	s, w := managedFixture(t)
	base := w.Planning.Repository.Candidate
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task("first")
	r := task.IndependentReview
	if task.Status != "accepted" || r == nil || r.State != "passed" || r.CandidateSHA == base || r.CandidateSHA != task.AutoValidation.CandidateSHA || r.CandidateSHA != w.Planning.Repository.Candidate || r.Producer == r.Reviewer {
		t.Fatalf("unreviewed candidate: %+v", task)
	}
	if e := s.independentReviewGuard(&w, task); e != nil {
		t.Fatal(e)
	}
	data, e := os.ReadFile(filepath.Join(s.root, "review-fixture/observed.json"))
	if e != nil {
		t.Fatal(e)
	}
	var observed struct {
		Context managedReviewContext `json:"context"`
		Prompt  string               `json:"prompt"`
		Args    []string             `json:"args"`
		CWD     string               `json:"cwd"`
		PID     int                  `json:"pid"`
	}
	if e = json.Unmarshal(data, &observed); e != nil {
		t.Fatal(e)
	}
	assertWorkflowDelivery(t, observed.Prompt, "reviewer", r.Workflow)
	if observed.PID == os.Getpid() || observed.CWD == a.CWD || !strings.Contains(observed.Context.Diff, "reviewed") || len(observed.Context.Tasks) != 1 || !strings.Contains(strings.Join(observed.Args, " "), "--tools  --safe-mode") {
		t.Fatalf("review not isolated or missing content: %+v", observed)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	if e = reopened.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	if managedReviewCalls(t, s) != 1 {
		t.Fatal("paid call duplicated on replay/restart")
	}
}

func TestManagedIndependentReviewRejectsFailuresUnknownAndWrongSHA(t *testing.T) {
	for _, mode := range []string{"fail", "unknown", "exit", "wrong-sha", "missing-task"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			base := w.Planning.Repository.Candidate
			a := managedCompleted(t, s, w, "first", "unapproved\n")
			managedReviewMode(t, s, mode)
			if e := s.integrateManagedAttempt(a); e != nil {
				t.Fatal(e)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task("first")
			if task.Status == "accepted" || w.Planning.Repository.Candidate != base || task.IndependentReview == nil || task.IndependentReview.State == "passed" {
				t.Fatalf("invalid verdict published: %+v", task)
			}
			if e := s.integrateManagedAttempt(a); e != nil {
				t.Fatal(e)
			}
			if managedReviewCalls(t, s) != 1 {
				t.Fatal("failed review repeated automatically")
			}
		})
	}
}

func TestManagedIndependentReviewMissingOrBudgetExhaustedCannotPublish(t *testing.T) {
	for _, missing := range []bool{true, false} {
		t.Run(map[bool]string{true: "missing", false: "budget"}[missing], func(t *testing.T) {
			s, w := managedFixture(t)
			base := w.Planning.Repository.Candidate
			a := managedCompleted(t, s, w, "first", "unapproved\n")
			w, _ = s.get(w.ID)
			if missing {
				w.Planning.Reviewer = nil
				w.Planning.ReviewerRequired = false
			} else {
				w.Planning.Reviewer.Calls = w.Planning.Reviewer.MaxCalls
			}
			data, _ := json.Marshal(w)
			s.db.Exec("UPDATE works SET body=? WHERE id=?", data, w.ID)
			_ = s.integrateManagedAttempt(a)
			w, _ = s.get(w.ID)
			if w.Tasks[0].Status == "accepted" || w.Planning.Repository.Candidate != base || managedReviewCalls(t, s) != 0 {
				t.Fatal("missing/budget guard bypassed")
			}
		})
	}
}

func TestManagedIndependentReviewBindsReportsContractAndCandidate(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task("first")
	original := task.IndependentReview.CandidateSHA
	task.IndependentReview.CandidateSHA = w.Planning.Repository.Base
	if e := s.independentReviewGuard(&w, task); e == nil {
		t.Fatal("wrong revision accepted")
	}
	task.IndependentReview.CandidateSHA = original
	task.Criteria = append(task.Criteria, "new criterion")
	if e := s.independentReviewGuard(&w, task); e == nil {
		t.Fatal("new criterion accepted using old review")
	}
	w, _ = s.get(w.ID)
	task, _ = w.task("first")
	os.WriteFile(filepath.Join(s.root, task.IndependentReview.Receipt), []byte("changed"), 0600)
	if e := s.independentReviewGuard(&w, task); e == nil {
		t.Fatal("changed receipt accepted")
	}
}

// Simulate the crash boundary after a durable review result but before the
// atomic candidate/acceptance transaction. No provider result is fabricated.
func managedRewindBeforePublication(t *testing.T, s *Store, w Work, a Agent, state string) {
	t.Helper()
	task, _ := w.task(a.TaskID)
	w.Planning.Repository.Candidate = task.IndependentReview.PreviousCandidate
	task.Status = "blocked"
	task.AutoValidation = nil
	task.Gate = nil
	task.IndependentReview.State = state
	inbox := []PlanningEvent{}
	for _, event := range w.Planning.Inbox {
		if event.ID == planningEventID(a.ID, "integrated") || event.ID == planningEventID("integrated-"+a.ID, "validation", a.TaskID) || (state == "running" && event.ID == task.IndependentReview.ID) {
			continue
		}
		inbox = append(inbox, event)
	}
	w.Planning.Inbox = inbox
	if _, e := s.db.Exec("DELETE FROM events WHERE id=?", "integrated-"+a.ID); e != nil {
		t.Fatal(e)
	}
	if state == "running" {
		if _, e := s.db.Exec("DELETE FROM events WHERE id=?", task.IndependentReview.ID+"-result"); e != nil {
			t.Fatal(e)
		}
	}
	data, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", data, w.ID); e != nil {
		t.Fatal(e)
	}
	if _, e := s.db.Exec("UPDATE managed_attempts SET state='integrating' WHERE agent_id=?", a.ID); e != nil {
		t.Fatal(e)
	}
}
func TestManagedIndependentReviewRestartAfterVerdictDoesNotPayTwice(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	candidate := w.Planning.Repository.Candidate
	managedRewindBeforePublication(t, s, w, a, "passed")
	next, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer next.db.Close()
	if e = next.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = next.get(w.ID)
	if w.Tasks[0].Status != "accepted" || w.Planning.Repository.Candidate != candidate || managedReviewCalls(t, s) != 1 {
		t.Fatalf("durable verdict not reused atomically: %+v", w.Tasks[0])
	}
}
func TestManagedIndependentReviewInterruptedCallRequiresExplicitBoundedRetry(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	candidate := w.Planning.Repository.Candidate
	managedRewindBeforePublication(t, s, w, a, "running")
	next, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer next.db.Close()
	if e = next.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = next.get(w.ID)
	task, _ := w.task("first")
	if task.Status != "blocked" || task.IndependentReview.State != "error" || w.Planning.Repository.Candidate == candidate || managedReviewCalls(t, s) != 1 {
		t.Fatalf("interrupted review accepted or repeated: %+v calls=%d", task, managedReviewCalls(t, s))
	}
	request := PlanningRequest{Schema: 1, EventID: "explicit-managed-review-retry", Revision: w.Revision, Task: "first", Reason: "Fournisseur contrôlé après interruption du processus"}
	if _, e = next.retryIndependentReview(w.ID, request); e != nil {
		t.Fatal(e)
	}
	if e = next.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = next.get(w.ID)
	if w.Tasks[0].Status != "accepted" || w.Planning.Repository.Candidate != candidate || w.Planning.Reviewer.Calls != 2 || managedReviewCalls(t, s) != 2 {
		t.Fatalf("explicit retry not accounted: %+v", w.Tasks[0])
	}
}
func TestManagedIndependentReviewRechecksPreviouslyAcceptedTasksOnNewSHA(t *testing.T) {
	s, w := managedFixture(t)
	first := managedCompleted(t, s, w, "first", "first\n")
	if e := s.integrateManagedAttempt(first); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	prior := *w.Tasks[0].IndependentReview
	second := managedCompleted(t, s, w, "second", "second\n")
	if e := s.integrateManagedAttempt(second); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	for i := range w.Tasks {
		task := &w.Tasks[i]
		if task.Status != "accepted" || task.IndependentReview.CandidateSHA != w.Planning.Repository.Candidate || task.IndependentReview.CandidateSHA == prior.CandidateSHA {
			t.Fatalf("old approval reused: %+v", task)
		}
		if e := s.independentReviewGuard(&w, task); e != nil {
			t.Fatal(e)
		}
	}
	if len(w.Tasks[0].IndependentReview.ManagedTasks) != 2 || managedReviewCalls(t, s) != 2 {
		t.Fatal("cumulative review missing or charged per task")
	}
	w.Tasks[0].IndependentReview = &prior
	if e := s.independentReviewGuard(&w, &w.Tasks[0]); e == nil {
		t.Fatal("old SHA review remains valid")
	}
}
func TestManagedIndependentReviewLegacyFlagCannotBypassAcceptance(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	w.Planning.Reviewer = nil
	w.Planning.ReviewerRequired = false
	if s.acceptedFresh(&w, &w.Tasks[0], map[string]bool{}) {
		t.Fatal("legacy accepted state bypassed reviewer")
	}
}

func TestManagedIndependentReviewRefusesUnreviewableContextWithoutTruncation(t *testing.T) {
	for _, kind := range []string{"binary", "oversize"} {
		t.Run(kind, func(t *testing.T) {
			s, w := managedFixture(t)
			base := w.Planning.Repository.Candidate
			a := managedCompleted(t, s, w, "first", "reviewed\n")
			var path string
			var content []byte
			if kind == "binary" {
				path = filepath.Join(a.CWD, "value.txt")
				content = []byte{0, 1, 2, 3}
			} else {
				path = filepath.Join(a.CWD, "docs/first.md")
				content = []byte(strings.Repeat("A long complete evidence record.\n", 10000))
			}
			if e := os.WriteFile(path, content, 0600); e != nil {
				t.Fatal(e)
			}
			if e := s.integrateManagedAttempt(a); e != nil {
				t.Fatal(e)
			}
			w, _ = s.get(w.ID)
			if w.Tasks[0].Status == "accepted" || w.Planning.Repository.Candidate != base || managedReviewCalls(t, s) != 0 {
				t.Fatal("unreviewable context silently truncated or accepted")
			}
			if !strings.Contains(w.Tasks[0].Blocker, map[string]string{"binary": "binaire", "oversize": "192 Kio"}[kind]) {
				t.Fatal(w.Tasks[0].Blocker)
			}
		})
	}
}
