//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"
	"unicode"
)

func terminalText(s string) string {
	return strings.Map(func(r rune) rune {
		if unicode.IsControl(r) || unicode.Is(unicode.Cf, r) {
			if r == '\n' || r == '\t' {
				return ' '
			}
			return -1
		}
		return r
	}, s)
}
func hostIdentity() string {
	name, _ := os.Hostname()
	boot, _ := os.ReadFile("/proc/sys/kernel/random/boot_id")
	namespace, _ := os.Readlink("/proc/self/ns/pid")
	return name + ":" + strings.TrimSpace(string(boot)) + ":" + namespace
}
func processStamp(pid int) string {
	b, e := os.ReadFile(fmt.Sprintf("/proc/%d/stat", pid))
	if e != nil {
		return ""
	}
	// comm may contain spaces and parentheses; fields after its LAST closing paren.
	at := strings.LastIndex(string(b), ")")
	if at < 0 {
		return ""
	}
	f := strings.Fields(string(b[at+1:]))
	if len(f) < 20 || f[0] == "Z" {
		return ""
	}
	return f[19]
}
func (s *Store) spawnAgent(a Agent) error {
	exe, e := os.Executable()
	if e != nil {
		return e
	}
	cmd := exec.Command(exe, "--root", s.root, "_supervise", a.ID)
	cmd.Dir = s.root
	cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	// The supervisor is detached from terminal; all diagnostics go through SQLite.
	null, e := os.OpenFile(os.DevNull, os.O_RDWR, 0)
	if e != nil {
		return e
	}
	defer null.Close()
	cmd.Stdin = null
	cmd.Stdout = null
	cmd.Stderr = null
	if e = cmd.Start(); e != nil {
		_ = s.finishAgent(a, "failed", "Impossible de démarrer le superviseur", nil)
		return e
	}
	return cmd.Process.Release()
}
func providerEnvironment(allow []string) []string {
	// Provider credentials are inherited only if explicitly allowlisted, never persisted.
	names := []string{"PATH", "HOME", "USER", "LOGNAME", "LANG", "LC_ALL", "TERM", "TMPDIR", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "CODEX_HOME", "SSL_CERT_FILE", "SSL_CERT_DIR", "NODE_EXTRA_CA_CERTS", "HTTPS_PROXY", "HTTP_PROXY", "NO_PROXY"}
	names = append(names, allow...)
	out := []string{}
	seen := map[string]bool{}
	for _, name := range names {
		if seen[name] || strings.ContainsAny(name, "=\x00") {
			continue
		}
		seen[name] = true
		if value, ok := os.LookupEnv(name); ok {
			out = append(out, name+"="+value)
		}
	}
	return out
}

const maxProviderEventBytes = 1 << 20

type outputSink struct {
	collectReply     bool
	reply            string
	usage            *Usage
	discarding       bool
	visibilityLogged bool
	guard            *loopGuard
	mu               sync.Mutex
	pending          []byte
	logs             []AgentLog
	s                *Store
	id               string
	capture          bool
	bytes            int
	lines            int
	truncated        bool
	activity         string
}

