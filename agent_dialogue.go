//go:build linux

package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"sync"
	"time"
)

const dialogueMonitoring = "Dialogue contrôlé : appels observables cumulés, arrêt réactif sur limite. Les permissions restent celles du fournisseur. /fin termine la session sans valider la tâche."

func agentMonitoring(a Agent) string {
	if a.Mode == "dialogue" {
		return dialogueMonitoring
	}
	return terminalMonitoring
}
func interactiveMode(mode string) bool { return mode == "terminal" || mode == "dialogue" }
func dialogueProvider(p Provider) error {
	defaults := map[string][]string{"codex": {"exec", "--json", "--sandbox", "workspace-write", "-"}, "claude": {"-p", "--output-format", "stream-json", "--verbose"}}
	if !reflect.DeepEqual(p.Args, defaults[filepath.Base(p.Command)]) || defaults[filepath.Base(p.Command)] == nil {
		return fmt.Errorf("Dialogue contrôlé : configuration standard Codex ou Claude requise ; aucune conversion implicite des permissions.")
	}
	return nil
}
func dialogueResumeArgs(a Agent, session string) ([]string, error) {
	args := append([]string{}, a.Args...)
	if session == "" {
		return args, nil
	}
	if !preparationKey(session) || strings.HasPrefix(session, "-") {
		return nil, fmt.Errorf("Identité de session fournisseur invalide")
	}
	switch filepath.Base(a.Command) {
	case "claude":
		return append(args, "--resume", session), nil
	case "codex":
		if len(args) == 0 || args[len(args)-1] != "-" {
			return nil, fmt.Errorf("Adaptateur Codex incompatible")
		}
		return append(args[:len(args)-1], "resume", session, "-"), nil
	}
	return nil, fmt.Errorf("Adaptateur indisponible")
}

type dialogueEvent struct {
	Kind string         `json:"kind"`
	Turn string         `json:"turn"`
	Data map[string]any `json:"data,omitempty"`
}

