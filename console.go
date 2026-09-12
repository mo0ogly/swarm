//go:build linux

package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/signal"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"
	"unicode/utf8"

	"golang.org/x/sys/unix"
)

func (s *Store) cockpitSnapshot(work string) (map[string]any, error) {
	s = s.readScope()
	w, e := s.get(work)
	if e != nil {
		return nil, e
	}
	agents, e := s.agents(work)
	if e != nil {
		return nil, e
	}
	views := []map[string]any{}
	for _, a := range agents {
		d, _ := s.desired(a.ID)
		views = append(views, map[string]any{"agent": a, "observed_status": observedAgent(a), "desired": d, "cost": "not_available"})
	}
	actions := map[string][]TaskAction{}
	for i := range w.Tasks {
		actions[w.Tasks[i].ID] = s.taskActions(&w, &w.Tasks[i], agents)
	}
	return map[string]any{"work": w, "validation": s.validationState(&w), "agents": views, "paused": s.paused(work), "priority": s.priorities(work), "task_actions": actions}, nil
}
func (s *Store) priorities(work string) map[string]int {
	out := map[string]int{}
	rows, e := s.db.Query("SELECT task_id,priority FROM cockpit_tasks WHERE work_id=?", work)
	if e != nil {
		return out
	}
	defer rows.Close()
	for rows.Next() {
		var id string
		var n int
		if rows.Scan(&id, &n) == nil {
			out[id] = n
		}
	}
	return out
}
func (s *Store) operatorTask(work string, r Request) error {
	w, e := s.get(work)
	if e != nil {
		return e
	}
	r.Schema = 1
	r.Revision = w.Revision
	r.EventID = newID("operator-")
	kind := "task.update"
	if r.Title != "" {
		kind = "task.add"
	}
	_, e = s.executeRequest(work, kind, r)
	return e
}
func (s *Store) priority(work, id string, n int) error {
	if n < 0 || n > 9 {
		return fmt.Errorf("priorité : 0 à 9 (9 = plus haute)")
	}
	w, e := s.get(work)
	if e != nil {
		return e
	}
	if _, e = w.task(id); e != nil {
		return e
	}
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	if _, e = tx.Exec("INSERT INTO cockpit_tasks(work_id,task_id,priority) VALUES(?,?,?) ON CONFLICT(work_id,task_id) DO UPDATE SET priority=excluded.priority", work, id, n); e != nil {
		return e
	}
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, now(), "priority", fmt.Sprintf("opérateur local : tâche %s priorité %d", id, n)); e != nil {
		return e
	}
	return tx.Commit()
}

type consoleState struct {
	visit                                     Visit
	dialog                                    *taskDialog
	helpParent                                *taskDialog
	selected, filter, status, message         string
	taskSelID, agentSelID                     string
	capture                                   bool
	frozen                                    bool
	focus, taskCursor, agentCursor, logOffset int
}

const consoleHelp = `COMMANDES (Entrée pour exécuter ; Ctrl-C/q ferme seulement la console)
start TACHE FOURNISSEUR [workspace] | stop AGENT | retry AGENT [consigne]
select AGENT | filter TEXTE | status ETAT | logs AGENT | note AGENT TEXTE
pause | unpause | reconcile AGENT | assign TACHE RESPONSABLE | priority TACHE 0..9
new ID | titre | livrable | critère 1 ; critère 2
ready TACHE | submit TACHE chemin/du/handoff.md | capture on/off | freeze | help | q
start/retry créent un processus. Capture désactivée par défaut (sorties sensibles).
ready rouvre une tâche bloquée ; submit enregistre le handoff, jamais une acceptation.
`

