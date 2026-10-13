//go:build linux

package engine

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

const assistReplyLimit = 1 << 20

// Single bounded inference in an empty directory, with coding tools disabled.
// Claim by compare-and-swap prevents duplicate HTTP confirmations spawning twice.
func assistTimeoutDuration(seconds int) (time.Duration, error) {
	if seconds < 0 || int64(seconds) > int64(^uint64(0)>>1)/int64(time.Second) {
		return 0, fmt.Errorf("Délai d’assistance entier positif ou nul requis ; représentation de durée valide requise")
	}
	return time.Duration(seconds) * time.Second, nil
}
func (s *Store) runAssistTurn(turn AssistTurn) {
	duration, err := assistTimeoutDuration(turn.TimeoutSeconds)
	if err != nil {
		duration = -1
	}
	s.runAssistTurnWithin(turn, duration)
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
	if deadline < 0 {
		_ = s.settleAssistTurn(turn, "", nil, refuse("invalid_timeout", refusalService, "Délai d’assistance invalide ; aucun appel exécuté.", ""))
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
	var expired <-chan time.Time
	if deadline > 0 {
		timer := time.NewTimer(deadline)
		defer timer.Stop()
		expired = timer.C
	}
	poll := time.NewTicker(250 * time.Millisecond)
	defer poll.Stop()
	var failure *AssistRefusal
	var waitErr error
	waiting := true
	for waiting {
		select {
		case waitErr = <-done:
			waiting = false
		case <-expired:
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

const structuredSchemaRefusal = "Schéma de réponse refusé par le fournisseur (invalid_json_schema) ; aucune revue produite"

// Classify only provider error envelopes. Never reflect arbitrary error text,
// URLs or credentials into public diagnostics, or classify quoted tool output.
func structuredProviderFailure(data map[string]any) error {
	if data["type"] != "error" && data["type"] != "turn.failed" && data["is_error"] != true {
		return nil
	}
	code, _ := data["code"].(string)
	message, _ := data["message"].(string)
	if nested, ok := data["error"].(map[string]any); ok {
		if v, ok := nested["code"].(string); ok {
			code = v
		}
		if v, ok := nested["message"].(string); ok {
			message = v
		}
	}
	if v, ok := data["result"].(string); ok {
		message += " " + v
	}
	if v, ok := data["error"].(string); ok {
		message += " " + v
	}
	text := strings.ToLower(code + " " + message)
	switch {
	case strings.Contains(text, "invalid_json_schema"):
		return fmt.Errorf("%s", structuredSchemaRefusal)
	case code == "model_capacity_exceeded" || strings.Contains(text, "model is at capacity"):
		return fmt.Errorf("provider_capacity : modèle à capacité ; réexaminer la disponibilité ou choisir explicitement une autre route ; aucune relance automatique / model at capacity")
	default:
		return fmt.Errorf("provider_error : erreur fournisseur non classée ; aucune réponse validée ; contenu privé non exposé / unclassified provider failure")
	}
}

type assistOutput struct {
	streamBytes       int64
	events            int
	lastEvent         string
	finalSeen         bool
	assistantEvents   int
	systemEvents      int
	systemSubtypes    map[string]int
	apiRetries        int
	lastSystemSubtype string

	cooldown      *ProviderCooldown
	cooldownError error
	reply         string
	usage         *Usage
	err           error
}

func readAssistOutput(r io.Reader, observers ...func(*ProviderCooldown) error) assistOutput {
	return readAssistOutputObserved(r, nil, observers...)
}

func readAssistOutputObserved(r io.Reader, activity func(), observers ...func(*ProviderCooldown) error) assistOutput {
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
		if activity != nil && assistProductiveActivity(data) {
			activity()
		}
		out.lastEvent = assistEventKind(data["type"])
		if data["type"] == "assistant" {
			out.assistantEvents++
		}
		if data["type"] == "system" {
			out.systemEvents++
			out.lastSystemSubtype = assistSystemSubtype(data["subtype"])
			if out.systemSubtypes == nil {
				out.systemSubtypes = make(map[string]int)
			}
			out.systemSubtypes[out.lastSystemSubtype]++
			if data["subtype"] == "api_retry" {
				out.apiRetries++
			}
		}
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
		if failure := structuredProviderFailure(data); failure != nil {
			out.err = failure
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

// Known reasoning/content events indicate liveness, not task success. Setup,
// errors, quota retries, unknown JSON and arbitrary output cannot extend silence.
func assistProductiveActivity(data map[string]any) bool {
	switch data["type"] {
	case "assistant", "item.updated", "item.completed":
		return true
	case "system":
		return data["subtype"] == "thinking_tokens" || data["subtype"] == "task_progress" || data["subtype"] == "hook_progress"
	case "stream_event":
		event, ok := data["event"].(map[string]any)
		return ok && event["type"] == "content_block_delta"
	}
	return false
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

// Only protocol labels are retained; arbitrary provider values may contain secrets.
func assistSystemSubtype(v any) string {
	switch v {
	case "init", "init_milestone", "thinking_tokens", "api_error", "informational", "notification", "turn_duration", "turn_starting", "model_fallback", "model_refusal_fallback", "model_refusal_no_fallback", "permission_denied", "permission_retry", "api_retry", "status", "compact_boundary", "hook_started", "hook_progress", "hook_response", "task_started", "task_progress", "task_notification":
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
	subtypes, _ := json.Marshal(o.systemSubtypes)
	return fmt.Sprintf("sortie fournisseur : %d octets, %d événements JSON, dernier type=%s, événement final=%t ; messages assistant=%d, événements système=%d, relances API signalées=%d, dernier sous-type système=%s ; types système=%s ; ceci ne vaut pas validation", o.streamBytes, o.events, last, o.finalSeen, o.assistantEvents, o.systemEvents, o.apiRetries, o.lastSystemSubtype, subtypes)
}
