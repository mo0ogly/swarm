//go:build linux

package main

import (
	"fmt"
	"strings"
)

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
	if d.mode == "help" {
		rows = []string{d.task.Title, ""}
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
			items := menuActions(d)
			first := max(0, d.row-visible+1)
			for i := first; i < min(len(items), first+visible); i++ {
				a := items[i]
				m := "  "
				if i == d.row {
					m = "> "
				}
				line := a.label
				if !a.enabled {
					line = truncateCells(line+" — indisponible : "+a.raison, inner)
				}
				rows = append(rows, m+line)
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
				detail += "\nAgent : " + agentLabel(d.agent) + "\nActivité : " + d.agent.Activity + "\nSignal : " + d.agent.Heartbeat
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
			lignes := []string{label + d.reportPath, button}
			if d.mode == "gate-load" {
				nom := d.gateName
				if strings.TrimSpace(nom) == "" {
					nom = "(saisir un nom humain pour cette évaluation)"
				}
				lignes = []string{label + d.reportPath, "Nom de la gate : " + nom, button}
			}
			for i, line := range lignes {
				mark := "  "
				if d.row == i {
					mark = "> "
				}
				rows = append(rows, mark+truncateCells(line, inner))
			}
			if d.mode == "gate-load" {
				rows = append(rows, "←→ choisir une gate détectée · Tab champ · Ctrl-U effacer", "Le nom distingue cette évaluation dans la console et l'assistant ; l'enregistrement n'accepte pas la tâche.")
			} else {
				rows = append(rows, "←→ choisir un rapport détecté · Tab champ · Ctrl-U effacer", "La soumission ne valide pas le travail ; aucun agent n'est lancé.")
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
			level := []string{"auto", "simple", "standard", "exigeant"}[d.modelLevel]
			model := d.modelDescription
			form := []string{"Fournisseur : " + provider, "Espace : " + d.workspace, "Consigne : " + d.instruction, "Rôle : " + role, "Niveau ◀ ▶ : " + level + " · " + model}
			if d.mode == "retry" && d.agent != nil && requiresEnvironmentVerification(*d.agent) {
				form = append(form, "Vérification nouvelle : "+d.preconditionEvidence)
			}
			form = append(form, "[ Confirmer le lancement ]")
			for i, line := range form {
				m := "  "
				if i == d.row {
					m = "> "
				}
				if i == d.row && len([]rune(line)) > inner-2 && (i == 1 || i == 2 || strings.HasPrefix(line, "Vérification nouvelle : ")) {
					prefix := "Espace : "
					if i == 2 {
						prefix = "Consigne : "
					} else if strings.HasPrefix(line, "Vérification nouvelle : ") {
						prefix = "Vérification nouvelle : "
					}
					r := []rune(line)
					n := max(1, inner-len([]rune(prefix))-4)
					line = prefix + "…" + string(r[max(0, len(r)-n):])
				}
				rows = append(rows, m+line)
			}
			rows = append(rows, "Tab : champ · ←→ : fournisseur/rôle/niveau · Ctrl-U : effacer", "Après un échec d’environnement, décrire une vérification observée est obligatoire. Entrée sur Confirmer lance réellement le fournisseur.")
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
	hint := "↑↓ choisir · Entrée confirmer · Échap annuler"
	if d.mode == "actions" {
		hint = "↑↓ choisir · Entrée ouvrir · Échap fermer"
	}
	if d.mode == "detail" || d.mode == "review" || d.mode == "report" {
		hint = "↑↓ défiler · ←→ rapports · Entrée actions · Échap retour"
	}
	if d.mode != "launch-error" {
		rows = append(rows, wrapDialog(d.message, inner)...)
	}
	if d.mode == "help" {
		hint = "↑↓ défiler · Entrée/Échap retour · F1 fermer l’aide"
	} else {
		hint += " · F1 aide"
	}
	rows = append(rows, hint)
	if len(rows) > max(1, height-4) {
		rows = append(rows[:max(0, height-5)], hint)
	}
	heading := truncateCells(" "+title+" ", boxWidth-2)
	box := []string{"┌" + heading + strings.Repeat("─", max(0, boxWidth-2-textWidth(heading))) + "┐"}
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
