//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const referencedFragmentDecisionFixture = `elif '\nSWARM_FRAGMENT_FINAL_EVIDENCE\n' in text:
 payload=json.loads(text.split('\nSWARM_FRAGMENT_FINAL_EVIDENCE\n',1)[1]);c=payload['task_contracts_reports_controls'];catalog=payload['citation_catalog']
 schema=json.loads(sys.argv[sys.argv.index('--json-schema')+1]);allowed=set(schema['$defs']['original-reference']['enum'])
 def original_ref(name):
  spans=next(o[4] for o in catalog['originals'] if o[0]==name)
  return next(n if n>=0 else -1-n for n in spans if (n if n>=0 else -1-n) in allowed)
 reply={'candidate_commit':c['candidate_commit'],'tasks':[]}
 for task in c['tasks']:
  name='tasks/'+task['task']+'/'+(task['binding'].get('report') or 'report.md');reference=original_ref(name)
  if mode=='reference-invalid': reference=-1
  reply['tasks'].append({'task':task['task'],'reason':'Independent deterministic evaluation of supplied originals','criteria':[{'index':i+1,'verdict':'pass','evidence_ref':reference} for i,_ in enumerate(task['criteria'])]})
 qs=payload['partial_inspections'].get('unresolved_questions',[])
 if qs:
  reference=original_ref('tasks/'+c['tasks'][0]['task']+'/'+(c['tasks'][0]['binding'].get('report') or 'report.md'))
  reply={'review':reply,'resolutions':[dict({'packet':q['packet'],'artifact':q['artifact'],'need_index':q['need_index'],'verdict':'unknown' if mode=='questions-open' else 'resolved','reason':'Independent evaluation of the original supplied report'},**({'evidence':'Missing original evidence remains explicitly unknown'} if mode=='questions-open' else {'evidence_ref':reference})) for q in qs]}
 reply['catalog_sha256']='0'*64 if mode=='reference-stale' else payload['catalog_sha256']
