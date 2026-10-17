//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestDialogueReceiptWrapperHelper(t *testing.T) {
	if os.Getenv("SWARM_TEST_DIALOGUE_WRAPPER") != "1" {
		return
	}
	s, err := openStore(os.Getenv("SWARM_TEST_DIALOGUE_ROOT"), false)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(5)
	}
	defer s.db.Close()
	if err := s.runAgentDialogue(os.Getenv("SWARM_TEST_DIALOGUE_AGENT")); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(5)
	}
}

func dialogueReceiptFixture(t *testing.T, name string, events []string) (*Store, Agent, string) {
	t.Helper()
	root, err := os.MkdirTemp("", "swarm-dialogue-")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(root) })
	s, err := openStore(root, true)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { s.db.Close() })
	w, launch := setupAgent(t, s)
	script := filepath.Join(t.TempDir(), name)
	body := "#!/bin/sh\ncat >>\"$0.questions\"\nprintf '\\n--END--\\n' >>\"$0.questions\"\nprintf 'call\\n' >>\"$0.calls\"\nprintf '%s\\n' \"$*\" >>\"$0.args\"\n"
	for _, line := range events {
		if !json.Valid([]byte(line)) {
			t.Fatal("invalid provider fixture")
		}
		body += "printf '%s\\n' '" + strings.ReplaceAll(line, "'", "'\"'\"'") + "'\n"
	}
	if err := os.WriteFile(script, []byte(body), 0700); err != nil {
		t.Fatal(err)
	}
	args := []string{"exec", "--json", "--sandbox", "workspace-write", "-"}
	if name == "claude" {
		args = []string{"-p", "--output-format", "stream-json", "--verbose"}
	}
	raw, err := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: script, Args: args}}})
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	launch.Mode, launch.Role = "dialogue", "worker"
	a, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	return s, a, script
}

// Run the actual wrapper in an owned subprocess with its real supervision and
// authorization descriptors. The provider is a local script, never a model.
func runDialogueReceiptWrapper(t *testing.T, s *Store, a Agent, input string) (*dialogueMonitor, error) {
	t.Helper()
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	eventsRead, eventsWrite, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer eventsRead.Close()
	defer eventsWrite.Close()
	authRead, authWrite, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer authRead.Close()
	defer authWrite.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 8*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, exe, "-test.run=^TestDialogueReceiptWrapperHelper$")
	cmd.Env = append(os.Environ(), "SWARM_TEST_DIALOGUE_WRAPPER=1", "SWARM_TEST_DIALOGUE_ROOT="+s.root, "SWARM_TEST_DIALOGUE_AGENT="+a.ID)
	cmd.Stdin = strings.NewReader(input)
	var stdout, stderr bytes.Buffer
	cmd.Stdout, cmd.Stderr = &stdout, &stderr
	cmd.ExtraFiles = []*os.File{eventsWrite, authRead}
	monitor := newDialogueMonitor(s, a)
	monitor.authorize = authWrite
	done := make(chan struct{})
	go func() { monitor.consume(eventsRead); close(done) }()
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	eventsWrite.Close()
	authRead.Close()
	err = cmd.Wait()
	select {
	case <-done:
	case <-ctx.Done():
		t.Fatal("dialogue monitor did not finish")
	}
	if ctx.Err() != nil {
		t.Fatal("wrapper handshake timed out", stderr.String())
	}
	return monitor, err
}

