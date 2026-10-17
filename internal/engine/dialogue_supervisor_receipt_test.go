//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

type dialogueAuthorizationFunc func([]byte) (int, error)

func (f dialogueAuthorizationFunc) Write(p []byte) (int, error) { return f(p) }

func TestDialogueProtocolRejectsReplayAndUnconfirmedEOF(t *testing.T) {
	for _, scenario := range []string{"replayed_start", "unconfirmed_eof", "session_changed"} {
		t.Run(scenario, func(t *testing.T) {
			s, a, _ := dialogueReceiptFixture(t, "codex", nil)
			monitor := newDialogueMonitor(s, a)
			var acks, stream bytes.Buffer
			if _, err := s.db.Exec("INSERT INTO agent_dialogue_turns(id,agent_id,question,status,session_id) VALUES(?,?,'fixture','running','')", "first-turn", a.ID); err != nil {
				t.Fatal(err)
			}
			monitor.authorize = dialogueAuthorizationFunc(func(p []byte) (int, error) {
				n, err := acks.Write(p)
				if err == nil && acks.Len() == 2 && p[0] == 1 && scenario == "session_changed" {
					if _, err = s.db.Exec("UPDATE agent_dialogue_turns SET status='answered' WHERE id='first-turn'"); err != nil {
						return n, err
					}
					_, err = s.db.Exec("INSERT INTO agent_dialogue_turns(id,agent_id,question,status,session_id) VALUES(?,?,'fixture','running','')", "second-turn", a.ID)
				}
				return n, err
			})
			enc := json.NewEncoder(&stream)
			enc.Encode(dialogueEvent{Kind: "start", Turn: "first-turn"})
			enc.Encode(dialogueEvent{Kind: "event", Turn: "first-turn", Data: map[string]any{"type": "thread.started", "thread_id": "fixture-session"}})
			enc.Encode(dialogueEvent{Kind: "event", Turn: "first-turn", Data: map[string]any{"type": "turn.completed"}})
			if scenario != "unconfirmed_eof" {
				enc.Encode(dialogueEvent{Kind: "end", Turn: "first-turn", Data: map[string]any{"session_id": "fixture-session"}})
				turn := "first-turn"
				if scenario == "session_changed" {
					turn = "second-turn"
				}
				enc.Encode(dialogueEvent{Kind: "start", Turn: turn})
				if scenario == "session_changed" {
					enc.Encode(dialogueEvent{Kind: "event", Turn: turn, Data: map[string]any{"type": "thread.started", "thread_id": "different-session"}})
					enc.Encode(dialogueEvent{Kind: "event", Turn: turn, Data: map[string]any{"type": "turn.completed"}})
					enc.Encode(dialogueEvent{Kind: "end", Turn: turn, Data: map[string]any{"session_id": "different-session"}})
				}
			}
			monitor.consume(&stream)
			_, _, reason, _ := monitor.snapshot()
			if reason == "" {
				t.Fatal("replayed, unconfirmed or foreign boundary accepted")
			}
			if scenario == "replayed_start" && !bytes.Equal(acks.Bytes(), []byte{1, 1, 0}) || scenario == "session_changed" && !bytes.Equal(acks.Bytes(), []byte{1, 1, 1, 0}) {
				t.Fatal("wrong handshake decisions", acks.Bytes())
			}
		})
	}
}

