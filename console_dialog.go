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
	modelLevel             int
	modelPolicyHash        string
	modelDescription       string
	gateRaw                json.RawMessage
	gateRevision           int
	gateName               string
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
	preconditionEvidence   string
	message                string
	actionsCache           []TaskAction
}

// actionItem est une entrée du menu d'actions de la console : libellé,
// disponibilité selon l'oracle et motif d'indisponibilité.
type actionItem struct {
	label   string
	enabled bool
	raison  string
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
		agents, _ := s.agents(work)
		d.actionsCache = s.taskActions(&w, &t, agents)
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
	d.row = advisedActionRow(d.actionsCache, selected != nil && selected.Status == "completed")
	c.dialog = d
}
func (s *Store) refreshDialogModel(d *taskDialog) {
	if d.mode != "start" && d.mode != "retry" {
		return
	}
	d.modelPolicyHash = ""
	d.modelDescription = "Fournisseur indisponible"
	provider := ""
	if len(d.providers) > 0 {
		provider = d.providers[d.provider]
	}
	if d.mode == "retry" && d.agent != nil {
		provider = d.agent.Provider
	}
	ps, err := s.providers()
	if err != nil {
		d.modelDescription = err.Error()
		return
	}
	p, ok := ps.Providers[provider]
	if !ok {
		return
	}
	_, r, err := resolveModel(p, []string{"auto", "simple", "standard", "exigeant"}[d.modelLevel], "work")
	if err != nil {
		d.modelDescription = err.Error()
		return
	}
	d.modelDescription = "géré par l’exécutable"
	if r != nil {
		d.modelDescription = r.Model
		if r.Effort != "" {
			d.modelDescription += " / " + r.Effort
		}
		d.modelPolicyHash = r.PolicyHash
	}
}

// actionsOracle fait correspondre les indices du menu aux kinds de l'oracle.
// Les entrées sans kind (journaux, détail, fermeture, panneaux) restent
// toujours disponibles.
var actionsOracle = []string{
	"start", "retry", "stop", "", "", "reconcile", "",
	"submit", "", "accepted", "report", "override", "reopen", "gate",
	"", "", "", "", "", "", "",
}

// menuActions assemble le menu complet : libellés statiques, disponibilité
// et motif issus de l'oracle.
func menuActions(d *taskDialog) []actionItem {
	oracle := map[string]TaskAction{}
	for _, ta := range d.actionsCache {
		oracle[ta.Kind] = ta
	}
	static := []string{"Lancer avec un fournisseur…", "Relancer cette tentative…", "Arrêter l'agent…", "Afficher les journaux", "Voir le détail complet", "Réconcilier l'état…", "Fermer", "[s] Soumettre le rapport…", "[v] Voir les gates et les preuves", "[a] Accepter la tâche…", "[l] Lire le rapport…", "[f] Forcer par dérogation…", "[o] Rouvrir cette tâche…", "[g] Charger une gate de validation…", "[h] Hiérarchie et dépendances", "[i] Décisions à traiter", "[j] Rechercher les journaux", "[u] Reprise depuis ma dernière visite", "[e] Consigner une boucle OODA", "[p] Changer le responsable", "[b] Budget et consommation"}
	items := make([]actionItem, len(static))
	for i, label := range static {
		it := actionItem{label: label, enabled: true}
		if kind := actionsOracle[i]; kind != "" {
			if ta, ok := oracle[kind]; ok {
				it.enabled = ta.Disponible
				it.raison = ta.Raison
			}
		}
		items[i] = it
	}
	return items
}

