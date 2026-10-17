//go:build linux

package engine

import (
	"strings"
	"testing"
	"time"
)

func TestDialogueCumulativeToolsAndIdle(t *testing.T) {
	s, a, _ := dialogueReceiptFixture(t, "codex", []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"item.started","item":{"id":"item_0","type":"command_execution","command":"pwd"}}`, `{"type":"item.completed","item":{"id":"item_0","type":"command_execution","status":"completed"}}`, `{"type":"turn.completed"}`})
	a.Limits, _ = (RunLimits{MaxToolCalls: 2}).normalized()
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	m, err := runDialogueReceiptWrapper(t, s, a, "second question\n/fin\n")
	if err == nil {
		t.Fatal("cumulative tool limit did not refuse continuation")
	}
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