func (s *Store) consoleCommand(work, line string, state *consoleState) (bool, error) {
	fields := strings.Fields(line)
	if len(fields) == 0 {
		return false, nil
	}
	arg := func(n int) string {
		if len(fields) > n {
			return fields[n]
		}
		return ""
	}
	switch fields[0] {
	case "q", "quit":
		return true, nil
	case "help":
		state.message = consoleHelp
		s.openTerminalHelp(work, state)
		return false, nil
	case "freeze":
		state.frozen = !state.frozen
		return false, nil
	case "capture":
		if arg(1) != "on" && arg(1) != "off" {
			return false, fmt.Errorf("capture on/off")
		}
		state.capture = arg(1) == "on"
		return false, nil
	case "filter":
		state.filter = strings.Join(fields[1:], " ")
		return false, nil
	case "status":
		state.status = arg(1)
		return false, nil
	case "select", "logs":
		a, e := s.agent(arg(1))
		if e != nil {
			return false, e
		}
		if a.WorkID != work {
			return false, fmt.Errorf("agent hors travail")
		}
		state.selected = a.ID
		return false, nil
	case "pause":
		return false, s.pause(work, true)
	case "unpause":
		return false, s.pause(work, false)
	case "stop", "reconcile", "note":
		a, e := s.agent(arg(1))
		if e != nil {
			return false, e
		}
		if a.WorkID != work {
			return false, fmt.Errorf("agent hors travail")
		}
		if fields[0] == "stop" {
			return false, s.stopAgent(a.ID)
		}
		if fields[0] == "reconcile" {
			return false, s.reconcile(a.ID)
		}
		if len(fields) < 3 {
			return false, fmt.Errorf("note AGENT TEXTE")
		}
		state.message = "Note conservée pour la reprise, pas envoyée au processus en cours"
		return false, s.log(a.ID, "operator-note", strings.Join(fields[2:], " "))
	case "start", "retry":
		if consoleBinaryReplaced() {
			return false, fmt.Errorf("Console ancienne : quitter avec q puis relancer swarm console.")
		}
		w, e := s.get(work)
		if e != nil {
			return false, e
		}
		r := Launch{Schema: 1, EventID: newID("agent-"), Revision: w.Revision, TaskID: arg(1), Provider: arg(2), Workspace: arg(3), Capture: state.capture}
		if fields[0] == "retry" {
			old, e := s.agent(arg(1))
			if e != nil {
				return false, e
			}
			if old.WorkID != work || activeAgent(old) {
				return false, fmt.Errorf("agent hors travail ou encore actif")
			}
			r.TaskID = old.TaskID
			r.Provider = old.Provider
			r.Workspace = old.CWD
			r.Previous = old.ID
			r.Role = old.Role
			r.Parent = old.Parent
			if old.ModelRoute != nil {
				r.Level = old.ModelRoute.Level
			}
			r.Instruction = strings.Join(fields[2:], " ")
			logs, e := s.logs(old.ID, 0)
			if e != nil {
				return false, e
			}
			for _, l := range logs {
				if l.Kind == "operator-note" {
					r.Instruction += "\nNote de reprise : " + l.Message
				}
			}
		}
		a, created, e := s.prepare(work, r)
		if e != nil {
			return false, e
		}
		state.selected = a.ID
		if created {
			e = s.spawnAgent(a)
		}
		state.message = "Lancement enregistré : " + a.ID
		return false, e
	case "assign":
		if len(fields) != 3 {
			return false, fmt.Errorf("assign TACHE RESPONSABLE")
		}
		return false, s.operatorTask(work, Request{ID: arg(1), Owner: arg(2)})
	case "priority":
		n, e := strconv.Atoi(arg(2))
		if e != nil {
			return false, e
		}
		return false, s.priority(work, arg(1), n)
	case "ready":
		return false, s.operatorTask(work, Request{ID: arg(1), Status: "todo", Next: "Prête pour lancement explicite"})
	case "submit":
		return false, s.submitReport(work, arg(1), arg(2))
	case "accept":
		return false, s.acceptReviewedTask(work, arg(1))
	case "new":
		segments := strings.SplitN(strings.TrimPrefix(line, "new "), "|", 4)
		if len(segments) != 4 {
			return false, fmt.Errorf("new ID | titre | livrable | critère 1 ; critère 2")
		}
		for i := range segments {
			segments[i] = strings.TrimSpace(segments[i])
		}
		criteria := []string{}
		for _, c := range strings.Split(segments[3], ";") {
			if nonempty(c) {
				criteria = append(criteria, strings.TrimSpace(c))
			}
		}
		return false, s.operatorTask(work, Request{ID: segments[0], Title: segments[1], Deliverable: segments[2], Criteria: criteria, Owner: "à attribuer", Next: "Choisir un fournisseur et lancer"})
	default:
		return false, fmt.Errorf("commande inconnue ; help")
	}
}

