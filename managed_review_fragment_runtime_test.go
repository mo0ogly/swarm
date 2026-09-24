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
if mode=='exit': sys.exit(9)
if '\nSWARM_FRAGMENT_PACKET\n' in text:
 packet=json.loads(text.split('\nSWARM_FRAGMENT_PACKET\n',1)[1])
 reply={'candidate_commit':packet['candidate_commit'],'context_sha256':packet['context_sha256'],'packet_sha256':re.search(r'packet_sha256=([a-f0-9]+)',text).group(1),'findings':[]}
 for i,a in enumerate(packet['artifacts']):
  reply['findings'].append({'artifact':i,'sha256':a['sha256'],'verdict':mode if mode in ('fail','unknown') else 'inspected','reason':'Fixture inspection','evidence':a['content'][:16],'needs':[]})
elif '\nSWARM_FRAGMENT_EVIDENCE_REQUEST\n' in text:
 payload=json.loads(text.split('\nSWARM_FRAGMENT_EVIDENCE_REQUEST\n',1)[1]);b=payload['partial_inspections']
 reply={'candidate_commit':b['candidate_commit'],'context_sha256':b['context_sha256'],'plan_sha256':b['plan_sha256'],'state':'unknown' if mode=='selection-unknown' else 'ready','reason':'Fixture selection of original evidence','references':[]}
else:
 payload=json.loads(text.split('\nSWARM_FRAGMENT_FINAL_EVIDENCE\n',1)[1]);c=payload['task_contracts_reports_controls']
 reply={'candidate_commit':c['candidate_commit'],'tasks':[{'task':t['task'],'reason':'Fixture original report and control evidence','criteria':[{'index':i+1,'verdict':'fail' if mode=='decision-fail' else 'pass','evidence':t['report'][:24]} for i,_ in enumerate(t['criteria'])]} for t in c['tasks']]}
print(json.dumps({'type':'result','result':json.dumps(reply)}))
`

func TestManagedFragmentRuntimeProviderProtocol(t *testing.T) {
	for _, mode := range []string{"pass", "fail", "unknown", "exit", "selection-unknown", "decision-fail"} {
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
			case "pass":
				if runErr != nil || state != "passed" || len(records) != len(c.Tasks) {
					t.Fatal(state, runErr)
				}
			case "fail", "unknown":
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
			// Existing durable evidence must not issue any second call on reentry.
			_, _, _ = s.runManagedFragmentReview(w, a, *task.IndependentReview)
			again, _ := s.get(w.ID)
			if managedReviewCalls(t, s)-before != wantCalls || again.Planning.Reviewer.Calls != after.Planning.Reviewer.Calls {
				t.Fatal("reentry spent more calls")
			}
		})
	}
}
