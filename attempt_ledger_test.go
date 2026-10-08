//go:build linux

package main

import (
	"bytes"
	"strings"
	"testing"
	"time"
)

func TestAttemptMetricsCumulativeAndDeduplicated(t *testing.T) {
	l, _ := (RunLimits{}).normalized()
	g := newLoopGuard(l)
	g.call("a", "Read", map[string]any{"path": "one"}, time.Now())
	g.call("a", "Read", map[string]any{"path": "one"}, time.Now())
	g.result("a", true)
	g.result("a", true)
	g.call("b", "Edit", map[string]any{"path": "two"}, time.Now())
	g.result("b", false)
	g.call("c", "Read", map[string]any{"path": "one"}, time.Now())
	g.result("c", false)
	g.call("d", "Bash", map[string]any{"command": "go test ./... && echo done"}, time.Now())
	g.result("d", false)
	p := g.summary()
	if p.ToolCalls != 4 || p.Reads != 2 || p.Writes != 1 || p.Unclassified != 1 || p.Errors != 1 || p.Repeats != 1 || g.errors != 0 {
		t.Fatalf("wrong cumulative metrics: %+v", p)
	}
	g.loseVisibility("missing protocol")
	if q := g.summary(); q.Errors != 1 || q.Repeats != 1 || q.Reads != 2 || q.Degraded == "" {
		t.Fatalf("visibility loss erased observations: %+v", q)
	}
}

func TestAttemptLedgerKeepsAttemptsAndUnknowns(t *testing.T) {
	w := Work{Tasks: []Task{{ID: "t", Title: "Result", Status: "accepted"}}}
	cost := 0.25
	agents := []Agent{
		{ID: "old", TaskID: "t", Attempt: "failed", Status: "interrupted", Progress: AgentProgress{ToolCalls: 25}},
		{ID: "new", TaskID: "t", Attempt: "finished", Status: "completed", Usage: &Usage{Input: 100, Output: 20, ReportedCost: &cost}, Progress: AgentProgress{MetricsVersion: 1, ToolCalls: 3, Reads: 1, Writes: 1, Unclassified: 1}},
		{ID: "partial", TaskID: "t", Attempt: "partial", Status: "running", Progress: AgentProgress{MetricsVersion: 1, ToolCalls: 2, Reads: 2, Degraded: "lost messages"}},
		{ID: "terminal", TaskID: "t", Attempt: "terminal", Status: "completed", Mode: "terminal", Progress: AgentProgress{MetricsVersion: 1}},
	}
	rows := attemptLedgers(w, agents)
	if len(rows) != 4 {
		t.Fatal(rows)
	}
	by := map[string]AttemptLedger{}
	for _, r := range rows {
		by[r.Attempt] = r
	}
	old := by["failed"]
	if old.Measured || !old.MissingUsage || old.Tools != 25 || old.Cost.Silent != 1 || old.ProcessState != "interrupted" {
		t.Fatal(old)
	}
	newer := by["finished"]
	if !newer.Measured || newer.Measurement != "observed" || newer.Cost.Reported != cost || newer.Input != 100 {
		t.Fatal(newer)
	}
	if by["partial"].Measurement != "partial" || !by["partial"].Measured || by["terminal"].Measured {
		t.Fatal(rows)
	}
	var out bytes.Buffer
	printAttemptLedgers(&out, rows)
	text := out.String()
	for _, word := range []string{"inconnues, pas zéro", "toutes tentatives", "appels internes", "Mesure partielle", "aucun signal fiable"} {
		if !strings.Contains(text, word) {
			t.Fatalf("missing %q: %s", word, text)
		}
	}
}

// REQ-QW7: each attempt reports its own start/end exactly as the agent
// recorded them (never computed), distinguishing a finished run from a still
// running one instead of reporting both as an identical absence.
func TestAttemptLedgerReportsDatedProvenanceWithoutInventingDuration(t *testing.T) {
	w := Work{Tasks: []Task{{ID: "t", Title: "Result", Status: "accepted"}}}
	agents := []Agent{
		{ID: "finished", TaskID: "t", Attempt: "a1", Status: "completed", Started: "2026-01-01T00:00:00Z", Ended: "2026-01-01T00:05:00Z"},
		{ID: "ongoing", TaskID: "t", Attempt: "a2", Status: "running", Started: "2026-01-02T00:00:00Z"},
		{ID: "unstarted", TaskID: "t", Attempt: "a3", Status: "queued"},
	}
	rows := attemptLedgers(w, agents)
	by := map[string]AttemptLedger{}
	for _, r := range rows {
		by[r.Attempt] = r
	}
	if by["a1"].Started != "2026-01-01T00:00:00Z" || by["a1"].Ended != "2026-01-01T00:05:00Z" {
		t.Fatal("finished attempt lost its recorded timestamps", by["a1"])
	}
	if by["a2"].Started != "2026-01-02T00:00:00Z" || by["a2"].Ended != "" {
		t.Fatal("running attempt must not get an invented end timestamp", by["a2"])
	}
	if by["a3"].Started != "" || by["a3"].Ended != "" {
		t.Fatal("queued attempt without timestamps must stay empty, not zero-valued", by["a3"])
	}
	var out bytes.Buffer
	printAttemptLedgers(&out, rows)
	text := out.String()
	if !strings.Contains(text, "2026-01-01T00:00:00Z") || !strings.Contains(text, "2026-01-01T00:05:00Z") {
		t.Fatalf("finished attempt dates missing from report: %s", text)
	}
	if !strings.Contains(text, "en cours") {
		t.Fatalf("running attempt without end must read as ongoing, not unreported: %s", text)
	}
	if !strings.Contains(text, "non rapporté") {
		t.Fatalf("queued attempt without any timestamp must read as not reported: %s", text)
	}
}
