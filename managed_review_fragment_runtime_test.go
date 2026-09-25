//go:build linux

package main

import (
	"os"
	"path/filepath"
	"testing"
)

// Real subprocess transport, deterministic replies: not a model-quality test.
const fragmentRuntimeProviderFixture = `#!/usr/bin/env python3
import json,os,re,sys
text=sys.stdin.read()
folder=os.path.dirname(__file__)
with open(os.path.join(folder,'calls'),'a') as f: f.write('call\n')
try:
 with open(os.path.join(folder,'mode')) as f: mode=f.read().strip()
except FileNotFoundError: mode='pass'
if mode=='exit' or (mode=='exit-second' and len(open(os.path.join(folder,'calls')).readlines())==2): sys.exit(9)
if mode=='exit-selection' and '\nSWARM_FRAGMENT_EVIDENCE_REQUEST\n' in text: sys.exit(9)
if mode=='exit-decision' and '\nSWARM_FRAGMENT_FINAL_EVIDENCE\n' in text: sys.exit(9)
if '\nSWARM_FRAGMENT_PACKET\n' in text:
 packet=json.loads(text.split('\nSWARM_FRAGMENT_PACKET\n',1)[1])
 schema=json.loads(sys.argv[sys.argv.index('--json-schema')+1]);props=schema['properties']
 assert props['findings']['minItems']==len(packet['artifacts'])==props['findings']['maxItems']
 assert props['candidate_commit']['enum']==[packet['candidate_commit']]
 assert props['context_sha256']['enum']==[packet['context_sha256']]
 reply={'candidate_commit':packet['candidate_commit'],'context_sha256':packet['context_sha256'],'packet_sha256':re.search(r'packet_sha256=([a-f0-9]+)',text).group(1),'findings':[]}
 for i,a in enumerate(packet['artifacts']):
  reply['findings'].append({'artifact':i,'sha256':a['sha256'],'verdict':('unknown' if mode.startswith('questions-') and i==0 else mode if mode in ('fail','unknown') else 'inspected'),'reason':'Fixture inspection','evidence':a['content'][:16],'needs':['Confirm original content is present'] if mode.startswith('questions-') and i==0 else []})
elif '\nSWARM_FRAGMENT_EVIDENCE_REQUEST\n' in text:
 payload=json.loads(text.split('\nSWARM_FRAGMENT_EVIDENCE_REQUEST\n',1)[1]);b=payload['partial_inspections']
 reply={'candidate_commit':b['candidate_commit'],'context_sha256':b['context_sha256'],'plan_sha256':b['plan_sha256'],'state':'unknown' if mode=='selection-unknown' else 'ready','reason':'Fixture selection of original evidence','references':[]}
else:
 payload=json.loads(text.split('\nSWARM_FRAGMENT_FINAL_EVIDENCE\n',1)[1]);c=payload['task_contracts_reports_controls']
 reply={'candidate_commit':c['candidate_commit'],'tasks':[{'task':t['task'],'reason':'Fixture original report and control evidence','criteria':[{'index':i+1,'verdict':'fail' if mode=='decision-fail' else 'pass','evidence':t['report'][:24]} for i,_ in enumerate(t['criteria'])]} for t in c['tasks']]}
 if mode.startswith('questions-'):
  qs=payload['partial_inspections']['unresolved_questions']
  reply={'review':reply,'resolutions':[{'packet':q['packet'],'artifact':q['artifact'],'need_index':q['need_index'],'verdict':'unknown' if mode=='questions-open' else 'resolved','reason':'Fixture cites original visible report','evidence':c['tasks'][0]['report'][:24]} for q in qs]}
  if mode=='questions-omitted': reply['resolutions']=[]
print(json.dumps({'type':'result','result':json.dumps(reply)}))
`

