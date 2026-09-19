//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
	"time"
)

func TestDialogueCumulativeToolsAndIdle(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	a.Limits, _ = (RunLimits{MaxToolCalls: 2}).normalized()
	m := newDialogueMonitor(s, a)
	var stream strings.Builder
	enc := json.NewEncoder(&stream)
	for _, turn := range []string{"turn-one", "turn-two"} {
		enc.Encode(dialogueEvent{Kind: "start", Turn: turn})
		enc.Encode(dialogueEvent{Kind: "event", Turn: turn, Data: map[string]any{"type": "item.started", "item": map[string]any{"id": "item_0", "type": "command_execution", "command": "pwd"}}})
		enc.Encode(dialogueEvent{Kind: "event", Turn: turn, Data: map[string]any{"type": "item.completed", "item": map[string]any{"id": "item_0", "type": "command_execution", "status": "completed"}}})
		enc.Encode(dialogueEvent{Kind: "end", Turn: turn})
	}
	m.consume(strings.NewReader(stream.String()))
	p, _, reason, _ := m.snapshot()
	if p.ToolCalls != 2 || reason == "" {
		t.Fatal(p, reason)
	}
	m = newDialogueMonitor(s, a)
	m.sink.guard.lastOutput = time.Now().Add(-time.Hour)
	_, _, reason, _ = m.snapshot()
	if reason != "" {
		t.Fatal("operator wait treated as provider silence")
	}
}
func TestDialogueResumeIdentityAndPermissions(t *testing.T) {
	a := Agent{Command: "/bin/codex", Args: []string{"exec", "--json", "--sandbox", "workspace-write", "-"}}
	args, e := dialogueResumeArgs(a, "session-known")
	if e != nil || strings.Join(args, " ") != "exec --json --sandbox workspace-write resume session-known -" {
		t.Fatal(args, e)
	}
	if _, e = dialogueResumeArgs(a, "--last"); e == nil {
		t.Fatal("option accepted as session")
	}
}

func TestDialogueUsageSnapshotDoesNotMutate(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	m := newDialogueMonitor(s, a)
	n := int64(10)
	m.sink.usage = &Usage{Input: 10, CacheRead: &n, CacheCreation: &n, CachedInput: &n}
	m.addUsage()
	_, before, _, _ := m.snapshot()
	m.sink.usage = &Usage{Input: 10, CacheRead: &n, CacheCreation: &n, CachedInput: &n}
	m.addUsage()
	if before.Input != 10 || *before.CacheRead != 10 || *before.CacheCreation != 10 || *before.CachedInput != 10 {
		t.Fatal("snapshot mutated", before)
	}
}
