//go:build linux

package main

import (
	"fmt"
	"sort"
	"strings"
)

type toolFailure struct {
	name, action, detail, technical string
}

var diagnosticOrder = map[string]int{"configuration": 0, "environment": 1, "tool": 2, "check": 3, "limit": 4, "unknown": 5}

func failureCategory(f toolFailure) string {
	text := strings.ToLower(strings.Join([]string{f.name, f.action, f.detail, f.technical}, " "))
	for _, marker := range []string{"permission denied", "operation not permitted", "read-only file system", "sandbox denied", "sandbox violation", "mount failed", "bwrap: can't bind mount", "unable to apply mount flags", "mount: permission denied", "cifs error", "socket: operation not permitted", "network is unreachable"} {
		if strings.Contains(text, marker) {
			return "environment"
		}
	}
	for _, marker := range []string{"go test", "pytest", "unittest", "npm test", "node --test", "diff --check", "lint", "check_", "check-", "validation"} {
		if strings.Contains(text, marker) {
			return "check"
		}
	}
	for _, marker := range []string{"not configured", "non configur", "configuration", "executable", "exécutable", "command not found", "no such file or directory"} {
		if strings.Contains(text, marker) {
			return "configuration"
		}
	}
	return "tool"
}

func diagnosticTemplate(category string, count int, example string) DiagnosticItem {
	item := DiagnosticItem{Category: category, Count: count, Traces: []string{}}
	suffix := ""
	if count > 1 {
		suffix = fmt.Sprintf(" (%d constats regroupés)", count)
	}
	switch category {
	case "configuration":
		item.Label, item.Cause = "Configuration", "La configuration nécessaire au lancement ou à l’outil est absente, invalide ou indisponible"+suffix+"."
		item.Consequence = "L’étape concernée n’a pas pu démarrer ou s’exécuter ; son résultat n’est pas démontré."
		item.Action, item.ActionKind = "Corriger la configuration indiquée, vérifier qu’elle est relue, puis demander explicitement une nouvelle tentative.", "correct_configuration"
	case "environment":
		item.Label, item.Cause = "Environnement d’exécution", "Les traces signalent un refus d’accès ou une ressource indisponible dans l’environnement"+suffix+"."
		item.Consequence = "La tentative ne peut pas prouver le résultat dans cet environnement ; répéter sans changement de précondition risque le même échec."
		item.Action, item.ActionKind = "Faire vérifier les droits et l’accès aux ressources sur la machine qui exécute l’agent ; reprendre après correction vérifiée.", "verify_environment"
	case "check":
		item.Label, item.Cause = "Contrôle en échec", "Un test ou contrôle observable a échoué"+suffix+"."
		item.Consequence = "Le critère couvert par ce contrôle reste non validé."
		item.Action, item.ActionKind = "Examiner la trace, corriger la cause, puis rejouer le même contrôle avec les mêmes options avant toute validation.", "fix_and_rerun_check"
	case "limit":
		item.Label, item.Cause = "Limite atteinte", example
		item.Consequence = "Le superviseur a interrompu la tentative ; cette interruption ne valide ni le livrable ni la tâche."
		item.Action, item.ActionKind = "Examiner les erreurs précédentes et les préconditions ; reprendre explicitement sans relever arbitrairement les protections.", "review_limit"
	case "unknown":
		item.Label, item.Cause, item.Unknown = "Cause inconnue", example, true
		item.Consequence = "Le résultat de la tentative reste indéterminé ; aucune reprise automatique sûre ne peut être déduite."
		item.Action, item.ActionKind = "Ouvrir les traces conservées et établir la cause avant de choisir une reprise.", "inspect_traces"
	default:
		item.Label, item.Cause = "Outil", "Un outil appelé pendant la tentative a signalé un échec"+suffix+"."
		item.Consequence = "L’opération demandée à cet outil n’est pas démontrée comme terminée."
		item.Action, item.ActionKind = "Examiner la trace et les paramètres de l’outil, corriger la cause, puis demander explicitement la reprise.", "fix_tool_call"
	}
	return item
}