func TestManagedFragmentRuntimeProviderProtocol(t *testing.T) {
	for _, mode := range []string{"pass", "fail", "unknown", "exit", "selection-unknown", "decision-fail", "questions-resolved", "questions-open", "questions-omitted"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, c, path, receipt := fragmentBeginFixture(t)
			if e := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(fragmentRuntimeProviderFixture), 0700); e != nil {
				t.Fatal(e)
			}
			managedReviewMode(t, s, mode)
			r, e := s.beginManagedFragmentReview(w, a, c, path, receipt)
			if e != nil {
				t.Fatal(e)
			}
			p, _, e := s.readFragmentJournalAnchor(r)
			if e != nil {
				t.Fatal(e)
			}
			before := managedReviewCalls(t, s)
			state, records, runErr := s.runManagedFragmentReview(w, a, r)
			wantCalls := len(p.Packets) + 2
			switch mode {
			case "pass", "questions-resolved":
				if runErr != nil || state != "passed" || len(records) != len(c.Tasks) {
					t.Fatal(state, runErr)
				}
			case "questions-open":
				if runErr != nil || state != "unknown" {
					t.Fatal(state, runErr)
				}
			case "questions-omitted":
				if runErr == nil || state == "passed" {
					t.Fatal("omitted reservations accepted", state, runErr)
				}
			case "unknown":
				wantCalls = len(p.Packets)
				if runErr == nil || state == "passed" {
					t.Fatal(state, runErr)
				}
			case "fail":
				wantCalls = 1
				if runErr == nil || state == "passed" {
					t.Fatal(state, runErr)
				}
			case "exit":
				wantCalls = 1
				if runErr == nil {
					t.Fatal("provider failure accepted")
				}
			case "selection-unknown":
				wantCalls = len(p.Packets) + 1
				if runErr != nil || state != "unknown" {
					t.Fatal(state, runErr)
				}
			case "decision-fail":
				if runErr != nil || state != "changes_requested" {
					t.Fatal(state, runErr)
				}
			}
			after, e := s.get(w.ID)
			if e != nil {
				t.Fatal(e)
			}
			task, _ := after.task(a.TaskID)
			if managedReviewCalls(t, s)-before != wantCalls || after.Planning.Reviewer.Calls-w.Planning.Reviewer.Calls != wantCalls {
				t.Fatal("wrong call count", managedReviewCalls(t, s)-before, wantCalls)
			}
			if task.Status == "accepted" {
				t.Fatal("runner bypassed publication")
			}
			claimed := *task.IndependentReview
			claimed.State = "passed"
			claimed.ManagedTasks = records
			for _, record := range records {
				if record.Task == a.TaskID {
					claimed.Criteria = record.Criteria
				}
			}
			proofErr := s.managedReviewFilesIntact(claimed)
			if mode == "pass" || mode == "questions-resolved" {
				if proofErr != nil {
					t.Fatal(proofErr)
				}
				if e = s.managedBatchPlanIntact(after, claimed); e != nil {
					t.Fatal(e)
				}
				forged := claimed
				forged.ManagedTasks = nil
				if s.managedReviewFilesIntact(forged) == nil {
					t.Fatal("invented final records accepted")
				}
				forged = claimed
				forged.Criteria = nil
				if s.managedReviewFilesIntact(forged) == nil {
					t.Fatal("missing criteria accepted")
				}
				anchor := *claimed.FragmentJournal
				anchor.FinalJournalDigest = "forged"
				forged = claimed
				forged.FragmentJournal = &anchor
				if s.managedReviewFilesIntact(forged) == nil {
					t.Fatal("corrupt final anchor accepted")
				}
				anchor = *claimed.FragmentJournal
				anchor.ModelConfigDigest = "changed-model"
				forged = claimed
				forged.FragmentJournal = &anchor
				if s.managedBatchPlanIntact(after, forged) == nil {
					t.Fatal("changed model accepted")
				}
				anchor = *claimed.FragmentJournal
				anchor.InspectionTask = ""
				forged.FragmentJournal = &anchor
				if s.managedReviewFilesIntact(forged) == nil {
					t.Fatal("missing origin accepted")
				}
			} else if proofErr == nil {
				t.Fatal("nonpassing provider reply published as passed")
			}
			// Existing durable evidence must not issue any second call on reentry.
			_, _, _ = s.runManagedFragmentReview(w, a, *task.IndependentReview)
			again, _ := s.get(w.ID)
			if managedReviewCalls(t, s)-before != wantCalls || again.Planning.Reviewer.Calls != after.Planning.Reviewer.Calls {
				t.Fatal("reentry spent more calls")
			}
			finishErr := s.finishManagedFragmentReview(w.ID, a, r.ID, runErr)
			if (mode == "pass" || mode == "questions-resolved") != (finishErr == nil) {
				t.Fatal("unexpected finalization", mode, finishErr)
			}
			finished, e := s.get(w.ID)
			if e != nil {
				t.Fatal(e)
			}
			ft, _ := finished.task(a.TaskID)
			if mode == "fail" && ft.IndependentReview.State != "changes_requested" {
				t.Fatal("proved inspection refusal lost", ft.IndependentReview.State)
			}
			if mode == "exit" && ft.IndependentReview.State != "error" {
				t.Fatal("transport failure became a refusal", ft.IndependentReview.State)
			}
			if ft.Status == "accepted" || finished.Planning.Reviewer.Calls != again.Planning.Reviewer.Calls {
				t.Fatal("finalization published or charged")
			}
			if (mode == "pass" || mode == "questions-resolved") && (ft.IndependentReview.State != "passed" || ft.IndependentReview.FragmentJournal.FinalJournalDigest != task.IndependentReview.FragmentJournal.FinalJournalDigest) {
				t.Fatal("lost final evidence")
			}
			if s.finishManagedFragmentReview(w.ID, a, r.ID, runErr) == nil {
				t.Fatal("finished twice")
			}

		})
	}
}
