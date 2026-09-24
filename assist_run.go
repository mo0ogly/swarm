//go:build linux

package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"time"
)

const assistDeadlineSeconds = 300
const assistReplyLimit = 1 << 20

// Single bounded inference in an empty directory, with coding tools disabled.
// Claim by compare-and-swap prevents duplicate HTTP confirmations spawning twice.
func (s *Store) runAssistTurn(turn AssistTurn) {
	seconds := turn.TimeoutSeconds
	if seconds < 1 || seconds > assistDeadlineSeconds {
		seconds = assistDeadlineSeconds
	}
	s.runAssistTurnWithin(turn, time.Duration(seconds)*time.Second)
}
func (s *Store) runAssistTurnWithin(turn AssistTurn, deadline time.Duration) {
	turn.Status = "running"
	turn.SupervisorPID = os.Getpid()
	turn.SupervisorStamp = processStamp(os.Getpid())
	turn.Host = hostIdentity()
	raw, _ := json.Marshal(turn)
	result, e := s.db.Exec("UPDATE assist_turns SET status='running',body=? WHERE id=? AND status='pending'", raw, turn.ID)
	if e != nil {
		return
	}
	n, _ := result.RowsAffected()
	if n != 1 {
		return
	}
	providers, e := s.providers()
	if e != nil {
		_ = s.settleAssistTurn(turn, "", nil, refuse("provider_unavailable", refusalService, e.Error(), ""))
		return
	}
	p, ok := providers.Providers[turn.Provider]
	if !ok {
		_ = s.settleAssistTurn(turn, "", nil, refuse("provider_unavailable", refusalService, "Fournisseur non configuré.", ""))
		return
	}
	providerRaw, _ := json.Marshal(p)
	if turn.ProviderDigest != "" && hash(providerRaw) != turn.ProviderDigest {
		_ = s.settleAssistTurn(turn, "", nil, refuse("provider_changed", refusalService, "Configuration du fournisseur modifiée depuis la confirmation ; reposer la question.", ""))
		return
	}
	p, e = assistantProvider(p)
	if e != nil {
		_ = s.settleAssistTurn(turn, "", nil, refuse("provider_unavailable", refusalService, e.Error(), ""))
		return
	}
	dir, e := os.MkdirTemp("", "swarm-page-question-")
	if e != nil {
		_ = s.settleAssistTurn(turn, "", nil, refuse("provider_unavailable", refusalService, "Répertoire isolé indisponible.", ""))
		return
	}
	defer os.RemoveAll(dir)
	p = applyModelRoute(p, turn.ModelRoute)
	p, e = assistantStructuredProvider(p, turn, dir)
	if e != nil {
		_ = s.settleAssistTurn(turn, "", nil, refuse("provider_schema", refusalService, "Contrat de réponse indisponible.", e.Error()))
		return
	}
	cmd := exec.Command(p.Command, p.Args...)
	cmd.Dir = dir
	cmd.Env = providerEnvironment(p.Env)
	cmd.Stdin = strings.NewReader(turn.Prompt)
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true, Pdeathsig: syscall.SIGKILL}
	// os/exec owns pipe-copy lifetimes. WaitDelay bounds inherited output handles,
	// independently of stdout EOF, so closing stdout cannot disable the deadline.
	reader, writer := io.Pipe()
	cmd.Stdout = writer
	diagnostic := &assistDiagnostic{}
	cmd.Stderr = diagnostic
	cmd.WaitDelay = 2 * time.Second
	output := make(chan assistOutput, 1)
	go func() { output <- readAssistOutput(reader) }()
	if e = cmd.Start(); e != nil {
		_ = writer.Close()
		<-output
		_ = reader.Close()
		_ = s.settleAssistTurn(turn, "", nil, refuse("provider_unavailable", refusalService, "Fournisseur non démarré.", e.Error()))
		return
	}
	turn.PID = cmd.Process.Pid
	turn.ProcessStamp = processStamp(turn.PID)
	turn.Host = hostIdentity()
	_ = s.saveAssistTurn(turn)
	done := make(chan error, 1)
	go func() { done <- cmd.Wait(); _ = writer.Close() }()
	timer := time.NewTimer(deadline)
	defer timer.Stop()
	poll := time.NewTicker(250 * time.Millisecond)
	defer poll.Stop()
	var failure *AssistRefusal
	var waitErr error
	waiting := true
	for waiting {
		select {
		case waitErr = <-done:
			waiting = false
		case <-timer.C:
			failure = refuse("timeout", refusalService, fmt.Sprintf("Délai de %.0f secondes dépassé ; aucune conclusion n’en est tirée.", deadline.Seconds()), "")
			_ = syscall.Kill(-turn.PID, syscall.SIGKILL)
			waitErr = <-done
			waiting = false
		case <-poll.C:
			current, err := s.assistTurn(turn.ID)
			if err != nil || !current.active() {
				failure = refuse("cancelled", refusalService, "Question interrompue ; aucune relance automatique.", "")
				_ = syscall.Kill(-turn.PID, syscall.SIGKILL)
				waitErr = <-done
				waiting = false
			}
		}
	}
	_ = syscall.Kill(-turn.PID, syscall.SIGKILL)
	out := <-output
	_ = reader.Close()
	if failure == nil && out.err != nil {
		failure = refuse("provider_output", refusalService, "Réponse du fournisseur indisponible.", out.err.Error())
	}
	if failure == nil && waitErr != nil {
		failure = refuse("provider_failed", refusalService, "Le fournisseur s’est arrêté en erreur ; ce n’est pas un résultat.", guardBlock(waitErr.Error()+" : "+diagnostic.String(), 600))
	}
	if failure == nil && out.err != nil {
		failure = refuse("provider_output", refusalService, "Sortie du fournisseur illisible ou trop volumineuse.", out.err.Error())
	}
	_ = s.settleAssistTurn(turn, out.reply, out.usage, failure)
}

