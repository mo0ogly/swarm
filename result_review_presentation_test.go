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

func TestManagedReviewFailurePresentationCLIAndHTTP(t *testing.T) {
	for _, mode := range []string{"exit", "fail", "unknown"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			base := w.Planning.Repository.Candidate
			a := managedCompleted(t, s, w, "first", "candidate\n")
			managedReviewMode(t, s, mode)
			if err := s.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task("first")
			r := task.IndependentReview
			if r == nil || r.Report == "" {
				t.Fatal("fixture has no review evidence")
			}
			reports := s.taskReportsForWork(w.ID, task.ID)
			if len(reports) == 0 || reports[0] != r.Report {
				t.Fatal("reviewed report missing from task actions", reports)
			}
			if task.Status != "blocked" || w.Planning.Repository.Candidate != base {
				t.Fatal("rejected review published")
			}
			d, err := s.missionStatus(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			p := d.Tasks[0].Result
			if p.State != "review_blocked" || p.ReportID != r.Report || p.ReportState != "submitted" || p.ReceiptID != r.Receipt || p.ValidationState == "fresh" {
				t.Fatalf("review/report evidence hidden or accepted: %+v", p)
			}
			if d.Tasks[0].State != "intervention" || d.Tasks[0].Understanding.What != p.Reason {
				t.Fatalf("review cause not actionable: %+v", d.Tasks[0])
			}
			if mode == "exit" && p.ValidationState != "review_unvalidated" {
				t.Fatal("provider error described as failed tests", p)
			}
			var cli bytes.Buffer
			if err := missionCLI(s, []string{"mission", "status", w.ID}, "", false, &cli); err != nil {
				t.Fatal(err)
			}
			if !strings.Contains(cli.String(), p.Label) || !strings.Contains(cli.String(), p.ReportLabel) {
				t.Fatal(cli.String())
			}
			const host, token = "127.0.0.1:18999", "review-presentation-fixture"
			req := httptest.NewRequest(http.MethodGet, "http://"+host+"/api/v1/snapshot?work="+w.ID, nil)
			req.Host = host
			req.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token})
			out := httptest.NewRecorder()
			newWebHandler(s, host, token).ServeHTTP(out, req)
			var web struct {
				Mission MissionStatus `json:"mission"`
			}
			if out.Code != 200 || json.Unmarshal(out.Body.Bytes(), &web) != nil {
				t.Fatal(out.Body.String())
			}
			if web.Mission.Tasks[0].Result != p {
				t.Fatal("HTTP and CLI facts diverge")
			}
			after, _ := s.get(w.ID)
			if after.Revision != w.Revision || after.Planning.Repository.Candidate != base || after.Planning.Reviewer.Calls != 1 || len(after.Tasks[0].Attempts) != 1 {
				t.Fatal("read-only presentation changed mission or charged another call")
			}
			for _, state := range []string{"running", "passed", "stale"} {
				r.State = state
				p = s.resultPresentation(&w, task, []Agent{a}, TaskValidation{})
				if p.ReportID != r.Report || p.ValidationState == "fresh" || !strings.HasPrefix(p.State, "review_") {
					t.Fatalf("review %s misrepresented: %+v", state, p)
				}
				if state == "running" && p.State != "review_in_progress" {
					t.Fatal(p)
				}
				if state == "passed" && p.State != "review_awaiting_publication" {
					t.Fatal("opinion became acceptance", p)
				}
			}
			r.State, r.Reason = "error", "délai du planificateur dépassé :"
			p = s.resultPresentation(&w, task, []Agent{a}, TaskValidation{})
			if p.Reason != "Le vérificateur indépendant n’a pas répondu dans le délai imparti." {
				t.Fatal(p)
			}
			r.State = "passed"
			contract := r.Contract
			r.Contract = "old-contract"
			p = s.resultPresentation(&w, task, []Agent{a}, TaskValidation{})
			if p.State != "review_blocked" || p.Label != "Avis périmé" {
				t.Fatal("stale contract hidden", p)
			}
			r.Contract = contract
			if err := os.WriteFile(filepath.Join(s.root, r.Report), []byte("changed"), 0600); err != nil {
				t.Fatal(err)
			}
			p = s.resultPresentation(&w, task, []Agent{a}, TaskValidation{})
			if p.ReportState != "stale" || p.ValidationState != "failed_or_stale" || p.State != "review_blocked" {
				t.Fatalf("changed report shown as current: %+v", p)
			}
			// An earlier attempt's review must not describe a new attempt.
			task.Attempts = append(task.Attempts, Attempt{ID: "new-attempt"})
			a.Attempt = "new-attempt"
			p = s.resultPresentation(&w, task, []Agent{a}, TaskValidation{})
			if strings.HasPrefix(p.State, "review_") {
				t.Fatal("old review attributed to new attempt")
			}
		})
	}
}

func TestManagedReviewPresentationEnglishLabels(t *testing.T) {
	t.Setenv("SWARM_LANG", "en")
	for _, source := range []string{
		"Revue indépendante en cours", "Publication non confirmée",
		"Le rapport est conservé ; le vérificateur indépendant examine le candidat testé.",
		"Le rapport transmis au vérificateur est indisponible ou modifié ; ses preuves doivent être réexaminées.",
	} {
		if uiEngineText(source) == source {
			t.Fatalf("untranslated review label %q", source)
		}
	}
	if got := uiEngineText("Revue indépendante : preserved technical cause"); got != "Independent review: preserved technical cause" {
		t.Fatal(got)
	}
}

func TestTaskUnderstandingUsesTerminalCauseBeforeHistoricalToolErrors(t *testing.T) {
	for _, tc := range []struct {
		name, result, agent string
		limit               bool
	}{
		{"tool limit", "stopped_early", "interrupted", true},
		{"completed", "completed_unproven", "completed", false},
		{"review", "review_blocked", "completed", false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			task := MissionTask{ID: "task", State: "intervention", Reason: "Actual terminal reason", Result: ResultPresentation{State: tc.result, Reason: "Actual terminal reason", NextStep: "Inspect the actual result"}}
			a := Agent{TaskID: "task", Status: tc.agent, Diagnostic: AttemptDiagnostic{LimitReached: tc.limit, Items: []DiagnosticItem{{Category: "configuration", Cause: "An optional file was missing"}}}}
			u := taskUnderstanding(task, MissionStatus{}, []Agent{a})
			if u.What != task.Reason {
				t.Fatalf("historical error overrides terminal cause: %+v", u)
			}
		})
	}
}