func (w *outputSink) Write(p []byte) (int, error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	n := len(p)
	if w.guard != nil && n > 0 {
		w.guard.lastOutput = time.Now()
	}
	// Provider activity is updated in memory; supervisor persists it on heartbeat.
	for len(p) > 0 {
		length := bytes.IndexByte(p, '\n')
		if length < 0 {
			length = len(p)
		}
		if !w.discarding && len(w.pending)+length <= maxProviderEventBytes {
			w.pending = append(w.pending, p[:length]...)
		} else {
			w.pending = nil
			w.discarding = true
			w.visibilityLost("Événement fournisseur trop volumineux ; chronométrage par outil suspendu")
		}
		if length == len(p) {
			break
		}
		if !w.discarding {
			w.line(w.pending)
		}
		w.discarding = false
		w.pending = nil
		p = p[length+1:]
	}
	return n, nil
}
func (w *outputSink) line(line []byte) {
	var data map[string]any
	decodeErr := json.Unmarshal(line, &data)
	if decodeErr != nil && bytes.HasPrefix(bytes.TrimSpace(line), []byte("{")) {
		w.visibilityLost("Événement JSON illisible ; chronométrage par outil suspendu")
	}
	if decodeErr == nil {
		if w.collectReply {
			if reply := providerReply(data); reply != "" {
				w.reply = reply
			}
		}
		if u := providerUsage(data); u != nil {
			w.usage = u
		}
		if w.guard != nil {
			beforeCalls, beforeResults := w.guard.calls, w.guard.completed
			w.guard.observe(data, time.Now())
			if w.guard.calls > beforeCalls {
				w.logs = append(w.logs, AgentLog{AgentID: w.id, At: now(), Kind: "activity", Message: fmt.Sprintf("%s · %s%s", w.guard.lastTool, w.guard.lastAction, operationTarget(w.guard.actionDetail))})
			}
			if w.guard.completed > beforeResults {
				w.logs = append(w.logs, AgentLog{AgentID: w.id, At: now(), Kind: "activity", Message: fmt.Sprintf("Résultat d'outil reçu · %d reçus · %d en attente", w.guard.completed, len(w.guard.pending))})
			}
		}
		kind, _ := data["type"].(string)
		switch kind {
		case "item.started", "item.completed":
			if item, ok := data["item"].(map[string]any); ok {
				if typ, ok := item["type"].(string); ok {
					w.activity = "Événement fournisseur : " + kind + " / " + terminalText(typ)
				}
			}
		case "assistant", "user", "result", "system", "thread.started", "turn.started", "turn.completed", "turn.failed":
			w.activity = "Événement fournisseur : " + kind
		}
	}
	if w.capture && !w.truncated {
		if w.bytes+len(line) > 1<<20 || w.lines >= 1000 {
			w.truncated = true
			w.logs = append(w.logs, AgentLog{AgentID: w.id, At: now(), Kind: "output-limit", Message: "Capture arrêtée à 1 Mio ou 1 000 lignes ; le processus continue"})
			return
		}
		w.bytes += len(line) + 1
		w.lines++
		w.logs = append(w.logs, AgentLog{AgentID: w.id, At: now(), Kind: "output", Message: boundedLogMessage(string(line))})
	}
}
func (w *outputSink) guardReason() string {
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.guard == nil {
		return ""
	}
	return w.guard.check(time.Now())
}
func (w *outputSink) current() string {
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.guard != nil && w.guard.calls > 0 {
		return fmt.Sprintf("%d appels · %d résultats · dernier outil : %s", w.guard.calls, w.guard.completed, w.guard.lastTool)
	}
	return w.activity
}
func (w *outputSink) progress() AgentProgress {
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.guard == nil {
		return AgentProgress{}
	}
	return w.guard.summary()
}
func (w *outputSink) visibilityLost(reason string) {
	if w.guard != nil {
		w.guard.loseVisibility(reason)
	}
	if !w.visibilityLogged {
		w.logs = append(w.logs, AgentLog{AgentID: w.id, At: now(), Kind: "monitoring", Message: reason + ". Durée totale et silence restent surveillés."})
		w.visibilityLogged = true
	}
}
func (w *outputSink) persist() error {
	w.mu.Lock()
	logs := w.logs
	w.logs = nil
	w.mu.Unlock()
	if len(logs) == 0 {
		return nil
	}
	return w.s.logBatch(w.id, logs)
}
func (w *outputSink) flush() error {
	w.mu.Lock()
	if len(w.pending) > 0 {
		if !w.discarding {
			w.line(w.pending)
		}
		w.discarding = false
		w.pending = nil
	}
	w.mu.Unlock()
	return w.persist()
}

