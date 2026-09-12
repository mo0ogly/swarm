//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"strings"
	"unicode/utf8"
)

type taskDialog struct {
	scope                  string
	fields                 []string
	decisions              []Decision
	decision               Decision
	search                 string
	logLimit               int
	logCursor              int64
	logBack                []int64
	logPage                LogPage
	logFollow              bool
	roleIndex              int
	gateRaw                json.RawMessage
	gateRevision           int
	returnMode             string
	blockedTask            string
	reason                 string
	reportPath             string
	reports                []string
	reportIndex            int
	review                 string
	history                []AgentLog
	task                   Task
	agent                  *Agent
	mode                   string
	row                    int
	providers              []string
	provider               int
	workspace, instruction string
	message                string
}

func (s *Store) openTaskDialog(work string, c *consoleState) {
	tasks := s.dashboardTasks(work)
	if len(tasks) == 0 {
		c.message = "Aucune tâche à sélectionner"
		return
	}
	t := tasks[min(c.taskCursor, len(tasks)-1)]
	var selected *Agent
	if c.focus == 1 && c.selected != "" {
		if a, e := s.agent(c.selected); e == nil && a.WorkID == work {
			selected = &a
			for _, candidate := range tasks {
				if candidate.ID == a.TaskID {
					t = candidate
				}
			}
		}
	}
	if selected == nil {
		agents, _ := s.agents(work)
		for _, a := range agents {
			if a.TaskID == t.ID && (selected == nil || a.Started > selected.Started) {
				copy := a
				selected = &copy
			}
		}
	}
	d := &taskDialog{task: t, agent: selected, mode: "actions", workspace: s.root, instruction: t.Next}
	if w, err := s.get(work); err == nil {
		d.scope = w.Scope
	}
	if selected != nil {
		d.workspace = selected.CWD
	}
	providers, e := s.providers()
	if e != nil {
		d.message = "Configuration fournisseur : " + e.Error()
	}
	for name := range providers.Providers {
		d.providers = append(d.providers, name)
	}
	sort.Strings(d.providers)
	if selected != nil {
		for i, name := range d.providers {
			if name == selected.Provider {
				d.provider = i
			}
		}
	}
	if t.Status == "submitted" || t.Status == "accepted" || t.Status == "waived" {
		d.row = 8
	} else if selected != nil && selected.Status == "completed" {
		d.row = 7
	}
	c.dialog = d
}
func (d *taskDialog) actions() []string {
	return []string{"Lancer avec un fournisseur…", "Relancer cette tentative…", "Arrêter l'agent…", "Afficher les journaux", "Voir le détail complet", "Réconcilier l'état…", "Fermer", "[s] Soumettre le rapport…", "[v] Voir les gates et les preuves", "[a] Accepter la tâche…", "[l] Lire le rapport…", "[f] Forcer par dérogation…", "[o] Rouvrir cette tâche…", "[g] Charger une gate de validation…", "[h] Hiérarchie et dépendances", "[i] Décisions à traiter", "[j] Rechercher les journaux", "[u] Reprise depuis ma dernière visite", "[e] Consigner une boucle OODA", "[p] Changer le responsable", "[b] Budget et consommation"}
}
func (s *Store) dialogKey(work string, c *consoleState, key string) {
	d := c.dialog
	if d == nil {
		return
	}
	if key == "escape" {
		c.dialog = nil
		return
	}
	if s.panelKey(work, c, key) {
		return
	}
	switch d.mode {
	case "actions":
		if strings.HasPrefix(key, "text:") {
			if row, ok := map[string]int{"s": 7, "v": 8, "a": 9, "l": 10, "r": 1, "f": 11, "o": 12, "g": 13, "h": 14, "i": 15, "j": 16, "u": 17, "e": 18, "p": 19, "b": 20}[strings.ToLower(strings.TrimPrefix(key, "text:"))]; ok {
				d.row = row
				key = "enter"
			}
		}
		if key == "up" {
			d.row = (d.row + len(d.actions()) - 1) % len(d.actions())
		}
		if key == "down" || key == "tab" {
			d.row = (d.row + 1) % len(d.actions())
		}
		if key != "enter" {
			return
		}
		d.message = ""
		switch d.row {
		case 0:
			if d.agent != nil && activeAgent(*d.agent) {
				d.message = "Agent actif : arrêter puis attendre sa fin avant un nouveau départ."
				return
			}
			if len(d.providers) == 0 {
				d.message = "Aucun fournisseur configuré."
				return
			}
			d.mode = "start"
			d.row = 4
		case 1:
			if d.agent == nil {
				d.message = "Aucune tentative à relancer. Choisir Lancer."
				return
			}
			if activeAgent(*d.agent) {
				d.message = "Agent actif : arrêter puis attendre sa fin avant de relancer."
				return
			}
			d.mode = "retry"
			d.row = 2
		case 2, 5:
			if d.agent == nil {
				d.message = "Aucun agent associé à cette tâche."
				return
			}
			if d.row == 2 && !activeAgent(*d.agent) {
				d.message = "Cet agent est déjà terminé."
				return
			}
			if d.row == 2 {
				d.mode = "stop"
			} else {
				d.mode = "reconcile"
			}
			d.row = 0
		case 3:
			if d.agent == nil {
				d.message = "Aucun journal disponible sans tentative."
				return
			}
			c.selected = d.agent.ID
			c.focus = 2
			c.dialog = nil
		case 4:
			d.mode = "detail"
			d.row = 0
		case 6:
			c.dialog = nil
		case 7:
			if d.agent != nil && activeAgent(*d.agent) {
				d.message = "Attendre la fin de l’agent avant de soumettre."
				return
			}
			d.reports = s.taskReports(d.task.ID)
			d.reportIndex = 0
			if len(d.reports) > 0 {
				d.reportPath = d.reports[0]
			}
			d.mode = "submit"
			d.row = 0
		case 8:
			d.mode = "review"
			d.row = 0
			d.review = s.reviewText(work, d)
		case 10:
			d.reports = s.taskReports(d.task.ID)
			d.reportIndex = 0
			d.row = 0
			d.mode = "report"
			if len(d.reports) == 0 {
				d.review = "Aucun rapport détecté pour cette tâche."
			} else {
				d.reportPath = d.reports[0]
				s.loadReport(d)
			}
		case 14, 15, 16, 17, 18, 19, 20:
			s.openPanel(work, c, []string{"hierarchy", "decisions", "log-query", "resume", "ooda", "assign", "budget"}[d.row-14])
		case 13:
			d.mode = "gate-load"
			d.row = 0
			d.reports = s.gateFiles(d.task.ID)
			d.reportIndex = 0
			d.reportPath = ""
			if len(d.reports) > 0 {
				d.reportPath = d.reports[0]
			}
		case 12:
			d.mode = "reopen"
			d.row = 0
		case 11:
			d.mode = "override"
			d.row = 0
			d.reason = ""
		case 9:
			d.mode = "accept"
			d.row = 0
			d.review = s.reviewText(work, d)

		}
	case "detail", "review", "report":
		if d.mode == "report" && len(d.reports) > 0 && (key == "left" || key == "right") {
			delta := 1
			if key == "left" {
				delta = -1
			}
			d.reportIndex = (d.reportIndex + delta + len(d.reports)) % len(d.reports)
			d.reportPath = d.reports[d.reportIndex]
			d.row = 0
			s.loadReport(d)
			return
		}
		if key == "up" {
			d.row = max(0, d.row-1)
		}
		if key == "down" {
			d.row++
		}
		if key == "enter" {
			d.mode = "actions"
			d.row = 0
		}
	case "submit", "gate-load":
		if key == "tab" || key == "up" || key == "down" {
			d.row = 1 - d.row
			return
		}
		if d.row == 0 && (key == "left" || key == "right") && len(d.reports) > 0 {
			delta := 1
			if key == "left" {
				delta = -1
			}
			d.reportIndex = (d.reportIndex + delta + len(d.reports)) % len(d.reports)
			d.reportPath = d.reports[d.reportIndex]
			return
		}
		if key == "enter" {
			if d.row == 0 {
				d.row = 1
				return
			}
			if d.mode == "gate-load" {
				if e := s.previewGate(work, d); e != nil {
					d.message = e.Error()
					return
				}
				d.mode = "gate-confirm"
				d.row = 0
				d.message = ""
				return
			}
			// Reuse the command service: the shell-style command grammar is not used for paths.
			if e := s.submitReport(work, d.task.ID, d.reportPath); e != nil {
				d.message = e.Error()
				return
			}
			c.message = "Rapport soumis : examiner les gates et preuves avant acceptation."
			c.dialog = nil
			return
		}
		if d.row == 0 {
			if key == "clear" {
				d.reportPath = ""
			} else if key == "backspace" && len(d.reportPath) > 0 {
				_, n := utf8.DecodeLastRuneInString(d.reportPath)
				d.reportPath = d.reportPath[:len(d.reportPath)-n]
			} else if strings.HasPrefix(key, "text:") && len(d.reportPath) < 8000 {
				d.reportPath += strings.TrimPrefix(key, "text:")
			}
		}
	case "gate-confirm":
		if key == "up" || key == "down" || key == "tab" {
			d.row = 1 - d.row
			return
		}
		if key == "enter" {
			if d.row == 1 {
				d.mode = "gate-load"
				d.row = 0
				d.message = "Enregistrement annulé."
				return
			}
			if e := s.recordDialogGate(work, d); e != nil {
				d.message = e.Error()
				return
			}
			d.mode = "review"
			d.row = 0
			d.message = "Gate enregistrée ; examiner puis [a] accepter depuis les actions."
			d.review = s.reviewText(work, d)
		}
	case "override":
		if key == "tab" || key == "down" {
			d.row = (d.row + 1) % 3
			return
		}
		if key == "up" {
			d.row = (d.row + 2) % 3
			return
		}
		if key == "enter" {
			if d.row == 0 {
				d.row = 1
				return
			}
			if d.row == 2 {
				c.message = "Dérogation annulée : aucun changement."
				c.dialog = nil
				return
			}
			if e := s.overrideReviewedTask(work, d.task.ID, d.reason); e != nil {
				d.message = e.Error()
				return
			}
			c.message = "Tâche acceptée par dérogation ; motif enregistré, gates inchangées."
			c.dialog = nil
			return
		}
		if d.row == 0 {
			if key == "clear" {
				d.reason = ""
			} else if key == "backspace" && len(d.reason) > 0 {
				_, n := utf8.DecodeLastRuneInString(d.reason)
				d.reason = d.reason[:len(d.reason)-n]
			} else if strings.HasPrefix(key, "text:") && len(d.reason) < 2000 {
				d.reason += strings.TrimPrefix(key, "text:")
			}
		}
	case "accept":
		if key == "text:f" {
			d.mode = "override"
			d.row = 0
			d.message = ""
			return
		}
		if key == "tab" || key == "up" || key == "down" {
			d.row = 1 - d.row
			return
		}
		if key == "enter" {
			if d.row == 1 {
				c.message = "Action annulée : aucun changement."
				c.dialog = nil
				return
			}
			if e := s.acceptReviewedTask(work, d.task.ID); e != nil {
				d.message = e.Error()
				return
			}
			c.message = "Tâche acceptée après revue ; gate et dépendances vérifiées."
			c.dialog = nil
		}
	case "launch-error":
		if key == "enter" {
			d.mode = d.returnMode
			d.row = 4
			d.message = ""
			return
		}
		if key == "text:o" {
			d.mode = "reopen"
			d.row = 0
			d.message = ""
			return
		}
		if key == "text:v" {
			if d.blockedTask != "" {
				w, e := s.get(work)
				if e == nil {
					t, e := w.task(d.blockedTask)
					if e == nil {
						d.task = *t
					}
				}
			}
			d.mode = "review"
			d.row = 0
			d.message = ""
			d.review = s.reviewText(work, d)
			return
		}
	case "reopen":
		if key == "tab" || key == "up" || key == "down" {
			d.row = 1 - d.row
			return
		}
		if key == "enter" {
			if d.row == 1 {
				c.dialog = nil
				c.message = "Réouverture annulée."
				return
			}
			if e := s.operatorTask(work, Request{ID: d.task.ID, Status: "todo", Next: "Tâche rouverte par l’opérateur ; sélectionner Lancer."}); e != nil {
				d.message = e.Error()
				return
			}
			c.dialog = nil
			c.message = "Tâche rouverte : Entrée puis Lancer pour une nouvelle tentative."
		}
	case "stop", "reconcile":
		if key == "tab" || key == "up" || key == "down" {
			d.row = 1 - d.row
		}
		if key == "enter" {
			if d.row == 1 {
				c.message = "Action annulée : aucun changement."
				c.dialog = nil
				return
			}
			_, e := s.consoleCommand(work, d.mode+" "+d.agent.ID, c)
			if e != nil {
				d.message = e.Error()
				return
			}
			c.message = "Demande enregistrée : " + d.mode + ". Vérifier l'état observé."
			c.dialog = nil
		}
	case "start", "retry":
		if key == "tab" || key == "down" {
			d.row = (d.row + 1) % 5
			return
		}
		if key == "up" {
			d.row = (d.row + 4) % 5
			return
		}
		if d.row == 0 && d.mode == "start" && (key == "left" || key == "right") {
			delta := 1
			if key == "left" {
				delta = -1
			}
			d.provider = (d.provider + delta + len(d.providers)) % len(d.providers)
			return
		}
		if d.row == 3 && d.mode == "start" && (key == "left" || key == "right") {
			delta := 1
			if key == "left" {
				delta = -1
			}
			d.roleIndex = (d.roleIndex + delta + 3) % 3
			return
		}
		if key == "enter" {
			if d.row != 4 {
				d.row = (d.row + 1) % 5
				return
			}
			if e := s.launchDialog(work, c); e != nil {
				d.returnMode = d.mode
				d.mode = "launch-error"
				d.row = 0
				d.message = e.Error()
				_, _ = s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, now(), "launch.refused", "Tâche "+d.task.ID+" : "+terminalText(e.Error()))
				return
			}
			c.dialog = nil
			return
		}
		if d.row == 2 || (d.row == 1 && d.mode == "start") {
			target := &d.instruction
			if d.row == 1 {
				target = &d.workspace
			}
			if key == "clear" {
				*target = ""
			} else if key == "backspace" {
				if len(*target) > 0 {
					_, n := utf8.DecodeLastRuneInString(*target)
					*target = (*target)[:len(*target)-n]
				}
			} else if strings.HasPrefix(key, "text:") && len(*target) < 8000 {
				*target += strings.TrimPrefix(key, "text:")
			}
		}
	}
}
func (s *Store) launchDialog(work string, c *consoleState) error {
	d := c.dialog
	if consoleBinaryReplaced() {
		return fmt.Errorf("Console ancienne : quitter avec q et relancer swarm console pour charger la mise à jour.")
	}
	w, e := s.get(work)
	if e != nil {
		return e
	}
	t, e := w.task(d.task.ID)
	if e != nil {
		return e
	}
	d.blockedTask = ""
	if t.Status == "accepted" || t.Status == "waived" || t.Status == "abandoned" {
		return fmt.Errorf("Cette tâche est déjà %s. [o] Rouvrir explicitement avant un nouveau lancement.", uiStatus(t.Status))
	}
	for _, id := range t.Depends {
		dep, _ := w.task(id)
		if !s.acceptedFresh(&w, dep, map[string]bool{}) {
			d.blockedTask = id
			return fmt.Errorf("Dépendance %s non validée ou preuves périmées. [v] Examiner ses gates ; accepter ou déroger avant de lancer %s.", id, t.ID)
		}
	}
	if d.mode == "retry" {
		_, e := s.consoleCommand(work, "retry "+d.agent.ID+" "+d.instruction, c)
		return e
	}
	r := Launch{Schema: 1, EventID: newID("agent-"), Revision: w.Revision, TaskID: d.task.ID, Role: []string{"worker", "planner", "subplanner"}[d.roleIndex], Provider: d.providers[d.provider], Workspace: d.workspace, Instruction: d.instruction, Capture: c.capture}
	a, created, e := s.prepare(work, r)
	if e != nil {
		return e
	}
	c.selected = a.ID
	if created {
		if e = s.spawnAgent(a); e != nil {
			return e
		}
	}
	c.message = "Agent lancé pour " + d.task.ID + " avec " + r.Provider + " ; sélectionner Journaux pour le suivi."
	return nil
}
func wrapDialog(text string, width int) []string {
	out := []string{}
	for _, line := range strings.Split(text, "\n") {
		clean := terminalText(line)
		if textWidth(clean) == 0 {
			out = append(out, "")
			continue
		}
		for textWidth(clean) > 0 {
			n := truncateCells(clean, max(1, width))
			if n == "" {
				out = append(out, "…")
				clean = clean[len(firstCluster(clean)):]
				continue
			}
			out = append(out, n)
			clean = clean[len(n):]
		}
	}
	return out
}
func renderTaskDialog(base string, d *taskDialog, width, height int) string {
	lines := strings.Split(base, "\r\n")
	boxWidth := min(width-4, 110)
	inner := max(1, boxWidth-4)
	title := "Actions — " + d.task.ID
	rows := []string{d.task.Title, "État tâche : " + uiStatus(d.task.Status)}
	if d.agent == nil {
		rows = append(rows, "Agent : aucune tentative")
	} else {
		rows = append(rows, "Agent : "+d.agent.Provider+" · "+uiStatus(observedAgent(*d.agent)))
	}
	if panelTitle, panelContent, ok := panelRows(d, inner, height); ok {
		title = panelTitle
		rows = append(rows, panelContent...)
	} else {
		switch d.mode {
		case "actions":
			if d.agent != nil && !activeAgent(*d.agent) {
				cause := readableWrap("MOTIF : "+d.agent.Activity, inner)
				if len(cause) > 2 {
					cause = cause[:2]
				}
				rows = append(rows, cause...)
				if d.agent.Progress.ToolCalls > 0 {
					rows = append(rows, fmt.Sprintf("Budget outils : %d / %d · résultats reçus : %d", d.agent.Progress.ToolCalls, d.agent.Limits.MaxToolCalls, d.agent.Progress.ToolResults))
				}
			}
			visible := max(1, height-len(rows)-6)
			first := max(0, d.row-visible+1)
			for i := first; i < min(len(d.actions()), first+visible); i++ {
				a := d.actions()[i]
				m := "  "
				if i == d.row {
					m = "> "
				}
				rows = append(rows, m+a)
			}
		case "detail":
			title = "Détails — " + d.task.ID
			prefix := ""
			if d.agent != nil && !activeAgent(*d.agent) {
				prefix = "MOTIF DE FIN : " + d.agent.Activity + "\n"
			}
			detail := prefix + "Tâche : " + d.task.Title + "\nLivrable : " + d.task.Deliverable + "\nPérimètre : " + d.scope + "\nCritères : " + strings.Join(d.task.Criteria, " ; ") + "\nBlocage : " + d.task.Blocker + "\nProchaine action : " + d.task.Next + "\nEspace : " + d.workspace
			if d.agent != nil {
				detail += "\nAction observée : " + d.agent.Progress.Action + "\nCommande / fichier : " + d.agent.Progress.Detail
				detail += fmt.Sprintf("\nOutils : %d appels / %d résultats\nDernier résultat reçu il y a : %s\nSurveillance : %s", d.agent.Progress.ToolCalls, d.agent.Progress.ToolResults, activityAge(d.agent.Progress.LastResult), monitoringLabel(d.agent.Progress.Degraded))
				detail += "\nAgent : " + d.agent.ID + "\nActivité : " + d.agent.Activity + "\nSignal : " + d.agent.Heartbeat
			}
			if len(d.history) > 0 {
				detail += "\n\nHISTORIQUE — du plus récent au plus ancien"
				for i := len(d.history) - 1; i >= 0; i-- {
					l := d.history[i]
					if l.Kind != "output" {
						detail += "\n" + eventClock(l.At) + "  " + l.Message
					}
				}
			}
			parts := wrapDialog(detail, inner)
			n := max(1, height-11)
			d.row = min(d.row, max(0, len(parts)-n))
			rows = append(rows, parts[d.row:min(len(parts), d.row+n)]...)
		case "review", "report":
			title = "Gates et preuves — " + d.task.ID
			if d.mode == "report" {
				title = "Rapport — " + d.task.ID
			}
			parts := wrapDialog(d.review, inner)
			n := max(1, height-11)
			d.row = min(d.row, max(0, len(parts)-n))
			rows = append(rows, parts[d.row:min(len(parts), d.row+n)]...)
		case "submit", "gate-load":
			title = "Soumettre le rapport — " + d.task.ID
			label, button := "Rapport : ", "[ Soumettre pour revue ]"
			if d.mode == "gate-load" {
				title = "Charger une gate — " + d.task.ID
				label = "Gate : "
				button = "[ Examiner cette évaluation ]"
			}
			for i, line := range []string{label + d.reportPath, button} {
				mark := "  "
				if d.row == i {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
			if d.mode == "gate-load" {
				rows = append(rows, "←→ choisir une gate détectée · Tab champ · Ctrl-U effacer", "Document méthode 2 ; l’enregistrement n’accepte pas la tâche.")
			} else {
				rows = append(rows, "←→ choisir un rapport détecté · Tab champ · Ctrl-U effacer", "La soumission ne valide pas le travail ; aucun agent n’est lancé.")
			}
		case "gate-confirm":
			title = "Enregistrer la gate — " + d.task.ID
			parts := wrapDialog(d.review, inner)
			parts = parts[:min(len(parts), max(1, height-14))]
			rows = append(rows, parts...)
			for i, line := range []string{"Confirmer l’enregistrement", "Annuler"} {
				mark := "  "
				if i == d.row {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
		case "override":
			title = "Forcer par dérogation — " + d.task.ID
			rows = append(rows, "Acceptation manuelle sans déclarer les contrôles réussis.", "Motif conservé avec date, opérateur local et état précédent.", "Les dépendances doivent être acceptées ; agent arrêté.")
			for i, line := range []string{"Motif : " + d.reason, "[ Confirmer la dérogation ]", "Annuler"} {
				mark := "  "
				if d.row == i {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
			rows = append(rows, "Saisir le motif · Entrée puis Entrée confirmer · Échap annuler")
		case "accept":
			title = "Accepter la tâche — " + d.task.ID
			rows = append(rows, "Confirmer que le rapport et ses preuves ont été examinés.", "La gate et les dépendances sont revérifiées à la confirmation.", "[f] Forcer par dérogation avec un motif")
			if d.task.Status != "submitted" {
				rows = append(rows, "INDISPONIBLE : soumettre le rapport pour revue d’abord.")
			} else if d.task.Gate == nil {
				rows = append(rows, "INDISPONIBLE : aucune gate enregistrée.")
			} else if !d.task.Gate.Evaluation.Allowed {
				rows = append(rows, "INDISPONIBLE : gate échouée ; voir les contrôles et preuves.")
			}
			for i, line := range []string{"Confirmer l’acceptation", "Annuler"} {
				mark := "  "
				if d.row == i {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
		case "launch-error":
			title = "LANCEMENT REFUSÉ — " + d.task.ID
			rows = append(rows, wrapDialog(d.message, inner)...)
			rows = append(rows, "", "[ Entrée : revenir au formulaire ]", "[v] Voir les gates de la tâche ou dépendance bloquante", "[o] Rouvrir la tâche sélectionnée", "Échap : retourner au tableau")
		case "reopen":
			title = "Rouvrir — " + d.task.ID
			rows = append(rows, "La validation courante sera retirée ; l’historique reste conservé.")
			for i, line := range []string{"Confirmer la réouverture", "Annuler"} {
				mark := "  "
				if i == d.row {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
		case "start", "retry":
			rows = append(rows, "Périmètre : "+d.scope)
			title = "Lancer — " + d.task.ID
			if d.mode == "retry" {
				title = "Relancer — " + d.task.ID
			}
			provider := ""
			if len(d.providers) > 0 {
				provider = d.providers[d.provider]
			}
			if d.mode == "retry" {
				provider = d.agent.Provider
			}
			role := []string{"worker", "planner", "subplanner"}[d.roleIndex]
			if d.mode == "retry" {
				role = d.agent.Role
			}
			form := []string{"Fournisseur : " + provider, "Espace : " + d.workspace, "Consigne : " + d.instruction, "Rôle : " + role, "[ Confirmer le lancement ]"}
			for i, line := range form {
				m := "  "
				if i == d.row {
					m = "> "
				}
				if i == d.row && len([]rune(line)) > inner-2 && (i == 1 || i == 2) {
					prefix := "Espace : "
					if i == 2 {
						prefix = "Consigne : "
					}
					r := []rune(line)
					n := max(1, inner-len([]rune(prefix))-4)
					line = prefix + "…" + string(r[max(0, len(r)-n):])
				}
				rows = append(rows, m+line)
			}
			rows = append(rows, "Tab : champ · ←→ : fournisseur/rôle · Ctrl-U : effacer", "Entrée sur Confirmer lance réellement le fournisseur.")
		case "stop", "reconcile":
			title = "Confirmer — " + d.task.ID
			text := "Demander l'arrêt de l'agent sélectionné ?"
			if d.mode == "reconcile" {
				text = "Vérifier les processus et réconcilier l'état ?"
			}
			rows = append(rows, text)
			for i, a := range []string{"Confirmer", "Annuler"} {
				m := "  "
				if i == d.row {
					m = "> "
				}
				rows = append(rows, m+a)
			}
		}
	}
	hint := "↑↓ choix · Entrée valider · Échap fermer"
	if d.mode == "detail" || d.mode == "review" || d.mode == "report" {
		hint = "↑↓ défiler · ←→ rapports · Entrée actions · Échap retour"
	}
	if d.mode != "launch-error" {
		rows = append(rows, wrapDialog(d.message, inner)...)
	}
	rows = append(rows, hint)
	box := []string{"┌" + pad(" "+title+" ", boxWidth-2) + "┐"}
	for _, r := range rows {
		box = append(box, "│ "+pad(r, inner)+" │")
	}
	box = append(box, "└"+strings.Repeat("─", boxWidth-2)+"┘")
	if len(box) > height-2 {
		box = box[:height-2]
	}
	top := max(0, (height-len(box))/2)
	left := max(0, (width-boxWidth)/2)
	for i, row := range box {
		if top+i >= len(lines) {
			break
		}
		under := []rune(pad(lines[top+i], width-1))
		end := min(len(under), left+boxWidth)
		lines[top+i] = strings.Repeat(" ", left) + row + strings.Repeat(" ", len(under)-end)
	}
	return strings.Join(lines, "\r\n")
}

func consoleBinaryReplaced() bool {
	path, e := os.Readlink("/proc/self/exe")
	return e == nil && strings.HasSuffix(path, " (deleted)")
}

func monitoringLabel(degraded string) string {
	if degraded == "" {
		return "aucune perte de flux détectée"
	}
	return degraded
}
