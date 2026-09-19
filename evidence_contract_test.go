//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestEvidenceContractReportClaimIsNotExecutedControl(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	report := "docs/t1.md"
	claim := []byte("npm test a réussi avec le code 0")
	if err := os.WriteFile(filepath.Join(s.root, report), claim, 0600); err != nil {
		t.Fatal(err)
	}
	raw := []byte(fmt.Sprintf(`{"method_version":"2","scope_id":"t1","artifacts":{%q:%q},"domains":{"quality":100},"checks":[{"id":"npm-test","domain":"quality","mandatory":true,"gate":"delivery","penalty":100,"max_penalty":100,"severity":"major"}],"results":[{"id":"npm-test","status":"PASS","count":0,"evidence":[%q]}]}`, report, hash(claim), report))
	w = gateTest(t, s, w, raw)
	attempt := latestAttemptID(&w.Tasks[0])
	mutation, _ := json.Marshal(map[string]string{"test": "report-review"})
	w, _ = s.mutate(w.ID, "test.report-review", newID("review-"), w.Revision, mutation, func(current *Work) error {
		task, _ := current.task("t1")
		task.IndependentReview = &IndependentReview{ID: "review-liar", Attempt: attempt, Reviewer: "reviewer://fixture", Report: report, Digest: hash(claim), Contract: reviewContract(task), State: "passed", Reason: "citation trouvée", Criteria: []ReviewCriterion{{Index: 1, Verdict: "pass", Evidence: string(claim)}}, Started: now(), Finished: now()}
		return nil
	})
	e := s.validationState(&w).Tasks["t1"].Evidence
	if e.ReportReview.State != "passed" || e.Controls.State != "unknown" || len(e.Controls.Items) != 1 || e.Controls.Items[0].Execution != "unknown" || e.Controls.Items[0].ExitCode != nil || len(e.Controls.Items[0].Command) != 0 {
		t.Fatalf("une citation a été transformée en exécution : %+v", e)
	}
	if e.Acceptance.State != "pending" {
		t.Fatalf("la revue ou la gate a accepté implicitement la tâche : %+v", e.Acceptance)
	}
}

func TestEvidenceContractExecutedControlAndStaleness(t *testing.T) {
	s, w, agent, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	s.conduct(agent, "completed")
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	e := s.validationState(&got).Tasks["t1"].Evidence
	if e.Controls.State != "passed" || e.Acceptance.State != "accepted" || e.Freshness != "fresh" || len(e.Controls.Items) != 1 {
		t.Fatalf("preuve moteur incomplète : %+v", e)
	}
	c := e.Controls.Items[0]
	if c.Execution != "executed" || strings.Join(c.Command, " ") != "go version" || c.ExitCode == nil || *c.ExitCode != 0 || c.Started == "unknown" || c.Finished == "unknown" || c.Revision == "unknown" {
		t.Fatalf("métadonnées d’exécution absentes : %+v", c)
	}
	// The shared-workspace automatic validation path never merges nor commits
	// a Git candidate: it must report candidate_sha as unknown, not fabricate
	// one from the business revision or from any other field.
	if c.CandidateSHA != "unknown" {
		t.Fatalf("SHA candidat inventé hors dépôt Git géré : %+v", c)
	}
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("preuve remplacée"), 0600); err != nil {
		t.Fatal(err)
	}
	stale := s.validationState(&got).Tasks["t1"].Evidence
	if stale.Freshness != "stale" || stale.Acceptance.State != "stale" || stale.Controls.Items[0].Freshness != "stale" {
		t.Fatalf("preuve périmée présentée comme actuelle : %+v", stale)
	}
}

func TestEvidenceContractDoesNotCallAnUnstartedCommandExecuted(t *testing.T) {
	s := storeTest(t)
	result := runValidationControl(s.root, ValidationControl{ID: "bad-dir", Command: []string{"go", "version"}, Dir: "missing", Timeout: 1})
	if result.Executed || result.Passed || result.Started == "" || result.Finished == "" {
		t.Fatalf("tentative non démarrée mal enregistrée : %+v", result)
	}
	task := Task{ID: "t1", Status: "submitted", Attempts: []Attempt{{ID: "attempt-bad"}}, AutoValidation: &AutomaticValidation{Attempt: "attempt-bad", Revision: 4, At: now(), Controls: []ValidationControlResult{result}}}
	work := Work{Revision: 5, Tasks: []Task{task}}
	e := s.taskEvidence(&work, &work.Tasks[0], false)
	if e.Controls.State != "failed" || e.Controls.Items[0].Execution != "not_executed" || e.Controls.Items[0].Result != "failed" {
		t.Fatalf("non-exécution présentée comme exécution : %+v", e.Controls)
	}
}