func (s *Store) supervise(id string) error {
	a, e := s.agent(id)
	if e != nil {
		return e
	}
	// Exactly one supervisor can claim a queued intent. No blind restart after crash.
	result, e := s.db.Exec("UPDATE agents SET status='starting' WHERE id=? AND status='queued'", id)
	if e != nil {
		return e
	}
	n, _ := result.RowsAffected()
	if n != 1 {
		return fmt.Errorf("session déjà prise en charge")
	}
	a.Status = "starting"
	a.Supervisor = os.Getpid()
	a.SupervisorStamp = processStamp(a.Supervisor)
	a.Heartbeat = now()
	if e = s.saveAgent(a); e != nil {
		return e
	}
	desired, e := s.desired(id)
	if e != nil {
		return e
	}
	if desired == "stop" {
		return s.finishAgent(a, "interrupted", "Lancement annulé avant démarrage", nil)
	}
	signals := make(chan os.Signal, 2)
	signal.Notify(signals, syscall.SIGTERM, syscall.SIGINT)
	defer signal.Stop(signals)
	cmd := exec.Command(a.Command, a.Args...)
	cmd.Dir = a.CWD
	cmd.Env = providerEnvironment(a.Env)
	cmd.Stdin = strings.NewReader(a.Prompt)
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	limits, e := a.Limits.normalized()
	if e != nil {
		return s.finishAgent(a, "failed", e.Error(), nil)
	}
	sink := &outputSink{collectReply: a.Brainstorm, guard: newLoopGuard(limits), s: s, id: id, capture: a.Capture, activity: "Processus actif ; aucune activité fournisseur reçue"}
	cmd.Stdout = sink
	cmd.Stderr = sink
	// Bound wait if an orphaned descendant keeps stdout/stderr open after parent exits.
	cmd.WaitDelay = 2 * time.Second
	if e = cmd.Start(); e != nil {
		return s.finishAgent(a, "failed", "Échec de lancement : "+e.Error(), nil)
	}
	a.Child = cmd.Process.Pid
	a.ChildStamp = processStamp(a.Child)
	a.Status = "running"
	a.Activity = "Processus démarré ; attente fournisseur"
	a.Heartbeat = now()
	if e = s.saveAgent(a); e != nil {
		_ = syscall.Kill(-a.Child, syscall.SIGKILL)
		_ = cmd.Wait()
		return e
	}
	_ = s.log(id, "lifecycle", "Processus fournisseur démarré")
	_ = s.log(id, "limits", fmt.Sprintf("Limites : durée %ds ; silence %ds ; outil observable %ds ; appels %d ; répétitions %d ; erreurs consécutives %d", a.Timeout, limits.SilenceSeconds, limits.ToolSeconds, limits.MaxToolCalls, limits.MaxRepeatedCalls, limits.MaxConsecutiveErrors))
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	tick := time.NewTicker(time.Second)
	defer tick.Stop()
	deadline := time.Now().Add(time.Duration(a.Timeout) * time.Second)
	stopping := false
	forced := false
	var stopAt time.Time
	reason := ""
	requestStop := func(message string) {
		if stopping {
			return
		}
		stopping = true
		stopAt = time.Now()
		reason = message
		a.Status = "stopping"
		a.Activity = message + " ; arrêt en attente"
		_ = syscall.Kill(-a.Child, syscall.SIGTERM)
		_ = s.saveAgent(a)
		_ = s.log(id, "lifecycle", a.Activity)
	}
	for {
		select {
		case <-signals:
			requestStop("Interruption du superviseur")
		case e = <-done:
			// Clean any descendants of this owned process group before releasing workspace.
			_ = syscall.Kill(-a.Child, syscall.SIGKILL)
			if err := sink.flush(); err != nil {
				e = err
			}
			if !stopping {
				if limitReason := sink.guardReason(); limitReason != "" {
					stopping = true
					reason = limitReason
				}
			}
			a.Progress = sink.progress()
			a.Usage = sink.usageSnapshot()
			a.Reply = sink.replySnapshot()
			code := cmd.ProcessState.ExitCode()
			state := "completed"
			message := "Processus terminé ; résultat à examiner, tâche non acceptée"
			if e != nil {
				state = "failed"
				message = "Processus en échec (code " + strconv.Itoa(code) + ") ; consulter les journaux"
			}
			if stopping {
				state = "interrupted"
				message = reason + " ; fin du processus confirmée"
			}
			return s.finishAgent(a, state, message, &code)
		case <-tick.C:
			if err := sink.persist(); err != nil {
				requestStop("Échec de persistance des journaux")
			}
			desired, e = s.desired(id)
			if e != nil {
				requestStop("Stockage du superviseur indisponible")
			}
			if desired == "stop" {
				requestStop("Arrêt demandé par opérateur")
			}
			if reason := sink.guardReason(); reason != "" {
				requestStop(reason)
			}
			if time.Now().After(deadline) {
				requestStop("Budget de temps atteint")
			}
			if stopping && time.Since(stopAt) > 3*time.Second && !forced {
				forced = true
				_ = syscall.Kill(-a.Child, syscall.SIGKILL)
				_ = s.log(id, "lifecycle", "Arrêt forcé après délai coopératif")
			}
			if !stopping {
				a.Activity = sink.current()
			}
			a.Progress = sink.progress()
			a.Usage = sink.usageSnapshot()
			a.Reply = sink.replySnapshot()
			a.Heartbeat = now()
			if e = s.saveAgent(a); e != nil {
				requestStop("Échec de persistance")
			}
		}
	}
}

