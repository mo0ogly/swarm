//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func resultFor(t *testing.T, s *Store, w Work, agents []Agent) ResultPresentation {
	t.Helper()
	validation := s.validationState(&w).Tasks["t1"]
	return s.resultPresentation(&w, &w.Tasks[0], agents, validation)
}

func TestResultPresentationSeparatesProcessReportAndValidation(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	completed := Agent{ID: "agent-complete", Attempt: "attempt-complete", TaskID: "t1", Status: "completed", Started: now(), Activity: "Processus terminé"}

	got := resultFor(t, s, w, []Agent{completed})
	if got.State != "completed_unproven" || got.Label != "Terminé, résultat non démontré" || got.ValidationState == "fresh" {
		t.Fatalf("exit zero presented as delivery: %+v", got)
	}
	failedWithoutReport := completed
	failedWithoutReport.Status = "failed"
	failedWithoutReport.Activity = "Échec avant production du rapport"
	got = resultFor(t, s, w, []Agent{failedWithoutReport})
	if got.State != "stopped_early" || got.ReportState != "absent" {
		t.Fatalf("failed attempt without report: %+v", got)
	}

	docs := filepath.Join(s.root, "docs")
	if err := os.MkdirAll(docs, 0700); err != nil {
		t.Fatal(err)
	}
	report := filepath.Join(docs, "t1.md")
	if err := os.WriteFile(report, []byte("PASS — terminé, mais rapport volontairement incomplet"), 0600); err != nil {
		t.Fatal(err)
	}
	got = resultFor(t, s, w, []Agent{completed})
	if got.State != "result_to_review" || got.ReportState != "attributable_unsubmitted" || got.ValidationState == "fresh" {
		t.Fatalf("provider prose fabricated completeness: %+v", got)
	}

	running := completed
	running.Status = "running"
	running.Host = hostIdentity()
	running.Heartbeat = now()
	got = resultFor(t, s, w, []Agent{running})
	if got.State != "in_progress" || got.ReportState != "early_unverified" || !strings.Contains(got.Reason, "précoce ou incomplet") {
		t.Fatalf("early report hidden or promoted: %+v", got)
	}

	failed := completed
	failed.Status = "failed"
	failed.Activity = "Commande fournisseur terminée avec le code 7"
	code := 7
	failed.ExitCode = &code
	got = resultFor(t, s, w, []Agent{failed})
	if got.State != "stopped_early" || got.ReportState != "partial_possible" || got.ValidationState == "fresh" {
		t.Fatalf("failed attempt promoted by report: %+v", got)
	}

	interrupted := failed
	interrupted.Status = "interrupted"
	interrupted.Activity = "Arrêt demandé par l’opérateur"
	got = resultFor(t, s, w, []Agent{interrupted})
	if got.State != "stopped_early" || !strings.Contains(got.Reason, "opérateur") {
		t.Fatalf("interruption not distinguished: %+v", got)
	}
}