func buildAttemptDiagnostic(agent, attempt string, failures []toolFailure, reason string, limit int) AttemptDiagnostic {
	d := AttemptDiagnostic{AgentID: agent, AttemptID: attempt, ObservedErrors: len(failures), ConsecutiveErrorLimit: limit, Items: []DiagnosticItem{}}
	groups := map[string][]toolFailure{}
	for _, failure := range failures {
		category := failureCategory(failure)
		groups[category] = append(groups[category], failure)
	}
	for category, entries := range groups {
		item := diagnosticTemplate(category, len(entries), "")
		for _, entry := range entries {
			trace := strings.TrimSpace(strings.Join([]string{entry.action, entry.detail, entry.technical}, " · "))
			trace = operationText(trace, 600)
			if trace != "" && len(item.Traces) < 3 {
				item.Traces = append(item.Traces, trace)
			}
		}
		d.Items = append(d.Items, item)
	}
	if reason != "" {
		lower := strings.ToLower(reason)
		if strings.Contains(lower, "limite") || strings.Contains(lower, "délai") || strings.Contains(lower, "budget de temps") {
			d.LimitReached = true
			d.LimitReason = reason
			d.ConsecutiveLimitReached = strings.Contains(lower, "erreurs d'outils consécutives")
			item := diagnosticTemplate("limit", 1, reason+".")
			item.Traces = append(item.Traces, operationText(reason, 600))
			d.Items = append(d.Items, item)
		}
	}
	if len(d.Items) == 0 && reason != "" {
		item := diagnosticTemplate("unknown", 1, "La cause exacte n’est pas disponible dans les événements structurés de cette tentative.")
		item.Traces = append(item.Traces, operationText(reason, 600))
		d.Items = append(d.Items, item)
	}
	sort.SliceStable(d.Items, func(i, j int) bool {
		return diagnosticOrder[d.Items[i].Category] < diagnosticOrder[d.Items[j].Category]
	})
	switch d.ObservedErrors {
	case 0:
		d.Summary = "Aucune erreur d’outil structurée n’a été observée."
	case 1:
		d.Summary = "1 erreur d’outil observée pendant cette tentative."
	default:
		d.Summary = fmt.Sprintf("%d erreurs d’outil observées pendant cette tentative.", d.ObservedErrors)
	}
	if d.ConsecutiveErrorLimit > 0 {
		d.Summary += fmt.Sprintf(" Le plafond configuré est de %d erreurs consécutives", d.ConsecutiveErrorLimit)
		if d.ConsecutiveLimitReached {
			d.Summary += " et il a interrompu la tentative."
		} else {
			d.Summary += " ; ce plafond est distinct du nombre total constaté."
		}
	}
	if d.LimitReached && !d.ConsecutiveLimitReached {
		d.Summary += " Une autre limite d’exécution a interrompu la tentative : " + reason + "."
	}
	return d
}

func fallbackAttemptDiagnostic(a Agent) AttemptDiagnostic {
	if len(a.Diagnostic.Items) > 0 || a.Diagnostic.ObservedErrors > 0 {
		return a.Diagnostic
	}
	if a.Status != "failed" && a.Status != "interrupted" {
		return AttemptDiagnostic{}
	}
	if a.Status == "interrupted" && a.StopKind == originOperator {
		return AttemptDiagnostic{}
	}
	reason := strings.TrimSpace(a.Activity)
	if reason == "" {
		reason = "Tentative terminée sans motif technique enregistré"
	}
	d := buildAttemptDiagnostic(a.ID, a.Attempt, nil, reason, a.Limits.MaxConsecutiveErrors)
	if len(d.Items) == 1 && d.Items[0].Category == "unknown" {
		failure := toolFailure{technical: reason}
		category := failureCategory(failure)
		if category != "tool" {
			d.Items[0] = diagnosticTemplate(category, 1, "")
			d.Items[0].Traces = []string{operationText(reason, 600)}
		}
	}
	return d
}

// requiresEnvironmentVerification ne se fonde que sur le diagnostic structuré
// conservé avec une tentative terminée. Un simple mot dans une consigne ou un
// rapport ne peut donc ni poser ni lever cette retenue.
func requiresEnvironmentVerification(a Agent) bool {
	if a.Status != "failed" && a.Status != "interrupted" {
		return false
	}
	if a.Status == "interrupted" && a.StopKind == originOperator {
		return false
	}
	for _, item := range fallbackAttemptDiagnostic(a).Items {
		if item.Category == "environment" {
			return true
		}
	}
	return false
}