func clip(text string, width int) string {
	r := []rune(terminalText(text))
	if width < 1 {
		return ""
	}
	if textWidth(text) > width {
		if width < 2 {
			return "…"
		}
		return truncateCells(text, width-1) + "…"
	}
	return string(r)
}
func (s *Store) renderConsole(work string, state *consoleState, width, height int, input string) string {
	var b strings.Builder
	line := func(format string, args ...any) { fmt.Fprintln(&b, clip(fmt.Sprintf(format, args...), width)) }
	w, e := s.get(work)
	if e != nil {
		return terminalText(e.Error())
	}
	line("SWARM — %s | révision %d | départs suspendus : %t", w.Title, w.Revision, s.paused(work))
	line("OBJECTIF : %s", w.Objective)
	line("SUITE : %s", w.Next)
	line("TÂCHES (priorité 9 haute) : ID | priorité | état | responsable | livrable")
	priority := s.priorities(work)
	sort.SliceStable(w.Tasks, func(i, j int) bool { return priority[w.Tasks[i].ID] > priority[w.Tasks[j].ID] })
	maxTasks := min(6, max(1, height/6))
	for i, t := range w.Tasks {
		if i >= maxTasks {
			line("… %d autres tâches (work show pour détail)", len(w.Tasks)-i)
			break
		}
		gate := "sans gate"
		if t.Gate != nil {
			gate = "gate bloquée/périmée"
			if s.validGate(&t) {
				gate = "gate delivery valide"
			}
		}
		line("%s | %d | %s | %s | %s | %s", t.ID, priority[t.ID], t.Status, t.Owner, t.Deliverable, gate)
	}
	line("AGENTS : ID | tâche | fournisseur/rôle | état observé | activité")
	agents, e := s.agents(work)
	if e != nil {
		line("Erreur : %s", e)
	}
	shown := 0
	for _, a := range agents {
		status := observedAgent(a)
		if state.status != "" && !strings.Contains(status, state.status) {
			continue
		}
		if state.filter != "" && !strings.Contains(strings.ToLower(a.ID+" "+a.TaskID+" "+a.Provider+" "+a.Activity), strings.ToLower(state.filter)) {
			continue
		}
		if shown >= max(1, height/8) {
			line("… autres agents : filtrer ou utiliser agent list")
			break
		}
		marker := " "
		if a.ID == state.selected {
			marker = ">"
		}
		d, _ := s.desired(a.ID)
		if d != "" {
			status += " (demande " + d + ")"
		}
		line("%s%s | %s | %s/%s", marker, a.ID, a.TaskID, a.Provider, a.Role)
		line("  %s | %s", status, a.Activity)
		shown++
	}
	if shown == 0 {
		line("Aucun agent correspondant. Une tâche running ne prouve pas un agent actif.")
	}
	if state.selected != "" {
		a, e := s.agent(state.selected)
		if e == nil {
			line("SÉLECTION : %s | signal %s | coûts : non disponibles", a.ID, a.Heartbeat)
			line("ESPACE : %s | tentative %s", a.CWD, a.Attempt)
			logs, _ := s.logs(a.ID, 0)
			n := min(len(logs), max(2, height/6))
			for _, l := range logs[len(logs)-n:] {
				line("#%d %s %s", l.Seq, l.Kind, l.Message)
			}
		}
	}
	line("Capture sorties : %t | affichage figé : %t | help : commandes", state.capture, state.frozen)
	for _, m := range strings.Split(state.message, "\n") {
		if m != "" {
			line("%s", m)
		}
	}
	line("swarm> %s", input)
	return b.String()
}

