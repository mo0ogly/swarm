//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func completeDelivery(t *Task, a Agent) ManagedDelivery {
	d := ManagedDelivery{Version: 1, Task: t.ID, Attempt: a.Attempt, Contract: reviewContract(t), Outcome: "complete"}
	for i := range t.Criteria {
		d.Criteria = append(d.Criteria, DeliveryCriterion{Index: i + 1, Status: "pass", Reason: "Observed behavior tested", Controls: []string{t.ValidationPolicy.Controls[0].ID}, Evidence: []string{"value.txt"}})
	}
	return d
}

func TestManagedDeliveryIncompleteStopsBeforePaidReview(t *testing.T) {
	for _, name := range []string{"missing", "partial", "not_tested", "wrong_attempt", "wrong_contract", "unknown_control", "absent_evidence", "path_escape", "symlink", "missing_criterion", "malformed"} {
		t.Run(name, func(t *testing.T) {
			s, w := managedFixture(t)
			a := managedCompleted(t, s, w, "first", "work retained\n")
			a.DeliveryVersion = 1
			if e := s.saveAgent(a); e != nil {
				t.Fatal(e)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task(a.TaskID)
			d := completeDelivery(task, a)
			switch name {
			case "partial":
				d.Outcome = "partial"
			case "not_tested":
				d.Criteria[0].Status = "not_tested"
				d.Criteria[0].Reason = "Browser verification not performed"
			case "wrong_attempt":
				d.Attempt = "older-attempt"
			case "wrong_contract":
				d.Contract = "obsolete"
			case "unknown_control":
				d.Criteria[0].Controls = []string{"invented-command"}
			case "absent_evidence":
				d.Criteria[0].Evidence = []string{"missing.txt"}
			case "path_escape":
				d.Criteria[0].Evidence = []string{"../outside"}
			case "symlink":
				os.Symlink("value.txt", filepath.Join(a.CWD, "link.txt"))
				d.Criteria[0].Evidence = []string{"link.txt"}
			case "missing_criterion":
				d.Criteria = nil
			}
			if name != "missing" {
				b, _ := json.Marshal(d)
				if name == "malformed" {
					b = []byte(`{"version":`)
				}
				if e := os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), b, 0600); e != nil {
					t.Fatal(e)
				}
			}
			for range 2 {
				if e := s.integrateManagedAttempt(a); e != nil {
					t.Fatal(e)
				}
			}
			after, _ := s.get(w.ID)
			got, _ := after.task(a.TaskID)
			if got.Status != "blocked" || !strings.HasPrefix(got.Blocker, "Livraison incomplète :") || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
				t.Fatalf("partial result published or diagnosis lost: %+v", got)
			}
			if after.Planning.Reviewer.Calls != 0 || managedReviewCalls(t, s) != 0 {
				t.Fatal("incomplete delivery consumed reviewer budget")
			}
			if _, e := os.Stat(filepath.Join(a.CWD, "docs/first.md")); e != nil {
				t.Fatal("report lost", e)
			}
			reports := s.taskReportsForWork(w.ID, a.TaskID)
			if len(reports) != 1 {
				t.Fatal("incomplete report is not available for examination", reports)
			}
			report, e := os.ReadFile(filepath.Join(s.root, reports[0]))
			if e != nil || !strings.Contains(string(report), "Rapport nouveau first") {
				t.Fatal("wrong retained report", e)
			}
			if len(got.Attempts) != 1 {
				t.Fatal("precheck spawned a new attempt")
			}
		})
	}
}

func TestManagedCompleteDeliveryStillRequiresIndependentReview(t *testing.T) {
	for _, verdict := range []string{"pass", "fail"} {
		t.Run(verdict, func(t *testing.T) {
			s, w := managedFixture(t)
			managedReviewMode(t, s, verdict)
			a := managedCompleted(t, s, w, "first", "tested result\n")
			a.DeliveryVersion = 1
			if e := s.saveAgent(a); e != nil {
				t.Fatal(e)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task(a.TaskID)
			d := completeDelivery(task, a)
			b, _ := json.Marshal(d)
			os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), b, 0600)
			if e := s.integrateManagedAttempt(a); e != nil {
				t.Fatal(e)
			}
			after, _ := s.get(w.ID)
			if (after.Tasks[0].Status == "accepted") != (verdict == "pass") {
				t.Fatalf("producer declaration bypassed review: %+v", after.Tasks[0])
			}
			if managedReviewCalls(t, s) != 1 {
				t.Fatal("missing independent call")
			}
			observed, _ := os.ReadFile(filepath.Join(s.root, "review-fixture/observed.json"))
			if !strings.Contains(string(observed), `"delivery"`) || !strings.Contains(string(observed), `"attempt": "`+a.Attempt+`"`) {
				t.Fatal("reviewer did not receive bound delivery")
			}
		})
	}
}

func TestManagedLocalDeliveryCannotBypassFailingEngineControl(t *testing.T) {
	s, w := managedFixture(t)
	w, e := s.mutate(w.ID, "test.policy", "local-proof-policy", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].ValidationPolicy = automaticPolicy("git", "diff", "--exit-code", "--no-index", "value.txt", "missing.txt")
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	a := managedCompleted(t, s, w, "first", "local result\n")
	a.DeliveryVersion = 1
	if e = s.saveAgent(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	d := completeDelivery(task, a)
	d.Criteria[0].Reason = "Local check passed; engine validation and independent review are still pending"
	raw, _ := json.Marshal(d)
	if e = os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), raw, 0600); e != nil {
		t.Fatal(e)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	if after.Tasks[0].Status == "accepted" || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
		t.Fatal("local declaration bypassed failing engine check")
	}
	if managedReviewCalls(t, s) != 0 {
		t.Fatal("failed engine check consumed paid review")
	}
	files, e := filepath.Glob(filepath.Join(w.Planning.Repository.Storage, "diagnostics", "control-failure-*.json"))
	if e != nil || len(files) == 0 {
		t.Fatal("engine control did not execute", e)
	}
}

func TestManagedDeliveryPromptSeparatesLocalAndIndependentEvidence(t *testing.T) {
	_, w := managedFixture(t)
	task := &w.Tasks[0]
	prompt := managedDeliveryInstructions(task, "attempt-test")
	if !strings.Contains(prompt, deliveryEvidenceBoundary) {
		t.Fatal("worker evidence responsibilities absent")
	}
	encoded, _ := json.MarshalIndent(task.ValidationPolicy.Controls, "", "  ")
	if !strings.Contains(prompt, string(encoded)) {
		t.Fatal("local check options absent")
	}
	marker := "BILAN DE LIVRAISON REQUIS"
	suffix := prompt[strings.Index(prompt, marker):]
	var d ManagedDelivery
	if e := json.Unmarshal([]byte(suffix[strings.Index(suffix, "\n{")+1:]), &d); e != nil {
		t.Fatal(e)
	}
	if d.Outcome != "partial" || d.Criteria[0].Status != "not_tested" {
		t.Fatal("template must not claim an unexecuted success")
	}
	context, e := planningContext(w, "root")
	if e != nil {
		t.Fatal(e)
	}
	var value map[string]any
	if e = json.Unmarshal(context, &value); e != nil {
		t.Fatal(e)
	}
	if value["delivery_evidence_contract"] != deliveryEvidenceBoundary {
		t.Fatal("planner evidence responsibilities absent")
	}
}
