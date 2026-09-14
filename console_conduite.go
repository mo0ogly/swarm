//go:build linux

package main

import (
	"fmt"
	"strings"
)

// Équivalent texte du mode Conduite : l'état du travail, ce qui attend une
// décision, et le plan avec l'action courante de chaque agent. Les journaux,
// l'administration et les outils restent joignables en mode expert.
func (s *Store) renderConduite(work string, c *consoleState, width, height int, input string) string {
	w, e := s.get(work)
	if e != nil {
		return clip(e.Error(), width)
	}
	agents, _ := s.agents(work)
	busy := 0
	for _, a := range agents {
		if activeAgent(a) {
			busy++
		}
	}
	departs := "départs autorisés"
	if s.paused(work) {
		departs = "DÉPARTS SUSPENDUS"
	}
	view, _ := s.budget(work)
	lines := []string{
		fmt.Sprintf("SWARM  │ %s  │ conduite", w.Title), "",
		fmt.Sprintf("%s · %d créneau(x) occupé(s) sur %d · %s", autonomyLabel(s.autonomy(work)), busy, s.slots(work), departs),
		fmt.Sprintf("Budget estimé : %.2f réservé + %.2f imputé sur %.2f USD ; estimation, jamais une dépense facturée", view.Reserved, view.Estimated, view.Budget.Limit),
		"",
	}

	waiting := []string{}
	decisions, e := s.decisions(work)
	if e != nil {
		waiting = append(waiting, "Escalades indisponibles : "+e.Error())
	}
	for _, d := range decisions {
		if d.ResolvedAt != "" {
			continue
		}
		waiting = append(waiting, fmt.Sprintf("%s · %s · %s", escalationSubject(d.TaskID), escalationLabel(d.Kind), d.Summary))
	}
	if len(waiting) == 0 {
		waiting = append(waiting, "Rien à traiter. Les agents avancent ; vous serez sollicité pour une décision, pas pour un relais.")
	}

	plan := []string{}
	for _, t := range w.Tasks {
		row := fmt.Sprintf("%s · %s · %s", t.ID, uiStatus(t.Status), t.Title)
		plan = append(plan, row)
		for _, a := range agents {
			if a.TaskID != t.ID || !activeAgent(a) {
				continue
			}
			detail := a.Progress.Detail
			if detail == "" {
				detail = a.Progress.Action
			}
			if detail == "" {
				detail = a.Activity
			}
			plan = append(plan, fmt.Sprintf("   %s · %s · %d appels · dernier résultat %s", a.Provider, detail, a.Progress.ToolCalls, activityAge(a.Progress.LastResult)))
		}
		if t.Status == "todo" && len(t.Depends) > 0 {
			plan = append(plan, "   en attente de : "+strings.Join(t.Depends, ", "))
		}
		if t.Status == "blocked" && t.Blocker != "" {
			plan = append(plan, "   "+t.Blocker)
		}
	}
	if len(plan) == 0 {
		plan = append(plan, "Aucune tâche. Passer en mode expert pour en ajouter.")
	}

	reste := height - len(lines) - 5
	hauteurAttente := max(3, min(len(waiting)+2, reste/2))
	lines = append(lines, panel("À TRAITER", waiting, width, hauteurAttente, false)...)
	lines = append(lines, panel("PLAN ET AGENTS", plan, width, max(3, reste-hauteurAttente), c.focus == 0)...)
	lines = append(lines,
		"mode : commande « mode » pour l'affichage expert · Entrée actions · ? aide · q",
		strings.ReplaceAll(c.message, "\n", " | "), "swarm> "+input)
	for i := range lines {
		lines[i] = clip(lines[i], width-1)
	}
	return strings.Join(lines, "\r\n")
}