func TestDialogueIncompleteTurnCannotBeAnswered(t *testing.T) {
	for _, tc := range []struct {
		name   string
		events []string
	}{
		{"missing_completion", []string{`{"type":"thread.started","thread_id":"fixture-session"}`}},
		{"provider_failed_exit_zero", []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"turn.failed","error":{"message":"fixture provider failure"}}`}},
		{"missing_tool_result", []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"item.started","item":{"id":"item_0","type":"command_execution","command":"fixture effect"}}`, `{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}`}},
	} {
		t.Run(tc.name, func(t *testing.T) {
			s, a, script := dialogueReceiptFixture(t, "codex", tc.events)
			monitor, wrapperErr := runDialogueReceiptWrapper(t, s, a, "/fin\n")
			var answered int
			if err := s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=? AND status='answered'", a.ID).Scan(&answered); err != nil {
				t.Fatal(err)
			}
			_, _, reason, _ := monitor.snapshot()
			if answered != 0 || wrapperErr == nil || reason == "" {
				t.Fatalf("unconfirmed turn advertised as answered: answered=%d wrapper_error=%v guard_reason=%q", answered, wrapperErr, reason)
			}
			calls, err := os.ReadFile(script + ".calls")
			if err != nil || string(calls) != "call\n" {
				t.Fatal("provider repeated or not exercised", err)
			}
		})
	}
}

