//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"testing"
	"time"
)

func TestExecutionReplanKeepsLongFailureCause(t *testing.T) {
	l, _ := (RunLimits{MaxConsecutiveErrors: 2}).normalized()
	g := newLoopGuard(l)
	for i := 0; i < 3; i++ {
		id := fmt.Sprint(i)
		g.call(id, "command_execution", "python3 tests/browser_recipe.py", time.Now())
		g.result(id, true, strings.Repeat("Traceback framework frame without cause\n", 60)+"OSError: listen tcp 127.0.0.1:0: socket: operation not permitted")
	}
	d := g.diagnostic("worker", "attempt", "")
	if len(d.Items) != 1 || d.Items[0].Category != "environment" || g.errors != 0 || d.LimitReached {
		t.Fatal("long infrastructure failure became ordinary worker errors", d, g.errors)
	}
	if !strings.Contains(strings.Join(d.Items[0].Traces, " "), "socket: operation not permitted") {
		t.Fatal("original leaf cause missing from delivered diagnostic")
	}
}

func TestExecutionReplanRoutesToolFailureAndRetainsLatestTrace(t *testing.T) {
	s, w, a := runSupervisionFixture(t, true)
	a.Status = "running"
	a.Diagnostic = buildAttemptDiagnostic(a.ID, a.Attempt, []toolFailure{{name: "command_execution", detail: "go test ./internal/engine", technical: "undefined: ExistingInterface"}}, "", 3)
	a.Progress = AgentProgress{MetricsVersion: 1, Errors: 1, ToolCalls: 1, ToolResults: 1, OutputState: "recent"}
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if _, err := s.observeRun(a); err != nil {
		t.Fatal(err)
	}
	if _, err := s.observeRun(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	count := 0
	for _, event := range w.Planning.Inbox {
		if event.RunObservation != nil && event.RunObservation.Trigger == "tool_failure" {
			count++
			if event.Scope != "child" || event.Attempt != a.Attempt || event.RunObservation.Diagnostic == nil || !strings.Contains(fmt.Sprint(event.RunObservation.Diagnostic), "ExistingInterface") {
				t.Fatal("wrong error recipient or lost cause", event)
			}
		}
	}
	if count != 1 {
		t.Fatalf("tool failure not delivered once to its owner: %d", count)
	}
	received, err := planningContext(w, "child")
	if err != nil || !strings.Contains(string(received), "ExistingInterface") || !strings.Contains(string(received), "tool_failure") || !strings.Contains(string(received), a.Attempt) {
		t.Fatal("owner context did not receive the attributed error", err)
	}
	var failures []toolFailure
	for i := 0; i < 5; i++ {
		failures = append(failures, toolFailure{name: "command_execution", technical: fmt.Sprintf("failure-%d", i)})
	}
	d := buildAttemptDiagnostic(a.ID, a.Attempt, failures, "", 3)
	if len(d.Items[0].Traces) > 3 || !strings.Contains(strings.Join(d.Items[0].Traces, " "), "failure-4") || !strings.Contains(strings.Join(d.Items[0].Traces, " "), "failure-0") {
		t.Fatal("bounded diagnostic lost original or latest error", d)
	}
}

func TestExecutionReplanNarrativeCannotReplaceVerifiedPreflight(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Diagnostic = buildAttemptDiagnostic(a.ID, a.Attempt, []toolFailure{{name: "command_execution", technical: "permission denied"}}, "", 3)
	if err = s.finishAgent(a, "failed", "Environment unavailable", nil); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	r.EventID, r.Revision, r.Previous = newID("unverified-retry-"), w.Revision, a.ID
	r.PreconditionEvidence = "The environment was checked and fixed by the operator"
	if _, created, err := s.prepare(w.ID, r); err == nil || created {
		t.Fatal("a narrative bypassed unverified provider capabilities")
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision || len(after.Tasks[0].Attempts) != len(w.Tasks[0].Attempts) {
		t.Fatal("denied restart consumed an attempt")
	}
	setPreflightProvider(t, s, successfulShellPreflight())
	next, created, err := s.prepare(w.ID, r)
	if err != nil || !created || next.PreconditionEvidence != r.PreconditionEvidence {
		t.Fatal("verified corrected context could not resume", created, err)
	}
	after, _ = s.get(w.ID)
	if len(after.Tasks[0].Attempts) != len(w.Tasks[0].Attempts)+1 {
		t.Fatal("verified recovery altered attempt history")
	}
}

func TestExecutionReplanOperatorStopDoesNotEraseEnvironmentFailure(t *testing.T) {
	a := Agent{Status: "interrupted", StopKind: originOperator, Diagnostic: buildAttemptDiagnostic("a", "attempt", []toolFailure{{technical: "socket: operation not permitted"}}, "", 3)}
	if !requiresEnvironmentVerification(a) {
		t.Fatal("manual stop bypassed observed environment failure")
	}
	if strings.Contains(boundedFailureText(strings.Repeat("frame ", 150)+"Authorization: Bearer confidential-value", 600), "confidential-value") {
		t.Fatal("bounded diagnostic leaked sensitive evidence")
	}
}

func TestExecutionReplanDeclaredCapabilitiesAreActuallyChecked(t *testing.T) {
	s := storeTest(t)
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	t.Setenv("SWARM_CONTEXT_PROBE_FIXTURE", "1")
	p := verifiedPreflightProvider(exe, "-test.run=^TestExecutionReplanProbeFixture$")
	p.Env = []string{"SWARM_CONTEXT_PROBE_FIXTURE"}
	p.PreflightCaps = append(p.PreflightCaps, "loopback_listen")
	setPreflightProvider(t, s, p)
	result, err := s.preparationPreflight("fixture", ".", "auto")
	if err == nil || result.Verdict != "intervention" {
		t.Fatal("declared blocked capability was ignored", result, err)
	}
}

func TestExecutionReplanProbeFixture(t *testing.T) {
	if os.Getenv("SWARM_CONTEXT_PROBE_FIXTURE") != "1" {
		return
	}
	raw, _ := json.Marshal(map[string]any{"schema_version": 1, "capabilities": map[string]string{"process": "verified", "workspace_read": "verified", "workspace_write": "verified", "loopback_listen": "blocked"}})
	fmt.Println(string(raw))
	os.Exit(0)
}

func TestExecutionReplanCompleteControlObservationAboveOldPreview(t *testing.T) {
	c := ValidationControl{ID: "long-observation", Command: []string{"python3", "-c", "print('observed '*1200+'FINAL_ASSERTION_CONFIRMED')"}, Timeout: 10, ReviewOutput: true}
	r, original, total := runValidationControlCaptured(t.TempDir(), c)
	if !r.Passed || total <= 8192 || r.ReviewOutputTruncated || r.ReviewOutput != string(original) || r.OutputHash != hash(original) || !strings.Contains(r.ReviewOutput, "FINAL_ASSERTION_CONFIRMED") {
		t.Fatal("complete captured control observation was reduced before review", r.OutputBytes, len(r.ReviewOutput), r.ReviewOutputTruncated)
	}
}

func TestExecutionReplanCompleteControlObservationReachesIndependentReview(t *testing.T) {
	s, w := managedFixture(t)
	w, err := s.mutate(w.ID, "fixture.control-observation", "full-control-proof", w.Revision, []byte(`{}`), func(w *Work) error {
		p := automaticPolicy("python3", "-c", "print('observed '*1200+'FINAL_ASSERTION_CONFIRMED')")
		p.Controls[0].ReviewOutput = true
		w.Tasks[0].ValidationPolicy = p
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	a := managedCompleted(t, s, w, "first", "fully observed candidate\n")
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(s.root + "/review-fixture/observed.json")
	if err != nil {
		t.Fatal(err)
	}
	var received struct {
		Context managedReviewContext `json:"context"`
	}
	if err := json.Unmarshal(raw, &received); err != nil {
		t.Fatal(err)
	}
	control := received.Context.Tasks[0].Controls[0]
	if control.ReviewOutputTruncated || !strings.Contains(control.ReviewOutput, "FINAL_ASSERTION_CONFIRMED") || control.OutputHash != hash([]byte(control.ReviewOutput)) {
		t.Fatal("independent context lost complete original control evidence")
	}
	after, _ := s.get(w.ID)
	if !s.acceptedFresh(&after, &after.Tasks[0], map[string]bool{}) {
		t.Fatal("complete observation bypassed or broke ordinary acceptance")
	}
}