func (s *Store) reconcile(id string) error {
	a, e := s.agent(id)
	if e != nil {
		return e
	}
	if !activeAgent(a) {
		return s.settleAgentTask(a)
	}
	if a.Host != hostIdentity() {
		return fmt.Errorf("hôte/redémarrage différent : vérifier les processus sur l'hôte d'origine ; aucune libération automatique")
	}
	if a.Supervisor > 0 && processStamp(a.Supervisor) == a.SupervisorStamp {
		return fmt.Errorf("superviseur encore présent")
	}
	if a.Status == "starting" && a.Child == 0 {
		return fmt.Errorf("lancement interrompu avant enregistrement PID : vérifier les processus manuellement, verrou conservé")
	}
	if a.Child > 0 {
		if e = syscall.Kill(-a.Child, 0); e == nil || e == syscall.EPERM {
			return fmt.Errorf("groupe de processus encore présent ; vérifier et arrêter sur l'hôte avant réconciliation")
		}
		if e != syscall.ESRCH {
			return e
		}
	}
	// CAS claim protects against a supervisor just starting an unclaimed intent.
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	var body []byte
	var status string
	if e = tx.QueryRow("SELECT body,status FROM agents WHERE id=?", id).Scan(&body, &status); e != nil {
		return e
	}
	var latest Agent
	_ = json.Unmarshal(body, &latest)
	if latest.Supervisor != a.Supervisor || status != a.Status {
		return fmt.Errorf("état modifié : relire")
	}
	if _, e = tx.Exec("UPDATE agents SET status='interrupted' WHERE id=?", id); e != nil {
		return e
	}
	if e = tx.Commit(); e != nil {
		return e
	}
	return s.finishAgent(a, "interrupted", "Absence de superviseur et de groupe confirmée ; reprise explicite requise", nil)
}

func (s *Store) initProviders() error {
	path := filepath.Join(s.root, ".swarm", "providers.json")
	p := Providers{Schema: 1, Providers: map[string]Provider{}}
	for name, args := range map[string][]string{"claude": {"-p", "--output-format", "stream-json", "--verbose"}, "codex": {"exec", "--json", "--sandbox", "workspace-write", "-"}} {
		command, e := exec.LookPath(name)
		if e != nil {
			continue
		}
		command, e = filepath.Abs(command)
		if e != nil {
			return e
		}
		p.Providers[name] = Provider{Command: command, Args: args, Env: []string{}}
	}
	b, _ := json.MarshalIndent(p, "", "  ")
	f, e := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
	if e != nil {
		return e
	}
	defer f.Close()
	_, e = f.Write(b)
	return e
}

// Keep io imported as an explicit compile-time contract for the bounded sink.
var _ io.Writer = (*outputSink)(nil)

func (w *outputSink) usageSnapshot() *Usage {
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.usage == nil {
		return nil
	}
	u := *w.usage
	return &u
}

func (w *outputSink) replySnapshot() string { w.mu.Lock(); defer w.mu.Unlock(); return w.reply }
