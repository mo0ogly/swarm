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
