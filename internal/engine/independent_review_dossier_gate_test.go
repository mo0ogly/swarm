//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// Exercises the exact same engine entrypoint used by the CLI (`planning
// review-step`) and by the web background loop (mission.go's reviewer
// goroutine): independentReviewStep. The dossier check (delivery documents +
// preauthorized engine controls) must refuse before any provider process is
// started when required evidence is missing, and must let an admissible
// dossier through to a real review once fresh matching evidence exists.
func TestIndependentReviewStepGatesDossierBeforeProviderCall(t *testing.T) {
	s, p := preparedTeam(t)
	p, e := s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
	if e != nil {
		t.Fatal(e)
	}
	w, _ := s.get(p.WorkID)
	task := &w.Tasks[0]
	task.Status = "submitted"
	task.Attempts = []Attempt{{ID: "production-attempt", Status: "completed"}}
	task.Criteria = []string{"Le rapport contient la phrase : preuve observée"}
	policy := ValidationPolicy{Mode: "human", Controls: []ValidationControl{{ID: "objective-check", Command: []string{"go", "version"}, Criteria: []int{1}, Justification: "La commande vérifie objectivement le critère annoncé.", Timeout: 10}}}
	task.ValidationPolicy = &policy
	task.AutoValidation = nil // no engine controls executed yet: dossier must be inadmissible
	if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	report := "docs/" + task.ID + ".md"
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("preuve observée dans ce rapport"), 0600); err != nil {
		t.Fatal(err)
	}
	a := Agent{ID: "producer-dossier-gate", WorkID: w.ID, TaskID: task.ID, Attempt: "production-attempt", Status: "completed", CWD: s.root, Started: time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano), Role: "worker"}

	ps, _ := s.providers()
	provider := ps.Providers[w.Planning.Reviewer.Provider]
	marker := filepath.Join(s.root, "provider-called.marker")
	refusalScript := "#!/bin/sh\ntouch '" + marker + "'\nprintf '%s\\n' '{\"type\":\"result\",\"result\":\"{}\"}'\n"
	if err := os.WriteFile(provider.Command, []byte(refusalScript), 0700); err != nil {
		t.Fatal(err)
	}

	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	body, _ := json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,?,?,?)", a.ID, a.WorkID, a.TaskID, a.CWD, "completed", "run", body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}

	before, _ := s.get(w.ID)

	// Negative case: missing engine-control evidence must refuse the dossier
	// without spending a review call or starting the provider process.
	if err := s.independentReviewStep(w.ID); err != nil {
		t.Fatal(err)
	}
	afterRefusal, _ := s.get(w.ID)
	refusedTask, _ := afterRefusal.task(task.ID)
	if refusedTask.IndependentReview != nil {
		t.Fatalf("review recorded without required engine-control evidence: %+v", refusedTask.IndependentReview)
	}
	if afterRefusal.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls {
		t.Fatal("review call spent despite inadmissible dossier")
	}
	if _, err := os.Stat(marker); err == nil {
		t.Fatal("provider process invoked despite missing validation evidence")
	} else if !os.IsNotExist(err) {
		t.Fatal(err)
	}

	// Supply fresh, matching engine-control evidence: the dossier becomes
	// admissible and the identical entrypoint (invoked here through the CLI
	// surface) must proceed to a real review call.
	digest := validationPolicyDigest(policy)
	reportBytes, err := os.ReadFile(filepath.Join(s.root, report))
	if err != nil {
		t.Fatal(err)
	}
	receipt := "docs/" + task.ID + "-controls.json"
	if err := os.WriteFile(filepath.Join(s.root, receipt), []byte("{}"), 0600); err != nil {
		t.Fatal(err)
	}
	stamp := now()
	av := &AutomaticValidation{
		Attempt:      a.Attempt,
		Producer:     a.ID,
		Controller:   validationController,
		PolicyDigest: digest,
		Policy:       policy,
		Artifacts:    map[string]string{receipt: hash([]byte("{}")), report: hash(reportBytes)},
		Controls:     []ValidationControlResult{{ID: "objective-check", Command: []string{"go", "version"}, Executed: true, Passed: true, ExitCode: 0, Started: stamp, Finished: stamp}},
		Receipt:      receipt,
		State:        "pending_human",
		At:           stamp,
	}
	current, _ := s.get(w.ID)
	ct, _ := current.task(task.ID)
	ct.AutoValidation = av
	raw2, _ := json.Marshal(current)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw2, w.ID); err != nil {
		t.Fatal(err)
	}

	// Supervisor additions: physical absence, unreadable shape and proof drift
	// must not spend a call even when control metadata is otherwise present.
	assertNoCall := func(name string) {
		t.Helper()
		if err := s.planningCLI([]string{"planning", "review-step", w.ID}, "", io.Discard); err != nil {
			t.Fatal(err)
		}
		current, err := s.get(w.ID)
		eventsBefore, _ := s.events(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		if current.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls {
			t.Fatalf("%s consumed a call", name)
		}
		if _, err := os.Stat(marker); !os.IsNotExist(err) {
			t.Fatalf("%s started provider: %v", name, err)
		}
		d, err := s.missionStatus(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		result := d.Tasks[0].Result
		if result.State != "review_blocked" || result.Reason == "" || result.NextStep == "" {
			t.Fatalf("%s: missing reason or action: %+v", name, result)
		}
		var cli bytes.Buffer
		if err := missionCLI(s, []string{"mission", "status", w.ID}, "", false, &cli); err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(cli.String(), result.Reason) {
			t.Fatalf("CLI lacks reason: %s", cli.String())
		}
		const host, token = "127.0.0.1:18999", "dossier-test"
		req := httptest.NewRequest(http.MethodGet, "http://"+host+"/api/v1/snapshot?work="+w.ID, nil)
		req.Host = host
		req.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token})
		out := httptest.NewRecorder()
		newWebHandler(s, host, token).ServeHTTP(out, req)
		var web struct {
			Mission MissionStatus `json:"mission"`
		}
		if out.Code != 200 || json.Unmarshal(out.Body.Bytes(), &web) != nil || web.Mission.Tasks[0].Result != result {
			t.Fatalf("web/CLI discrepancy: %s", out.Body.String())
		}
		unchanged, _ := s.get(w.ID)
		eventsAfter, _ := s.events(w.ID)
		if len(eventsAfter) != len(eventsBefore) {
			t.Fatal("diagnostic changed event history")
		}
		if unchanged.Revision != current.Revision || unchanged.Planning.Reviewer.Calls != current.Planning.Reviewer.Calls {
			t.Fatal("diagnostic reads mutated history")
		}
		t.Logf("%s: reason=%s, action=%s; CLI and HTTP identical; history unchanged", name, result.Reason, result.NextStep)
		t.Logf("%s: provider_started=false, review_calls_before=%d, review_calls_after=%d", name, before.Planning.Reviewer.Calls, current.Planning.Reviewer.Calls)
	}
	receiptPath := filepath.Join(s.root, receipt)
	if err := os.Remove(receiptPath); err != nil {
		t.Fatal(err)
	}
	assertNoCall("missing required receipt")
	if err := os.Mkdir(receiptPath, 0700); err != nil {
		t.Fatal(err)
	}
	assertNoCall("required receipt is unreadable directory")
	if err := os.Remove(receiptPath); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(receiptPath, []byte("{}"), 0600); err != nil {
		t.Fatal(err)
	}
	reportPath := filepath.Join(s.root, report)
	if err := os.WriteFile(reportPath, []byte("changed proof"), 0600); err != nil {
		t.Fatal(err)
	}
	assertNoCall("report hash drift")
	if err := os.WriteFile(reportPath, reportBytes, 0600); err != nil {
		t.Fatal(err)
	}

	response := `{"reason":"Contrôles préautorisés et preuve conformes","criteria":[{"index":1,"verdict":"pass","evidence":"preuve observée"}]}`
	env, _ := json.Marshal(map[string]any{"type": "result", "result": response})
	acceptScript := "#!/bin/sh\ntouch '" + marker + "'\nprintf '%s\\n' '" + string(env) + "'\n"
	if err := os.WriteFile(provider.Command, []byte(acceptScript), 0700); err != nil {
		t.Fatal(err)
	}

	if err := s.planningCLI([]string{"planning", "review-step", w.ID}, "", io.Discard); err != nil {
		t.Fatal(err)
	}
	final, _ := s.get(w.ID)
	ft, _ := final.task(task.ID)
	if ft.IndependentReview == nil || ft.IndependentReview.State != "passed" {
		t.Fatalf("admissible dossier was not reviewed: %+v", ft.IndependentReview)
	}
	if final.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls+1 {
		t.Fatal("review call not spent once for the admissible dossier")
	}
	if _, err := os.Stat(marker); err != nil {
		t.Fatal("provider was not invoked for the admissible dossier")
	}
	if ft.Status == "accepted" {
		t.Fatal("review invented task acceptance")
	}
	t.Logf("admissible restored dossier: provider_started=true, review_calls_before=%d, review_calls_after=%d, task_status=%s", before.Planning.Reviewer.Calls, final.Planning.Reviewer.Calls, ft.Status)
}
