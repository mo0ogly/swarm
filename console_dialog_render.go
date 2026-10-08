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
	title := uiText("Actions — ") + d.task.ID
	rows := []string{d.task.Title, uiText("État tâche : ") + uiText(uiStatus(d.task.Status))}
	if d.agent == nil {
		rows = append(rows, uiText("Agent : aucune tentative"))
	} else {
		rows = append(rows, uiText("Agent : ")+d.agent.Provider+" · "+uiText(uiStatus(observedAgent(*d.agent))))
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
				cause := readableWrap(uiText("MOTIF : ")+d.agent.Activity, inner)
				if len(cause) > 2 {
					cause = cause[:2]
				}
				rows = append(rows, cause...)
				if d.agent.Progress.ToolCalls > 0 {
					rows = append(rows, fmt.Sprintf(uiText("Budget outils : %d / %d · résultats reçus : %d"), d.agent.Progress.ToolCalls, d.agent.Limits.MaxToolCalls, d.agent.Progress.ToolResults))
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
					line = truncateCells(line+uiText(" — indisponible : ")+a.raison, inner)
				}
				rows = append(rows, m+line)
			}
		case "detail":
			title = uiText("Détails — ") + d.task.ID
			prefix := ""
			if d.agent != nil && !activeAgent(*d.agent) {
				prefix = uiText("MOTIF DE FIN : ") + d.agent.Activity + "\n"
			}
			detail := prefix + uiText("Tâche : ") + d.task.Title + uiText("\nLivrable : ") + d.task.Deliverable + uiText("\nPérimètre : ") + d.scope + uiText("\nCritères : ") + strings.Join(d.task.Criteria, " ; ") + uiText("\nBlocage : ") + d.task.Blocker + uiText("\nProchaine action : ") + d.task.Next + uiText("\nEspace : ") + d.workspace
			if d.agent != nil {
				detail += uiText("\nAction observée : ") + d.agent.Progress.Action + uiText("\nCommande / fichier : ") + d.agent.Progress.Detail
				detail += fmt.Sprintf(uiText("\nOutils : %d appels / %d résultats\nDernier résultat reçu il y a : %s\nSurveillance : %s"), d.agent.Progress.ToolCalls, d.agent.Progress.ToolResults, activityAge(d.agent.Progress.LastResult), monitoringLabel(d.agent.Progress.Degraded))
				detail += uiText("\nAgent : ") + agentLabel(d.agent) + uiText("\nActivité : ") + d.agent.Activity + uiText("\nSignal : ") + d.agent.Heartbeat
			}
			if len(d.history) > 0 {
				detail += uiText("\n\nHISTORIQUE — du plus récent au plus ancien")
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
			title = uiText("Gates et preuves — ") + d.task.ID
			if d.mode == "report" {
				title = uiText("Rapport — ") + d.task.ID
			}
			parts := wrapDialog(d.review, inner)
			n := max(1, height-11)
			d.row = min(d.row, max(0, len(parts)-n))
			rows = append(rows, parts[d.row:min(len(parts), d.row+n)]...)
		case "submit", "gate-load":
			title = uiText("Soumettre le rapport — ") + d.task.ID
			label, button := uiText("Rapport : "), uiText("[ Soumettre pour revue ]")
			if d.mode == "gate-load" {
				title = uiText("Charger une gate — ") + d.task.ID
				label = uiText("Gate : ")
				button = uiText("[ Examiner cette évaluation ]")
			}
			lignes := []string{label + d.reportPath, button}
			if d.mode == "gate-load" {
				nom := d.gateName
				if strings.TrimSpace(nom) == "" {
					nom = uiText("(saisir un nom humain pour cette évaluation)")
				}
				lignes = []string{label + d.reportPath, uiText("Nom de la gate : ") + nom, button}
			}
			for i, line := range lignes {
				mark := "  "
				if d.row == i {
					mark = "> "
				}
				rows = append(rows, mark+truncateCells(line, inner))
			}
			if d.mode == "gate-load" {
				rows = append(rows, uiText("←→ choisir une gate détectée · Tab champ · Ctrl-U effacer"), uiText("Le nom distingue cette évaluation dans la console et l'assistant ; l'enregistrement n'accepte pas la tâche."))
			} else {
				rows = append(rows, uiText("←→ choisir un rapport détecté · Tab champ · Ctrl-U effacer"), uiText("La soumission ne valide pas le travail ; aucun agent n'est lancé."))
			}
		case "gate-confirm":
			title = uiText("Enregistrer la gate — ") + d.task.ID
			parts := wrapDialog(d.review, inner)
			parts = parts[:min(len(parts), max(1, height-14))]
			rows = append(rows, parts...)
			for i, line := range []string{uiText("Confirmer l’enregistrement"), uiText("Annuler")} {
				mark := "  "
				if i == d.row {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
		case "override":
			title = uiText("Forcer par dérogation — ") + d.task.ID
			rows = append(rows, uiText("Acceptation manuelle sans déclarer les contrôles réussis."), uiText("Motif conservé avec date, opérateur local et état précédent."), uiText("Les dépendances doivent être acceptées ; agent arrêté."))
			for i, line := range []string{uiText("Motif : ") + d.reason, uiText("[ Confirmer la dérogation ]"), uiText("Annuler")} {
				mark := "  "
				if d.row == i {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
			rows = append(rows, uiText("Saisir le motif · Entrée puis Entrée confirmer · Échap annuler"))
		case "accept":
			title = uiText("Accepter la tâche — ") + d.task.ID
			rows = append(rows, uiText("Confirmer que le rapport et ses preuves ont été examinés."), uiText("La gate et les dépendances sont revérifiées à la confirmation."), uiText("[f] Forcer par dérogation avec un motif"))
			if d.task.Status != "submitted" {
				rows = append(rows, uiText("INDISPONIBLE : soumettre le rapport pour revue d’abord."))
			} else if d.task.Gate == nil {
				rows = append(rows, uiText("INDISPONIBLE : aucune gate enregistrée."))
			} else if !d.task.Gate.Evaluation.Allowed {
				rows = append(rows, uiText("INDISPONIBLE : gate échouée ; voir les contrôles et preuves."))
			}
			for i, line := range []string{uiText("Confirmer l’acceptation"), uiText("Annuler")} {
				mark := "  "
				if d.row == i {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
		case "launch-error":
			title = uiText("LANCEMENT REFUSÉ — ") + d.task.ID
			rows = append(rows, wrapDialog(d.message, inner)...)
			rows = append(rows, "", uiText("[ Entrée : revenir au formulaire ]"), uiText("[v] Voir les gates de la tâche ou dépendance bloquante"), uiText("[o] Rouvrir la tâche sélectionnée"), uiText("Échap : retourner au tableau"))
		case "reopen":
			title = uiText("Rouvrir — ") + d.task.ID
			rows = append(rows, uiText("La validation courante sera retirée ; l’historique reste conservé."))
			for i, line := range []string{uiText("Confirmer la réouverture"), uiText("Annuler")} {
				mark := "  "
				if i == d.row {
					mark = "> "
				}
				rows = append(rows, mark+line)
			}
		case "start", "retry":
			rows = append(rows, uiText("Périmètre : ")+d.scope)
			title = uiText("Lancer — ") + d.task.ID
			if d.mode == "retry" {
				title = uiText("Relancer — ") + d.task.ID
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
			form := []string{uiText("Fournisseur : ") + provider, uiText("Espace : ") + d.workspace, uiText("Consigne : ") + d.instruction, uiText("Rôle : ") + role, uiText("Niveau ◀ ▶ : ") + level + " · " + model}
			if d.mode == "retry" && d.agent != nil && requiresEnvironmentVerification(*d.agent) {
				form = append(form, uiText("Vérification nouvelle : ")+d.preconditionEvidence)
			}
			form = append(form, uiText("[ Confirmer le lancement ]"))
			for i, line := range form {
				m := "  "
				if i == d.row {
					m = "> "
				}
				if i == d.row && len([]rune(line)) > inner-2 && (i == 1 || i == 2 || strings.HasPrefix(line, uiText("Vérification nouvelle : "))) {
					prefix := uiText("Espace : ")
					if i == 2 {
						prefix = uiText("Consigne : ")
					} else if strings.HasPrefix(line, uiText("Vérification nouvelle : ")) {
						prefix = uiText("Vérification nouvelle : ")
					}
					r := []rune(line)
					n := max(1, inner-len([]rune(prefix))-4)
					line = prefix + "…" + string(r[max(0, len(r)-n):])
				}
				rows = append(rows, m+line)
			}
			rows = append(rows, uiText("Tab : champ · ←→ : fournisseur/rôle/niveau · Ctrl-U : effacer"), uiText("Après un échec d’environnement, décrire une vérification observée est obligatoire. Entrée sur Confirmer lance réellement le fournisseur."))
		case "resume-launch":
			title = uiText("Reprendre le lancement préparé")
			if d.prepared != nil {
				rows = append(rows, uiText("Fournisseur : ")+d.prepared.Provider, uiText("Espace : ")+d.prepared.Workspace, uiText("Consigne : ")+d.prepared.Instruction)
			}
			rows = append(rows, uiText("La copie et les réglages sont conservés. Les conditions sont vérifiées à nouveau."))
			for i, label := range []string{uiText("Reprendre le lancement préparé"), uiText("Annuler")} {
				mark := "  "
				if i == d.row {
					mark = "> "
				}
				rows = append(rows, mark+label)
			}
		case "stop", "reconcile":
			title = uiText("Confirmer — ") + d.task.ID
			text := uiText("Demander l'arrêt de l'agent sélectionné ?")
			if d.mode == "reconcile" {
				text = uiText("Vérifier les processus et réconcilier l'état ?")
			}
			rows = append(rows, text)
			for i, a := range []string{uiText("Confirmer"), uiText("Annuler")} {
				m := "  "
				if i == d.row {
					m = "> "
				}
				rows = append(rows, m+a)
			}
		}
	}
	hint := uiText("↑↓ choisir · Entrée confirmer · Échap annuler")
	if d.mode == "actions" {
		hint = uiText("↑↓ choisir · Entrée ouvrir · Échap fermer")
	}
	if d.mode == "detail" || d.mode == "review" || d.mode == "report" {
		hint = uiText("↑↓ défiler · ←→ rapports · Entrée actions · Échap retour")
	}
	if d.mode != "launch-error" {
		rows = append(rows, wrapDialog(d.message, inner)...)
	}
	if d.mode == "help" {
		hint = uiText("↑↓ défiler · Entrée/Échap retour · F1 fermer l’aide")
	} else {
		hint += uiText(" · F1 aide")
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