// TestEvidenceContractControlAggregationIsOrderIndependent covers the review
// finding that a later "unknown" historical control silently overwrote an
// earlier "failed" one, and that an empty Controls list defaulted to
// "passed" even though nothing was ever executed. failed must always win,
// unknown must never be masked, and passed requires at least one executed,
// passing control regardless of slice order.
func TestEvidenceContractControlAggregationIsOrderIndependent(t *testing.T) {
	s := storeTest(t)
	failed := ValidationControlResult{ID: "a", Executed: true, Passed: false, ExitCode: 1, Started: now(), Finished: now()}
	unknown := ValidationControlResult{ID: "b", Executed: false, Started: ""}
	passed := ValidationControlResult{ID: "c", Executed: true, Passed: true, ExitCode: 0, Started: now(), Finished: now()}

	for _, order := range [][]ValidationControlResult{{failed, unknown}, {unknown, failed}, {passed, unknown, failed}, {failed, passed}} {
		task := Task{ID: "t1", Status: "submitted", Attempts: []Attempt{{ID: "attempt-x"}},
			AutoValidation: &AutomaticValidation{Attempt: "attempt-x", Revision: 1, At: now(), Controls: order}}
		work := Work{Revision: 1, Tasks: []Task{task}}
		e := s.taskEvidence(&work, &work.Tasks[0], false)
		if e.Controls.State != "failed" {
			t.Fatalf("failed control masked by order %+v: got state %s", order, e.Controls.State)
		}
	}

	onlyUnknown := Task{ID: "t1", Status: "submitted", Attempts: []Attempt{{ID: "attempt-x"}},
		AutoValidation: &AutomaticValidation{Attempt: "attempt-x", Revision: 1, At: now(), Controls: []ValidationControlResult{unknown}}}
	work := Work{Revision: 1, Tasks: []Task{onlyUnknown}}
	if e := s.taskEvidence(&work, &work.Tasks[0], false); e.Controls.State != "unknown" {
		t.Fatalf("unknown-only controls must stay unknown, got %s", e.Controls.State)
	}
	if e := s.taskEvidence(&work, &work.Tasks[0], false); e.Controls.Items[0].ExitCode != nil {
		t.Fatalf("historical unknown execution must not report exit code 0 by default : %+v", e.Controls.Items[0])
	}

	empty := Task{ID: "t1", Status: "submitted", Attempts: []Attempt{{ID: "attempt-x"}},
		AutoValidation: &AutomaticValidation{Attempt: "attempt-x", Revision: 1, At: now(), Controls: nil}}
	work2 := Work{Revision: 1, Tasks: []Task{empty}}
	if e := s.taskEvidence(&work2, &work2.Tasks[0], false); e.Controls.State != "unknown" {
		t.Fatalf("an empty control list must never report passed, got %s", e.Controls.State)
	}

	onlyPassed := Task{ID: "t1", Status: "submitted", Attempts: []Attempt{{ID: "attempt-x"}},
		AutoValidation: &AutomaticValidation{Attempt: "attempt-x", Revision: 1, At: now(), Controls: []ValidationControlResult{passed}}}
	work3 := Work{Revision: 1, Tasks: []Task{onlyPassed}}
	if e := s.taskEvidence(&work3, &work3.Tasks[0], false); e.Controls.State != "passed" {
		t.Fatalf("all-passed controls must report passed, got %s", e.Controls.State)
	}
}

// TestEvidenceContractReviewerConfiguredWithoutVerdictIsNotNotConfigured
// covers the review finding that a reviewer configured for the work, but
// which has not produced any verdict yet for this attempt, was reported as
// not_configured — indistinguishable from no reviewer at all.
func TestEvidenceContractReviewerConfiguredWithoutVerdictIsNotNotConfigured(t *testing.T) {
	s := storeTest(t)
	task := Task{ID: "t1", Status: "submitted", Attempts: []Attempt{{ID: "attempt-x"}}}
	work := Work{Revision: 1, Tasks: []Task{task}, Planning: &PlanningState{Reviewer: &ReviewerConfig{Provider: "fixture", MaxCalls: 3}}}
	e := s.taskEvidence(&work, &work.Tasks[0], false)
	if e.ReportReview.State == "not_configured" {
		t.Fatalf("a work-level reviewer configuration must not be reported as not_configured : %+v", e.ReportReview)
	}
	if e.ReportReview.State != "pending" {
		t.Fatalf("unexpected report review state for a configured, not-yet-run reviewer : %+v", e.ReportReview)
	}

	noReviewer := Work{Revision: 1, Tasks: []Task{task}}
	e2 := s.taskEvidence(&noReviewer, &noReviewer.Tasks[0], false)
	if e2.ReportReview.State != "not_configured" {
		t.Fatalf("a work with no reviewer configuration must still report not_configured : %+v", e2.ReportReview)
	}
}

