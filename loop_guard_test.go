//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func feedGuard(g *loopGuard, s string) {
	var d map[string]any
	_ = json.Unmarshal([]byte(s), &d)
	g.observe(d, time.Now())
}
func TestLoopGuardEvents(t *testing.T) {
	limits, _ := (RunLimits{}).normalized()
	limits.MaxRepeatedCalls = 2
	g := newLoopGuard(limits)
	first := `{"type":"assistant","message":{"content":[{"type":"tool_use","id":"1","name":"Bash","input":{"command":"pwd"}}]}}`
	feedGuard(g, first)
	feedGuard(g, first)
	if g.calls != 1 {
		t.Fatal("replayed tool counted twice")
	}
	feedGuard(g, strings.Replace(first, `"id":"1"`, `"id":"2"`, 1))
	if !strings.Contains(g.check(time.Now()), "répétitions") {
		t.Fatal("repeat guard missing")
	}
	g = newLoopGuard(limits)
	for i := 0; i < 3; i++ {
		feedGuard(g, fmt.Sprintf(`{"type":"item.started","item":{"id":"%d","type":"command_execution","command":"test %d"}}`, i, i))
		feedGuard(g, fmt.Sprintf(`{"type":"item.completed","item":{"id":"%d","type":"command_execution","exit_code":1}}`, i))
	}
	if !strings.Contains(g.check(time.Now()), "erreurs") {
		t.Fatal("codex errors not counted")
	}
	g = newLoopGuard(limits)
	g.call("1", "Bash", "x", time.Now())
	g.result("1", true)
	g.result("1", true)
	if g.errors != 1 {
		t.Fatal("duplicate error counted")
	}
	g.call("2", "Bash", "y", time.Now())
	g.result("2", false)
	if g.errors != 0 {
		t.Fatal("success did not reset consecutive errors")
	}
	g.pending["slow"] = time.Now().Add(-301 * time.Second)
	g.lastOutput = time.Now()
	if !strings.Contains(g.check(time.Now()), "outil observable") {
		t.Fatal("heartbeat hid long tool")
	}
}
func TestLoopGuardCaptureOffAndDefaults(t *testing.T) {
	l, e := (RunLimits{}).normalized()
	if e != nil || l.MaxToolCalls != 100 {
		t.Fatal(l, e)
	}
	if _, e = (RunLimits{MaxToolCalls: -1}).normalized(); e == nil {
		t.Fatal("negative limit accepted")
	}
	l.MaxToolCalls = 1
	sink := &outputSink{guard: newLoopGuard(l), capture: false}
	_, _ = sink.Write([]byte(`{"type":"assistant","message":{"content":[{"type":"tool_use","id":"1","name":"Read","input":{}}]}}` + "\n"))
	if sink.guardReason() == "" || len(sink.logs) != 1 || sink.logs[0].Kind != "activity" {
		t.Fatal("capture-off disabled guard or failed structured activity")
	}
	g := newLoopGuard(l)
	g.lastOutput = time.Now().Add(-181 * time.Second)
	if !strings.Contains(g.check(time.Now()), "sans sortie") {
		t.Fatal("silence not detected")
	}
}
func TestLoopProvider(t *testing.T) {
	if os.Getenv("SWARM_LOOP_FIXTURE") != "1" {
		return
	}
	prompt, _ := io.ReadAll(os.Stdin)
	switch {
	case strings.Contains(string(prompt), "GUARD_INTERLEAVED"):
		for i := 0; i < 3; i++ {
			fmt.Printf(`{"type":"item.started","item":{"id":"f%d","type":"command_execution","command":"build"}}`+"\n", i)
			fmt.Printf(`{"type":"item.completed","item":{"id":"f%d","type":"command_execution","exit_code":1}}`+"\n", i)
			fmt.Printf(`{"type":"item.started","item":{"id":"r%d","type":"command_execution","command":"read %d"}}`+"\n", i, i)
			fmt.Printf(`{"type":"item.completed","item":{"id":"r%d","type":"command_execution","exit_code":0}}`+"\n", i)
		}
	case strings.Contains(string(prompt), "GUARD_REPEAT"):
		for i := 0; i < 3; i++ {
			fmt.Printf("{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"%d\",\"name\":\"Bash\",\"input\":{\"command\":\"pwd\"}}]}}\n", i)
		}
	case strings.Contains(string(prompt), "GUARD_TOOL"):
		fmt.Println(`{"type":"assistant","message":{"content":[{"type":"tool_use","id":"one","name":"Bash","input":{"command":"long"}}]}}`)
		for {
			fmt.Println(`{"type":"system"}`)
			time.Sleep(100 * time.Millisecond)
		}
	}
	if strings.Contains(string(prompt), "FAST") {
		os.Exit(0)
	}
	time.Sleep(60 * time.Second)
	os.Exit(0)
}
func TestSupervisorGuardStopsAndBlocksTask(t *testing.T) {
	for _, mode := range []string{"GUARD_INTERLEAVED", "GUARD_REPEAT", "GUARD_REPEAT_FAST", "GUARD_SILENCE", "GUARD_TOOL"} {
		t.Run(mode, func(t *testing.T) {
			s := storeTest(t)
			w, r := setupAgent(t, s)
			exe, _ := os.Executable()
			t.Setenv("SWARM_LOOP_FIXTURE", "1")
			limits := RunLimits{MaxRepeatedCalls: 2, SilenceSeconds: 1, ToolSeconds: 1}
			p := Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: exe, Args: []string{"-test.run=^TestLoopProvider$"}, Env: []string{"SWARM_LOOP_FIXTURE"}, Limits: limits}}}
			b, _ := json.Marshal(p)
			if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), b, 0600); e != nil {
				t.Fatal(e)
			}
			r.Instruction = mode
			r.Capture = false
			a, _, e := s.prepare(w.ID, r)
			if e != nil {
				t.Fatal(e)
			}
			if a.Limits.MaxRepeatedCalls != 2 || !strings.Contains(a.Prompt, "pas de find /") {
				t.Fatal("launch not bounded")
			}
			if e = s.supervise(a.ID); e != nil {
				t.Fatal(e)
			}
			a, _ = s.agent(a.ID)
			if a.Status != "interrupted" || a.ExitCode == nil {
				t.Fatalf("not interrupted: %+v", a)
			}
			if len(a.Diagnostic.Items) == 0 || !a.Diagnostic.LimitReached {
				t.Fatalf("diagnostic de garde absent : %+v", a.Diagnostic)
			}
			if mode == "GUARD_INTERLEAVED" && (a.Diagnostic.ObservedErrors != 2 || !strings.Contains(a.Diagnostic.Summary, "plafond configuré")) {
				t.Fatalf("erreurs observées et plafond confondus : %+v", a.Diagnostic)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task(r.TaskID)
			if task.Status != "blocked" {
				t.Fatal("task not blocked")
			}
			logs, _ := s.logs(a.ID, 0)
			found := false
			for _, l := range logs {
				if l.Kind == "output" {
					t.Fatal("raw output captured")
				}
				if strings.Contains(l.Message, "atteint") {
					found = true
				}
			}
			if !found {
				t.Fatal("stop cause not persisted")
			}
		})
	}
}