// The wrapper runs in the terminal's process group; provider children inherit it.
// Only wrapper messages travel over fd3. Provider stdout is parsed as data.
func (s *Store) runAgentDialogue(id string) error {
	a, e := s.agent(id)
	if e != nil {
		return e
	}
	if a.Mode != "dialogue" {
		return fmt.Errorf("Session non dialoguée")
	}
	channel := os.NewFile(3, "dialogue-events")
	if channel == nil {
		return fmt.Errorf("Canal de supervision absent")
	}
	defer channel.Close()
	enc := json.NewEncoder(channel)
	authorization := os.NewFile(4, "dialogue-authorization")
	if authorization == nil {
		return fmt.Errorf("Canal d’autorisation absent")
	}
	defer authorization.Close()
	// No implicit restart: a dead wrapper must not reset accumulated limits.
	var count int
	if e = s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=?", id).Scan(&count); e != nil {
		return e
	}
	if count != 0 {
		return fmt.Errorf("Dialogue déjà commencé ; créer une nouvelle tentative après revue")
	}
	fmt.Fprintln(os.Stdout, dialogueMonitoring)
	input := bufio.NewScanner(os.Stdin)
	input.Buffer(make([]byte, 4096), 16001)
	session := ""
	question := a.Prompt
	for turn := 0; turn < 20; turn++ {
		if turn > 0 {
			fmt.Fprint(os.Stdout, "\nVous > ")
			if !input.Scan() {
				return input.Err()
			}
			question = strings.TrimSpace(input.Text())
			if question == "/fin" {
				return nil
			}
			if question == "" {
				turn--
				continue
			}
		}
		args, err := dialogueResumeArgs(a, session)
		if err != nil {
			return err
		}
		turnID := newID("dt-")
		if _, e = s.db.Exec("INSERT INTO agent_dialogue_turns(id,agent_id,question,status,session_id) VALUES(?,?,?,'running',?)", turnID, a.ID, question, session); e != nil {
			return e
		}
		if e = enc.Encode(dialogueEvent{Kind: "start", Turn: turnID}); e != nil {
			return e
		}
		var ack [1]byte
		if _, e = io.ReadFull(authorization, ack[:]); e != nil || ack[0] != 1 {
			return fmt.Errorf("Le superviseur refuse un nouvel échange ; limite ou arrêt demandé")
		}
		cmd := exec.Command(a.Command, args...)
		cmd.Dir = a.CWD
		cmd.Env = providerEnvironment(a.Env)
		cmd.Stdin = strings.NewReader(question)
		cmd.WaitDelay = 2 * time.Second
		pipe, err := cmd.StdoutPipe()
		if err != nil {
			return err
		}
		diagnostic := &assistDiagnostic{}
		cmd.Stderr = diagnostic
		if err = cmd.Start(); err != nil {
			return err
		}
		scanner := bufio.NewScanner(pipe)
		scanner.Buffer(make([]byte, 4096), maxProviderEventBytes)
		observed := ""
		var parseErr error
		for scanner.Scan() {
			var d map[string]any
			if json.Unmarshal(scanner.Bytes(), &d) != nil {
				parseErr = fmt.Errorf("Événement fournisseur illisible ; dialogue arrêté")
				break
			}
			candidate := ""
			if d["type"] == "thread.started" {
				candidate, _ = d["thread_id"].(string)
			}
			if d["type"] == "system" || d["type"] == "result" {
				candidate, _ = d["session_id"].(string)
			}
			if candidate != "" {
				if !preparationKey(candidate) || strings.HasPrefix(candidate, "-") || (observed != "" && candidate != observed || session != "" && candidate != session) {
					parseErr = fmt.Errorf("Identité de session fournisseur modifiée")
					break
				}
				observed = candidate
			}
			if e = enc.Encode(dialogueEvent{Kind: "event", Turn: turnID, Data: d}); e != nil {
				parseErr = e
				break
			}
			if text := providerReply(d); text != "" {
				fmt.Fprintln(os.Stdout, "\nAgent > "+terminalText(text))
			}
		}
		if scanner.Err() != nil {
			parseErr = scanner.Err()
		}
		if parseErr != nil {
			_ = cmd.Process.Kill()
		}
		err = cmd.Wait()
		if parseErr != nil {
			return parseErr
		}
		if err != nil {
			return fmt.Errorf("Échange fournisseur en échec ; consulter les journaux")
		}
		if observed == "" {
			return fmt.Errorf("Aucune identité de conversation reçue ; reprise refusée")
		}
		session = observed
		if _, e = s.db.Exec("UPDATE agent_dialogue_turns SET status='answered',session_id=? WHERE id=?", session, turnID); e != nil {
			return e
		}
		if e = enc.Encode(dialogueEvent{Kind: "end", Turn: turnID}); e != nil {
			return e
		}
	}
	return fmt.Errorf("Limite de 20 échanges atteinte")
}

// One sink and one guard for the entire attempt. Namespace tool ids per turn:
// Codex can reuse item_0 on each exec resume.
type dialogueMonitor struct {
	authorize io.Writer
	mu        sync.Mutex
	sink      *outputSink
	active    bool
	turn      string
	usage     *Usage
	failure   string
}

