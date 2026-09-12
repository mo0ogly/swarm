//go:build linux

package main

import (
	"fmt"
	"sort"
	"strings"
	"time"
)

func (c *consoleState) move(delta int) {
	switch c.focus {
	case 0:
		c.taskSelID = ""
		c.taskCursor = max(0, c.taskCursor+delta)
	case 1:
		c.agentSelID = ""
		c.agentCursor = max(0, c.agentCursor+delta)
	case 2:
		c.logOffset = max(0, c.logOffset-delta)
	}
}
func (s *Store) dashboardTasks(work string) []Task {
	w, e := s.get(work)
	if e != nil {
		return nil
	}
	priorities := s.priorities(work)
	sort.SliceStable(w.Tasks, func(i, j int) bool { return priorities[w.Tasks[i].ID] > priorities[w.Tasks[j].ID] })
	return w.Tasks
}
func (s *Store) dashboardAgents(work string, c *consoleState) []Agent {
	agents, _ := s.agents(work)
	out := []Agent{}
	for _, a := range agents {
		if c.status != "" && !strings.Contains(observedAgent(a), c.status) {
			continue
		}
		if c.filter != "" && !strings.Contains(strings.ToLower(a.ID+" "+a.TaskID+" "+a.Provider+" "+a.Activity), strings.ToLower(c.filter)) {
			continue
		}
		out = append(out, a)
	}
	return out
}