// TestEvidenceContractCandidateSHADistinctFromRevisionAndUnknownForLegacy
// covers R2: the Git commit actually tested must be readable on its own
// field, never collide with the business revision counter or the receipt
// timestamp, and stay "unknown" — not a fabricated or prose-parsed value —
// for a receipt recorded before this field existed.
func TestEvidenceContractCandidateSHADistinctFromRevisionAndUnknownForLegacy(t *testing.T) {
	s := storeTest(t)
	sha := "a1b2c3d4e5f60718293a4b5c6d7e8f901234567"
	executed := ValidationControlResult{ID: "a", Executed: true, Passed: true, ExitCode: 0, Started: now(), Finished: now()}
	task := Task{ID: "t1", Status: "submitted", Attempts: []Attempt{{ID: "attempt-x"}},
		AutoValidation: &AutomaticValidation{Attempt: "attempt-x", Revision: 7, CandidateSHA: sha, At: now(), Controls: []ValidationControlResult{executed}}}
	work := Work{Revision: 7, Tasks: []Task{task}}
	e := s.taskEvidence(&work, &work.Tasks[0], false)
	if e.Controls.Items[0].CandidateSHA != sha {
		t.Fatalf("SHA candidat non exposé : %+v", e.Controls.Items[0])
	}
	if e.Controls.Items[0].CandidateSHA == e.Controls.Items[0].Revision {
		t.Fatalf("SHA candidat confondu avec la révision métier : %+v", e.Controls.Items[0])
	}
	if e.Controls.Items[0].CandidateSHA == e.ObservedAt || e.Controls.Items[0].CandidateSHA == work.Tasks[0].AutoValidation.At {
		t.Fatalf("SHA candidat confondu avec un horodatage : %+v", e.Controls.Items[0])
	}

	legacy := Task{ID: "t1", Status: "submitted", Attempts: []Attempt{{ID: "attempt-x"}},
		AutoValidation: &AutomaticValidation{Attempt: "attempt-x", Revision: 7, At: now(), Controls: []ValidationControlResult{executed}}}
	legacyWork := Work{Revision: 7, Tasks: []Task{legacy}}
	le := s.taskEvidence(&legacyWork, &legacyWork.Tasks[0], false)
	if le.Controls.Items[0].CandidateSHA != "unknown" {
		t.Fatalf("ancien reçu sans SHA candidat n’est pas resté unknown : %+v", le.Controls.Items[0])
	}
}

func TestEvidenceContractSameTruthCLIAndWeb(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	w = gateTest(t, s, w, fixture(t, s.root))

	var stdout, stderr bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "work", "show", w.ID}, &stdout, &stderr); code != 0 {
		t.Fatalf("CLI (%d): %s", code, stderr.String())
	}
	var cli struct {
		Validation WorkValidation `json:"validation"`
	}
	if err := json.Unmarshal(stdout.Bytes(), &cli); err != nil {
		t.Fatal(err)
	}
	h := newWebHandler(s, "local.test", "token")
	req := httptest.NewRequest(http.MethodGet, "http://local.test/api/v1/task?work="+w.ID+"&task=t1", nil)
	req.Host = "local.test"
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "token"})
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, req)
	if rr.Code != http.StatusOK {
		t.Fatal(rr.Code, rr.Body.String())
	}
	var web struct {
		Evidence TaskEvidence `json:"evidence"`
		Review   string       `json:"review"`
	}
	if err := json.Unmarshal(rr.Body.Bytes(), &web); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(cli.Validation.Tasks["t1"].Evidence, web.Evidence) {
		t.Fatalf("vérités CLI/web divergentes :\nCLI %+v\nweb %+v", cli.Validation.Tasks["t1"].Evidence, web.Evidence)
	}
	for _, want := range []string{"PREUVES STRUCTURÉES", "exécution=unknown", "code de sortie=unknown", "Acceptation : pending"} {
		if !strings.Contains(web.Review, want) {
			t.Fatalf("détail web incomplet (%s) : %s", want, web.Review)
		}
	}
	stdout.Reset()
	if code := run([]string{"--root", s.root, "work", "show", w.ID}, &stdout, &stderr); code != 0 || !strings.Contains(stdout.String(), "exécution=unknown") || !strings.Contains(stdout.String(), "code=unknown") {
		t.Fatalf("détail CLI incomplet (%d) : %s / %s", code, stdout.String(), stderr.String())
	}
}
