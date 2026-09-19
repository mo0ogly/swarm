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