func TestResultPresentationDoesNotChooseEmptyOrAmbiguousReport(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	completed := Agent{ID: "agent-complete", Attempt: "attempt-complete", TaskID: "t1", Status: "completed", Started: now()}
	docs := filepath.Join(s.root, "docs")
	if err := os.MkdirAll(docs, 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(docs, "t1.md"), nil, 0600); err != nil {
		t.Fatal(err)
	}
	got := resultFor(t, s, w, []Agent{completed})
	if got.State != "completed_unproven" || !strings.Contains(got.Reason, "aucun rapport lisible et non vide") {
		t.Fatalf("empty report accepted: %+v", got)
	}
	if err := os.WriteFile(filepath.Join(docs, "t1.md"), []byte("premier"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(docs, "t1-handoff-second.md"), []byte("second"), 0600); err != nil {
		t.Fatal(err)
	}
	got = resultFor(t, s, w, []Agent{completed})
	if got.State != "completed_unproven" || !strings.Contains(got.Reason, "plusieurs rapports candidats") {
		t.Fatalf("ambiguous report selected: %+v", got)
	}
}

func TestResultPresentationCoversMissingStaleAndFreshGate(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(s.root, "docs", "t1.md"), []byte("rapport sans assertion de complétude"), 0600); err != nil {
		t.Fatal(err)
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed", Next: "Évaluer les preuves et la gate delivery ; handoff : docs/t1.md"})
	got := resultFor(t, s, w, nil)
	if got.State != "result_to_review" || got.ValidationState != "missing_gate" || got.ReportID != "docs/t1.md" {
		t.Fatalf("missing gate ambiguity: %+v", got)
	}

	w = gateTest(t, s, w, fixture(t, s.root))
	got = resultFor(t, s, w, nil)
	if got.State != "result_to_review" || got.ValidationState != "ready_for_decision" || got.GateID == "" {
		t.Fatalf("fresh gate before decision: %+v", got)
	}
	if err := os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("preuve modifiée"), 0600); err != nil {
		t.Fatal(err)
	}
	got = resultFor(t, s, w, nil)
	if got.State != "validation_withheld" || got.ValidationState != "failed_or_stale" || !strings.Contains(got.Reason, "Preuve modifiée") {
		t.Fatalf("stale evidence hidden: %+v", got)
	}

	if err := os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("proof"), 0600); err != nil {
		t.Fatal(err)
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
	got = resultFor(t, s, w, nil)
	if got.State != "validated" || got.ValidationState != "fresh" {
		t.Fatalf("fresh acceptance not validated: %+v", got)
	}
	if err := os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("dérive après acceptation"), 0600); err != nil {
		t.Fatal(err)
	}
	got = resultFor(t, s, w, nil)
	if got.State != "validation_stale" || got.Label == "Validé" {
		t.Fatalf("historical acceptance presented as current: %+v", got)
	}
}

func TestMissionStatusAndCLIUseSameResultPresentation(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)
	code := 0
	if err := s.finishAgent(a, "completed", "Processus terminé", &code); err != nil {
		t.Fatal(err)
	}
	d, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if len(d.Tasks) != 1 || d.Tasks[0].Result.State != "completed_unproven" || d.Tasks[0].State != "intervention" {
		t.Fatalf("mission projection: %+v", d.Tasks)
	}
	var out bytes.Buffer
	if err = missionCLI(s, []string{"mission", "status", w.ID}, "", false, &out); err != nil {
		t.Fatal(err)
	}
	for _, want := range []string{
		"Résultat : " + d.Tasks[0].Result.Label,
		"Processus : " + d.Tasks[0].Result.ProcessLabel,
		"rapport : " + d.Tasks[0].Result.ReportLabel,
		"validation : " + d.Tasks[0].Result.ValidationLabel,
	} {
		if !strings.Contains(out.String(), want) {
			t.Fatalf("CLI diverges from shared result %q:\n%s", want, out.String())
		}
	}

	const host, token = "127.0.0.1:18789", "q4-test-token"
	req := httptest.NewRequest(http.MethodGet, "http://"+host+"/api/v1/snapshot?work="+w.ID, nil)
	req.Host = host
	req.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token})
	recorder := httptest.NewRecorder()
	newWebHandler(s, host, token).ServeHTTP(recorder, req)
	if recorder.Code != http.StatusOK {
		t.Fatalf("web snapshot: %d %s", recorder.Code, recorder.Body.String())
	}
	var web struct {
		Mission MissionStatus `json:"mission"`
	}
	if err = json.Unmarshal(recorder.Body.Bytes(), &web); err != nil {
		t.Fatal(err)
	}
	if len(web.Mission.Tasks) != 1 || web.Mission.Tasks[0].Result != d.Tasks[0].Result {
		t.Fatalf("web/engine result mismatch: web=%+v engine=%+v", web.Mission.Tasks, d.Tasks)
	}
}

func TestSubmittedReportDoesNotInventProcessCompletion(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w.Tasks[0].Status = "submitted"
	p := resultFor(t, s, w, nil)
	if p.ProcessState != "unknown" || p.ProcessLabel != "Exécution non documentée" || p.ReportLabel == "" || p.ValidationLabel == "" {
		t.Fatalf("unproven execution: %+v", p)
	}
}