// This runs the built application, not a replacement of superviseTerminal.
// The provider still is a local script. No model or live mission is involved.
func TestDialogueActualSupervisorConfirmsTurnReceipts(t *testing.T) {
	cwd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	binary := filepath.Join(t.TempDir(), "swarm")
	buildContext, cancelBuild := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancelBuild()
	build := exec.CommandContext(buildContext, "go", "build", "-o", binary, "./cmd/swarm")
	build.Dir = filepath.Clean(filepath.Join(cwd, "..", ".."))
	if output, err := build.CombinedOutput(); err != nil {
		t.Fatal("candidate build failed", err, string(output))
	}
	for _, scenario := range []string{"two_confirmed_turns", "provider_failure", "tool_result_missing"} {
		t.Run(scenario, func(t *testing.T) {
			events := []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"item.started","item":{"id":"item_0","type":"command_execution","command":"fixture effect"}}`, `{"type":"item.completed","item":{"id":"item_0","type":"command_execution","status":"completed","exit_code":0}}`, `{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}`}
			if scenario == "provider_failure" {
				events = []string{`{"type":"thread.started","thread_id":"fixture-session"}`, `{"type":"turn.failed","error":{"message":"fixture provider failure"}}`}
			} else if scenario == "tool_result_missing" {
				events = []string{events[0], events[1], events[3]}
			}
			s, a, script := dialogueReceiptFixture(t, "codex", events)
			ctx, cancel := context.WithTimeout(context.Background(), 25*time.Second)
			defer cancel()
			cmd := exec.CommandContext(ctx, binary, "--root", s.root, "_supervise", a.ID)
			var stderr bytes.Buffer
			cmd.Stderr = &stderr
			if err := cmd.Start(); err != nil {
				t.Fatal(err)
			}
			done := make(chan error, 1)
			go func() { done <- cmd.Wait() }()
			finished := false
			defer func() {
				if !finished {
					_ = s.stopAgent(a.ID)
					cancel()
					select {
					case <-done:
					case <-time.After(5 * time.Second):
						t.Error("owned supervisor did not stop")
					}
				}
			}()
			if scenario == "two_confirmed_turns" {
				waitAnswered := func(want int) {
					t.Helper()
					deadline := time.Now().Add(5 * time.Second)
					for time.Now().Before(deadline) {
						var count int
						if err := s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=? AND status='answered'", a.ID).Scan(&count); err != nil {
							t.Fatal(err)
						}
						if count == want {
							return
						}
						time.Sleep(20 * time.Millisecond)
					}
					t.Fatal("confirmed turn not persisted", want)
				}
				waitAnswered(1)
				running, err := s.agent(a.ID)
				if err != nil {
					t.Fatal(err)
				}
				lease, err := s.terminalRPC(running, terminalRequest{Kind: "claim", Client: "receipt-fixture"})
				if err != nil || !lease.Writable {
					t.Fatal("operator control unavailable", err)
				}
				input := terminalRequest{Kind: "input", Client: "receipt-fixture", Lease: lease.Lease, Seq: 1, Data: []byte("public-follow-up\n")}
				if _, err := s.terminalRPC(running, input); err != nil {
					t.Fatal(err)
				}
				waitAnswered(2)
				input.Seq, input.Data = 2, []byte("/fin\n")
				if _, err := s.terminalRPC(running, input); err != nil {
					t.Fatal(err)
				}
			}
			select {
			case err := <-done:
				finished = true
				if err != nil {
					t.Fatal("supervisor command failed", err, stderr.String())
				}
			case <-ctx.Done():
				t.Fatal("supervisor timed out")
			}
			read := exec.Command(binary, "--root", s.root, "--json", "agent", "show", a.ID)
			raw, err := read.Output()
			if err != nil {
				t.Fatal("public CLI agent read failed", err)
			}
			var receipt struct {
				Agent Agent `json:"agent"`
			}
			if err := json.Unmarshal(raw, &receipt); err != nil {
				t.Fatal(err)
			}
			var answered, interrupted int
			if err := s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=? AND status='answered'", a.ID).Scan(&answered); err != nil {
				t.Fatal(err)
			}
			if err := s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=? AND status='interrupted'", a.ID).Scan(&interrupted); err != nil {
				t.Fatal(err)
			}
			calls, _ := os.ReadFile(script + ".calls")
			if scenario == "two_confirmed_turns" {
				if receipt.Agent.Status != "completed" || answered != 2 || interrupted != 0 || string(calls) != "call\ncall\n" || receipt.Agent.Progress.ToolCalls != 2 || receipt.Agent.Progress.ToolResults != 2 || receipt.Agent.Usage == nil || receipt.Agent.Usage.Input != 20 || receipt.Agent.Usage.Output != 4 {
					t.Fatal("actual supervisor lost confirmed boundaries or cumulative progress", receipt.Agent.Status, answered, interrupted)
				}
			} else if receipt.Agent.Status != "interrupted" || answered != 0 || interrupted != 1 || string(calls) != "call\n" {
				t.Fatal("actual supervisor advertised incomplete turn", receipt.Agent.Status, answered, interrupted)
			}
			if scenario == "tool_result_missing" && (receipt.Agent.Progress.ToolCalls != 1 || receipt.Agent.Progress.ToolResults != 0) {
				t.Fatal("pending effect history lost")
			}
			work, err := s.get(a.WorkID)
			if err != nil || work.Tasks[0].Status == "accepted" {
				t.Fatal("technical turn implicitly accepted task", err)
			}
			if out := os.Getenv("SWARM_DIALOGUE_RECEIPT_OUT"); out != "" {
				if !filepath.IsAbs(out) {
					t.Fatal("output must be absolute")
				}
				if err := os.MkdirAll(out, 0700); err != nil {
					t.Fatal(err)
				}
				path := filepath.Join(out, scenario+".json")
				file, err := os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
				if err != nil {
					t.Fatal(err)
				}
				data, _ := json.MarshalIndent(map[string]any{"public_cli": json.RawMessage(raw), "answered": answered, "interrupted": interrupted, "provider_invocations": strings.Count(string(calls), "call\n"), "task_accepted": false}, "", "  ")
				_, err = file.Write(data)
				file.Close()
				if err != nil {
					t.Fatal(err)
				}
			}
		})
	}
}