`

func referencedFragmentRuntimeFixture(t *testing.T) (*Store, Work, Agent, managedReviewContext, string, []byte) {
	t.Helper()
	s, w, a, legacy, path, receipt := fragmentBeginFixture(t)
	c, err := s.managedReviewContext(w, a, legacy.Candidate, receipt)
	if err != nil || c.CitationContractVersion != 2 {
		t.Fatal("new reviews must select the reference contract", err)
	}
	provider := strings.Replace(fragmentRuntimeProviderFixture, "else:\n payload=json.loads(text.split('\\nSWARM_FRAGMENT_FINAL_EVIDENCE\\n',1)[1]);c=payload['task_contracts_reports_controls']", referencedFragmentDecisionFixture+"else:\n payload=json.loads(text.split('\\nSWARM_FRAGMENT_FINAL_EVIDENCE\\n',1)[1]);c=payload['task_contracts_reports_controls']", 1)
	if provider == fragmentRuntimeProviderFixture {
		t.Fatal("reference provider fixture was not installed")
	}
	if err = os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(provider), 0700); err != nil {
		t.Fatal(err)
	}
	return s, w, a, c, path, receipt
}

func TestReferencedCitationRuntimeReopenAndUnchangedRetryGuard(t *testing.T) {
	for _, mode := range []string{"pass", "questions-resolved", "questions-open", "reference-invalid", "reference-stale"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, c, path, receipt := referencedFragmentRuntimeFixture(t)
			managedReviewMode(t, s, mode)
			r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
			if err != nil {
				t.Fatal(err)
			}
			state, _, runErr := s.runManagedFragmentReview(w, a, r)
			finishErr := s.finishManagedFragmentReview(w.ID, a, r.ID, runErr)
			positive := mode == "pass" || mode == "questions-resolved"
			if positive && (runErr != nil || finishErr != nil || state != "passed") {
				t.Fatal(state, runErr, finishErr)
			}
			if !positive && finishErr == nil {
				t.Fatal("uncertain or invalid wire accepted")
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			current, err := reopened.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			task, _ := current.task(a.TaskID)
			saved := *task.IndependentReview
			before := managedReviewCalls(t, reopened)
			if positive {
				if err = reopened.managedReviewFilesIntact(saved); err != nil {
					t.Fatal(err)
				}
				profileFixture(t, reopened)
				if err = reopened.managedReviewFilesIntact(saved); err != nil {
					t.Fatal("historical prompt drift lost proof", err)
				}
				if managedReviewCalls(t, reopened) != before {
					t.Fatal("historical reading spent a call")
				}
				return
			}
			if mode == "questions-open" {
				if saved.State != "unknown" {
					t.Fatal(saved.State)
				}
				return
			}
			if saved.State != "error" || saved.FailureCode != reviewFragmentProtocolError {
				t.Fatal(saved.State, saved.FailureCode)
			}
			_, err = reopened.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "unchanged-reference-retry", Revision: current.Revision, Task: a.TaskID, Reason: "Only the wording changed; no new protocol or original proof.", ConfirmReviewErrorRepair: true})
			if err == nil || commandFailure(err).Code != "review_precondition_unchanged" {
				t.Fatal("unchanged retry was not refused", err)
			}
			after, _ := reopened.get(w.ID)
			if after.Revision != current.Revision || after.Planning.Reviewer.Calls != current.Planning.Reviewer.Calls || managedReviewCalls(t, reopened) != before {
				t.Fatal("rejected retry changed state or spending")
			}
			current, request := planningClaim(t, reopened, current, "root")
			request.Operations = []PlanningOperation{{Kind: "review-retry", ID: a.TaskID, Next: "Different words cannot establish a repaired precondition."}}
			if _, err = reopened.planningChange(w.ID, "decide", request); err == nil || commandFailure(err).Code != "review_precondition_unchanged" {
				t.Fatal("supervisor bypassed the same guard", err)
			}
			after, _ = reopened.get(w.ID)
			if after.Revision != current.Revision || managedReviewCalls(t, reopened) != before {
				t.Fatal("supervisor rejection changed state or spending")
			}
		})
	}
}

func TestReferencedCitationOrdinaryRetryRequiresChangedPrecondition(t *testing.T) {
	s, w := managedFixture(t)
	path := filepath.Join(s.root, "review-fixture/claude")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	fixture := strings.Replace(string(raw), "if mode=='wrong-sha':", "reply['tasks'][0]['criteria'][0]['evidence_ref']=-1\nif mode=='wrong-sha':", 1)
	if fixture == string(raw) {
		t.Fatal("invalid-reference fixture missing")
	}
	if err = os.WriteFile(path, []byte(fixture), 0700); err != nil {
		t.Fatal(err)
	}
	a := managedCompleted(t, s, w, "first", "candidate original\n")
	_ = s.integrateManagedAttempt(a)
	before, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := before.task(a.TaskID)
	if task.IndependentReview == nil || task.IndependentReview.State != "error" {
		t.Fatal("invalid reference did not fail")
	}
	calls := managedReviewCalls(t, s)
	_, err = s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "ordinary-unchanged-protocol", Revision: before.Revision, Task: a.TaskID, Reason: "Only wording changed; no corrected protocol or original.", ConfirmReviewErrorRepair: true})
	if err == nil || commandFailure(err).Code != "review_precondition_unchanged" {
		t.Fatal("ordinary path bypassed protocol recovery", err)
	}
	after, _ := s.get(w.ID)
	if after.Revision != before.Revision || managedReviewCalls(t, s) != calls {
		t.Fatal("rejected ordinary retry mutated state or spending")
	}
}

func TestReferencedCitationInstructionsAloneCannotRestart(t *testing.T) {
	s, w := managedFixture(t)
	path := filepath.Join(s.root, "review-fixture/claude")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	fixture := strings.Replace(string(raw), "if mode=='wrong-sha':", "reply['tasks'][0]['criteria'][0]['evidence_ref']=-1\nif mode=='wrong-sha':", 1)
	if err = os.WriteFile(path, []byte(fixture), 0700); err != nil {
		t.Fatal(err)
	}
	a := managedCompleted(t, s, w, "first", "candidate original\n")
	a.Ended = now()
	encoded, _ := json.Marshal(a)
	if _, err = s.db.Exec("UPDATE agents SET body=? WHERE id=?", encoded, a.ID); err != nil {
		t.Fatal(err)
	}
	_ = s.integrateManagedAttempt(a)
	before, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := before.task(a.TaskID)
	old := *task.IndependentReview
	profileFixture(t, s)
	calls := managedReviewCalls(t, s)
	preview, err := s.reviewRecoveryPreview(w.ID, a.TaskID)
	if err != nil || preview.State != "change_required" {
		t.Fatal(preview, err)
	}
	_, err = s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "wording-is-not-protocol-repair", Revision: before.Revision, Task: a.TaskID, Reason: "The project guidance changed but the reference contract is identical.", ConfirmReviewErrorRepair: true})
	if err == nil || commandFailure(err).Code != "review_precondition_unchanged" {
		t.Fatal("instructions alone authorized a new protocol expense", err)
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = after.task(a.TaskID)
	if after.Revision != before.Revision || task.IndependentReview == nil || task.IndependentReview.ID != old.ID || managedReviewCalls(t, s) != calls {
		t.Fatal("wording-only rejection changed history or spending")
	}

}
