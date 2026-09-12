//go:build linux

package main

import (
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

func TestLargeToolResultMustClosePendingTool(t *testing.T) {
	limits, _ := (RunLimits{}).normalized()
	sink := &outputSink{guard: newLoopGuard(limits), capture: false}
	_, _ = sink.Write([]byte(`{"type":"assistant","message":{"content":[{"type":"tool_use","id":"read-large","name":"Read","input":{"file_path":"large.txt"}}]}}` + "\n"))
	result := map[string]any{"type": "user", "message": map[string]any{"content": []any{map[string]any{"type": "tool_result", "tool_use_id": "read-large", "content": strings.Repeat("large file content ", 3000)}}}}
	b, _ := json.Marshal(result)
	b = append(b, '\n')
	for len(b) > 0 {
		n := min(8192, len(b))
		_, _ = sink.Write(b[:n])
		b = b[n:]
	}
	if len(sink.guard.pending) != 0 {
		t.Fatal("completed Read remains pending after a >32KiB result")
	}
	sink.guard.lastOutput = time.Now().Add(301 * time.Second)
	if reason := sink.guard.check(time.Now().Add(301 * time.Second)); reason != "" {
		t.Fatal("completed tool timed out", reason)
	}
}

func TestOversizedEventDegradesTimingWithoutFalseFailure(t *testing.T) {
	l, _ := (RunLimits{}).normalized()
	sink := &outputSink{guard: newLoopGuard(l), capture: false}
	_, _ = sink.Write([]byte(`{"type":"assistant","message":{"content":[{"type":"tool_use","id":"old","name":"Read","input":{}}]}}` + "\n"))
	sink.guard.pending["old"] = time.Now().Add(-600 * time.Second)
	huge := []byte(`{"type":"user","message":{"content":"` + strings.Repeat("x", maxProviderEventBytes+100) + `"}}` + "\n")
	for len(huge) > 0 {
		n := min(4096, len(huge))
		_, _ = sink.Write(huge[:n])
		huge = huge[n:]
	}
	if sink.guardReason() != "" || sink.guard.degraded == "" {
		t.Fatal("lost event treated as timed-out tool")
	}
	if len(sink.logs) != 2 || sink.logs[1].Kind != "monitoring" {
		t.Fatal("degraded monitoring not reported without capture")
	}
	_, _ = sink.Write([]byte(`{"type":"assistant","message":{"content":[{"type":"tool_use","id":"next","name":"Bash","input":{}}]}}` + "\n"))
	if sink.guard.calls != 2 {
		t.Fatal("parser did not recover at next line")
	}
	sink.guard.lastOutput = time.Now().Add(-181 * time.Second)
	if !strings.Contains(sink.guardReason(), "sans sortie") {
		t.Fatal("silence guard disabled")
	}
}

func TestReplayGLMRetryEventShapes(t *testing.T) {
	var fixture struct {
		Events []struct {
			Kind, ID, Name string
			Elapsed        int `json:"elapsed_ms"`
			Failed         bool
			Content        int `json:"content_bytes"`
			Extra          int `json:"extra_bytes"`
		}
		Calls   int `json:"expected_calls"`
		Results int `json:"expected_results"`
	}
	b, e := os.ReadFile("tests/testdata/glm-retry-event-shapes.json")
	if e != nil {
		t.Fatal(e)
	}
	if e = json.Unmarshal(b, &fixture); e != nil {
		t.Fatal(e)
	}
	l, _ := (RunLimits{}).normalized()
	sink := &outputSink{guard: newLoopGuard(l), capture: false}
	base := time.Now()
	at := base
	for _, event := range fixture.Events {
		at = base.Add(time.Duration(event.Elapsed) * time.Millisecond)
		var wire map[string]any
		if event.Kind == "call" {
			wire = map[string]any{"type": "assistant", "message": map[string]any{"content": []any{map[string]any{"type": "tool_use", "id": event.ID, "name": event.Name, "input": map[string]any{"fixture_operation": event.ID}}}}}
		} else {
			wire = map[string]any{"type": "user", "message": map[string]any{"content": []any{map[string]any{"type": "tool_result", "tool_use_id": event.ID, "is_error": event.Failed, "content": strings.Repeat("x", event.Content)}}}, "tool_use_result": strings.Repeat("x", event.Extra)}
		}
		b, _ := json.Marshal(wire)
		b = append(b, '\n')
		for len(b) > 0 {
			n := min(8192, len(b))
			_, _ = sink.Write(b[:n])
			b = b[n:]
		}
		if event.Kind == "call" {
			sink.guard.pending[event.ID] = at
		}
		sink.guard.lastOutput = at
		if reason := sink.guard.check(at); reason != "" {
			t.Fatalf("false stop at %dms: %s", event.Elapsed, reason)
		}
	}
	if sink.guard.calls != fixture.Calls || sink.guard.completed != fixture.Results || len(sink.guard.pending) != 1 {
		t.Fatalf("incorrect accounting: %+v", sink.guard.summary())
	}
	sink.guard.lastOutput = at.Add(8 * time.Second)
	if reason := sink.guard.check(at.Add(8 * time.Second)); reason != "" {
		t.Fatal("reproduced false stop", reason)
	}
	if reason := sink.guard.check(at.Add(301 * time.Second)); reason == "" {
		t.Fatal("genuine stall not bounded")
	}
}

func TestSelectedTaskShowsStopCauseWithoutAgentSelection(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	code := 143
	if e = s.finishAgent(a, "interrupted", "Délai d'outil observable atteint (300s)", &code); e != nil {
		t.Fatal(e)
	}
	c := &consoleState{}
	frame := s.renderDashboard(w.ID, c, 80, 24, "")
	if c.selected != a.ID || !strings.Contains(frame, "Délai d'outil") || !strings.Contains(frame, "À faire :") {
		t.Fatal("task does not show cause and next action", frame)
	}
}

// Optional local probe: raw provider streams can contain private context and are
// deliberately not committed. CI uses the sanitized event-shape fixture above.
func TestLiveProviderStreamReplay(t *testing.T) {
	path := os.Getenv("SWARM_REPLAY_STREAM")
	if path == "" {
		t.Skip("optional local provider stream")
	}
	b, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	limits, _ := (RunLimits{}).normalized()
	sink := &outputSink{guard: newLoopGuard(limits)}
	for len(b) > 0 {
		n := min(8192, len(b))
		_, _ = sink.Write(b[:n])
		b = b[n:]
	}
	p := sink.progress()
	if p.ToolCalls != 1 || p.ToolResults != 1 || p.PendingTools != 0 || p.Degraded != "" {
		t.Fatalf("live Read stream not fully observed: %+v", p)
	}
	sink.guard.lastOutput = time.Now().Add(301 * time.Second)
	if reason := sink.guard.check(time.Now().Add(301 * time.Second)); reason != "" {
		t.Fatal(reason)
	}
}

func TestActivityJournalWithoutRawCapture(t *testing.T) {
	limits, _ := (RunLimits{}).normalized()
	sink := &outputSink{guard: newLoopGuard(limits)}
	wire := `{"type":"assistant","message":{"content":[{"type":"tool_use","id":"r","name":"Read","input":{"file_path":"example.go"}}]}}` + "\n" + `{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"r","content":"PRIVATE_CONTENT"}]}}` + "\n"
	_, _ = sink.Write([]byte(wire))
	if len(sink.logs) != 2 {
		t.Fatalf("missing activity: %+v", sink.logs)
	}
	for _, l := range sink.logs {
		if l.Kind != "activity" || strings.Contains(l.Message, "PRIVATE") {
			t.Fatalf("raw payload exposed: %+v", l)
		}
	}
	if !strings.Contains(sink.logs[0].Message, "Lecture d’un fichier") || !strings.Contains(sink.logs[1].Message, "Résultat") {
		t.Fatal(sink.logs)
	}
}
func TestLiveDashboardShowsAgeAndPendingAction(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Status = "running"
	a.Heartbeat = now()
	a.Progress = AgentProgress{ToolCalls: 2, ToolResults: 1, PendingTools: 1, LastTool: "Read", LastResult: now()}
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	frame := s.renderDashboard(w.ID, &consoleState{}, 80, 24, "")
	for _, want := range []string{"SUIVI EN DIRECT", "EN COURS : Read", "Suivi reçu il y a", "2 appels / 1 résultats", "Dernier résultat reçu"} {
		if !strings.Contains(frame, want) {
			t.Fatalf("missing %s: %s", want, frame)
		}
	}
}