func (s *Store) console(work string, in *os.File, out io.Writer, asJSON bool) error {
	if work == "" {
		var e error
		work, e = s.chooseConsoleWork(in, out, asJSON, false)
		if e != nil || work == "" {
			return e
		}
	}
	if _, e := s.get(work); e != nil {
		return e
	}
	term, e := unix.IoctlGetTermios(int(in.Fd()), unix.TCGETS)
	if asJSON || e != nil {
		snapshot, e := s.cockpitSnapshot(work)
		if e != nil {
			return e
		}
		return printJSON(out, snapshot)
	}
	if os.Getenv("TERM") == "dumb" {
		return s.plainConsole(work, in, out, false)
	}
	old := *term
	raw := old
	raw.Lflag &^= unix.ICANON | unix.ECHO | unix.ISIG | unix.IEXTEN
	raw.Iflag &^= unix.ICRNL | unix.IXON
	raw.Cc[unix.VMIN] = 1
	raw.Cc[unix.VTIME] = 0
	if e = unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, &raw); e != nil {
		return e
	}
	defer unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, &old)
	fmt.Fprint(out, "\x1b[?1049h")
	defer fmt.Fprint(out, "\x1b[?1049l")
	keys := make(chan byte, 64)
	go func() {
		defer close(keys)
		reader := bufio.NewReader(in)
		for {
			b, e := reader.ReadByte()
			if e != nil {
				return
			}
			keys <- b
		}
	}()
	signals := make(chan os.Signal, 4)
	signal.Notify(signals, syscall.SIGTERM, syscall.SIGHUP, syscall.SIGWINCH)
	defer signal.Stop(signals)
	ticker := time.NewTicker(time.Second)
	defer ticker.Stop()
	state := &consoleState{message: "help pour les commandes ; q ferme l'affichage, les agents continuent"}
	state.visit, _ = s.visit(work, operatorIdentity())
	defer func() {
		if w, e := s.get(work); e == nil {
			_ = s.markVisit(work, operatorIdentity(), w.Revision)
		}
	}()
	input := []byte{}
	draw := func() {
		width, height := 120, 40
		if size, e := unix.IoctlGetWinsize(int(in.Fd()), unix.TIOCGWINSZ); e == nil {
			width = int(size.Col)
			height = int(size.Row)
		}
		width, height = consoleDimensions(width, height)
		if d := state.dialog; d != nil && d.mode == "log-query" && d.logFollow && d.agent != nil {
			if d.logPage.More {
				d.logCursor = d.logPage.Next
			}
			if p, e := s.queryLogs(work, d.agent.ID, d.search, "", d.logCursor, max(1, d.logLimit)); e == nil {
				d.logPage = p
			}
		}
		fmt.Fprint(out, "\x1b[H\x1b[2J", styleTerminalFrame(s.renderDashboard(work, state, width, height, string(input))))
	}
	draw()
	escape := false
	csi := []byte{}
	paste := false
	fmt.Fprint(out, "\x1b[?2004h")
	defer fmt.Fprint(out, "\x1b[?2004l")
	escapeTimer := time.NewTimer(time.Hour)
	defer escapeTimer.Stop()
	if !escapeTimer.Stop() {
		<-escapeTimer.C
	}
	for {
		select {
		case sig := <-signals:
			if sig == syscall.SIGWINCH {
				draw()
				continue
			}
			return nil
		case <-escapeTimer.C:
			hadSequence := len(csi) > 0
			csi = nil
			escape = false
			if hadSequence {
				continue
			}
			if state.dialog != nil {
				s.dialogKey(work, state, "escape")
				draw()
			}
		case key, ok := <-keys:
			if !ok || ((key == 3 || key == 4) && !paste) {
				return nil
			}
			if key == 27 {
				escape = true
				csi = nil
				escapeTimer.Reset(60 * time.Millisecond)
				continue
			}
			if escape {
				csi = append(csi, key)
				complete, name := decodeSequence(csi, paste)
				if !complete {
					escapeTimer.Reset(60 * time.Millisecond)
					continue
				}
				escape = false
				csi = nil
				escapeTimer.Stop()
				if name == "paste-start" {
					paste = true
					continue
				}
				if name == "paste-end" {
					paste = false
					draw()
					continue
				}
				if paste {
					continue
				}
				if state.dialog != nil {
					s.dialogKey(work, state, name)
				} else if name == "help" {
					s.openTerminalHelp(work, state)
				} else if name == "up" {
					state.move(-1)
				} else if name == "down" {
					state.move(1)
				}
				draw()
				continue
			}
			if paste {
				if state.dialog != nil {
					d := state.dialog
					if (d.mode == "ooda" || d.mode == "assign" || d.mode == "decision" || d.mode == "budget") && d.row < len(d.fields) {
						dialogTypeText(d, &d.fields[d.row], key)
					} else if d.mode == "log-query" && d.row == 0 {
						dialogTypeText(d, &d.search, key)
					} else if d.mode == "start" || d.mode == "retry" {
						if d.row == 2 {
							dialogTypeText(d, &d.instruction, key)
						} else if d.row == 1 && d.mode == "start" {
							dialogTypeText(d, &d.workspace, key)
						}
					} else if d.mode == "override" && d.row == 0 {
						dialogTypeText(d, &d.reason, key)
					} else if (d.mode == "submit" || d.mode == "gate-load") && d.row == 0 {
						dialogTypeText(d, &d.reportPath, key)
					}
				} else {
					appendInput(&input, key)
				}
				continue
			}

			if state.dialog != nil {
				name := ""
				switch key {
				case '\t':
					name = "tab"
				case '\r', '\n':
					name = "enter"
				case 127, 8:
					name = "backspace"
				case 21:
					name = "clear"
				default:
					if key >= 32 {
						name = "text:" + string([]byte{key})
					}
				}
				s.dialogKey(work, state, name)
				draw()
				continue
			}
			if len(input) == 0 && key == 'q' {
				return nil
			}
			if len(input) == 0 && key == 'd' {
				s.openTaskDialog(work, state)
				if state.dialog != nil {
					state.dialog.mode = "detail"
				}
				draw()
				continue
			}
			if len(input) == 0 && key == 't' {
				toggleTerminalTheme()
				draw()
				continue
			}
			if key == '\t' {
				state.focus = (state.focus + 1) % 3
				draw()
				continue
			}
			if len(input) == 0 && key == '?' {
				s.openTerminalHelp(work, state)
				draw()
				continue
			}
			if len(input) == 0 && (key == '\r' || key == '\n') {
				s.openTaskDialog(work, state)
				draw()
				continue
			}
			switch key {
			case '\r', '\n':
				state.message = ""
				quit, e := s.consoleCommand(work, string(input), state)
				input = nil
				if e != nil {
					state.message = "Erreur : " + e.Error()
				}
				if quit {
					return nil
				}
			case 21:
				input = nil
			case 127, 8:
				if len(input) > 0 {
					_, size := utf8.DecodeLastRune(input)
					input = input[:len(input)-size]
				}
			default:
				if key >= 32 && len(input) < 16000 {
					input = append(input, key)
				}
			}
			draw()
		case <-ticker.C:
			if !state.frozen {
				draw()
			}
		}
	}
}

