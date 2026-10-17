//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strings"
	"testing"
	"time"
)

func TestInfrastructureFailuresDoNotConsumeWorkerErrorCeiling(t *testing.T) {
	l, _ := (RunLimits{MaxConsecutiveErrors: 2}).normalized()
	g := newLoopGuard(l)
	for i := 0; i < 3; i++ {
		id := fmt.Sprint(i)
		g.call(id, "command_execution", fmt.Sprintf("go test ./... -run Case%d", i), time.Now())
		g.result(id, true, "bwrap: Can't bind mount /oldroot on /newroot: Unable to apply mount flags: No such device")
	}
	if g.errors != 0 || g.totalErrors != 3 || g.reason != "" {
		t.Fatalf("infrastructure attributed to worker: streak=%d total=%d reason=%s", g.errors, g.totalErrors, g.reason)
	}
	d := g.diagnostic("worker", "attempt", "")
	if len(d.Items) != 1 || d.Items[0].Category != "environment" || d.ObservedErrors != 3 || d.ConsecutiveLimitReached {
		t.Fatal("infrastructure evidence lost", d)
	}
	// Ordinary tool failures still consume the configured protection.
	for i := 0; i < 2; i++ {
		id := fmt.Sprintf("tool%d", i)
		g.call(id, "Bash", id, time.Now())
		g.result(id, true, "invalid option")
	}
	if g.errors != 2 || !strings.Contains(g.reason, "erreurs") {
		t.Fatal("ordinary error protection lost")
	}
}

func TestInfrastructureFailureIsDeliveredBeforeOwnerStopsProcess(t *testing.T) {
	for _, child := range []bool{false, true} {
		t.Run(map[bool]string{false: "root", true: "child"}[child], func(t *testing.T) {
			s, w, a := runSupervisionFixtureInstruction(t, child, "INFRASTRUCTURE_LIVE")
			done := make(chan error, 1)
			go func() { done <- s.supervise(a.ID) }()
			defer func() {
				_ = s.stopAgent(a.ID)
				select {
				case err := <-done:
					if err != nil {
						t.Error(err)
					}
				case <-time.After(7 * time.Second):
					t.Error("owned worker did not stop")
				}
			}()
			a = waitAgent(t, s, a.ID, func(c Agent) bool { return c.Progress.Errors >= 3 || !activeAgent(c) })
			if a.Status != "running" || a.Ended != "" {
				t.Fatal("engine stopped before owner decision", a.Status, a.Activity)
			}
			w, _ = s.get(w.ID)
			scope := w.Tasks[0].ScopeID
			var event *PlanningEvent
			for i := range w.Planning.Inbox {
				e := &w.Planning.Inbox[i]
				if e.RunObservation != nil && e.RunObservation.Trigger == "infrastructure" {
					event = e
				}
			}
			if event == nil || event.Scope != scope || event.Attempt != a.Attempt {
				t.Fatal("missing attributed infrastructure delivery", event)
			}
			raw, _ := json.Marshal(event.RunObservation)
			if !strings.Contains(string(raw), "bwrap") {
				t.Fatal("original technical evidence not delivered")
			}
			w, req := planningClaim(t, s, w, scope)
			req.Operations = []PlanningOperation{{Kind: "stop-run", ID: a.TaskID, Next: "Owner pauses the faulty execution environment; retain result, verify sandbox health, then resume through a public recovery"}}
			var err error
			w, err = s.planningChange(w.ID, "decide", req)
			if err != nil {
				t.Fatal(err)
			}
			a = waitAgent(t, s, a.ID, func(c Agent) bool { return !activeAgent(c) })
			if a.Status != "interrupted" || a.StopKind != "planning" || a.Ended == "" || w.Tasks[0].Status != "running" {
				t.Fatal("stop not attributable to owner or manufactured acceptance", a.Status, a.StopKind)
			}
		})
	}
}

func TestInfrastructureFailuresKeepEvidenceAndSpendingCapWithoutRepeatStop(t *testing.T) {
	for _, spent := range []bool{false, true} {
		t.Run(fmt.Sprint(spent), func(t *testing.T) {
			l, _ := (RunLimits{MaxRepeatedCalls: 2, MaxToolCalls: 8}).normalized()
			if spent {
				l.MaxToolCalls = 4
			}
			g := newLoopGuard(l)
			for i := 0; i < 4; i++ {
				id := fmt.Sprint(i)
				g.call(id, "Bash", "go test ./...", time.Now())
				g.result(id, true, "bwrap: Can't bind mount source on target: Unable to apply mount flags")
			}
			if g.totalErrors != 4 || g.errors != 0 {
				t.Fatal("counter attribution lost")
			}
			if spent && g.reason != "Limite d'appels d'outils atteinte" {
				t.Fatal("spending ceiling bypassed", g.reason)
			}
			if !spent && g.reason != "" {
				t.Fatal("infrastructure caused repeated-call stop", g.reason)
			}
		})
	}
}
func TestInfrastructureObservationDoesNotRepeatForUnrelatedFailure(t *testing.T) {
	s, w, a := runSupervisionFixture(t, true)
	a.Status = "running"
	a.Diagnostic = buildAttemptDiagnostic(a.ID, a.Attempt, []toolFailure{{technical: "mount failed: permission denied"}}, "", 3)
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if _, err := s.observeRun(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	a.Diagnostic = buildAttemptDiagnostic(a.ID, a.Attempt, []toolFailure{{technical: "mount failed: permission denied"}, {technical: "invalid tool option"}}, "", 3)
	if _, err := s.observeRun(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	infrastructure, tools := 0, 0
	for _, event := range w.Planning.Inbox {
		if event.RunObservation == nil {
			continue
		}
		switch event.RunObservation.Trigger {
		case "infrastructure":
			infrastructure++
		case "tool_failure":
			tools++
		}
	}
	if infrastructure != 1 || tools != 1 {
		t.Fatal("infrastructure duplicated or new ordinary failure lost", infrastructure, tools)
	}
}