type assistOutput struct {
	streamBytes int64
	events      int
	lastEvent   string
	finalSeen   bool

	cooldown      *ProviderCooldown
	cooldownError error
	reply         string
	usage         *Usage
	err           error
}

func readAssistOutput(r io.Reader, observers ...func(*ProviderCooldown) error) assistOutput {
	out := assistOutput{}
	counted := &assistCountingReader{Reader: r}
	limited := &io.LimitedReader{R: counted, N: assistReplyLimit + 1}
	scanner := bufio.NewScanner(limited)
	scanner.Buffer(make([]byte, 0, 64*1024), assistReplyLimit)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" || !strings.HasPrefix(line, "{") {
			continue
		}
		var data map[string]any
		if json.Unmarshal([]byte(line), &data) != nil {
			continue
		}
		out.events++
		out.lastEvent = assistEventKind(data["type"])
		if data["type"] == "result" || data["type"] == "turn.completed" {
			out.finalSeen = true
		}
		if c := observedProviderCooldown(data, time.Now()); c != nil {
			if out.cooldown != nil && c.ResetAt == 0 {
				c.ResetAt = out.cooldown.ResetAt
			}
			for _, observe := range observers {
				if observe != nil {
					if e := observe(c); e != nil {
						out.cooldownError = e
					}
				}
			}
			out.cooldown = c
		}
		if data["is_error"] == true || data["type"] == "error" {
			out.err = fmt.Errorf("%s", guardBlock(fmt.Sprint(data["result"], " ", data["error"]), 600))
		}
		if reply := providerReply(data); reply != "" {
			out.reply = reply
		}
		if structured, ok := data["structured_output"].(map[string]any); ok && data["type"] == "result" {
			b, err := json.Marshal(structured)
			if err == nil {
				out.reply = string(b)
			}
		}
		if u := providerUsage(data); u != nil {
			out.usage = u
		}
	}
	if e := scanner.Err(); e != nil {
		out.err = e
	}
	if limited.N == 0 {
		out.err = io.ErrShortBuffer
	}
	_, _ = io.Copy(io.Discard, counted)
	out.streamBytes = counted.bytes
	return out
}

type assistDiagnostic struct{ bytes.Buffer }

func (d *assistDiagnostic) Write(p []byte) (int, error) {
	n := len(p)
	left := 4096 - d.Len()
	if left > 0 {
		if len(p) > left {
			p = p[:left]
		}
		_, _ = d.Buffer.Write(p)
	}
	return n, nil
}

// Same detached-process mechanism as coding supervisors: a cockpit restart does
// not lose the deadline, output or settlement of a confirmed page question.
func (s *Store) spawnAssistTurn(turn AssistTurn) error {
	exe, e := os.Executable()
	if e != nil {
		return e
	}
	cmd := exec.Command(exe, "--root", s.root, "_assist", turn.ID)
	cmd.Dir = s.root
	cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	null, e := os.OpenFile(os.DevNull, os.O_RDWR, 0)
	if e != nil {
		return e
	}
	defer null.Close()
	cmd.Stdin = null
	cmd.Stdout = null
	cmd.Stderr = null
	if e = cmd.Start(); e != nil {
		_ = s.settleAssistTurn(turn, "", nil, refuse("supervisor_unavailable", refusalService, "Supervision de la question non démarrée.", e.Error()))
		return e
	}
	return cmd.Process.Release()
}

func stopVerifiedAssist(t AssistTurn) {
	if t.PID > 0 && t.Host == hostIdentity() && t.ProcessStamp != "" && processStamp(t.PID) == t.ProcessStamp {
		_ = syscall.Kill(-t.PID, syscall.SIGKILL)
	}
}

// Counters only: never retain provider text, identifiers or arbitrary event types.
type assistCountingReader struct {
	io.Reader
	bytes int64
}

func (r *assistCountingReader) Read(p []byte) (int, error) {
	n, e := r.Reader.Read(p)
	r.bytes += int64(n)
	return n, e
}
func assistEventKind(v any) string {
	switch v {
	case "system", "assistant", "user", "result", "error", "stream_event", "thread.started", "turn.started", "turn.completed", "turn.failed", "item.started", "item.updated", "item.completed":
		return v.(string)
	default:
		return "other"
	}
}
func (o assistOutput) progressDiagnostic() string {
	last := o.lastEvent
	if last == "" {
		last = "none"
	}
	return fmt.Sprintf("sortie fournisseur : %d octets, %d événements JSON, dernier type=%s, événement final=%t ; ceci ne vaut pas validation", o.streamBytes, o.events, last, o.finalSeen)
}