func dialogTypeText(d *taskDialog, target *string, key byte) {
	if len(*target) >= 8000 {
		return
	}
	switch key {
	case '\r', '\n', '\t':
		*target += " "
	case 127, 8:
		if len(*target) > 0 {
			_, n := utf8.DecodeLastRuneInString(*target)
			*target = (*target)[:len(*target)-n]
		}
	default:
		if key >= 32 {
			*target += string([]byte{key})
		}
	}
}

// appendInput appends a byte as text, flattening line and tab controls so a
// paste can not execute an intermediate command or switch panel focus.
func appendInput(input *[]byte, key byte) {
	if len(*input) >= 16000 {
		return
	}
	if key == '\r' || key == '\n' {
		*input = append(*input, ' ')
		return
	}
	if key == '\t' {
		*input = append(*input, ' ')
		return
	}
	if key >= 32 {
		*input = append(*input, key)
	}
}

func appendRune(input *[]byte, r rune) {
	if len(*input) >= 16000 {
		return
	}
	*input = utf8.AppendRune(*input, r)
}

// dialogName maps a single control byte to the dialog key vocabulary.
func dialogName(key byte) string {
	switch key {
	case '\t':
		return "tab"
	case '\r', '\n':
		return "enter"
	case 127, 8:
		return "backspace"
	case 21:
		return "clear"
	}
	if key >= 32 {
		return "text:" + string([]byte{key})
	}
	return ""
}

// decodeSequence classifies a completed or partial escape sequence.
// It returns terminal=true when the sequence is complete, and a key name:
// arrows, tab/enter/backspace/clear, paste markers, or "text:" payload.
// Unknown sequences (mouse reports, edit keys) return terminal=true with an
// empty name so they are ignored rather than typed into the command line.
func decodeSequence(seq []byte, paste bool) (bool, string) {
	if len(seq) < 2 {
		return false, ""
	}
	if seq[0] == 'O' {
		// SS3: complete on the single following byte.
		if len(seq) < 2 {
			return false, ""
		}
		switch seq[1] {
		case 'A':
			return true, "up"
		case 'B':
			return true, "down"
		case 'C':
			return true, "right"
		case 'D':
			return true, "left"
		case 'P':
			return true, "help"
		case 'H':
			return true, ""
		case 'F':
			return true, ""
		}
		return true, ""
	}
	if len(seq) >= 2 && seq[0] == '[' && seq[1] == 'M' {
		return len(seq) >= 5, ""
	}
	if seq[0] != '[' {
		return true, ""
	}
	final := seq[len(seq)-1]
	if final < 0x40 || final > 0x7E {
		if len(seq) > 32 {
			return true, "" // runaway sequence: drop it
		}
		return false, ""
	}
	params := string(seq[1 : len(seq)-1])
	switch final {
	case 'A':
		return true, "up"
	case 'B':
		return true, "down"
	case 'C':
		return true, "right"
	case 'D':
		return true, "left"
	case '~':
		// Edit keys and paste markers, identified by parameters.
		switch params {
		case "11":
			return true, "help"
		case "200":
			return true, "paste-start"
		case "201":
			return true, "paste-end"
		}
		return true, ""
	}
	// Mouse reports (<0;…M/m) and other CSI finals: ignore.
	return true, ""
}

