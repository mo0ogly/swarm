//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestEngineContractTruthReviewedReceiptWebCLI(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "verified value\n")
	managedReviewMode(t, s, "fail")
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task("first")
	r := task.IndependentReview
	if r == nil || r.State != "changes_requested" {
		t.Fatal("fixture review missing")
	}
	state := s.validationState(&w).Tasks["first"].Evidence
	if len(state.Controls.Items) != 1 {
		t.Fatal("rejected review lost executed control", state)
	}
	c := state.Controls.Items[0]
	if c.CandidateSHA != r.CandidateSHA || c.ExitCode == nil || *c.ExitCode != 0 || len(c.Command) == 0 || c.Started == "unknown" || c.Finished == "unknown" || state.Acceptance.State == "accepted" {
		t.Fatal("false or incomplete evidence", state)
	}
	before, _ := json.Marshal(w)
	for _, lang := range []string{"fr", "en"} {
		t.Setenv("SWARM_LANG", lang)
		var cli bytes.Buffer
		if e := missionCLI(s, []string{"mission", "status", w.ID}, "", false, &cli); e != nil {
			t.Fatal(e)
		}
		for _, value := range []string{r.CandidateSHA, r.Producer, r.Reviewer, c.Started, c.Finished, strings.Join(c.Command, " "), "changes_requested"} {
			if !strings.Contains(cli.String(), value) {
				t.Fatalf("%s CLI omitted %s", lang, value)
			}
		}
		if lang == "en" && (!strings.Contains(cli.String(), "STRUCTURED EVIDENCE") || strings.Contains(cli.String(), "PREUVES STRUCTURÉES")) {
			t.Fatal(cli.String())
		}
		snap, e := s.cockpitSnapshot(w.ID)
		if e != nil {
			t.Fatal(e)
		}
		raw, _ := json.Marshal(snap)
		for _, value := range []string{r.CandidateSHA, r.Producer, r.Reviewer, c.Started, c.Finished, "changes_requested"} {
			if !bytes.Contains(raw, []byte(value)) {
				t.Fatalf("web omitted %s", value)
			}
		}
	}
	after, _ := s.get(w.ID)
	raw, _ := json.Marshal(after)
	if !bytes.Equal(before, raw) {
		t.Fatal("read-only language switch changed work")
	}
	if os.Getenv("SWARM_E5_UI_EXPORT") == "1" {
		if err := s.pause(w.ID, true); err != nil {
			t.Fatal(err)
		}
		dir, err := os.MkdirTemp("/tmp", "swarm-e5-ui-")
		if err != nil {
			t.Fatal(err)
		}
		s.db.Exec("PRAGMA wal_checkpoint(TRUNCATE)")
		if b, err := exec.Command("cp", "-a", s.root+"/.", dir).CombinedOutput(); err != nil {
			t.Fatal(err, string(b))
		}
		t.Log("BROWSER_FIXTURE", dir, w.ID)
	}
	// An altered receipt must never retain claimed command execution facts.
	os.WriteFile(filepath.Join(s.root, r.Receipt), []byte("tampered"), 0600)
	if got := s.taskEvidence(&w, task, false); got.Controls.State != "unknown" {
		t.Fatal("tampered receipt reported executed", got)
	}
}

func TestEngineContractTruthProcessExitIsNotAcceptance(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	a := Agent{ID: "producer", Attempt: "attempt", TaskID: "t1", Status: "completed", Started: now()}
	got := resultFor(t, s, w, []Agent{a})
	if got.State != "completed_unproven" || got.ReportState != "absent_or_unattributed" || got.ValidationState == "fresh" {
		t.Fatal(got)
	}
	if got := s.taskEvidence(&w, &w.Tasks[0], false); got.Acceptance.State == "accepted" || got.ReportReview.State != "not_configured" {
		t.Fatal(got)
	}
}

func TestEngineContractTruthConfiguredReviewerIsPending(t *testing.T) {
	s, w := managedFixture(t)
	got := s.taskEvidence(&w, &w.Tasks[0], false)
	if got.ReportReview.State != "pending" || got.ReportReview.Reviewer == "unknown" {
		t.Fatal(got)
	}
	for _, limit := range got.ReportReview.Limits {
		if strings.Contains(limit, "Aucune revue indépendante du rapport n’est configurée") {
			t.Fatal("configured reviewer reported absent")
		}
	}
}
