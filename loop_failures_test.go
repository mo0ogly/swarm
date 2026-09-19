//go:build linux

package main

import (
	"fmt"
	"strings"
	"testing"
	"time"
)

func TestInterleavedFailures(t *testing.T) {
	l, _ := (RunLimits{}).normalized()
	l.MaxRepeatedCalls = 3
	g := newLoopGuard(l)
	for i := 0; i < 3; i++ {
		id := fmt.Sprint(i)
		g.call(id, "Bash", "failing-build", time.Now())
		g.result(id, true)
		g.result(id, true) // replay must not increase count
		if i < 2 && g.reason != "" {
			t.Fatal("premature stop", g.reason)
		}
		g.call("read"+id, "Read", id, time.Now())
		g.result("read"+id, false)
	}
	if !strings.Contains(g.reason, "entrelacés") {
		t.Fatal("interleaved failures bypassed guard", g.reason)
	}
}
func TestFailureHistoryRecoveryAndExpiry(t *testing.T) {
	for _, mode := range []string{"success", "expiry", "visibility"} {
		t.Run(mode, func(t *testing.T) {
			l, _ := (RunLimits{}).normalized()
			l.MaxRepeatedCalls = 2
			g := newLoopGuard(l)
			g.call("a", "Bash", "build", time.Now())
			g.result("a", true)
			g.call("r", "Read", "one", time.Now())
			g.result("r", false)
			switch mode {
			case "success":
				g.call("b", "Bash", "build", time.Now())
				g.result("b", false)
			case "expiry":
				for i := 0; i < 8; i++ {
					id := fmt.Sprint(i)
					g.call(id, "Read", id, time.Now())
					g.result(id, false)
				}
			case "visibility":
				g.loseVisibility("missing events")
			}
			g.call("s", "Read", "two", time.Now())
			g.result("s", false)
			g.call("c", "Bash", "build", time.Now())
			g.result("c", true)
			if g.reason != "" {
				t.Fatal("stale failure counted", g.reason)
			}
		})
	}
}
func TestMissionLimits(t *testing.T) {
	base := RunLimits{MaxToolCalls: 40}
	l, e := base.tightened(RunLimits{MaxToolCalls: 15})
	if e != nil || l.MaxToolCalls != 15 || l.ToolSeconds != 300 {
		t.Fatal(l, e)
	}
	for _, n := range []int{-1, 41} {
		if _, e = base.tightened(RunLimits{MaxToolCalls: n}); e == nil {
			t.Fatal("invalid budget accepted", n)
		}
	}
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Limits = &RunLimits{MaxToolCalls: 15}
	a, _, e := s.prepare(w.ID, r)
	if e != nil || a.Limits.MaxToolCalls != 15 || !strings.Contains(a.Prompt, "15 appels") {
		t.Fatal("mission budget not frozen", a.Limits, e)
	}
}

func TestRetryPreservesCeilings(t *testing.T) {
	current, _ := (RunLimits{MaxToolCalls: 100, SilenceSeconds: 60}).normalized()
	capped := current.cappedBy(RunLimits{MaxToolCalls: 15, SilenceSeconds: 180})
	if capped.MaxToolCalls != 15 || capped.SilenceSeconds != 60 || capped.ToolSeconds != 300 {
		t.Fatal(capped)
	}
	if current.cappedBy(RunLimits{}) != current {
		t.Fatal("legacy zero limits changed")
	}
}