func agentCLI(s *Store, pos []string, input, output string, asJSON bool, out io.Writer) error {
	plain := false
	filtered := []string{}
	for _, part := range pos {
		if part == "--plain" {
			plain = true
		} else {
			filtered = append(filtered, part)
		}
	}
	pos = filtered
	arg := func(n int) string {
		if len(pos) > n {
			return pos[n]
		}
		return ""
	}
	switch pos[0] {
	case "control":
		b, e := readInput(input)
		if e != nil {
			return e
		}
		var request struct {
			Command string `json:"command"`
			Capture bool   `json:"capture_output"`
		}
		if e = strict(b, &request); e != nil {
			return e
		}
		state := &consoleState{capture: request.Capture}
		_, e = s.consoleCommand(arg(1), request.Command, state)
		if e != nil {
			return e
		}
		return printJSON(out, map[string]any{"message": state.message, "selected_agent": state.selected})
	case "web":
		return s.serveWeb(arg(1), out)
	case "_assist":
		if len(pos) != 2 {
			return fmt.Errorf("identifiant de question requis")
		}
		turn, e := s.assistTurn(pos[1])
		if e != nil {
			return e
		}
		s.runAssistTurn(turn)
		return nil
	case "_supervise":
		return s.supervise(arg(1))
	case "console":
		if plain {
			return s.plainConsole(arg(1), os.Stdin, out, asJSON)
		}
		return s.console(arg(1), os.Stdin, out, asJSON)
	case "providers":
		if arg(1) == "init" {
			if e := s.initProviders(); e != nil {
				return e
			}
			fmt.Fprintln(out, "Configuration créée : .swarm/providers.json (aucun fournisseur lancé)")
			return nil
		}
		p, e := s.providers()
		if e != nil {
			return e
		}
		return printJSON(out, p)
	case "agent":
		switch arg(1) {
		case "list":
			a, e := s.cockpitSnapshot(arg(2))
			if e != nil {
				return e
			}
			return printJSON(out, a)
		case "history":
			path, e := localFile(s.root, ".swarm/imports/"+arg(2)+"/manifest.json")
			if e != nil {
				return e
			}
			b, e := os.ReadFile(path)
			if e != nil {
				return e
			}
			var bundle Bundle
			if e = json.Unmarshal(b, &bundle); e != nil {
				return e
			}
			return printJSON(out, map[string]any{"historical_only": true, "history": bundle.Cockpit})
		case "show":
			a, e := s.agent(arg(2))
			if e != nil {
				return e
			}
			d, _ := s.desired(a.ID)
			return printJSON(out, map[string]any{"agent": a, "observed_status": observedAgent(a), "desired": d})
		case "start":
			b, e := readInput(input)
			if e != nil {
				return e
			}
			var r Launch
			if e = strict(b, &r); e != nil {
				return e
			}
			a, created, e := s.prepare(arg(2), r)
			if e != nil {
				return e
			}
			if created {
				if e = s.spawnAgent(a); e != nil {
					return e
				}
			}
			return printJSON(out, map[string]any{"agent": a, "created": created})
		case "stop":
			if e := s.stopAgent(arg(2)); e != nil {
				return e
			}
			fmt.Fprintln(out, "Arrêt demandé ; vérifier agent show pour la confirmation.")
			return nil
		case "reconcile":
			return s.reconcile(arg(2))
		case "logs":
			after := int64(0)
			if arg(3) != "" {
				var e error
				after, e = strconv.ParseInt(arg(3), 10, 64)
				if e != nil {
					return e
				}
			}
			if _, e := s.agent(arg(2)); e != nil {
				return e
			}
			logs, e := s.logs(arg(2), after)
			if e != nil {
				return e
			}
			if output != "" {
				f, e := os.OpenFile(output, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
				if e != nil {
					return e
				}
				defer f.Close()
				for _, l := range logs {
					if e = json.NewEncoder(f).Encode(l); e != nil {
						return e
					}
				}
				return nil
			}
			return printJSON(out, logs)
		}
	}
	return fmt.Errorf("commande cockpit inconnue")
}
