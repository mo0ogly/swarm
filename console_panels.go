//go:build linux

package main

import (
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"unicode/utf8"
)

func editPanelText(value *string, key string) {
	if key == "clear" {
		*value = ""
	} else if key == "backspace" && len(*value) > 0 {
		_, n := utf8.DecodeLastRuneInString(*value)
		*value = (*value)[:len(*value)-n]
	} else if strings.HasPrefix(key, "text:") && len(*value) < 8000 {
		*value += strings.TrimPrefix(key, "text:")
	}
}
func (s *Store) openPanel(work string, c *consoleState, mode string) {
	d := c.dialog
	d.mode = mode
	d.row = 0
	d.message = ""
	switch mode {
	case "hierarchy":
		d.review = s.hierarchyText(work)
	case "resume":
		d.review = s.resumeSinceText(work, c.visit)
	case "decisions":
		var e error
		d.decisions, e = s.decisions(work)
		if e != nil {
			d.message = e.Error()
		}
	case "log-query":
		d.search = ""
		d.logCursor = 0
		d.logBack = nil
		d.logPage = LogPage{}
		d.row = 0
	case "ooda":
		d.fields = []string{"", "", "", "", ""}
	case "budget":
		v, e := s.budget(work)
		if e != nil {
			d.message = e.Error()
		}
		d.fields = []string{fmt.Sprint(v.Budget.Limit), fmt.Sprint(v.Budget.Reserve), v.Budget.Source, v.Budget.PriceDate}
		d.review = fmt.Sprintf("Réservé %.2f USD · imputé estimé %.2f USD · restant %.2f USD\n%s", v.Reserved, v.Estimated, v.Remaining, v.Policy)
	case "assign":
		d.fields = []string{d.task.Owner}
	}
}
func (s *Store) panelKey(work string, c *consoleState, key string) bool {
	d := c.dialog
	switch d.mode {
	case "help", "hierarchy", "resume":
		if key == "up" {
			d.row = max(0, d.row-1)
		}
		if key == "down" {
			d.row++
		}
		if key == "enter" {
			if d.mode == "help" {
				c.dialog = c.helpParent
				c.helpParent = nil
				return true
			}
			d.mode = "actions"
			d.row = 0
		}
		return true
	case "decisions":
		if key == "up" {
			d.row = max(0, d.row-1)
		}
		if key == "down" {
			d.row = min(max(0, len(d.decisions)-1), d.row+1)
		}
		if key == "enter" && len(d.decisions) > 0 {
			d.decision = d.decisions[d.row]
			d.mode = "decision"
			d.fields = []string{""}
			d.row = 0
		}
		return true
	case "decision":
		if key == "text:v" && d.row == 1 {
			w, e := s.get(work)
			if e == nil {
				t, e := w.task(d.decision.TaskID)
				if e == nil {
					d.task = *t
					d.mode = "review"
					d.row = 0
					d.review = s.reviewText(work, d)
				}
			}
			return true
		}
		if key == "tab" || key == "down" || key == "up" {
			d.row = 1 - d.row
			return true
		}
		if key == "enter" {
			if d.row == 0 {
				d.row = 1
				return true
			}
			if e := s.resolveDecision(work, d.decision.ID, operatorIdentity(), d.fields[0]); e != nil {
				d.message = e.Error()
				return true
			}
			s.openPanel(work, c, "decisions")
			d.message = "Décision enregistrée ; aucune acceptation ni commande d’arrêt implicite."
			return true
		}
		if d.row == 0 {
			editPanelText(&d.fields[0], key)
		}
		return true
	case "assign", "ooda", "budget":
		n := len(d.fields)
		if key == "tab" || key == "down" {
			d.row = (d.row + 1) % (n + 1)
			return true
		}
		if key == "up" {
			d.row = (d.row + n) % (n + 1)
			return true
		}
		if key == "enter" {
			if d.row < n {
				d.row++
				return true
			}
			var e error
			if d.mode == "budget" {
				limit, err := strconv.ParseFloat(d.fields[0], 64)
				reserve, err2 := strconv.ParseFloat(d.fields[1], 64)
				if err != nil || err2 != nil {
					d.message = "Montants numériques requis."
					return true
				}
				e = s.setBudget(work, Budget{Limit: limit, Reserve: reserve, Source: d.fields[2], PriceDate: d.fields[3]})
			} else if d.mode == "assign" {
				e = s.operatorTask(work, Request{ID: d.task.ID, Owner: d.fields[0]})
			} else {
				w, err := s.get(work)
				if err != nil {
					d.message = err.Error()
					return true
				}
				_, e = s.executeRequest(work, "ooda", Request{Schema: 1, EventID: newID("operator-"), Revision: w.Revision, Observation: d.fields[0], Orientation: d.fields[1], Decision: d.fields[2], Result: d.fields[3], Next: d.fields[4], Owner: operatorIdentity()})
			}
			if e != nil {
				d.message = e.Error()
				return true
			}
			c.message = "Enregistré : " + d.mode
			c.dialog = nil
			return true
		}
		if d.row < n {
			editPanelText(&d.fields[d.row], key)
		}
		return true
	case "log-query":
		if d.agent == nil {
			d.message = "Aucune tentative à consulter."
			return true
		}
		if key == "enter" {
			d.logCursor = 0
			d.logBack = nil
			d.row = 1
		}
		if d.row == 0 && key != "enter" {
			editPanelText(&d.search, key)
			return true
		}
		if key == "text:/" {
			d.row = 0
			return true
		}
		if key == "right" && d.logPage.More {
			d.logBack = append(d.logBack, d.logCursor)
			d.logCursor = d.logPage.Next
		}
		if key == "left" && len(d.logBack) > 0 {
			d.logCursor = d.logBack[len(d.logBack)-1]
			d.logBack = d.logBack[:len(d.logBack)-1]
		}
		if key == "text:f" {
			d.logFollow = !d.logFollow
		}
		if key == "text:e" {
			path := filepath.Join(s.root, ".swarm", newID("logs-")+".json")
			if e := s.exportLogPage(work, d.agent.ID, d.search, path); e != nil {
				d.message = e.Error()
			} else {
				d.message = "Export borné (200 lignes) : " + path
			}
			return true
		}
		p, e := s.queryLogs(work, d.agent.ID, d.search, "", d.logCursor, max(1, d.logLimit))
		if e != nil {
			d.message = e.Error()
		} else {
			d.logPage = p
		}
		return true
	}
	return false
}
func panelRows(d *taskDialog, inner, height int) (string, []string, bool) {
	rows := []string{}
	switch d.mode {
	case "help", "hierarchy", "resume":
		parts := wrapDialog(d.review, inner)
		if d.mode == "help" {
			parts = nil
			for _, line := range strings.Split(d.review, "\n") {
				if line == "" {
					parts = append(parts, "")
				} else {
					parts = append(parts, readableWrap(line, inner)...)
				}
			}
		}
		n := max(1, height-12)
		d.row = min(d.row, max(0, len(parts)-n))
		rows = append(rows, parts[d.row:min(len(parts), d.row+n)]...)
		if d.mode == "help" {
			return "AIDE · PARCOURS ET RACCOURCIS", rows, true
		}
		return strings.ToUpper(d.mode), rows, true
	case "decisions":
		rows = append(rows, "Acquitter une décision ne valide pas la tâche et n’arrête aucun agent.")
		n := max(1, height-13)
		first := max(0, d.row-n+1)
		for i := first; i < min(len(d.decisions), first+n); i++ {
			v := d.decisions[i]
			mark := "  "
			if i == d.row {
				mark = "> "
			}
			state := "À traiter"
			if v.ResolvedAt != "" {
				state = "Acquittée"
			}
			rows = append(rows, mark+state+" · "+v.TaskID+" · "+v.Kind+" · "+v.Summary)
		}
		if len(d.decisions) == 0 {
			rows = append(rows, "Aucune décision en attente détectée.")
		}
		return "DÉCISIONS", rows, true
	case "decision":
		rows = append(rows, wrapDialog(d.decision.Summary+"\nPreuves : "+d.decision.Evidence+"\nAuteur précédent : "+d.decision.Author+"\nRésolution : "+d.decision.Resolution, inner)...)
		for i, line := range []string{"Décision motivée : " + d.fields[0], "[ Acquitter sans modifier la tâche ]"} {
			mark := "  "
			if d.row == i {
				mark = "> "
			}
			rows = append(rows, mark+line)
		}
		rows = append(rows, "Tab champ · [v] sur le bouton : examiner la tâche")
		return "EXAMINER LA DÉCISION", rows, true
	case "assign", "ooda", "budget":
		labels := []string{"Responsable"}
		if d.mode == "budget" {
			labels = []string{"Plafond USD (0 désactive)", "Réservation USD par départ", "Source estimation", "Date de référence AAAA-MM-JJ"}
			rows = append(rows, wrapDialog(d.review, inner)...)
		}
		if d.mode == "ooda" {
			labels = []string{"Observation", "Orientation", "Décision", "Résultat", "Prochaine action"}
		}
		for i, label := range labels {
			mark := "  "
			if d.row == i {
				mark = "> "
			}
			rows = append(rows, mark+label+" : "+d.fields[i])
		}
		mark := "  "
		if d.row == len(labels) {
			mark = "> "
		}
		rows = append(rows, mark+"[ Enregistrer ]", "Tab champ · Ctrl-U effacer · Échap annuler")
		return strings.ToUpper(d.mode), rows, true
	case "log-query":
		d.logLimit = max(1, height-17)
		rows = append(rows, "Recherche : "+d.search, fmt.Sprintf("Curseur %d → %d · début conservé %d · suite %t", d.logCursor, d.logPage.Next, d.logPage.RetainedFrom, d.logPage.More))
		if d.logPage.Gap {
			rows = append(rows, "RÉTENTION : une partie de l’historique n’est plus disponible.")
		}
		for _, l := range d.logPage.Entries {
			rows = append(rows, clip(fmt.Sprintf("#%d %s %s · %s", l.Seq, l.At, l.Kind, l.Message), inner))
		}
		rows = rows[:min(len(rows), max(1, height-13))]
		follow := "pause"
		if d.logFollow {
			follow = "direct"
		}
		rows = append(rows, "Entrée rechercher · / modifier · ←→ pages · f "+follow+" · e exporter")
		return "JOURNAUX", rows, true
	}
	return "", nil, false
}