func newDialogueMonitor(s *Store, a Agent) *dialogueMonitor {
	return &dialogueMonitor{sink: &outputSink{s: s, id: a.ID, guard: newLoopGuard(a.Limits), capture: a.Capture}}
}
func namespaceDialogueTools(d map[string]any, prefix string) {
	if item, ok := d["item"].(map[string]any); ok {
		if id, ok := item["id"].(string); ok {
			item["id"] = prefix + ":" + id
		}
	}
	if m, ok := d["message"].(map[string]any); ok {
		if content, ok := m["content"].([]any); ok {
			for _, v := range content {
				if b, ok := v.(map[string]any); ok {
					for _, key := range []string{"id", "tool_use_id"} {
						if id, ok := b[key].(string); ok {
							b[key] = prefix + ":" + id
						}
					}
				}
			}
		}
	}
}
func (m *dialogueMonitor) consume(r io.Reader) {
	scan := bufio.NewScanner(r)
	scan.Buffer(make([]byte, 4096), 2*maxProviderEventBytes)
	for scan.Scan() {
		var event dialogueEvent
		err := json.Unmarshal(scan.Bytes(), &event)
		m.mu.Lock()
		if err != nil || !preparationKey(event.Turn) {
			m.failure = "Canal de dialogue illisible"
			m.mu.Unlock()
			return
		}
		switch event.Kind {
		case "start":
			desired, err := m.sink.s.desired(m.sink.id)
			allowed := !m.active && m.failure == "" && m.sink.guard.reason == "" && err == nil && desired != "stop"
			if m.authorize != nil {
				ack := byte(0)
				if allowed {
					ack = 1
				}
				if _, err = m.authorize.Write([]byte{ack}); err != nil {
					m.failure = "Canal d’autorisation indisponible"
					allowed = false
				}
			}
			if !allowed {
				if m.failure == "" {
					m.failure = "Nouvel échange refusé : arrêt demandé ou limite atteinte"
				}
			} else {
				m.active = true
				m.turn = event.Turn
				m.sink.guard.lastOutput = time.Now()
				m.sink.usage = nil
			}
		case "event":
			if !m.active || m.turn != event.Turn {
				m.failure = "Événement hors échange"
			} else {
				if event.Data["type"] == "item.started" || event.Data["type"] == "item.completed" {
					if item, ok := event.Data["item"].(map[string]any); ok {
						switch item["type"] {
						case "command_execution", "mcp_tool_call", "agent_message", "reasoning", "todo_list", "file_change", "web_search":
						default:
							m.failure = "Type d’activité fournisseur non pris en charge ; dialogue arrêté"
						}
					}
				}
				namespaceDialogueTools(event.Data, event.Turn)
				b, _ := json.Marshal(event.Data)
				m.sink.line(b)
				m.sink.guard.lastOutput = time.Now()
				if m.sink.guard.degraded != "" {
					m.failure = "Visibilité des outils perdue"
				}
			}
		case "end":
			if !m.active || m.turn != event.Turn {
				m.failure = "Fin d’échange incohérente"
			} else {
				m.active = false
				m.addUsage()
				if len(m.sink.guard.pending) > 0 && m.sink.guard.reason == "" {
					m.failure = "Résultats d’outils manquants ; reprise refusée"
				}
			}
		default:
			m.failure = "Événement de dialogue inconnu"
		}
		if err := m.sink.persist(); err != nil {
			m.failure = "Persistance du dialogue indisponible"
		}
		m.mu.Unlock()
	}
	m.mu.Lock()
	if m.active {
		m.addUsage()
		m.active = false
	}
	m.mu.Unlock()
	if scan.Err() != nil {
		m.mu.Lock()
		m.failure = "Canal de dialogue interrompu"
		m.mu.Unlock()
	}
}
func (m *dialogueMonitor) addUsage() {
	u := m.sink.usage
	if u == nil {
		return
	}
	if m.usage == nil {
		m.usage = &Usage{Source: "Événements fournisseur du dialogue", Scope: "Somme des usages rapportés par échange ; couverture partielle si un échange n’en fournit pas"}
	}
	for _, pair := range []struct {
		dst **int64
		src *int64
	}{{&m.usage.CacheRead, u.CacheRead}, {&m.usage.CacheCreation, u.CacheCreation}, {&m.usage.CachedInput, u.CachedInput}} {
		if pair.src != nil {
			if *pair.dst == nil {
				v := int64(0)
				*pair.dst = &v
			}
			**pair.dst += *pair.src
		}
	}
	m.usage.Input += u.Input
	m.usage.Output += u.Output
	m.usage.At = u.At
	if u.ReportedCost != nil {
		if m.usage.ReportedCost == nil {
			n := 0.0
			m.usage.ReportedCost = &n
		}
		*m.usage.ReportedCost += *u.ReportedCost
	}
	m.sink.usage = nil
}
func (m *dialogueMonitor) snapshot() (AgentProgress, *Usage, string, string) {
	m.mu.Lock()
	defer m.mu.Unlock()
	reason := m.failure
	if reason == "" {
		reason = m.sink.guard.reason
	}
	if reason == "" && m.active {
		reason = m.sink.guard.check(time.Now())
	}
	activity := "À vous de répondre dans le dialogue"
	if m.active {
		activity = "Échange fournisseur en cours"
	}
	var usage *Usage
	if m.usage != nil {
		u := *m.usage
		for _, dst := range []**int64{&u.CacheRead, &u.CacheCreation, &u.CachedInput} {
			if *dst != nil {
				n := **dst
				*dst = &n
			}
		}
		if u.ReportedCost != nil {
			n := *u.ReportedCost
			u.ReportedCost = &n
		}
		usage = &u
	}
	return m.sink.guard.summary(), usage, reason, activity
}