// advisedActionRow choisit la ligne présélectionnée : l'action conseillée par
// l'oracle, sinon la revue pour une tentative terminée, sinon le début.
func advisedActionRow(oracle []TaskAction, completed bool) int {
	for i, kind := range actionsOracle {
		if kind == "" {
			continue
		}
		for _, ta := range oracle {
			if ta.Kind == kind && ta.Conseillee {
				return i
			}
		}
	}
	if completed {
		return 7
	}
	return 0
}
func (s *Store) dialogKey(work string, c *consoleState, key string) {
	d := c.dialog
	if d == nil {
		return
	}
	if key == "help" || (key == "text:?" && (d.mode == "actions" || d.mode == "help")) {
		s.openTerminalHelp(work, c)
		return
	}
	if key == "escape" {
		if d.mode == "help" {
			c.dialog = c.helpParent
			c.helpParent = nil
			return
		}
		c.dialog = nil
		return
	}
	if s.panelKey(work, c, key) {
		return
	}
	switch d.mode {
	case "actions":
		items := menuActions(d)
		if strings.HasPrefix(key, "text:") {
			if row, ok := map[string]int{"s": 7, "v": 8, "a": 9, "l": 10, "r": 1, "f": 11, "o": 12, "g": 13, "h": 14, "i": 15, "j": 16, "u": 17, "e": 18, "p": 19, "b": 20}[strings.ToLower(strings.TrimPrefix(key, "text:"))]; ok {
				if items[row].enabled {
					d.row = row
					key = "enter"
				} else {
					d.message = items[row].raison
					return
				}
			}
		}
		if key == "up" || key == "down" || key == "tab" {
			delta := -1
			if key == "up" {
				delta = 1
			}
			for step := 0; step < len(items); step++ {
				d.row = (d.row + delta + len(items)) % len(items)
				if items[d.row].enabled {
					break
				}
			}
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
			d.row = 5
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
			if d.agent.ModelRoute != nil {
				for i, l := range []string{"auto", "simple", "standard", "exigeant"} {
					if l == d.agent.ModelRoute.Level {
						d.modelLevel = i
					}
				}
			}
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
			d.gateName = ""
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
		// gate-load : ligne 0 = document, ligne 1 = nom de la gate, ligne 2 = bouton.
		// submit : ligne 0 = rapport, ligne 1 = bouton.
		derniere := 1
		if d.mode == "gate-load" {
			derniere = 2
		}
		if key == "tab" || key == "up" || key == "down" {
			delta := -1
			if key == "down" || key == "tab" {
				delta = 1
			}
			d.row = (d.row + delta + derniere + 1) % (derniere + 1)
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
			if d.row != derniere {
				d.row = derniere
				return
			}
			if d.mode == "gate-load" {
				if strings.TrimSpace(d.gateName) == "" {
					d.message = "Nom de la gate requis : nom humain de cette évaluation."
					d.row = 1
					return
				}
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
		if key == "clear" {
			if d.mode == "gate-load" && d.row == 1 {
				d.gateName = ""
			} else {
				d.reportPath = ""
			}
		} else if key == "backspace" {
			if d.mode == "gate-load" && d.row == 1 {
				if len(d.gateName) > 0 {
					_, n := utf8.DecodeLastRuneInString(d.gateName)
					d.gateName = d.gateName[:len(d.gateName)-n]
				}
			} else if len(d.reportPath) > 0 {
				_, n := utf8.DecodeLastRuneInString(d.reportPath)
				d.reportPath = d.reportPath[:len(d.reportPath)-n]
			}
		} else if strings.HasPrefix(key, "text:") {
			t := strings.TrimPrefix(key, "text:")
			if d.mode == "gate-load" && d.row == 1 {
				if len(d.gateName) < 200 {
					d.gateName += t
				}
			} else if d.row == 0 && len(d.reportPath) < 8000 {
				d.reportPath += t
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
			d.row = 5
			if d.mode == "retry" && d.agent != nil && requiresEnvironmentVerification(*d.agent) {
				d.row = 6
			}
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
		rows := 6
		confirmRow := 5
		evidenceRow := -1
		if d.mode == "retry" && d.agent != nil && requiresEnvironmentVerification(*d.agent) {
			rows, confirmRow, evidenceRow = 7, 6, 5
		}
		if key == "tab" || key == "down" {
			d.row = (d.row + 1) % rows
			return
		}
		if key == "up" {
			d.row = (d.row + rows - 1) % rows
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
		if d.row == 4 && (key == "left" || key == "right") {
			delta := 1
			if key == "left" {
				delta = 3
			}
			d.modelLevel = (d.modelLevel + delta) % 4
			return
		}
		if key == "enter" {
			if d.row != confirmRow {
				d.row = (d.row + 1) % rows
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
		if d.row == 2 || d.row == evidenceRow || (d.row == 1 && d.mode == "start") {
			target := &d.instruction
			if d.row == 1 {
				target = &d.workspace
			} else if d.row == evidenceRow {
				target = &d.preconditionEvidence
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

	r := Launch{Level: []string{"auto", "simple", "standard", "exigeant"}[d.modelLevel], ModelPolicyHash: d.modelPolicyHash, Schema: 1, EventID: newID("agent-"), Revision: w.Revision, TaskID: d.task.ID, Role: []string{"worker", "planner", "subplanner"}[d.roleIndex], Provider: d.providers[d.provider], Workspace: d.workspace, Instruction: d.instruction, Capture: c.capture}
	if d.mode == "retry" {
		r.Provider = d.agent.Provider
		r.Role = d.agent.Role
		r.Previous = d.agent.ID
		r.Parent = d.agent.Parent
		r.Workspace = d.agent.CWD
		r.PreconditionEvidence = d.preconditionEvidence
		logs, err := s.logs(d.agent.ID, 0)
		if err != nil {
			return err
		}
		for _, l := range logs {
			if l.Kind == "operator-note" {
				r.Instruction += "\nNote de reprise : " + l.Message
			}
		}
	}
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

// agentLabel présente une tentative sous une forme lisible : fournisseur,
// état observé et identifiant court, pour remplacer l'identifiant brut a-…
// dans les écrans opérateur.
func agentLabel(a *Agent) string {
	if a == nil {
		return "aucune tentative"
	}
	return a.Provider + " · " + uiStatus(observedAgent(*a)) + " · " + shortID(a.ID)
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

// shortID abrège un identifiant long (a-, t-, w-) en gardant un suffixe
// reconnaissable, pour les écrans où l'identifiant complet reste nécessaire.
func shortID(id string) string {
	if len(id) <= 14 {
		return id
	}
	return id[:12] + "…"
}