func TestDialogueConfirmedTurnsKeepSessionUsageAndPermissions(t *testing.T) {
	for _, provider := range []string{"codex", "claude"} {
		t.Run(provider, func(t *testing.T) {
			events := []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"item.started","item":{"id":"item_0","type":"command_execution","command":"fixture effect"}}`, `{"type":"item.completed","item":{"id":"item_0","type":"command_execution","status":"completed","exit_code":0}}`, `{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}`}
			if provider == "claude" {
				events = []string{`{"type":"system","session_id":"fixture-session"}`, `{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool_0","name":"Read","input":{"file_path":"fixture.txt"}}]}}`, `{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tool_0","content":"fixture value"}]}}`, `{"type":"result","subtype":"success","is_error":false,"session_id":"fixture-session","usage":{"input_tokens":10,"output_tokens":2}}`}
			}
			s, a, script := dialogueReceiptFixture(t, provider, events)
			monitor, err := runDialogueReceiptWrapper(t, s, a, "follow-up-fixture\n/fin\n")
			if err != nil {
				t.Fatal("valid wrapper failed", err)
			}
			progress, usage, reason, _ := monitor.snapshot()
			if reason != "" || progress.ToolCalls != 2 || progress.ToolResults != 2 || usage == nil || usage.Input != 20 || usage.Output != 4 {
				t.Fatal("cumulative progress or usage lost", progress, usage, reason)
			}
			var answered, sessions int
			if err := s.db.QueryRow("SELECT count(*), count(DISTINCT session_id) FROM agent_dialogue_turns WHERE agent_id=? AND status='answered'", a.ID).Scan(&answered, &sessions); err != nil || answered != 2 || sessions != 1 {
				t.Fatal("confirmed session boundaries lost", answered, sessions, err)
			}
			a.Progress, a.Usage = progress, usage
			if err := s.finishAgent(a, "completed", "fixture dialogue ended without acceptance", nil); err != nil {
				t.Fatal(err)
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			stored, err := reopened.agent(a.ID)
			if err != nil || stored.Usage == nil || stored.Usage.Input != 20 || stored.Progress.ToolCalls != 2 {
				t.Fatal("restart lost aggregate consumption", err)
			}
			before, _ := json.Marshal(stored)
			if _, err := runDialogueReceiptWrapper(t, reopened, stored, "/fin\n"); err == nil {
				t.Fatal("implicit restart allowed")
			}
			after, err := reopened.agent(a.ID)
			afterRaw, _ := json.Marshal(after)
			if err != nil || string(before) != string(afterRaw) {
				t.Fatal("denied restart changed history", err)
			}
			calls, _ := os.ReadFile(script + ".calls")
			questions, _ := os.ReadFile(script + ".questions")
			args, _ := os.ReadFile(script + ".args")
			if string(calls) != "call\ncall\n" || strings.Count(string(questions), a.Prompt) != 1 || !strings.Contains(string(questions), "follow-up-fixture") || !strings.Contains(string(args), "resume fixture-session") {
				t.Fatal("provider repeated, original prompt replayed or session not reused")
			}
			if provider == "codex" && strings.Count(string(args), "--sandbox workspace-write") != 2 {
				t.Fatal("resume changed configured permissions")
			}
		})
	}
}

func TestDialogueCompletionRejectsContradictoryAndPrivateFailures(t *testing.T) {
	for _, tc := range []struct {
		name, provider string
		events         []string
	}{
		{"claude_error_overrides_success_subtype", "claude", []string{`{"type":"system","session_id":"fixture-session"}`, `{"type":"result","subtype":"success","is_error":true,"session_id":"fixture-session","result":"PRIVATE_MARKER https://private.invalid","usage":{"input_tokens":10,"output_tokens":2}}`}},
		{"duplicate_terminal", "codex", []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"turn.completed"}`, `{"type":"turn.completed"}`}},
		{"wrong_adapter", "claude", []string{`{"type":"system","session_id":"fixture-session"}`, `{"type":"turn.completed"}`}},
		{"malformed_success_flag", "claude", []string{`{"type":"system","session_id":"fixture-session"}`, `{"type":"result","subtype":"success","is_error":"false","session_id":"fixture-session"}`}},
		{"unsupported_activity", "codex", []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"item.completed","item":{"type":"future_tool"}}`, `{"type":"turn.completed"}`}},
	} {
		t.Run(tc.name, func(t *testing.T) {
			s, a, script := dialogueReceiptFixture(t, tc.provider, tc.events)
			monitor, err := runDialogueReceiptWrapper(t, s, a, "must-not-start\n/fin\n")
			_, _, reason, _ := monitor.snapshot()
			if err == nil || reason == "" || strings.Contains(reason, "PRIVATE_MARKER") || strings.Contains(reason, "private.invalid") {
				t.Fatal("ambiguous completion or private failure exposed", err, reason)
			}
			var answered int
			if err := s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=? AND status='answered'", a.ID).Scan(&answered); err != nil || answered != 0 {
				t.Fatal("unconfirmed answer persisted", answered, err)
			}
			calls, _ := os.ReadFile(script + ".calls")
			if string(calls) != "call\n" {
				t.Fatal("provider called again after refused completion")
			}
		})
	}
}

func TestDialogueConfirmationStorageFailureBlocksContinuation(t *testing.T) {
	for _, stage := range []string{"events", "answered"} {
		t.Run(stage, func(t *testing.T) {
			s, a, script := dialogueReceiptFixture(t, "codex", []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}`})
			trigger := `CREATE TRIGGER reject_dialogue_receipt BEFORE INSERT ON agent_logs WHEN NEW.kind='output' BEGIN SELECT RAISE(ABORT,'fixture event write refused'); END`
			if stage == "answered" {
				trigger = `CREATE TRIGGER reject_dialogue_receipt BEFORE UPDATE ON agent_dialogue_turns WHEN NEW.status='answered' BEGIN SELECT RAISE(ABORT,'fixture answer write refused'); END`
			}
			if _, err := s.db.Exec(trigger); err != nil {
				t.Fatal(err)
			}
			monitor, err := runDialogueReceiptWrapper(t, s, a, "must-not-start\n/fin\n")
			if err == nil {
				t.Fatal("persistence error allowed continuation")
			}
			a.Progress, a.Usage, _, _ = monitor.snapshot()
			if err := s.finishAgent(a, "failed", "fixture persistence failure", nil); err != nil {
				t.Fatal(err)
			}
			var answered, interrupted int
			if err := s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=? AND status='answered'", a.ID).Scan(&answered); err != nil || answered != 0 {
				t.Fatal("failed persistence advertised as answer", err)
			}
			if err := s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=? AND status='interrupted'", a.ID).Scan(&interrupted); err != nil || interrupted != 1 {
				t.Fatal("interruption not retained", err)
			}
			calls, _ := os.ReadFile(script + ".calls")
			if string(calls) != "call\n" || a.Usage == nil || a.Usage.Input != 10 {
				t.Fatal("failure repeated provider or lost observed usage")
			}
		})
	}
}