func panel(title string, rows []string, width, height int, active bool) []string {
	marker := " "
	if active {
		marker = ">"
	}
	heading := marker + "── " + title + " "
	out := []string{clip(heading+strings.Repeat("─", max(0, width-textWidth(heading))), width)}
	for len(out) < height {
		i := len(out) - 1
		row := ""
		if i < len(rows) {
			row = rows[i]
		}
		out = append(out, clip(row, width))
	}
	return out
}
func pad(s string, n int) string {
	s = clip(s, n)
	return s + strings.Repeat(" ", max(0, n-textWidth(s)))
}
func (s *Store) renderDashboard(work string, c *consoleState, width, height int, input string) string {
	width, height = consoleDimensions(width, height)
	w, e := s.get(work)
	if e != nil {
		return clip(e.Error(), width)
	}
	if width < 64 || height < 20 {
		return strings.Join([]string{clip("SWARM — terminal trop petit (minimum 64 × 20)", width), clip("Agrandir ou relancer avec --plain. q puis Entrée quitte.", width), clip("swarm> "+input, width)}, "\r\n")
	}
	lines := []string{fmt.Sprintf("SWARM  │ %s", w.Title), "", "Sélectionnez une tâche puis Entrée pour agir."}
	tasks := s.dashboardTasks(work)
	agents := s.dashboardAgents(work, c)
	// Selection is anchored on identifiers, not indexes: a new agent row or a
	// priority reorder must not move what the operator selected.
	if c.taskSelID != "" {
		for i, t := range tasks {
			if t.ID == c.taskSelID {
				c.taskCursor = i
				break
			}
		}
	}
	if c.agentSelID != "" {
		for i, a := range agents {
			if a.ID == c.agentSelID {
				c.agentCursor = i
				break
			}
		}
	}
	c.taskCursor = min(c.taskCursor, max(0, len(tasks)-1))
	c.agentCursor = min(c.agentCursor, max(0, len(agents)-1))
	if len(tasks) > 0 {
		c.taskSelID = tasks[c.taskCursor].ID
	} else {
		c.taskSelID = ""
	}
	if c.focus == 0 && len(tasks) > 0 {
		c.selected = ""
		var latest string
		for _, a := range agents {
			if a.TaskID == tasks[c.taskCursor].ID && a.Started > latest {
				latest = a.Started
				c.selected = a.ID
			}
		}
	}
	if c.focus == 1 && len(agents) > 0 {
		c.selected = agents[c.agentCursor].ID
		c.agentSelID = c.selected
	} else if c.focus == 1 {
		c.agentSelID = ""
	}
	topHeight := max(4, (height-14)/2)
	tr := []string{}
	ar := []string{}
	start := max(0, c.taskCursor-topHeight+2)
	for i := start; i < len(tasks) && len(tr) < topHeight-1; i++ {
		t := tasks[i]
		marker := " "
		if i == c.taskCursor {
			marker = ">"
		}
		tr = append(tr, fmt.Sprintf("%s %-10s %s · %s", marker, uiStatus(t.Status), t.ID, t.Title))
	}
	start = max(0, c.agentCursor-topHeight+2)
	for i := start; i < len(agents) && len(ar) < topHeight-1; i++ {
		a := agents[i]
		marker := " "
		if i == c.agentCursor {
			marker = ">"
		}
		ar = append(ar, fmt.Sprintf("%s %s %s · %s", marker, a.Provider, uiStatus(observedAgent(a)), a.TaskID))
	}
	if len(tr) == 0 {
		tr = append(tr, "Aucune tâche. help pour créer une tâche.")
	}
	if len(ar) == 0 {
		ar = append(ar, "Aucun agent correspondant.")
	}
	if width < 110 {
		if c.focus == 1 {
			lines = append(lines, panel(fmt.Sprintf("AGENTS · %d", len(agents)), ar, width-1, topHeight, true)...)
		} else {
			lines = append(lines, panel(fmt.Sprintf("TÂCHES · %d", len(tasks)), tr, width-1, topHeight, c.focus == 0)...)
		}
	} else {
		left := (width - 3) / 2
		right := width - left - 3
		tp := panel(fmt.Sprintf("TÂCHES · %d", len(tasks)), tr, left, topHeight, c.focus == 0)
		ap := panel(fmt.Sprintf("AGENTS · %d", len(agents)), ar, right, topHeight, c.focus == 1)
		for i := range tp {
			lines = append(lines, pad(tp[i], left)+" │ "+ap[i])
		}
	}
	detail := "Sélectionner une tâche"
	criteria := ""
	if len(tasks) > 0 {
		t := tasks[c.taskCursor]
		gate := "en attente"
		if t.Gate != nil {
			gate = gateLabel(&t) + " — bloquée/périmée"
			if s.validGate(&t) {
				gate = gateLabel(&t) + " — delivery valide"
				if t.Gate.Evaluation.Quality != nil {
					gate += fmt.Sprintf(" · score %.1f/100", *t.Gate.Evaluation.Quality)
				}
			}
		}
		if t.Status == "waived" {
			gate = "DÉROGATION MANUELLE · gate : " + gate
		}
		detail = "Responsable : " + t.Owner + " · Validation : " + gate
		criteria = "LIVRABLE : " + t.Deliverable + " | CRITÈRES : " + strings.Join(t.Criteria, " ; ")
	}
	lines = append(lines, detail, criteria)
	logRows := []string{}
	if c.selected != "" {
		a, err := s.agent(c.selected)
		if err == nil {
			logRows = append(logRows, "Agent : "+a.Provider+" · "+uiStatus(observedAgent(a)))
			if activeAgent(a) {
				logRows = append(logRows, "Suivi reçu il y a "+activityAge(a.Heartbeat)+" · durée "+activityAge(a.Started))
				if a.Progress.PendingTools > 0 {
					action := a.Progress.Action
					if action == "" {
						action = a.Progress.LastTool + " — explication non fournie par cette tentative"
					}
					actionRows := readableWrap("EN COURS : "+action, width-2)
					if len(actionRows) > 2 {
						actionRows = actionRows[:2]
						actionRows[1] = clip(actionRows[1], width-5) + "…"
					}
					logRows = append(logRows, actionRows...)
					if a.Progress.Detail != "" {
						logRows = append(logRows, clip("Cible : "+a.Progress.Detail, width-2))
					}
				} else {
					logRows = append(logRows, "EN ATTENTE : prochaine sortie du fournisseur")
					if a.Progress.Action != "" {
						logRows = append(logRows, clip("Dernière action : "+a.Progress.Action, width-2))
					}
				}
			}
			if activeAgent(a) {
				if a.Progress.ToolCalls > 0 {
					logRows = append(logRows, fmt.Sprintf("Outils : %d appels / %d résultats · dernier : %s", a.Progress.ToolCalls, a.Progress.ToolResults, a.Progress.LastTool))
				} else {
					logRows = append(logRows, "Activité : "+a.Activity)
				}
				if a.Progress.LastResult != "" {
					logRows = append(logRows, "Dernier résultat reçu il y a "+activityAge(a.Progress.LastResult))
				}
			} else {
				logRows = append(logRows, readableWrap("Fin : "+a.Activity, width-1)...)
				if t, e := w.task(a.TaskID); e == nil {
					next := readableWrap("À faire : "+t.Next, width-1)
					if len(next) > 2 {
						next = next[:2]
					}
					logRows = append(logRows, next...)
				}
			}
			if a.Progress.Degraded != "" {
				logRows = append(logRows, "Surveillance incomplète : chronométrage par outil suspendu.")
			}

			logs, _ := s.logs(a.ID, 0)
			available := max(0, height-len(lines)-6-len(logRows))
			c.logOffset = min(c.logOffset, max(0, len(logs)-1))
			end := len(logs) - c.logOffset
			begin := max(0, end-available)
			for _, l := range logs[begin:end] {
				logRows = append(logRows, fmt.Sprintf("%s  %s", eventClock(l.At), l.Message))
			}
		}
	}
	if len(logRows) == 0 {
		logRows = append(logRows, "Tab vers AGENTS, puis flèches pour choisir un agent et lire son journal.")
	}
	if c.frozen {
		lines[2] = "AFFICHAGE FIGÉ — commande freeze pour reprendre le suivi."
	}
	if c.message == consoleHelp {
		logRows = strings.Split(consoleHelp, "\n")
	}

	logHeight := height - len(lines) - 4
	lines = append(lines, panel("SUIVI EN DIRECT / ACTIVITÉ", logRows, width, logHeight, c.focus == 2)...)
	lines = append(lines, fmt.Sprintf("%d agents · %s · %s", len(agents), map[bool]string{true: "Départs suspendus", false: "Départs autorisés"}[s.paused(work)], map[bool]string{true: "Capture activée", false: "Capture désactivée"}[c.capture]), "↑↓ tâche · Entrée actions · d détail · ? aide · t thème · q", strings.ReplaceAll(c.message, "\n", " | "), "swarm> "+input)
	for i := range lines {
		lines[i] = clip(lines[i], width-1)
	}
	if consoleBinaryReplaced() {
		lines[len(lines)-2] = clip("Mise à jour installée : q puis relancer swarm console.", width-1)
	}
	frame := strings.Join(lines, "\r\n")
	if c.dialog != nil {
		if task, err := w.task(c.dialog.task.ID); err == nil {
			c.dialog.task = *task
		}
		if c.dialog.mode == "review" {
			c.dialog.review = s.reviewText(work, c.dialog)
		}
		if c.dialog.agent != nil {
			if latest, err := s.agent(c.dialog.agent.ID); err == nil {
				c.dialog.agent = &latest
			}
			c.dialog.history, _ = s.logs(c.dialog.agent.ID, 0)
		}
		s.refreshDialogModel(c.dialog)
		return renderTaskDialog(frame, c.dialog, width, height)
	}
	return frame
}

// Relative time remains useful across terminal locales and time zones.
func activityAge(stamp string) string {
	t, err := time.Parse(time.RFC3339Nano, stamp)
	if err != nil {
		return "inconnu"
	}
	d := time.Since(t).Truncate(time.Second)
	if d < 0 {
		d = 0
	}
	return d.String()
}
func eventClock(stamp string) string {
	t, err := time.Parse(time.RFC3339Nano, stamp)
	if err != nil {
		return "--:--:--"
	}
	return t.Local().Format("15:04:05")
}
